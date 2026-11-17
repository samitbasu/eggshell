//! The nodes of a laid-out frame, behind accessors that cannot fail.
//!
//! The nodes are in pre-order, and each node's `end` is one past its last
//! descendant, so a subtree is the run of nodes from the node to its `end`.
//! Every index the library holds was handed out by [`Nodes::push`]; one that
//! is out of range is a bug of the library: it is reported as
//! [`Problem::Internal`] and the accessor answers nothing.

use std::ops::Range;
use std::panic::Location;

use egui::Context;

use crate::Problem;
use crate::pipeline::PlacedNode;

pub(crate) struct Nodes {
    list: Vec<PlacedNode>,
    /// Where misses are reported.
    ctx: Context,
}

impl Nodes {
    pub(crate) fn new(ctx: &Context) -> Self {
        Self {
            list: Vec::new(),
            ctx: ctx.clone(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, PlacedNode> {
        self.list.iter()
    }

    /// The node at `index`, if there is one: for an index that may run past
    /// the frame, such as the one after a subtree.
    pub(crate) fn get(&self, index: usize) -> Option<&PlacedNode> {
        self.list.get(index)
    }

    /// The node at `index`, which the library handed out.
    #[track_caller]
    pub(crate) fn node(&self, index: usize) -> Option<&PlacedNode> {
        let node = self.list.get(index);
        if node.is_none() {
            self.missing(Location::caller(), index);
        }
        node
    }

    #[track_caller]
    pub(crate) fn node_mut(&mut self, index: usize) -> Option<&mut PlacedNode> {
        if index >= self.list.len() {
            self.missing(Location::caller(), index);
        }
        self.list.get_mut(index)
    }

    /// The indices of the node at `index` and its descendants.
    #[track_caller]
    pub(crate) fn span(&self, index: usize) -> Range<usize> {
        match self.node(index) {
            Some(node) => index..node.end.min(self.list.len()).max(index),
            None => 0..0,
        }
    }

    /// The node at `index` followed by its descendants.
    #[track_caller]
    pub(crate) fn subtree(&self, index: usize) -> &[PlacedNode] {
        self.list.get(self.span(index)).unwrap_or_default()
    }

    /// The descendants of the node at `index`.
    #[track_caller]
    pub(crate) fn descendants(&self, index: usize) -> &[PlacedNode] {
        self.subtree(index).get(1..).unwrap_or_default()
    }

    /// The indices of the children of the node at `index`, in layout order.
    #[track_caller]
    pub(crate) fn children(&self, index: usize) -> Children<'_> {
        let span = self.span(index);
        Children {
            list: &self.list,
            next: span.start.saturating_add(1),
            end: span.end,
        }
    }

    /// The indices of the children of the node at `index`, bottom first:
    /// stable-sorted by `z`, the order they are painted and hit in.
    #[track_caller]
    pub(crate) fn in_paint_order(&self, index: usize) -> Vec<usize> {
        let mut children: Vec<usize> = self.children(index).collect();
        children.sort_by_key(|&child| self.get(child).map_or(0, PlacedNode::z));
        children
    }

    /// The nodes before the one at `index`.
    pub(crate) fn before(&self, index: usize) -> &[PlacedNode] {
        self.list.get(..index).unwrap_or(&self.list)
    }

    /// The indices of the ancestors of the node at `index`, nearest first.
    pub(crate) fn ancestors(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        self.before(index)
            .iter()
            .enumerate()
            .rev()
            .filter(move |(_, above)| above.end > index)
            .map(|(above, _)| above)
    }

    /// The child of `ancestor` whose subtree holds `index`.
    #[track_caller]
    pub(crate) fn child_towards(&self, ancestor: usize, index: usize) -> Option<usize> {
        let found = self
            .children(ancestor)
            .find(|&child| self.span(child).contains(&index));
        if found.is_none() {
            self.missing(Location::caller(), index);
        }
        found
    }

    /// The node at `index`, and the nodes after it.
    #[track_caller]
    pub(crate) fn split_after_mut(
        &mut self,
        index: usize,
    ) -> Option<(&mut PlacedNode, &mut [PlacedNode])> {
        if index >= self.list.len() {
            self.missing(Location::caller(), index);
        }
        self.list.get_mut(index..)?.split_first_mut()
    }

    /// Adds `node` as the last node, a leaf until [`close`](Self::close)
    /// ends its subtree, and answers its index.
    pub(crate) fn push(&mut self, mut node: PlacedNode) -> usize {
        let index = self.list.len();
        node.end = self.list.len().saturating_add(1);
        self.list.push(node);
        index
    }

    /// Ends the subtree of the node at `index` after the last node pushed.
    #[track_caller]
    pub(crate) fn close(&mut self, index: usize) {
        let end = self.list.len();
        if let Some(node) = self.node_mut(index) {
            node.end = end;
        }
    }

    fn missing(&self, at: &'static Location<'static>, index: usize) {
        crate::report::report_at(
            &self.ctx,
            Problem::Internal("nodes: missing index"),
            None,
            at,
            || {
                format!(
                    "node {index} is not among the frame's {} nodes; it is skipped",
                    self.list.len()
                )
            },
        );
    }
}

impl<'a> IntoIterator for &'a Nodes {
    type Item = &'a PlacedNode;
    type IntoIter = std::slice::Iter<'a, PlacedNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The indices of a node's children: each child's `end` is where the next
/// one starts.
pub(crate) struct Children<'a> {
    list: &'a [PlacedNode],
    next: usize,
    end: usize,
}

impl Iterator for Children<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.next >= self.end {
            return None;
        }
        let child = self.next;
        self.next = self
            .list
            .get(child)
            .map_or(self.end, |node| node.end.max(child.saturating_add(1)));
        Some(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_no_reports, assert_reported};

    const MISSING: Problem = Problem::Internal("nodes: missing index");

    /// A root with two children, the first of which has a child of its own.
    fn tree(ctx: &Context) -> Nodes {
        let mut nodes = Nodes::new(ctx);
        let root = nodes.push(PlacedNode::stub(0));
        let first = nodes.push(PlacedNode::stub(1));
        nodes.push(PlacedNode::stub(2));
        nodes.close(first);
        nodes.push(PlacedNode::stub(1));
        nodes.close(root);
        nodes
    }

    #[test]
    fn the_subtrees_follow_the_pre_order() {
        let ctx = Context::default();
        let nodes = tree(&ctx);
        assert_eq!(nodes.span(0), 0..4);
        assert_eq!(nodes.span(1), 1..3);
        assert_eq!(nodes.subtree(1).len(), 2);
        assert_eq!(nodes.descendants(0).len(), 3);
        assert_eq!(nodes.children(0).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(nodes.children(1).collect::<Vec<_>>(), [2]);
        assert_eq!(nodes.children(2).count(), 0);
        assert_eq!(nodes.child_towards(0, 2), Some(1));
        assert_no_reports(&ctx);
    }

    #[test]
    fn a_missing_index_answers_nothing_and_is_reported() {
        let ctx = Context::default();
        let mut nodes = tree(&ctx);
        assert!(nodes.get(4).is_none());
        assert_no_reports(&ctx);
        assert!(nodes.node(4).is_none());
        assert_reported(&ctx, MISSING);
        assert!(nodes.node_mut(9).is_none());
        assert_reported(&ctx, MISSING);
        assert_eq!(nodes.span(7), 0..0);
        assert_reported(&ctx, MISSING);
        assert!(nodes.subtree(7).is_empty());
        assert_reported(&ctx, MISSING);
        assert!(nodes.descendants(7).is_empty());
        assert_reported(&ctx, MISSING);
        assert_eq!(nodes.children(7).count(), 0);
        assert_reported(&ctx, MISSING);
        assert_eq!(nodes.child_towards(1, 3), None);
        assert_reported(&ctx, MISSING);
        nodes.close(8);
        assert_reported(&ctx, MISSING);
    }

    #[test]
    fn an_end_past_the_frame_is_cut_to_it() {
        let ctx = Context::default();
        let mut nodes = tree(&ctx);
        if let Some(root) = nodes.node_mut(0) {
            root.end = 99;
        }
        assert_eq!(nodes.span(0), 0..4);
        assert_eq!(nodes.children(0).collect::<Vec<_>>(), [1, 3]);
        assert_no_reports(&ctx);
    }

    #[test]
    fn an_end_before_its_node_still_moves_on() {
        let ctx = Context::default();
        let mut nodes = tree(&ctx);
        if let Some(first) = nodes.node_mut(1) {
            first.end = 0;
        }
        assert_eq!(nodes.children(0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(nodes.span(1), 1..1);
    }
}
