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

use crate::dismiss::{Cause, Input, Layer, dismissals};
use crate::focus;
use crate::nav::{self, Menu};
use crate::overlay::{Anchor, Nav};
use crate::pipeline::{DismissFn, PlacedNode};
use crate::roots::{PlacedOverlay, PlacedRoot, stacked_top_down};
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
            roots[root]
                .overlay()
                .is_some_and(|overlay| overlay.anchored == crate::overlay::Anchored::Attached)
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

/// Judges the frame's input against the open overlays, as the last step of a
/// draw. `subtrees` are the focusable widgets each identified node registered
/// this frame.
pub(crate) fn judge<A>(
    ctx: &Context,
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    subtrees: &[(Id, Box<[Id]>)],
    dismiss: &mut HashMap<Id, DismissFn<A>>,
    app: &mut A,
    drag: Drag,
) {
    let open = open_overlays(roots);
    nav::forget_closed(
        ctx,
        &open.iter().map(|open| open.overlay.id).collect::<Vec<_>>(),
    );
    if open.is_empty() {
        return;
    }
    let mut stack = Stack::load(ctx);
    let position = |root: usize| open.iter().position(|open| open.root == root);
    let layers: Vec<Layer> = open
        .iter()
        .map(|open| {
            let overlay = open.overlay;
            let focus_before = stack
                .get_mut(overlay.id)
                .map(|entry| entry.focus)
                .unwrap_or_default();
            Layer {
                parent: roots[open.root].parent.and_then(position),
                rect: nodes[roots[open.root].first].rect(),
                trigger: matches!(overlay.anchor_decl, Anchor::Node(_) | Anchor::Rect(_))
                    .then_some(overlay.anchor),
                band: overlay.band,
                dismiss_on: overlay.behavior.dismiss_on,
                outside: overlay.behavior.outside(),
                opening: overlay.opening,
                focus_before,
                focus_now: focus::at(ctx, overlay.id, &open.nested, open.anchor_node()),
            }
        })
        .collect();
    let input = Input {
        press: press_origin(ctx),
        escape: ctx.input(|input| input.key_pressed(Key::Escape)),
        drag_live: drag == Drag::Live,
        window_blurred: ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, Event::WindowFocused(false)))
        }),
    };
    for open in &open {
        describe_trigger(ctx, open);
    }
    let found = dismissals(&layers, &input);
    if found.iter().any(|found| found.cause == Cause::Esc) {
        ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    }
    let dismissed: Vec<Id> = found
        .iter()
        .map(|found| open[found.layer].overlay.id)
        .collect();
    for id in &dismissed {
        if let Some(then) = dismiss.remove(id) {
            then(app);
        }
    }

    for (open, layer) in open.iter().zip(&layers) {
        let Some(entry) = stack.get_mut(open.overlay.id) else {
            continue;
        };
        entry.focus = layer.focus_now;
        if entry.entered == Entered::Not {
            entry.entered = Entered::Yet;
            if open.overlay.behavior.focus == crate::overlay::Focus::Enter
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
