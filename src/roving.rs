//! Groups of items on the page that the arrow keys move the focus through:
//! a tab list, a radio group ([`NodeHandle::nav`](crate::NodeHandle::nav)).
//!
//! A group is one Tab stop. While the focus is on one of its items, that
//! item is the stop; otherwise the item marked selected is, or, with none,
//! the first enabled item. The other items are left out of the Tab order, so
//! the stop is settled before the frame's widgets are drawn: the arrow keys
//! move the focus first, and the item they reach is focusable in the same
//! frame.

use std::collections::HashSet;

use egui::{Context, Id, Rect};

use crate::kept::Handle;
use crate::nodes::Nodes;
use crate::overlay::Nav;
use crate::owned::Mark;
use crate::pipeline::PlacedNode;

/// What a node declared with [`NodeHandle::nav`](crate::NodeHandle::nav)
/// does with its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) nav: Nav,
    pub(crate) follows: Follows,
}

impl Group {
    pub(crate) fn new(nav: Nav) -> Self {
        Self {
            nav,
            follows: Follows::Focus,
        }
    }
}

/// What reaching an item with an arrow key does besides focusing it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Follows {
    /// Nothing: Enter or Space activates it.
    #[default]
    Focus,
    /// It activates the item, as a click would.
    Selection,
}

/// A group of a laid-out frame and its items, in paint order.
struct Placed {
    group: Group,
    items: Vec<Item>,
}

struct Item {
    id: Id,
    rect: Rect,
    selected: bool,
    enabled: bool,
}

impl Placed {
    fn enabled(&self) -> Vec<(Id, Rect)> {
        self.items
            .iter()
            .filter(|item| item.enabled)
            .map(|item| (item.id, item.rect))
            .collect()
    }

    /// The item that is the group's Tab stop while `focused` has the focus.
    fn stop(&self, focused: Option<Id>) -> Option<Id> {
        let enabled = || self.items.iter().filter(|item| item.enabled);
        enabled()
            .find(|item| Some(item.id) == focused)
            .or_else(|| enabled().find(|item| item.selected))
            .or_else(|| enabled().next())
            .map(|item| item.id)
    }
}

/// The indices of the items under the node at `first`, an overlay's root
/// or a group: the nodes with an item role, leaving out the items of a group
/// nested under it, which are that group's.
pub(crate) fn items_under(nodes: &Nodes, first: usize) -> Vec<usize> {
    let span = nodes.span(first);
    let mut items = Vec::new();
    let mut at = span.start.saturating_add(1);
    while at < span.end {
        let Some(node) = nodes.get(at) else { break };
        if node.group().is_some() {
            at = node.end.max(at.saturating_add(1));
            continue;
        }
        if node.is_item() && node.id().is_some() {
            items.push(at);
        }
        at = at.saturating_add(1);
    }
    items
}

/// Whether the node at `index` and every node above it up to the one at
/// `top` take input.
fn live(nodes: &Nodes, top: usize, index: usize) -> bool {
    let inert = |node: Option<&PlacedNode>| node.is_some_and(PlacedNode::is_inert);
    !inert(nodes.get(index))
        && !nodes
            .ancestors(index)
            .take_while(|&above| above >= top)
            .any(|above| inert(nodes.get(above)))
}

fn placed(nodes: &Nodes) -> Vec<Placed> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| Some((index, node.group()?)))
        .filter(|(_, group)| group.nav.on_page())
        .map(|(index, group)| Placed {
            group,
            items: items_under(nodes, index)
                .into_iter()
                .filter_map(|at| {
                    let node = nodes.get(at)?;
                    Some(Item {
                        id: node.id()?,
                        rect: node.rect(),
                        selected: node.selection() == Some(Mark::On),
                        enabled: node.senses() && live(nodes, index, at),
                    })
                })
                .collect(),
        })
        .collect()
}

/// Moves the focus on the frame's arrow key within the group that holds it,
/// and answers the items that are not their group's Tab stop. Run before the
/// frame's widgets.
pub(crate) fn settle(ctx: &Context, kept: &Handle, nodes: &Nodes) -> HashSet<Id> {
    let groups = placed(nodes);
    let focused = ctx.memory(|memory| memory.focused());
    if let Some(focused) = focused
        && let Some(group) = groups
            .iter()
            .find(|group| group.items.iter().any(|item| item.id == focused))
        && let Some(to) = crate::nav::rove(ctx, group.group.nav, &group.enabled(), focused)
    {
        ctx.memory_mut(|memory| memory.request_focus(to));
        if group.group.follows == Follows::Selection {
            crate::nav::request_activation(ctx, kept, to);
        }
    }
    let focused = ctx.memory(|memory| memory.focused());
    groups
        .iter()
        .flat_map(|group| {
            let stop = group.stop(focused);
            group
                .items
                .iter()
                .map(|item| item.id)
                .filter(move |&id| Some(id) != stop)
        })
        .collect()
}
