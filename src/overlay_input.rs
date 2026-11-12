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
use crate::dismiss::{Boundary, Cause, FocusAt, Input, Layer, dismissals, esc_target};
use crate::draw::children_of;
use crate::focus;
use crate::nav::{self, Menu};
use crate::overlay::{Anchor, AnchorLookup, Behavior, Focus, Nav};
use crate::pipeline::{DismissFn, PlacedNode};
use crate::roots::{Form, PlacedOverlay, PlacedRoot, stacked_top_down};
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

/// An attached overlay.
struct Open<'a> {
    root: usize,
    overlay: &'a PlacedOverlay,
    /// The ids of the overlays nested in it.
    nested: Vec<Id>,
}

impl Open<'_> {
    fn anchor_node(&self) -> Option<Id> {
        match self.overlay.anchor_decl {
            Anchor::Node(id) => Some(id),
            _ => None,
        }
    }
}

fn is_nested_in(roots: &[PlacedRoot], root: usize, ancestor: usize) -> bool {
    std::iter::successors(roots[root].parent, |&at| roots[at].parent).any(|at| at == ancestor)
}

/// The attached overlays, the one on the bottom first.
fn open_overlays(roots: &[PlacedRoot]) -> Vec<Open<'_>> {
    let order: Vec<usize> = stacked_top_down(roots)
        .into_iter()
        .rev()
        .filter(|&root| {
            roots[root].overlay().is_some_and(|overlay| {
                overlay.anchored == crate::overlay::Anchored::Attached
                    || overlay.lookup == AnchorLookup::Missing
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

/// The keys the frame pressed that the library answers, decided from the
/// layout before any draw closure runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Keys {
    /// Whether Esc was pressed, even if it was then taken.
    pub(crate) escape: Pressed,
    /// The default button Return goes to.
    pub(crate) default: Option<Id>,
}

/// Decides who Esc and Return belong to and takes them from the frame's
/// input, so that no draw closure (a custom canvas under a menu) sees a key
/// that dismissed an overlay or pressed a button: it belongs to the overlay
/// alone, as with `stopPropagation`. `judge` applies the decisions.
pub(crate) fn claim_keys(
    ctx: &Context,
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    drag: Drag,
) -> Keys {
    Source::note(ctx);
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
    if !matches!(escape_goes(&layers, nodes, &input), EscapeGoes::Nobody) {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    }
    let top_down: Vec<usize> = open.iter().rev().map(|open| open.root).collect();
    let default = if pressed(Key::Enter) {
        actions::return_button(ctx, nodes, roots, &top_down)
    } else {
        None
    };
    if default.is_some() {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    }
    Keys { escape, default }
}

/// What `judge` reads of the frame.
pub(crate) struct Frame<'a> {
    pub(crate) ctx: &'a Context,
    /// The `LayoutState`'s overlay stack.
    pub(crate) scope: Id,
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
        scope,
        nodes,
        roots,
        subtrees,
        drag,
        keys,
    } = frame;
    let open = open_overlays(roots);
    nav::forget_closed(
        ctx,
        &open.iter().map(|open| open.overlay.id).collect::<Vec<_>>(),
    );
    let source = Source::last(ctx);
    let mut stack = Stack::load(ctx, scope);
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
            }
        }
    }
    stack.store(ctx);

    let menus: Vec<Menu> = open
        .iter()
        .filter(|open| open.overlay.behavior.nav != Nav::None)
        .filter(|open| !dismissed.contains(&open.overlay.id))
        .filter(|open| open.overlay.lookup == AnchorLookup::Found)
        .map(|open| Menu {
            id: open.overlay.id,
            nav: open.overlay.behavior.nav,
            items: items(ctx, nodes, roots[open.root].first),
            own: subtrees
                .iter()
                .find(|(node, _)| *node == open.overlay.id)
                .map(|(_, focusable)| focusable.to_vec())
                .unwrap_or_default(),
            anchor: open.anchor_node(),
        })
        .collect();
    for id in nav::navigate(ctx, &menus) {
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
    fn note(ctx: &Context) {
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
