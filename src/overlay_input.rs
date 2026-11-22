//! What a frame's input does to the open overlays, once they are drawn.
//!
//! This is the glue between egui's input and focus state and the pure rules
//! of [`dismissals`]: it reads the frame into a [`Layer`] per attached
//! overlay, runs the `on_dismiss` of the ones the rules name, moves focus
//! into overlays that just opened, and applies the keyboard pattern of
//! overlays with a [`Nav`]. Closing an overlay gives focus back in
//! [`focus::restore`], at the next layout, when the overlay is gone.

use std::collections::HashMap;

use egui::{Context, Event, Id, Key, Modifiers, Pos2, Rect};

use crate::actions::{self, Action};
use crate::dismiss::{
    Boundary, Cause, Closer, FocusAt, Input, Layer, Seen, Trigger, WindowFocus, dismissals,
    esc_target,
};
use crate::focus::{self, PointerOn};
use crate::kept::Handle;
use crate::nav::{self, Menu};
use crate::nodes::Nodes;
use crate::overlay::{Anchor, AnchorLookup, Anchored, Behavior, Focus, Nav, Outside, Showing};
use crate::pipeline::{DismissFn, PlacedNode};
use crate::relation::Relation;
use crate::roots::{
    Form, Here, PlacedOverlay, PlacedRoot, ancestors, root_and_node, root_at, stacked_top_down,
};
use crate::shell::Shell;
use crate::stack::Stack;

/// Whether an in-app drag is live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Drag {
    Live,
    Idle,
}

/// Where the pointer went down this frame, if it did.
pub(crate) fn press_origin(ctx: &Context) -> Option<Pos2> {
    ctx.input(|input| {
        input
            .pointer
            .any_pressed()
            .then(|| input.pointer.press_origin())
            .flatten()
    })
}

/// Whether the pointer is on an overlay that leaves focus on its anchor.
pub(crate) fn pointer_on(
    ctx: &Context,
    nodes: &Nodes,
    roots: &[PlacedRoot],
    here: Here,
) -> PointerOn {
    let Some(at) = ctx.input(|input| input.pointer.interact_pos()) else {
        return PointerOn::Elsewhere;
    };
    let root = root_at(roots, nodes, at, here);
    let on_it = root_and_node(roots, nodes, root).is_some_and(|(root, node)| {
        root.overlay().is_some_and(|overlay| {
            overlay.behavior.focus == Focus::Stay && node.rect().contains(at)
        })
    });
    if on_it {
        PointerOn::StayingOverlay
    } else {
        PointerOn::Elsewhere
    }
}

/// An attached overlay.
struct Open<'a> {
    root: usize,
    overlay: &'a PlacedOverlay,
    /// The ids of the overlays nested in it.
    nested: Vec<Id>,
}

/// Whether a menu that swallows Tab was open at the end of the last pass: the
/// one context-wide piece of eggshell state (D11a), because the pass's focus
/// move is made in egui's `begin_pass`, before any layout, and a layout
/// drawn earlier in the pass (a header, or an app's own) would act on it
/// before this one's keys are claimed. Every layout sets it; the plugin reads
/// and clears it at the start of the next pass.
fn tab_held_id(ctx: &Context) -> Id {
    crate::key::viewport_id(ctx, "tab-held")
}

fn note_tab_held(ctx: &Context) {
    let key = tab_held_id(ctx);
    ctx.data_mut(|data| data.insert_temp(key, true));
}

/// Cancels Tab at the start of the pass while a menu is open: egui has set
/// the pass's focus direction but nothing has acted on it, so no widget
/// gains focus, scrolls into view or tells assistive technology.
struct HoldTab;

impl egui::Plugin for HoldTab {
    fn debug_name(&self) -> &'static str {
        "eggshell-ui hold tab"
    }

    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let key = tab_held_id(ctx);
        let held = ctx
            .data_mut(|data| data.remove_temp::<bool>(key))
            .unwrap_or(false);
        if !held {
            return;
        }
        let tabbed = ctx.input_mut(|input| {
            let plain = input.consume_key(Modifiers::NONE, Key::Tab);
            let shifted = input.consume_key(Modifiers::SHIFT, Key::Tab);
            plain || shifted
        });
        if tabbed {
            ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
        }
    }
}

fn register_hold_tab(ctx: &Context) {
    if ctx.plugin_opt::<HoldTab>().is_none() {
        ctx.add_plugin(HoldTab);
    }
}

impl Open<'_> {
    /// Whether the overlay is a menu, which swallows Tab as Radix's does:
    /// nothing moves until Esc, a press outside or a choice.
    fn holds_tab(&self) -> bool {
        self.overlay.behavior.outside() == Outside::Swallow || self.overlay.behavior.nav.is_menu()
    }

    fn anchor_node(&self) -> Option<Id> {
        match self.overlay.anchor_decl {
            Anchor::Node(id) => Some(id),
            _ => None,
        }
    }
}

fn is_nested_in(roots: &[PlacedRoot], root: usize, ancestor: usize) -> bool {
    ancestors(roots, root).any(|at| at == ancestor)
}

/// The attached overlays, the one on the bottom first. An overlay that is
/// leaving is not among them: it takes no key and is not judged again.
fn open_overlays(roots: &[PlacedRoot], here: Here) -> Vec<Open<'_>> {
    overlays_where(roots, here, |overlay| {
        overlay.anchored == Anchored::Attached || overlay.lookup == AnchorLookup::Missing
    })
}

/// The overlays dismissal judges: the attached ones, and those hidden because
/// their anchor left the window, the one on the bottom first.
fn judged_overlays(roots: &[PlacedRoot], here: Here) -> Vec<Open<'_>> {
    overlays_where(roots, here, |_| true)
}

fn overlays_where(
    roots: &[PlacedRoot],
    here: Here,
    keep: impl Fn(&PlacedOverlay) -> bool,
) -> Vec<Open<'_>> {
    let order: Vec<(usize, &PlacedOverlay)> = stacked_top_down(roots, here)
        .into_iter()
        .rev()
        .filter_map(|root| Some((root, roots.get(root)?.overlay()?)))
        .filter(|(_, overlay)| !overlay.staged.is_exiting() && keep(overlay))
        .collect();
    order
        .iter()
        .map(|&(root, overlay)| Open {
            root,
            overlay,
            nested: order
                .iter()
                .filter(|&&(other, _)| is_nested_in(roots, other, root))
                .map(|(_, other)| other.id)
                .collect(),
        })
        .collect()
}

/// The root of the topmost open overlay, or the page's.
pub(crate) fn topmost_root(roots: &[PlacedRoot], here: Here) -> usize {
    open_overlays(roots, here)
        .last()
        .map_or(here.root, |open| open.root)
}

/// The box of the overlay at `root`: the root's rect, or for a modal, whose
/// root is the scrim, what its content takes of it.
fn box_of(nodes: &Nodes, roots: &[PlacedRoot], root: usize) -> egui::Rect {
    let Some((root, node)) = root_and_node(roots, nodes, root) else {
        return egui::Rect::NOTHING;
    };
    match root.overlay().map(|overlay| overlay.form) {
        Some(Form::Modal(_)) => nodes
            .children(root.first)
            .filter_map(|child| nodes.node(child))
            .fold(egui::Rect::NOTHING, |rect, child| rect.union(child.rect())),
        _ => node.rect(),
    }
}

/// What an open overlay's past frames left for dismissal to judge it by.
#[derive(Default)]
struct Remembered {
    focus_before: FocusAt,
    focus_now: FocusAt,
    window_focus: WindowFocus,
}

/// The layers dismissal judges; `remembered` says, for each, where focus was
/// and is relative to it, and whether the window has had the focus.
fn layers_of(
    ctx: &Context,
    (open, closers): (&[Open<'_>], &[Id]),
    nodes: &Nodes,
    roots: &[PlacedRoot],
    mut remembered: impl FnMut(&Open<'_>) -> Remembered,
) -> Vec<Layer> {
    let position = |root: usize| open.iter().position(|open| open.root == root);
    open.iter()
        .map(|open| {
            let overlay = open.overlay;
            let Remembered {
                focus_before,
                focus_now,
                window_focus,
            } = remembered(open);
            Layer {
                parent: roots
                    .get(open.root)
                    .and_then(|root| root.parent)
                    .and_then(position),
                rect: box_of(nodes, roots, open.root),
                boundary: match overlay.form {
                    Form::Modal(_) => Boundary::Scrim,
                    Form::Floating(_) => Boundary::Open,
                },
                cancel: roots
                    .get(open.root)
                    .and_then(|root| actions::button_in(nodes, root.first, Action::Cancel)),
                trigger: trigger_of(ctx, overlay),
                band: overlay.band,
                dismiss_on: overlay.behavior.dismiss_on,
                outside: overlay.behavior.outside(),
                opening: overlay.opening,
                focus_before,
                lookup: overlay.lookup,
                focus_now,
                seen: match (overlay.anchored, overlay.showing) {
                    (Anchored::Attached, Showing::Drawn) => Seen::Drawn,
                    (Anchored::Detached, _) | (_, Showing::Measuring) => Seen::Hidden,
                },
                window_focus,
                closer: closer_of(nodes, roots.get(open.root), closers),
            }
        })
        .collect()
}

/// Whether one of `closers` is a button of the overlay whose root is `root`,
/// not of one nested in it.
fn closer_of(nodes: &Nodes, root: Option<&PlacedRoot>, closers: &[Id]) -> Closer {
    let pressed = root.is_some_and(|root| {
        nodes
            .subtree(root.first)
            .iter()
            .filter_map(PlacedNode::id)
            .any(|id| closers.contains(&id))
    });
    if pressed {
        Closer::Pressed
    } else {
        Closer::Idle
    }
}

/// The overlay's trigger as dismissal judges a press on it. A trigger that
/// senses is pressed when egui says it is, since egui gives a press within its
/// reach to the closest widget and the click goes there; one that does not is
/// judged by where it is.
fn trigger_of(ctx: &Context, overlay: &PlacedOverlay) -> Option<Trigger> {
    match overlay.anchor_decl {
        Anchor::Node(id) => Some(match ctx.read_response(id) {
            Some(response) if response.is_pointer_button_down_on() => Trigger::Pressed,
            Some(response) if response.sense.interactive() => Trigger::Unpressed,
            _ => Trigger::Rect(overlay.anchor),
        }),
        Anchor::Rect(_) => Some(Trigger::Rect(overlay.anchor)),
        _ => None,
    }
}

/// Where Esc goes, when it is pressed.
enum EscapeGoes {
    /// To an overlay: its dismissal, or its cancel button.
    Overlay,
    /// To the window's cancel button.
    Window(Id),
    /// To nobody: the application's own.
    Nobody,
}

/// The one place Esc's destination is decided: the topmost overlay that takes
/// it (`dismissals` then dismisses it or presses its cancel button), else the
/// window's cancel button.
fn escape_goes(layers: &[Layer], nodes: &Nodes, input: &Input) -> EscapeGoes {
    if !input.escape || input.drag_live {
        return EscapeGoes::Nobody;
    }
    if esc_target(layers).is_some() {
        return EscapeGoes::Overlay;
    }
    actions::button_in(nodes, 0, Action::Cancel).map_or(EscapeGoes::Nobody, EscapeGoes::Window)
}

/// Whether a text input of this layout had focus at the end of the last pass,
/// which is what tells an Esc that only unfocused it from one that is the
/// application's: egui drops the field's focus before the frame is built.
/// Another layout's field does not count, or the first layout drawn would take
/// the Esc from the one that has the field.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Typing {
    #[default]
    No,
    Yes,
}

impl Typing {
    /// Notes whether a text input among `subtrees` has focus now. Run at the
    /// end of a pass.
    pub(crate) fn note(ctx: &Context, kept: &Handle, subtrees: &[(Id, Box<[Id]>)]) {
        let own = ctx
            .memory(|memory| memory.focused())
            .is_some_and(|focused| {
                subtrees
                    .iter()
                    .any(|(_, widgets)| widgets.contains(&focused))
            });
        let now = if own && ctx.text_edit_focused() {
            Self::Yes
        } else {
            Self::No
        };
        kept.viewport(ctx.viewport_id(), |here| here.typing = now);
    }

    /// Whether the Esc of this pass was the one that took a text input's
    /// focus: it had focus at the end of the last pass and nothing has now.
    fn escape_blurred(ctx: &Context, kept: &Handle) -> bool {
        let before = kept.viewport(ctx.viewport_id(), |here| here.typing);
        before == Self::Yes && ctx.memory(|memory| memory.focused()).is_none()
    }
}

/// Why a text field lost focus, as `when_blurred` is told. It is known the
/// frame the blur is reported: Esc and Enter are told from the same input
/// that decides who owns them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blur {
    /// Esc took the focus: the user means to abandon the edit.
    Escaped,
    /// Enter took the focus: the user means to commit the edit.
    Submitted,
    /// Anything else: a click elsewhere, Tab, a programmatic focus change,
    /// the window losing focus.
    Elsewhere,
}

/// The keys the frame pressed that the library answers, decided from the
/// layout before any draw closure runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Keys {
    /// Whether Esc was pressed, even if it was then taken.
    pub(crate) escape: bool,
    /// Whether Tab was pressed, even if a menu then took it.
    pub(crate) tab: bool,
    /// The default button Return goes to.
    pub(crate) default: Option<Id>,
    /// Why a text field that loses focus in this pass does.
    pub(crate) blur: Blur,
}

/// Decides who Esc and Return belong to and takes them from the frame's
/// input, so that no draw closure (a custom canvas under a menu) sees a key
/// that dismissed an overlay or pressed a button: it belongs to the overlay
/// alone, as with `stopPropagation`. An Esc whose only effect was to
/// unfocus a text field belongs to the field the same way. `judge` applies
/// the decisions.
pub(crate) fn claim_keys(
    ctx: &Context,
    kept: &Handle,
    nodes: &Nodes,
    (roots, here): (&[PlacedRoot], Here),
    drag: Drag,
) -> Keys {
    Source::note(ctx);
    Pointing::note(ctx);
    register_hold_tab(ctx);
    let open = open_overlays(roots, here);
    let pressed = |key| ctx.input(|input| input.key_pressed(key));
    let escape = pressed(Key::Escape);
    let input = Input {
        escape,
        drag_live: drag == Drag::Live,
        ..Input::default()
    };
    let layers = layers_of(
        ctx,
        (&judged_overlays(roots, here), &[]),
        nodes,
        roots,
        |_| Remembered::default(),
    );
    let belongs_to_the_library = match escape_goes(&layers, nodes, &input) {
        EscapeGoes::Nobody => input.escape && !input.drag_live && Typing::escape_blurred(ctx, kept),
        EscapeGoes::Overlay | EscapeGoes::Window(_) => true,
    };
    if belongs_to_the_library {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    }
    let tab = pressed(Key::Tab);
    let typing_before = kept.viewport(ctx.viewport_id(), |here| here.typing) == Typing::Yes;
    let escaped = escape && Typing::escape_blurred(ctx, kept);
    let top_down: Vec<usize> = open.iter().rev().map(|open| open.root).collect();
    let default = if pressed(Key::Enter) {
        actions::return_button(ctx, kept, nodes, (roots, here), &top_down)
    } else {
        None
    };
    if default.is_some() {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    }
    let cause = if escaped {
        Blur::Escaped
    } else if typing_before && default.is_none() && pressed(Key::Enter) {
        Blur::Submitted
    } else {
        Blur::Elsewhere
    };
    Keys {
        escape,
        tab,
        default,
        blur: cause,
    }
}

/// What `judge` reads of the frame.
pub(crate) struct Frame<'a> {
    pub(crate) ctx: &'a Context,
    pub(crate) kept: &'a Handle,
    pub(crate) nodes: &'a Nodes,
    pub(crate) roots: &'a [PlacedRoot],
    pub(crate) here: Here,
    pub(crate) subtrees: &'a [(Id, Box<[Id]>)],
    pub(crate) drag: Drag,
    pub(crate) keys: Keys,
}

/// Judges the frame's input against the open overlays, as the last step of a
/// draw. `subtrees` are the focusable widgets each identified node registered
/// this frame.
///
/// Returns the relations of the open overlays to the nodes they are anchored
/// to.
pub(crate) fn judge<A>(
    frame: Frame<'_>,
    dismiss: &mut HashMap<Id, DismissFn<A>>,
    app: &mut A,
) -> Vec<Relation> {
    let Frame {
        ctx,
        kept,
        nodes,
        roots,
        here,
        subtrees,
        drag,
        keys,
    } = frame;
    let open = open_overlays(roots, here);
    if open.iter().any(Open::holds_tab) {
        note_tab_held(ctx);
    }
    nav::forget_closed(
        kept,
        &open.iter().map(|open| open.overlay.id).collect::<Vec<_>>(),
    );
    let source = Source::last(ctx);
    let shell = Shell::new(ctx, kept);
    let mut stack = Stack::load(kept, ctx.viewport_id());
    let judged = judged_overlays(roots, here);
    let closers = actions::take_closers(ctx, kept);
    let layers = layers_of(ctx, (&judged, &closers), nodes, roots, |open| {
        let entry = stack.get_mut(open.overlay.id);
        Remembered {
            focus_before: entry.as_ref().map(|entry| entry.focus).unwrap_or_default(),
            focus_now: focus::at(&shell, open.overlay.id, &open.nested, open.anchor_node()),
            window_focus: entry.map(|entry| entry.window_focus).unwrap_or_default(),
        }
    });
    let input = Input {
        press: press_origin(ctx),
        escape: keys.escape,
        tab: keys.tab,
        drag_live: drag == Drag::Live,
        window_blurred: ctx.input(|input| crate::window::blurred(&input.events)),
    };
    let popups: Vec<Relation> = open
        .iter()
        .filter(|open| open.overlay.lookup == AnchorLookup::Found)
        .filter_map(|open| describe_trigger(&shell, roots, open))
        .collect();
    let found = dismissals(&layers, &input);
    let mut dismissed = Vec::new();
    for found in &found {
        let (Some(layer), Some(open)) = (layers.get(found.layer), judged.get(found.layer)) else {
            continue;
        };
        match (found.cause, layer.cancel) {
            (Cause::Cancel, Some(button)) => actions::press(ctx, kept, button),
            _ => {
                let id = open.overlay.id;
                dismissed.push(id);
                if open.overlay.behavior.hover.is_some() {
                    kept.viewport(ctx.viewport_id(), |here| here.hover_cards.dismissed());
                }
                if let Some(then) = dismiss.remove(&id) {
                    then(app);
                }
            }
        }
    }
    if let EscapeGoes::Window(button) = escape_goes(&layers, nodes, &input) {
        actions::press(ctx, kept, button);
    }
    if let Some(button) = keys.default {
        actions::press(ctx, kept, button);
    }
    if open.is_empty() {
        return popups;
    }

    for (open, layer) in judged.iter().zip(&layers) {
        let Some(entry) = stack.get_mut(open.overlay.id) else {
            continue;
        };
        entry.focus = layer.focus_now;
        if layer.seen == Seen::Drawn && ctx.input(|input| input.focused) {
            entry.window_focus = WindowFocus::Had;
        }
        if !entry.entered && layer.lookup == AnchorLookup::Found && layer.seen == Seen::Drawn {
            entry.entered = true;
            if enters_focus(open.overlay.behavior, source)
                && let Some(root) = roots.get(open.root)
                && let Some(first) = opening_focus(ctx, nodes, root.first, subtrees, open)
            {
                ctx.memory_mut(|memory| memory.request_focus(first));
            } else if open.holds_tab()
                && layer.focus_now == FocusAt::Elsewhere
                && let Some(focused) = ctx.memory(|memory| memory.focused())
            {
                // Opened with the pointer: nothing is focused, not the page
                // under the menu (a closing popover gives its focus back as
                // the menu opens).
                ctx.memory_mut(|memory| memory.surrender_focus(focused));
            }
        }
    }
    stack.store(kept, ctx.viewport_id());

    let menus: Vec<Menu> = open
        .iter()
        .filter(|open| open.overlay.behavior.nav != Nav::None)
        .filter(|open| !dismissed.contains(&open.overlay.id))
        .filter(|open| open.overlay.lookup == AnchorLookup::Found)
        .map(|open| Menu {
            id: open.overlay.id,
            nav: open.overlay.behavior.nav,
            wrap: open.overlay.behavior.nav.wrap(),
            items: roots
                .get(open.root)
                .map(|root| items(ctx, nodes, root.first))
                .unwrap_or_default(),
            own: subtrees
                .iter()
                .find(|(node, _)| *node == open.overlay.id)
                .map(|(_, focusable)| focusable.to_vec())
                .unwrap_or_default(),
            anchor: open.anchor_node(),
        })
        .collect();
    for id in nav::navigate(ctx, kept, &menus) {
        if let Some(then) = dismiss.remove(&id) {
            then(app);
        }
    }
    popups
}

/// What the user last operated, which decides whether a menu takes focus
/// when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Pointer,
    Keyboard,
}

impl Source {
    fn key(ctx: &Context) -> Id {
        crate::key::viewport_id(ctx, "input-source")
    }

    /// Notes the source of this frame's input, if it has any.
    pub(crate) fn note(ctx: &Context) {
        let latest = ctx.input(|input| {
            input.events.iter().rev().find_map(|event| match event {
                Event::PointerButton { pressed: true, .. } => Some(Self::Pointer),
                Event::Key { pressed: true, .. } => Some(Self::Keyboard),
                _ => None,
            })
        });
        if let Some(latest) = latest {
            let key = Self::key(ctx);
            ctx.data_mut(|data| data.insert_temp(key, latest));
        }
    }

    /// Whether the last input was a key (or there was none).
    pub(crate) fn last_is_keyboard(ctx: &Context) -> bool {
        Self::last(ctx) == Self::Keyboard
    }

    /// Where the last input came from; with none (an overlay opened by the
    /// application), the keyboard.
    fn last(ctx: &Context) -> Self {
        let key = Self::key(ctx);
        ctx.data(|data| data.get_temp(key))
            .unwrap_or(Self::Keyboard)
    }
}

/// Whether an overlay that just opened takes the focus. A menu opened with
/// the pointer does not, as in Radix: nothing is highlighted until an arrow
/// key, whose first press goes to the first (Down) or last (Up) item.
fn enters_focus(behavior: Behavior, source: Source) -> bool {
    let menu = behavior.nav.is_menu();
    behavior.focus == Focus::Enter && !(menu && source == Source::Pointer)
}

/// Where focus goes as the overlay whose root is `nodes[first]` opens: the
/// first focusable widget of the node marked with
/// [`focus_on_open`](crate::NodeHandle::focus_on_open), the node itself or
/// one inside it, else the overlay's first focusable. Of two marked nodes the
/// first declared is the one.
fn opening_focus(
    ctx: &Context,
    nodes: &Nodes,
    first: usize,
    subtrees: &[(Id, Box<[Id]>)],
    open: &Open<'_>,
) -> Option<Id> {
    let focusable = subtrees
        .iter()
        .find(|(node, _)| *node == open.overlay.id)
        .map(|(_, focusable)| &**focusable)?;
    let mut marked = nodes
        .subtree(first)
        .iter()
        .filter(|node| node.focuses_on_open())
        .filter_map(PlacedNode::id);
    let chosen = marked.next();
    if marked.next().is_some() {
        crate::report::report(
            ctx,
            crate::Problem::FocusOnOpenTwice,
            Some(open.overlay.id),
            || {
                format!(
                    "two nodes of the overlay {} are marked `focus_on_open`; the first takes focus",
                    nodes.node(first).map_or("", PlacedNode::path)
                )
            },
        );
    }
    chosen
        .and_then(|chosen| {
            subtrees
                .iter()
                .find(|(node, _)| *node == chosen)
                .and_then(|(_, widgets)| widgets.first().copied())
        })
        .filter(|widget| focusable.contains(widget))
        .or_else(|| focusable.first().copied())
}

/// The items of the overlay whose root is `nodes[first]`, in paint order,
/// that take input, where they were laid out.
fn items(ctx: &Context, nodes: &Nodes, first: usize) -> Vec<(Id, Rect)> {
    crate::roving::items_under(nodes, first)
        .into_iter()
        .filter_map(|at| nodes.get(at))
        .filter_map(|node| Some((node.id()?, node.rect())))
        .filter(|&(id, _)| {
            ctx.read_response(id)
                .is_some_and(|response| response.enabled())
        })
        .collect()
}

/// Tells assistive technology that the node the overlay is anchored to has
/// its popup open. Only while it is open: the widget that owns the trigger
/// says what it is when closed. Returns that the trigger controls a listbox.
fn describe_trigger(shell: &Shell, roots: &[PlacedRoot], open: &Open<'_>) -> Option<Relation> {
    use egui::accesskit::HasPopup;
    let trigger = nav::speaker(shell, open.anchor_node()?);
    let popup = match open.overlay.description.role {
        Some(egui::Role::Menu) => Some(HasPopup::Menu),
        Some(egui::Role::ListBox) => Some(HasPopup::Listbox),
        Some(egui::Role::Dialog) => Some(HasPopup::Dialog),
        _ => None,
    };
    shell.ctx.accesskit_node_builder(trigger, |node| {
        node.set_expanded(true);
        if let Some(popup) = popup {
            node.set_has_popup(popup);
        }
    });
    let listbox = roots.get(open.root)?.first;
    (popup == Some(HasPopup::Listbox)).then_some(Relation::Controls {
        node: trigger,
        target: crate::draw::blocker_id(listbox),
    })
}

/// What the pointer was the last time it moved or pressed: a mouse (or a
/// pen), or a finger on a touch screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pointing {
    Mouse,
    Touch,
}

impl Pointing {
    fn key(ctx: &Context) -> Id {
        crate::key::viewport_id(ctx, "pointing")
    }

    /// Notes what this frame's pointer input came from, if it has any. A
    /// touch also comes as pointer events, in the same frame.
    fn note(ctx: &Context) {
        let latest = ctx.input(|input| {
            let events = &input.events;
            if events
                .iter()
                .any(|event| matches!(event, Event::Touch { .. }))
            {
                Some(Self::Touch)
            } else {
                events
                    .iter()
                    .any(|event| {
                        matches!(event, Event::PointerMoved(_) | Event::PointerButton { .. })
                    })
                    .then_some(Self::Mouse)
            }
        });
        if let Some(latest) = latest {
            let key = Self::key(ctx);
            ctx.data_mut(|data| data.insert_temp(key, latest));
        }
    }
}

/// Whether the last pointer input was a touch.
pub(crate) fn touch_was_last(ctx: &Context) -> bool {
    let key = Pointing::key(ctx);
    ctx.data(|data| data.get_temp(key)) == Some(Pointing::Touch)
}

/// Whether the last input was from the keyboard, as opposed to a press.
pub(crate) fn keyboard_was_last(ctx: &Context) -> bool {
    Source::last_is_keyboard(ctx)
}
