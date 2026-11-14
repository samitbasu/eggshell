//! Regions: a subtree built after the layout around it, at the size layout
//! gave its leaf.
//!
//! [`Builder::region`] adds a leaf that nothing but layout sizes. Once the
//! root it is in has been laid out (with the content of that root's scroll
//! containers), its closure runs with the leaf's content-box size and builds a
//! tree of its own, laid out at exactly that size in the same frame. What the
//! closure declares (scroll containers, overlays, more regions) is handled as
//! if it had been declared in the root the region is in: the scroll containers
//! and regions first, in the order they were declared, an enclosing region's
//! closure always before the ones inside it.

use egui::{Id, Vec2, vec2};
use taffy::{AvailableSpace, Display, NodeId, Size, Style, TaffyTree, style_helpers::length};

use crate::builder::{Ambient, Builder};
use crate::pipeline::{BuildState, Frame, Node, NodeKind, solve};
use crate::text::TextStyle;

/// What a region's closure is given, besides its size: a builder under the
/// region's key.
pub(crate) type RegionFn<'f, A> = Box<dyn FnOnce(&mut Builder<'_, 'f, A>, Vec2) + 'f>;

/// A region from the moment it is declared until its closure runs.
pub(crate) struct RegionDecl<'f, A> {
    pub(crate) leaf: NodeId,
    /// The id the region's children are keyed under.
    pub(crate) id: Id,
    pub(crate) readable: String,
    pub(crate) text: TextStyle,
    pub(crate) ambient: Ambient,
    pub(crate) body: RegionFn<'f, A>,
}

/// The distance from a laid-out leaf's border box to its content box, and the
/// content box's size.
pub(crate) fn content_box(tree: &TaffyTree<Node>, leaf: NodeId) -> (Vec2, Vec2) {
    let layout = tree.layout(leaf).expect("node exists");
    let inset = vec2(
        layout.border.left + layout.padding.left,
        layout.border.top + layout.padding.top,
    );
    let size = vec2(
        (layout.size.width - inset.x - layout.border.right - layout.padding.right).max(0.0),
        (layout.size.height - inset.y - layout.border.bottom - layout.padding.bottom).max(0.0),
    );
    (inset, size)
}

/// The style of the root of a region's content: the size of the leaf's
/// content box, with its children placed as the base root's are.
fn content_style(size: Vec2) -> Style {
    Style {
        display: Display::Block,
        size: Size {
            width: length(size.x),
            height: length(size.y),
        },
        ..Style::default()
    }
}

/// Runs the closure of `region`, declared in the root `at`, and lays out what
/// it built. Returns the scroll containers and regions it declared, which are
/// still to be laid out.
pub(crate) fn run<'f, A>(
    ctx: &egui::Context,
    st: &mut BuildState,
    frame: &mut Frame<'f, A>,
    at: usize,
    region: RegionDecl<'f, A>,
) -> (Vec<NodeId>, Vec<RegionDecl<'f, A>>) {
    let RegionDecl {
        leaf,
        id,
        readable,
        text,
        ambient,
        body,
    } = region;
    let (_, size) = content_box(&st.tree, leaf);
    frame.current = at;
    st.names.reset_to(&readable);
    let mut kids = Vec::new();
    body(
        &mut Builder::new(st, frame, id, &mut kids, (&text, ambient)),
        size,
    );
    let content = st
        .tree
        .new_with_children(content_style(size), &kids)
        .expect("children were created in this tree");
    st.tree
        .set_node_context(
            content,
            Some(Node::bare(
                NodeKind::RegionContent,
                format!("{readable}/<regioncontent>"),
                text,
            )),
        )
        .expect("node was just created");
    st.tree
        .get_node_context_mut(leaf)
        .expect("a region leaf has a context")
        .region = Some(content);
    solve(
        ctx,
        &frame.measures,
        &mut st.tree,
        content,
        Size {
            width: AvailableSpace::Definite(size.x),
            height: AvailableSpace::Definite(size.y),
        },
    );
    let declared = &mut frame.roots[at];
    (
        std::mem::take(&mut declared.scrolls),
        std::mem::take(&mut declared.regions),
    )
}
