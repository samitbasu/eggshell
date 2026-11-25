//! Default and cancel actions: the buttons Return and Esc press.
//!
//! A button is marked with [`button::Chain::default_action`] or
//! [`button::Chain::cancel_action`](crate::owned::button::Chain::cancel_action).
//! Esc is part of dismissal (`dismissals` decides which overlay it goes to
//! and whether that overlay's cancel button answers it); the functions here
//! find the buttons and decide Return. A button is pressed by asking the next
//! frame to activate it as a click (see [`nav::request_activation`]).
//!
//! The rules are in `docs/overlays.md`, section 4.
//!
//! [`button::Chain::default_action`]: crate::owned::button::Chain::default_action

use egui::{Context, Id};

use crate::kept::Handle;
use crate::nodes::Nodes;
use crate::shell::Shell;

use crate::nav;
use crate::pipeline::PlacedNode;
use crate::roots::{Here, PlacedRoot, root_and_node};

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
pub(crate) fn button_in(nodes: &Nodes, first: usize, action: Action) -> Option<Id> {
    nodes
        .subtree(first)
        .iter()
        .find(|node| node.action() == Some(action))
        .and_then(PlacedNode::id)
}

/// Presses `button` on the next frame, unless it is disabled.
pub(crate) fn press(ctx: &Context, kept: &Handle, button: Id) {
    if ctx
        .read_response(button)
        .is_some_and(|response| response.enabled())
    {
        nav::request_activation(ctx, kept, button);
    }
}

/// Notes that `button`, which dismisses its overlay, was pressed: the pass's
/// dismissal closes the overlay it is in.
pub(crate) fn note_closer(ctx: &Context, kept: &Handle, button: Id) {
    kept.viewport(ctx.viewport_id(), |here| here.closers.push(button));
}

/// The buttons that dismiss their overlay pressed since the last time this
/// was asked. Asking takes them.
pub(crate) fn take_closers(ctx: &Context, kept: &Handle) -> Vec<Id> {
    kept.viewport(ctx.viewport_id(), |here| std::mem::take(&mut here.closers))
}

/// Notes that the multi-line text field `id` has focus: Return is its own.
pub(crate) fn note_multiline_focus(ctx: &Context, kept: &Handle, id: Id) {
    kept.viewport(ctx.viewport_id(), |here| here.focus.multiline = Some(id));
}

/// Whether the focused widget `focused` takes Return itself: a button or a
/// multi-line text field. Any other control, such as a checkbox or a radio
/// button, lets Return reach the default button.
fn takes_return(ctx: &Context, kept: &Handle, nodes: &Nodes, focused: Id) -> bool {
    kept.viewport(ctx.viewport_id(), |here| here.focus.multiline) == Some(focused)
        || nodes
            .iter()
            .any(|node| node.id() == Some(focused) && node.is_button())
}

/// The root Return goes to: the overlay (or the page) that holds the focus,
/// else the topmost open overlay, else the page. `open` are the roots of the
/// attached overlays, the topmost first.
fn return_root(
    shell: &Shell,
    (roots, here): (&[PlacedRoot], Here),
    nodes: &Nodes,
    open: &[usize],
) -> usize {
    let holder = open.iter().copied().find(|&root| {
        root_and_node(roots, nodes, root)
            .and_then(|(_, node)| node.id())
            .is_some_and(|id| shell.focus_within(id))
    });
    match (holder, shell.ctx.memory(|memory| memory.focused())) {
        (Some(root), _) => root,
        (None, Some(_)) => here.root,
        (None, None) => open.first().copied().unwrap_or(here.root),
    }
}

/// The default button Return goes to, if nothing focused takes Return
/// itself. `open` are the roots of the attached overlays, topmost first.
pub(crate) fn return_button(
    ctx: &Context,
    kept: &Handle,
    nodes: &Nodes,
    (roots, here): (&[PlacedRoot], Here),
    open: &[usize],
) -> Option<Id> {
    if let Some(focused) = ctx.memory(|memory| memory.focused())
        && takes_return(ctx, kept, nodes, focused)
    {
        return None;
    }
    let root = return_root(&Shell::new(ctx, kept), (roots, here), nodes, open);
    button_in(nodes, roots.get(root)?.first, Action::Default)
}
