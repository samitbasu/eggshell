//! Collapsible regions: a part of the page that opens and closes by its size.
//!
//! [`Builder::collapsible`] builds a wrapper that clips a content block. While
//! the region is opening or closing, layout gives the wrapper a fraction of the
//! size its content wants, so what lies after it moves with it. The content
//! keeps its natural size and is clipped, never squeezed.
//!
//! The progress is the presence of [`presence`](crate::presence), stepped at
//! build in the layout's kept state, so a toggle in the middle of the motion
//! reverses from where the wrapper is. The wrapper's natural size is found in
//! the same frame: [`solve`](crate::pipeline::solve) lays the tree out once
//! with every wrapper `auto`, then again with the wrappers that are in motion
//! set to their fraction of what the first pass gave them.

use core::time::Duration;

use taffy::{Dimension, NodeId, TaffyTree, style_helpers::length};

use crate::builder::{Builder, NodeHandle, Scope, Slot};
use crate::pipeline::{DismissFn, Node};
use crate::presence::Shown;
use crate::tw::{Easing, Progress, Tw, tw};

/// How long a region takes to open or close unless it says otherwise, and
/// along which curve: shadcn's accordion.
pub(crate) const DURATION: Duration = Duration::from_millis(200);
pub(crate) const EASING: Easing = Easing::Out;

/// The dimension that opens and closes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CollapseAxis {
    /// The region is as tall as its content wants, or not there: an accordion.
    #[default]
    Height,
    /// The region is as wide as its content wants, or not there: a sidebar.
    Width,
}

/// A collapsible's wrapper that is opening or closing, as laid out: how big
/// it will be and how big it is, along its axis. Read it from
/// [`PlacedNode::size_flight`](crate::PlacedNode::size_flight) to size what
/// contains a region by where the region is going, as a list that caches the
/// heights of its rows does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// The dimension that is moving.
    pub axis: CollapseAxis,
    /// The size along `axis` when the region has finished moving.
    pub target: f32,
    /// The size along `axis` in this frame.
    pub current: f32,
}

/// What a collapsible's wrapper holds for layout.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Collapse {
    pub(crate) axis: CollapseAxis,
    /// How much of its natural size the wrapper has while it opens or closes.
    pub(crate) motion: Option<Progress>,
    /// Where it is going, once laid out.
    pub(crate) flight: Option<Flight>,
}

/// A collapsible region: see [`Builder::collapsible`].
#[must_use = "a collapsible is declared by `children`"]
pub struct CollapsibleChain<'b, 'f, A> {
    b: Builder<'b, 'f, A>,
    slot: Slot,
    shown: Shown,
    axis: CollapseAxis,
    transition: Duration,
    easing: Easing,
    classes: Tw,
    content: Tw,
    on_collapsed: Option<DismissFn<A>>,
}

/// What the chain declared, for the builder.
pub(crate) struct Declared<A> {
    pub(crate) shown: Shown,
    pub(crate) axis: CollapseAxis,
    pub(crate) transition: Duration,
    pub(crate) easing: Easing,
    pub(crate) classes: Tw,
    pub(crate) content: Tw,
    pub(crate) on_collapsed: Option<DismissFn<A>>,
}

impl<'b, 'f, A> CollapsibleChain<'b, 'f, A> {
    pub(crate) fn new(b: Builder<'b, 'f, A>, slot: Slot, shown: Shown) -> Self {
        Self {
            b,
            slot,
            shown,
            axis: CollapseAxis::Height,
            transition: DURATION,
            easing: EASING,
            classes: tw(),
            content: tw(),
            on_collapsed: None,
        }
    }

    /// The dimension that opens and closes. Height unless told otherwise.
    pub fn axis(mut self, axis: CollapseAxis) -> Self {
        self.axis = axis;
        self
    }

    /// How long opening and closing take: 200 ms unless told otherwise;
    /// zero opens and closes at once, as does a style whose egui
    /// `animation_time` is zero.
    pub fn transition(mut self, duration: Duration) -> Self {
        self.transition = duration;
        self
    }

    /// The timing function of opening and closing: ease-out unless told
    /// otherwise.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// `ease-linear`.
    pub fn ease_linear(self) -> Self {
        self.easing(Easing::Linear)
    }

    /// `ease-in`.
    pub fn ease_in(self) -> Self {
        self.easing(Easing::In)
    }

    /// `ease-out`.
    pub fn ease_out(self) -> Self {
        self.easing(Easing::Out)
    }

    /// `ease-in-out`.
    pub fn ease_in_out(self) -> Self {
        self.easing(Easing::InOut)
    }

    /// The wrapper's classes: how it sits among its siblings (width, flex
    /// grow, margin). Its padding and border do not open and close with it;
    /// put those on [`content`](Self::content). The wrapper clips.
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// The content block's classes: padding, background, direction and gap of
    /// what the body builds. The block is as large as its content wants, and
    /// does not shrink with the wrapper.
    pub fn content(mut self, content: Tw) -> Self {
        self.content = content;
        self
    }

    /// Runs `then` once the region has finished closing, in the frame's draw:
    /// the place to drop what the region was about from the application's
    /// state, as [`on_exited`](crate::overlay::OverlayChain::on_exited) does
    /// for an overlay.
    pub fn on_collapsed(mut self, then: impl FnOnce(&mut A) + 'static) -> Self {
        self.on_collapsed = Some(Box::new(then));
        self
    }
}

impl<'b, 'f, A> CollapsibleChain<'b, 'f, A> {
    /// Builds the content, which is not built while the region is closed and
    /// not moving. The node that is returned is the wrapper.
    ///
    /// A closed region is a wrapper of no size and no children, so a parent's
    /// `gap` still counts it: put spacing in the content's padding.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) -> NodeHandle<'b, 'f, A> {
        self.children_with_header(|_, _| {}, body)
    }

    /// [`children`](Self::children), with a header built just before the
    /// wrapper, as a sibling of it. The header is given how far open the
    /// region is, the eased progress its size follows: use it for what moves
    /// with the region, such as a chevron turning over, so that the two cannot
    /// drift. Zero is closed and one is open; it is the same value in the
    /// frame, and a style without animation time snaps it as it snaps the
    /// size.
    #[track_caller]
    pub fn children_with_header(
        self,
        header: impl FnOnce(&mut Scope<'_, 'f, A>, Progress),
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'b, 'f, A> {
        let Self {
            b,
            slot,
            shown,
            axis,
            transition,
            easing,
            classes,
            content,
            on_collapsed,
        } = self;
        b.collapsible_with(
            &slot,
            Declared {
                shown,
                axis,
                transition,
                easing,
                classes,
                content,
                on_collapsed,
            },
            header,
            body,
        )
    }
}

/// The wrappers in motion under `root`, with how far each has opened.
fn in_motion(tree: &TaffyTree<Node>, root: NodeId) -> Vec<(NodeId, CollapseAxis, Progress)> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if let Some((axis, progress)) = tree
            .get_node_context(node)
            .and_then(|context| context.collapse)
            .and_then(|collapse| Some((collapse.axis, collapse.motion?)))
        {
            found.push((node, axis, progress));
        }
        stack.extend(tree.children(node).expect("node exists"));
    }
    found
}

fn set_extent(tree: &mut TaffyTree<Node>, node: NodeId, axis: CollapseAxis, extent: Dimension) {
    let mut style = tree.style(node).expect("node exists").clone();
    match axis {
        CollapseAxis::Height => style.size.height = extent,
        CollapseAxis::Width => style.size.width = extent,
    }
    tree.set_style(node, style).expect("node exists");
}

/// Lays out the tree under `root` with `compute`, and again if a wrapper is
/// in motion, with it at its fraction of the size the first layout gave it.
pub(crate) fn solve_in_motion(
    tree: &mut TaffyTree<Node>,
    root: NodeId,
    mut compute: impl FnMut(&mut TaffyTree<Node>),
) {
    let moving = in_motion(tree, root);
    for &(node, axis, _) in &moving {
        set_extent(tree, node, axis, Dimension::auto());
    }
    compute(tree);
    if moving.is_empty() {
        return;
    }
    for &(node, axis, progress) in &moving {
        let natural = tree.layout(node).expect("node exists").size;
        let natural = match axis {
            CollapseAxis::Height => natural.height,
            CollapseAxis::Width => natural.width,
        };
        let current = natural * f32::from(progress);
        set_extent(tree, node, axis, length(current));
        if let Some(collapse) = tree
            .get_node_context_mut(node)
            .and_then(|context| context.collapse.as_mut())
        {
            collapse.flight = Some(Flight {
                axis,
                target: natural,
                current,
            });
        }
    }
    compute(tree);
}

/// How much farther the wrappers moving along `axis` under `node` have to go
/// than they have gone: the sum of target − current. What a container whose
/// height is cached while it is laid out (a virtual list's row) adds to its
/// measured size to know where it is going. Valid once the tree is solved.
pub(crate) fn flight_in(tree: &TaffyTree<Node>, node: NodeId, axis: CollapseAxis) -> f32 {
    let mut remaining = 0.0;
    let mut stack = vec![node];
    while let Some(at) = stack.pop() {
        if let Some(flight) = tree
            .get_node_context(at)
            .and_then(|context| context.collapse)
            .and_then(|collapse| collapse.flight)
            .filter(|flight| flight.axis == axis)
        {
            remaining += flight.target - flight.current;
        }
        stack.extend(tree.children(at).expect("node exists"));
    }
    remaining
}

#[cfg(test)]
mod tests {
    use egui::{Color32, FontFamily};
    use taffy::Style;

    use super::*;
    use crate::pipeline::NodeKind;
    use crate::text::{TextSpec, TextStyle};

    fn node(flight: Option<Flight>) -> Node {
        let text = TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: FontFamily::Proportional,
            },
            Color32::BLACK,
        );
        let mut node = Node::bare(NodeKind::Div, String::new(), text);
        node.collapse = flight.map(|flight| Collapse {
            axis: flight.axis,
            motion: None,
            flight: Some(flight),
        });
        node
    }

    fn flight(axis: CollapseAxis, target: f32, current: f32) -> Option<Flight> {
        Some(Flight {
            axis,
            target,
            current,
        })
    }

    #[test]
    fn the_flight_in_a_subtree_sums_what_is_left_along_the_axis() {
        let mut tree = TaffyTree::<Node>::new();
        let mut leaf = |f| {
            tree.new_leaf_with_context(Style::default(), node(f))
                .expect("leaf")
        };
        let opening = leaf(flight(CollapseAxis::Height, 100.0, 30.0));
        let closing = leaf(flight(CollapseAxis::Height, 50.0, 80.0));
        let sideways = leaf(flight(CollapseAxis::Width, 40.0, 0.0));
        let still = leaf(None);
        let row = tree
            .new_with_children(Style::default(), &[opening, closing, sideways, still])
            .expect("row");
        tree.set_node_context(row, Some(node(None))).expect("row");
        assert_eq!(flight_in(&tree, row, CollapseAxis::Height), 70.0 - 30.0);
        assert_eq!(flight_in(&tree, row, CollapseAxis::Width), 40.0);
        assert_eq!(flight_in(&tree, still, CollapseAxis::Height), 0.0);
    }
}
