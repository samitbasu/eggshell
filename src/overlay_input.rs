//! What a frame's input does to the open overlays, once they are drawn.
//!
//! This is the glue between egui's input and focus state and the pure rules
//! of [`dismissals`]: it reads the frame into a [`Layer`] per attached
//! overlay, runs the `on_dismiss` of the ones the rules name, moves focus
//! into overlays that just opened, and applies the keyboard pattern of
//! overlays with a [`Nav`]. Closing an overlay gives focus back in
//! [`focus::restore`], at the next layout, when the overlay is gone.

use std::collections::HashMap;

use egui::{Context, Event, Id, Key, Modifiers, Pos2};

use crate::actions::{self, Action};
use crate::dismiss::{Boundary, Cause, FocusAt, Input, Layer, Opening, dismissals, esc_target};
use crate::draw::children_of;
use crate::focus::{self, PointerOn};
use crate::kept::Handle;
use crate::nav::{self, Menu};
use crate::overlay::{Anchor, AnchorLookup, Band, Behavior, DismissOn, Focus, Nav, Outside};
use crate::pipeline::{DismissFn, PlacedNode};
use crate::roots::{Form, PlacedOverlay, PlacedRoot, root_at, stacked_top_down};
use crate::stack::{Entered, Stack};

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
pub(crate) fn pointer_on(ctx: &Context, nodes: &[PlacedNode], roots: &[PlacedRoot]) -> PointerOn {
    let Some(at) = ctx.input(|input| input.pointer.interact_pos()) else {
        return PointerOn::Elsewhere;
    };
    let root = root_at(roots, nodes, at);
    let on_it = roots[root].overlay().is_some_and(|overlay| {
        overlay.behavior.focus == Focus::Stay && nodes[roots[root].first].rect().contains(at)
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
fn tab_held_id() -> Id {
    crate::key::root_id().with("tab-held")
}

fn note_tab_held(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(tab_held_id(), true));
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
        let held = ctx
            .data_mut(|data| data.remove_temp::<bool>(tab_held_id()))
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
        self.overlay.behavior.outside() == Outside::Swallow
            || matches!(self.overlay.behavior.nav, Nav::Menu | Nav::Submenu)
    }

    fn anchor_node(&self) -> Option<Id> {
        match self.overlay.anchor_decl {
            Anchor::Node(id) => Some(id),
            _ => None,
        }
    }
}

/// The menus that are open but hidden, because their anchor left the window,
/// that an overlay opened this frame makes way for: menus are exclusive, seen
/// or not, or the hidden one would come back over the one opened beside it.
/// Hidden overlays take no part in the other rules, so `dismissals` does not
/// see them.
fn hidden_menus_made_way_for(roots: &[PlacedRoot], open: &[Open<'_>], layers: &[Layer]) -> Vec<Id> {
    let opened: Vec<&Open<'_>> = open
        .iter()
        .zip(layers)
        .filter(|(_, layer)| {
            layer.opening == Opening::Opened
                && layer.band != Band::Tooltip
                && layer.lookup == AnchorLookup::Found
        })
        .map(|(open, _)| open)
        .collect();
    roots
        .iter()
        .enumerate()
        .filter_map(|(root, placed)| Some((root, placed.overlay()?)))
        .filter(|(_, hidden)| {
            hidden.anchored == crate::overlay::Anchored::Detached
                && !hidden.staged.is_exiting()
                && hidden.lookup == AnchorLookup::Found
                && hidden.behavior.outside() == Outside::Swallow
                && hidden.behavior.dismiss_on == DismissOn::Default
        })
        .filter(|(root, hidden)| {
            opened.iter().any(|opened| {
                opened.overlay.stack_index > hidden.stack_index
                    && !is_nested_in(roots, opened.root, *root)
            })
        })
        .map(|(_, hidden)| hidden.id)
        .collect()
}

fn is_nested_in(roots: &[PlacedRoot], root: usize, ancestor: usize) -> bool {
    std::iter::successors(roots[root].parent, |&at| roots[at].parent).any(|at| at == ancestor)
}

/// The attached overlays, the one on the bottom first. An overlay that is
/// leaving is not among them: it takes no key and is not judged again.
fn open_overlays(roots: &[PlacedRoot]) -> Vec<Open<'_>> {
    let order: Vec<usize> = stacked_top_down(roots)
        .into_iter()
        .rev()
        .filter(|&root| {
            roots[root].overlay().is_some_and(|overlay| {
                !overlay.staged.is_exiting()
                    && (overlay.anchored == crate::overlay::Anchored::Attached
                        || overlay.lookup == AnchorLookup::Missing)
            })
        })
        .collect();
    order
        .iter()
        .map(|&root| Open {
            root,
            overlay: roots[root].overlay().expect("an overlay root"),
            nested: order
                .iter()
                .filter(|&&other| is_nested_in(roots, other, root))
                .filter_map(|&other| Some(roots[other].overlay()?.id))
                .collect(),
        })
        .collect()
}

/// The box of the overlay at `root`: the root's rect, or for a modal, whose
/// root is the scrim, what its content takes of it.
fn box_of(nodes: &[PlacedNode], roots: &[PlacedRoot], root: usize) -> egui::Rect {
    let first = roots[root].first;
    match roots[root].overlay().map(|overlay| overlay.form) {
        Some(Form::Modal(_)) => children_of(nodes, first)
            .into_iter()
            .fold(egui::Rect::NOTHING, |rect, child| {
                rect.union(nodes[child].rect())
            }),
        _ => nodes[first].rect(),
    }
}

/// The layers dismissal judges, from the attached overlays; `focus` says
/// where focus was and is relative to each.
fn layers_of(
    open: &[Open<'_>],
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    mut focus: impl FnMut(&Open<'_>) -> (FocusAt, FocusAt),
) -> Vec<Layer> {
    let position = |root: usize| open.iter().position(|open| open.root == root);
    open.iter()
        .map(|open| {
            let overlay = open.overlay;
            let (focus_before, focus_now) = focus(open);
            Layer {
                parent: roots[open.root].parent.and_then(position),
                rect: box_of(nodes, roots, open.root),
                boundary: match overlay.form {
                    Form::Modal(_) => Boundary::Scrim,
                    Form::Floating(_) => Boundary::Open,
                },
                cancel: actions::button_in(nodes, roots[open.root].first, Action::Cancel),
                trigger: matches!(overlay.anchor_decl, Anchor::Node(_) | Anchor::Rect(_))
                    .then_some(overlay.anchor),
                band: overlay.band,
                dismiss_on: overlay.behavior.dismiss_on,
                outside: overlay.behavior.outside(),
                opening: overlay.opening,
                focus_before,
                lookup: overlay.lookup,
                focus_now,
            }
        })
        .collect()
}

/// Whether a key was pressed this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pressed {
    Yes,
    No,
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
fn escape_goes(layers: &[Layer], nodes: &[PlacedNode], input: &Input) -> EscapeGoes {
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
        kept.with(|kept| kept.typing = now);
    }

    /// Whether the Esc of this pass was the one that took a text input's
    /// focus: it had focus at the end of the last pass and nothing has now.
    fn escape_blurred(ctx: &Context, kept: &Handle) -> bool {
        let before = kept.with(|kept| kept.typing);
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
    pub(crate) escape: Pressed,
    /// Whether Tab was pressed, even if a menu then took it.
    pub(crate) tab: Pressed,
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
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    drag: Drag,
) -> Keys {
    Source::note(ctx);
    register_hold_tab(ctx);
    let open = open_overlays(roots);
    let pressed = |key| ctx.input(|input| input.key_pressed(key));
    let escape = if pressed(Key::Escape) {
        Pressed::Yes
    } else {
        Pressed::No
    };
    let input = Input {
        escape: escape == Pressed::Yes,
        drag_live: drag == Drag::Live,
        ..Input::default()
    };
    let layers = layers_of(&open, nodes, roots, |_| {
        (FocusAt::Nowhere, FocusAt::Nowhere)
    });
    let belongs_to_the_library = match escape_goes(&layers, nodes, &input) {
        EscapeGoes::Nobody => input.escape && !input.drag_live && Typing::escape_blurred(ctx, kept),
        EscapeGoes::Overlay | EscapeGoes::Window(_) => true,
    };
    if belongs_to_the_library {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    }
    let tab = if pressed(Key::Tab) {
        Pressed::Yes
    } else {
        Pressed::No
    };
    let typing_before = kept.with(|kept| kept.typing) == Typing::Yes;
    let escaped = escape == Pressed::Yes && Typing::escape_blurred(ctx, kept);
    let top_down: Vec<usize> = open.iter().rev().map(|open| open.root).collect();
    let default = if pressed(Key::Enter) {
        actions::return_button(ctx, nodes, roots, &top_down)
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
    pub(crate) nodes: &'a [PlacedNode],
    pub(crate) roots: &'a [PlacedRoot],
    pub(crate) subtrees: &'a [(Id, Box<[Id]>)],
    pub(crate) drag: Drag,
    pub(crate) keys: Keys,
}

/// Judges the frame's input against the open overlays, as the last step of a
/// draw. `subtrees` are the focusable widgets each identified node registered
/// this frame.
pub(crate) fn judge<A>(frame: Frame<'_>, dismiss: &mut HashMap<Id, DismissFn<A>>, app: &mut A) {
    let Frame {
        ctx,
        kept,
        nodes,
        roots,
        subtrees,
        drag,
        keys,
    } = frame;
    let open = open_overlays(roots);
    if open.iter().any(Open::holds_tab) {
        note_tab_held(ctx);
    }
    nav::forget_closed(
        kept,
        &open.iter().map(|open| open.overlay.id).collect::<Vec<_>>(),
    );
    let source = Source::last(ctx);
    let mut stack = Stack::load(kept);
    let layers = layers_of(&open, nodes, roots, |open| {
        let before = stack
            .get_mut(open.overlay.id)
            .map(|entry| entry.focus)
            .unwrap_or_default();
        (
            before,
            focus::at(ctx, open.overlay.id, &open.nested, open.anchor_node()),
        )
    });
    let input = Input {
        press: press_origin(ctx),
        escape: keys.escape == Pressed::Yes,
        tab: keys.tab == Pressed::Yes,
        drag_live: drag == Drag::Live,
        window_blurred: ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, Event::WindowFocused(false)))
        }),
    };
    for open in open
        .iter()
        .filter(|open| open.overlay.lookup == AnchorLookup::Found)
    {
        describe_trigger(ctx, open);
    }
    let found = dismissals(&layers, &input);
    let mut dismissed = Vec::new();
    for found in &found {
        match (found.cause, layers[found.layer].cancel) {
            (Cause::Cancel, Some(button)) => actions::press(ctx, button),
            _ => {
                let id = open[found.layer].overlay.id;
                dismissed.push(id);
                if let Some(then) = dismiss.remove(&id) {
                    then(app);
                }
            }
        }
    }
    for id in hidden_menus_made_way_for(roots, &open, &layers) {
        if let Some(then) = dismiss.remove(&id) {
            then(app);
        }
    }
    if let EscapeGoes::Window(button) = escape_goes(&layers, nodes, &input) {
        actions::press(ctx, button);
    }
    if let Some(button) = keys.default {
        actions::press(ctx, button);
    }
    if open.is_empty() {
        return;
    }

    for (open, layer) in open.iter().zip(&layers) {
        let Some(entry) = stack.get_mut(open.overlay.id) else {
            continue;
        };
        entry.focus = layer.focus_now;
        if entry.entered == Entered::Not && layer.lookup == AnchorLookup::Found {
            entry.entered = Entered::Yet;
            if enters_focus(open.overlay.behavior, source)
                && let Some(first) = first_focusable(subtrees, open.overlay.id)
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
    stack.store(kept);

    let menus: Vec<Menu> = open
        .iter()
        .filter(|open| open.overlay.behavior.nav != Nav::None)
        .filter(|open| !dismissed.contains(&open.overlay.id))
        .filter(|open| open.overlay.lookup == AnchorLookup::Found)
        .map(|open| Menu {
            id: open.overlay.id,
            nav: open.overlay.behavior.nav,
            wrap: open.overlay.behavior.wrap,
            items: items(ctx, nodes, roots[open.root].first),
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
}

/// What the user last operated, which decides whether a menu takes focus
/// when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Pointer,
    Keyboard,
}

impl Source {
    fn key() -> Id {
        crate::key::root_id().with("input-source")
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
            ctx.data_mut(|data| data.insert_temp(Self::key(), latest));
        }
    }

    /// Whether the last input was a key (or there was none).
    pub(crate) fn last_is_keyboard(ctx: &Context) -> bool {
        Self::last(ctx) == Self::Keyboard
    }

    /// Where the last input came from; with none (an overlay opened by the
    /// application), the keyboard.
    fn last(ctx: &Context) -> Self {
        ctx.data(|data| data.get_temp(Self::key()))
            .unwrap_or(Self::Keyboard)
    }
}

/// Whether an overlay that just opened takes the focus. A menu opened with
/// the pointer does not, as in Radix: nothing is highlighted until an arrow
/// key, whose first press goes to the first (Down) or last (Up) item.
fn enters_focus(behavior: Behavior, source: Source) -> bool {
    let menu = matches!(behavior.nav, Nav::Menu | Nav::Submenu);
    behavior.focus == Focus::Enter && !(menu && source == Source::Pointer)
}

fn first_focusable(subtrees: &[(Id, Box<[Id]>)], id: Id) -> Option<Id> {
    subtrees
        .iter()
        .find(|(node, _)| *node == id)
        .and_then(|(_, focusable)| focusable.first().copied())
}

/// The items of the overlay whose root is `nodes[first]`, in paint order,
/// that take input.
fn items(ctx: &Context, nodes: &[PlacedNode], first: usize) -> Vec<Id> {
    nodes[first..nodes[first].end]
        .iter()
        .filter(|node| node.is_item())
        .filter_map(PlacedNode::id)
        .filter(|&id| {
            ctx.read_response(id)
                .is_some_and(|response| response.enabled())
        })
        .collect()
}

/// Tells assistive technology that the node the overlay is anchored to has
/// its popup open. Only while it is open: the widget that owns the trigger
/// says what it is when closed.
fn describe_trigger(ctx: &Context, open: &Open<'_>) {
    use egui::accesskit::HasPopup;
    let Some(trigger) = open.anchor_node() else {
        return;
    };
    let popup = match open.overlay.description.role {
        Some(egui::Role::Menu) => Some(HasPopup::Menu),
        Some(egui::Role::ListBox) => Some(HasPopup::Listbox),
        Some(egui::Role::Dialog) => Some(HasPopup::Dialog),
        _ => None,
    };
    ctx.accesskit_node_builder(trigger, |node| {
        node.set_expanded(true);
        if let Some(popup) = popup {
            node.set_has_popup(popup);
        }
    });
}

/// Whether the last input was from the keyboard, as opposed to a press.
pub(crate) fn keyboard_was_last(ctx: &Context) -> bool {
    Source::last_is_keyboard(ctx)
}
