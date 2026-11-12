//! Default and cancel actions: the buttons Return and Esc press.
//!
//! A button is marked with [`ButtonChain::default_action`] or
//! [`ButtonChain::cancel_action`](crate::owned::ButtonChain::cancel_action).
//! Esc is part of dismissal (`dismissals` decides which overlay it goes to
//! and whether that overlay's cancel button answers it); the functions here
//! find the buttons and decide Return. A button is pressed by asking the next
//! frame to activate it as a click (see [`nav::request_activation`]).
//!
//! The rules are in `docs/overlays.md`, section 4.
//!
//! [`ButtonChain::default_action`]: crate::owned::ButtonChain::default_action

use egui::{Context, Id};

use crate::focus;
use crate::nav;
use crate::pipeline::PlacedNode;
use crate::roots::PlacedRoot;

/// The key a button answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    /// Return.
    Default,
    /// Esc.
    Cancel,
}

/// The first button of `action` declared in the root whose node is
/// `nodes[first]`.
pub(crate) fn button_in(nodes: &[PlacedNode], first: usize, action: Action) -> Option<Id> {
    nodes[first..nodes[first].end]
        .iter()
        .find(|node| node.action() == Some(action))
        .and_then(PlacedNode::id)
}

/// Presses `button` on the next frame, unless it is disabled.
pub(crate) fn press(ctx: &Context, button: Id) {
    if ctx
        .read_response(button)
        .is_some_and(|response| response.enabled())
    {
        nav::request_activation(ctx, button);
    }
}

fn multiline_key() -> Id {
    crate::key::root_id().with("multiline-focus")
}

/// Notes that the multi-line text field `id` has focus: Return is its own.
pub(crate) fn note_multiline_focus(ctx: &Context, id: Id) {
    ctx.data_mut(|data| data.insert_temp(multiline_key(), id));
}

/// Whether the focused widget `focused` takes Return itself: a button or
/// other control that Return activates, or a multi-line text field.
fn takes_return(ctx: &Context, nodes: &[PlacedNode], focused: Id) -> bool {
    ctx.data(|data| data.get_temp::<Id>(multiline_key())) == Some(focused)
        || nodes
            .iter()
            .any(|node| node.id() == Some(focused) && node.senses())
}

/// The root Return goes to: the overlay (or the page) that holds the focus,
/// else the topmost open overlay, else the page. `open` are the roots of the
/// attached overlays, the topmost first.
fn return_root(ctx: &Context, roots: &[PlacedRoot], nodes: &[PlacedNode], open: &[usize]) -> usize {
    let holder = open.iter().copied().find(|&root| {
        nodes[roots[root].first]
            .id()
            .is_some_and(|id| focus::focus_within(ctx, id))
    });
    match (holder, ctx.memory(|memory| memory.focused())) {
        (Some(root), _) => root,
        (None, Some(_)) => 0,
        (None, None) => open.first().copied().unwrap_or(0),
    }
}

/// The default button Return goes to, if nothing focused takes Return
/// itself. `open` are the roots of the attached overlays, topmost first.
pub(crate) fn return_button(
    ctx: &Context,
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    open: &[usize],
) -> Option<Id> {
    if let Some(focused) = ctx.memory(|memory| memory.focused())
        && takes_return(ctx, nodes, focused)
    {
        return None;
    }
    let root = return_root(ctx, roots, nodes, open);
    button_in(nodes, roots[root].first, Action::Default)
}
