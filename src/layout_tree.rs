//! The taffy tree a frame is laid out in, behind accessors that cannot fail.
//!
//! Every node is made by [`LayoutTree::new_node`] with its [`Node`], so a
//! node is in the tree exactly when it has one: that is the check made
//! before each taffy call, whose own indexing panics on a missing node. A
//! miss is a bug of the library: it is reported as
//! [`Problem::Internal`] and the accessor falls back to a default.

use std::panic::Location;

use egui::Context;
use taffy::{AvailableSpace, Layout, LayoutInput, LayoutOutput, NodeId, Size, Style, TaffyTree};

use crate::Problem;
use crate::pipeline::{Node, ScrollNode};

const DEFAULT_STYLE: &Style = &Style::DEFAULT;

/// The id [`LayoutTree::new_node`] gives when taffy refuses a node: one that
/// no tree holds, so every accessor treats it as missing.
const DANGLING: NodeId = NodeId::new(u64::MAX);

pub(crate) struct LayoutTree {
    taffy: TaffyTree<Node>,
    /// Where misses are reported: the context of the build that last reset
    /// the tree.
    ctx: Option<Context>,
}

impl LayoutTree {
    pub(crate) fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.disable_rounding();
        Self { taffy, ctx: None }
    }

    /// An empty tree reporting where this one does.
    pub(crate) fn scratch(&self) -> Self {
        Self {
            ctx: self.ctx.clone(),
            ..Self::new()
        }
    }

    /// Drops every node, and reports to `ctx` from now on.
    pub(crate) fn reset(&mut self, ctx: &Context) {
        self.taffy.clear();
        self.ctx = Some(ctx.clone());
    }

    /// A node of `style` holding `node`, with `kids` as its children.
    #[track_caller]
    pub(crate) fn new_node(&mut self, style: Style, kids: &[NodeId], node: Node) -> NodeId {
        let at = Location::caller();
        let kept: Vec<NodeId> = kids
            .iter()
            .copied()
            .filter(|&kid| {
                let has = self.has(kid);
                if !has {
                    self.missing(at, kid);
                }
                has
            })
            .collect();
        let made = self.taffy.new_with_children(style, &kept).and_then(|id| {
            self.taffy.set_node_context(id, Some(node))?;
            Ok(id)
        });
        made.unwrap_or_else(|error| {
            self.failed(at, &error);
            DANGLING
        })
    }

    /// Makes `kids` the children of `id`, in place of any it had.
    #[track_caller]
    pub(crate) fn set_children(&mut self, id: NodeId, kids: &[NodeId]) {
        let at = Location::caller();
        if !self.has(id) {
            self.missing(at, id);
            return;
        }
        let kept: Vec<NodeId> = kids
            .iter()
            .copied()
            .filter(|&kid| {
                let has = self.has(kid);
                if !has {
                    self.missing(at, kid);
                }
                has
            })
            .collect();
        if let Err(error) = self.taffy.set_children(id, &kept) {
            self.failed(at, &error);
        }
    }

    #[track_caller]
    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        let node = self.taffy.get_node_context(id);
        if node.is_none() {
            self.missing(Location::caller(), id);
        }
        node
    }

    #[track_caller]
    pub(crate) fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if !self.has(id) {
            self.missing(Location::caller(), id);
        }
        self.taffy.get_node_context_mut(id)
    }

    /// The scroll container's leaf `id` carries until layout.
    #[track_caller]
    pub(crate) fn scroll(&self, id: NodeId) -> Option<&ScrollNode> {
        let at = Location::caller();
        let scroll = self.node(id).and_then(|node| node.identity.scroll());
        if scroll.is_none() {
            self.not_a_scroll(at, id);
        }
        scroll
    }

    #[track_caller]
    pub(crate) fn scroll_mut(&mut self, id: NodeId) -> Option<&mut ScrollNode> {
        let at = Location::caller();
        if self
            .node(id)
            .and_then(|node| node.identity.scroll())
            .is_none()
        {
            self.not_a_scroll(at, id);
        }
        self.taffy
            .get_node_context_mut(id)
            .and_then(|node| node.identity.scroll_mut())
    }

    /// The node's layout from the last solve; zero if it is missing.
    #[track_caller]
    pub(crate) fn layout(&self, id: NodeId) -> Layout {
        self.checked(Location::caller(), id, |taffy| taffy.layout(id).copied())
            .unwrap_or_default()
    }

    #[track_caller]
    pub(crate) fn style(&self, id: NodeId) -> &Style {
        self.checked(Location::caller(), id, |taffy| taffy.style(id))
            .unwrap_or(DEFAULT_STYLE)
    }

    #[track_caller]
    pub(crate) fn set_style(&mut self, id: NodeId, style: Style) {
        let at = Location::caller();
        if !self.has(id) {
            self.missing(at, id);
        } else if let Err(error) = self.taffy.set_style(id, style) {
            self.failed(at, &error);
        }
    }

    #[track_caller]
    pub(crate) fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.checked(Location::caller(), id, |taffy| taffy.children(id))
            .unwrap_or_default()
    }

    /// Solves the tree under `root` in the room `available`, measuring each
    /// leaf with `measure`.
    #[track_caller]
    pub(crate) fn compute(
        &mut self,
        root: NodeId,
        available: Size<AvailableSpace>,
        measure: impl FnMut(LayoutInput, NodeId, Option<&mut Node>, &Style) -> LayoutOutput,
    ) {
        let at = Location::caller();
        if !self.has(root) {
            self.missing(at, root);
        } else if let Err(error) = self
            .taffy
            .compute_layout_with_measure(root, available, measure)
        {
            self.failed(at, &error);
        }
    }

    fn has(&self, id: NodeId) -> bool {
        self.taffy.get_node_context(id).is_some()
    }

    fn checked<'t, T>(
        &'t self,
        at: &'static Location<'static>,
        id: NodeId,
        read: impl FnOnce(&'t TaffyTree<Node>) -> taffy::TaffyResult<T>,
    ) -> Option<T> {
        if !self.has(id) {
            self.missing(at, id);
            return None;
        }
        read(&self.taffy)
            .inspect_err(|error| self.failed(at, error))
            .ok()
    }

    fn missing(&self, at: &'static Location<'static>, id: NodeId) {
        if let Some(ctx) = &self.ctx {
            crate::report::report_at(
                ctx,
                Problem::Internal("taffy: missing node"),
                None,
                at,
                || format!("taffy: node {id:?} is not in the layout tree; it is skipped"),
            );
        }
    }

    fn not_a_scroll(&self, at: &'static Location<'static>, id: NodeId) {
        if let Some(ctx) = &self.ctx {
            crate::report::report_at(
                ctx,
                Problem::Internal("taffy: not a scroll container"),
                None,
                at,
                || format!("taffy: node {id:?} is not a scroll container's leaf; it is skipped"),
            );
        }
    }

    fn failed(&self, at: &'static Location<'static>, error: &taffy::TaffyError) {
        if let Some(ctx) = &self.ctx {
            crate::report::report_at(ctx, Problem::Internal("taffy: refused"), None, at, || {
                format!("taffy: {error}")
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::NodeKind;
    use crate::testing::{assert_no_reports, assert_reported};
    use taffy::prelude::TaffyMaxContent;

    fn node() -> Node {
        Node::bare(NodeKind::Div, String::new(), crate::text::tests::base())
    }

    /// A tree reporting to a context of its own, and a node that was in it
    /// before it was reset.
    fn with_gone() -> (Context, LayoutTree, NodeId) {
        let ctx = Context::default();
        let mut tree = LayoutTree::new();
        tree.reset(&ctx);
        let gone = tree.new_node(Style::default(), &[], node());
        tree.reset(&ctx);
        assert!(!tree.has(gone), "the reset dropped the node");
        assert_no_reports(&ctx);
        (ctx, tree, gone)
    }

    #[test]
    fn a_node_made_with_children_holds_them_and_its_node() {
        let ctx = Context::default();
        let mut tree = LayoutTree::new();
        tree.reset(&ctx);
        let leaf = tree.new_node(Style::default(), &[], node());
        let parent = tree.new_node(Style::default(), &[leaf], node());
        assert_eq!(tree.children(parent), vec![leaf]);
        assert!(tree.node(parent).is_some());
        tree.set_style(leaf, Style::DEFAULT);
        tree.compute(parent, Size::MAX_CONTENT, |_, _, _, _| LayoutOutput::HIDDEN);
        assert_no_reports(&ctx);
    }

    #[test]
    fn reading_a_missing_node_gives_defaults_and_reports() {
        let (ctx, tree, gone) = with_gone();
        assert_eq!(tree.layout(gone), Layout::new());
        assert_eq!(*tree.style(gone), Style::DEFAULT);
        assert!(tree.children(gone).is_empty());
        assert!(tree.node(gone).is_none());
        assert_reported(&ctx, Problem::Internal("taffy: missing node"));
    }

    #[test]
    fn writing_to_a_missing_node_does_nothing_and_reports() {
        let (ctx, mut tree, gone) = with_gone();
        tree.set_style(gone, Style::default());
        assert!(tree.node_mut(gone).is_none());
        tree.compute(gone, Size::MAX_CONTENT, |_, _, _, _| LayoutOutput::HIDDEN);
        let reports = crate::testing::reports(&ctx);
        assert_eq!(reports.len(), 3, "one per call site: {reports:#?}");
        assert_reported(&ctx, Problem::Internal("taffy: missing node"));
    }

    #[test]
    fn a_missing_child_is_left_out_and_reported() {
        let (ctx, mut tree, gone) = with_gone();
        let kept = tree.new_node(Style::default(), &[], node());
        let parent = tree.new_node(Style::default(), &[gone, kept], node());
        assert_eq!(tree.children(parent), vec![kept]);
        assert_reported(&ctx, Problem::Internal("taffy: missing node"));
    }

    #[test]
    fn set_children_replaces_them_and_leaves_out_a_missing_one() {
        let (ctx, mut tree, gone) = with_gone();
        let first = tree.new_node(Style::default(), &[], node());
        let second = tree.new_node(Style::default(), &[], node());
        let parent = tree.new_node(Style::default(), &[first], node());
        tree.set_children(parent, &[second]);
        assert_eq!(tree.children(parent), vec![second]);
        assert_no_reports(&ctx);
        tree.set_children(parent, &[gone, first]);
        assert_eq!(tree.children(parent), vec![first]);
        assert_reported(&ctx, Problem::Internal("taffy: missing node"));
    }

    #[test]
    fn a_node_that_is_no_scroll_container_has_no_scroll_and_reports() {
        let ctx = Context::default();
        let mut tree = LayoutTree::new();
        tree.reset(&ctx);
        let div = tree.new_node(Style::default(), &[], node());
        assert!(tree.scroll(div).is_none());
        assert!(tree.scroll_mut(div).is_none());
        assert_reported(&ctx, Problem::Internal("taffy: not a scroll container"));
    }
}
