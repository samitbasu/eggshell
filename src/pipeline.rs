//! The frame pipeline: [`LayoutState::build`] gives [`Built`], whose
//! [`layout`](Built::layout) gives [`Placed`].

use std::collections::HashMap;

use egui::{Id, Rect, vec2};
use taffy::{
    AvailableSpace, Display, NodeId, Overflow, Point, Position, Size, Style, TaffyTree,
    style_helpers::length,
};

use crate::builder::Builder;
use crate::key::{ReadablePath, Seen, root_id};
use crate::measure::{Content, Measure, measure_content};
use crate::prim::Prim;
use crate::text::TextStyle;
use crate::tw::{BoxPaint, Grouping, Sensing, Sides, Variants};

/// The kind of a node, for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// The root, as large as the area.
    Root,
    /// An anonymous container.
    Div,
    /// An identified container.
    DivId,
    /// An identified application leaf.
    Leaf,
    /// An identified text leaf.
    Span,
    /// An identified icon.
    Icon,
    /// Anonymous text.
    Text,
    /// An anonymous icon.
    Glyph,
    /// An anonymous image.
    Image,
    /// An anonymous empty box, typically a separator.
    Rule,
    /// Anonymous custom paint.
    Paint,
}

/// taffy's per-node context.
pub(crate) struct Node {
    pub id: Option<Id>,
    pub path: String,
    pub kind: NodeKind,
    pub paint: BoxPaint,
    pub text: TextStyle,
    pub prim: Option<Prim>,
    pub measure: Option<usize>,
    pub draw: Option<usize>,
    pub z: i16,
    pub inert: bool,
    pub interaction: Interaction,
}

/// Where a node's text colour comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ColorSource {
    #[default]
    Own,
    Inherited,
}

/// What a node does when the pointer meets it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Interaction {
    pub sensing: Sensing,
    pub grouping: Grouping,
    pub variants: Variants,
    pub color_source: ColorSource,
}

pub(crate) type MeasureFn<'f> = Box<dyn Fn(&Measure<'_>) -> Size<f32> + 'f>;
pub(crate) type DrawFn<A> = Box<dyn FnOnce(&mut A, &mut egui::Ui)>;

/// Build data that persists across frames so allocations are reused.
pub(crate) struct BuildState {
    pub tree: TaffyTree<Node>,
    pub seen: Seen,
    pub names: ReadablePath,
}

/// One frame's closures. `'f` is how long the build may borrow application data.
pub(crate) struct Frame<'f, A> {
    pub measures: Vec<MeasureFn<'f>>,
    pub draws: Vec<DrawFn<A>>,
}

/// Owns the state a frame is built in; reuse one across frames.
///
/// The root's text style defaults to [`TextStyle::from_egui`], recomputed on
/// every build so theme changes apply. [`with_base_text`](Self::with_base_text)
/// replaces it. Any subtree can still override fields with text classes on a
/// div.
pub struct LayoutState {
    st: BuildState,
    base_text: Option<TextStyle>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutState {
    /// An empty state.
    pub fn new() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            st: BuildState {
                tree,
                seen: Seen::default(),
                names: ReadablePath::default(),
            },
            base_text: None,
        }
    }

    /// Uses `base` as the root text style of every build instead of the
    /// egui-derived default.
    pub fn with_base_text(mut self, base: TextStyle) -> Self {
        self.base_text = Some(base);
        self
    }

    /// Builds one frame. This is the only way to obtain a [`Builder`].
    ///
    /// The root inherits the override from
    /// [`with_base_text`](Self::with_base_text) if set, else
    /// [`TextStyle::from_egui`] of `ctx`. `ctx` is also used by
    /// [`Built::layout`] to measure text. `'f` covers this borrow, `ctx`, and
    /// the application data that the frame's measure closures capture.
    pub fn build<'f, A>(
        &'f mut self,
        ctx: &'f egui::Context,
        f: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) -> Built<'f, A> {
        let base = self
            .base_text
            .clone()
            .unwrap_or_else(|| TextStyle::from_egui(ctx));
        let st = &mut self.st;
        st.tree.clear();
        st.tree.disable_rounding();
        st.seen.clear();
        st.names.clear();
        let mut frame = Frame {
            measures: Vec::new(),
            draws: Vec::new(),
        };
        let mut kids = Vec::new();
        f(&mut Builder::new(
            st,
            &mut frame,
            root_id(),
            &mut kids,
            &base,
        ));
        let root = st
            .tree
            .new_with_children(Style::default(), &kids)
            .expect("children were created in this tree");
        st.tree
            .set_node_context(
                root,
                Some(Node {
                    id: None,
                    path: "<root>".to_owned(),
                    kind: NodeKind::Root,
                    paint: BoxPaint::default(),
                    text: base,
                    prim: None,
                    measure: None,
                    draw: None,
                    z: 0,
                    inert: false,
                    interaction: Interaction::default(),
                }),
            )
            .expect("root was just created");
        Built {
            st,
            ctx,
            frame,
            root,
        }
    }
}

/// A built frame, ready to be laid out.
pub struct Built<'f, A> {
    st: &'f mut BuildState,
    ctx: &'f egui::Context,
    frame: Frame<'f, A>,
    root: NodeId,
}

impl<'f, A> Built<'f, A> {
    /// Solves layout in `area`, calling each application leaf's measure.
    ///
    /// Fonts must be available: run at least one egui pass on the context
    /// given to [`LayoutState::build`] first.
    /// The measure closures are dropped here, ending their borrows of
    /// application data. Rounding is disabled; rects are exact.
    pub fn layout(self, area: Rect) -> Placed<A> {
        let Self {
            st,
            ctx,
            frame,
            root,
        } = self;
        let Frame { measures, draws } = frame;
        st.tree
            .set_style(
                root,
                Style {
                    display: Display::Block,
                    size: Size {
                        width: length(area.width()),
                        height: length(area.height()),
                    },
                    ..Style::default()
                },
            )
            .expect("root exists");
        st.tree
            .compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(area.width()),
                    height: AvailableSpace::Definite(area.height()),
                },
                |input, _, node, _| {
                    let Some(node) = node else {
                        return taffy::LayoutOutput::from_outer_size(Size::ZERO);
                    };
                    let content = match (&node.prim, node.measure) {
                        (Some(prim), _) => Content::Prim(prim),
                        (None, Some(index)) => Content::Leaf(&*measures[index]),
                        (None, None) => Content::Empty,
                    };
                    measure_content(ctx, content, &node.text, &input)
                },
            )
            .expect("tree is well formed");
        drop(measures);

        let mut nodes = Vec::new();
        let mut prims = Vec::new();
        collect(&mut st.tree, root, area.min, 0, &mut nodes, &mut prims);
        let draws = draws.into_iter().map(Some).collect();
        let by_id = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| Some((node.id?, index)))
            .collect();
        Placed {
            nodes,
            by_id,
            prims,
            draws,
        }
    }
}

fn collect(
    tree: &mut TaffyTree<Node>,
    id: NodeId,
    parent_origin: egui::Pos2,
    depth: usize,
    nodes: &mut Vec<PlacedNode>,
    prims: &mut Vec<Option<Prim>>,
) {
    let layout = *tree.layout(id).expect("node exists");
    let min = parent_origin + vec2(layout.location.x, layout.location.y);
    let rect = Rect::from_min_size(min, vec2(layout.size.width, layout.size.height));
    let style = tree.style(id).expect("node exists");
    let flow = match (style.display, style.position) {
        (Display::None, _) => Flow::Hidden,
        (_, Position::Absolute) => Flow::Absolute,
        _ => Flow::Normal,
    };
    let overflow = style.overflow;
    let edges = |rect: taffy::Rect<f32>| Sides {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
    };
    let (border, padding) = (edges(layout.border), edges(layout.padding));
    let node = tree
        .get_node_context_mut(id)
        .expect("every node has a context");
    prims.push(node.prim.take());
    let index = nodes.len();
    nodes.push(PlacedNode {
        id: node.id,
        path: std::mem::take(&mut node.path),
        kind: node.kind,
        depth,
        end: index + 1,
        rect,
        border,
        padding,
        overflow,
        flow,
        paint: node.paint,
        text: node.text.clone(),
        z: node.z,
        inert: node.inert,
        interaction: node.interaction,
        draw: node.draw,
    });
    for child in tree.children(id).expect("node exists") {
        collect(tree, child, min, depth + 1, nodes, prims);
    }
    nodes[index].end = nodes.len();
}

/// How a node takes part in its parent's layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    Normal,
    Absolute,
    /// `display: none`: neither laid out nor drawn.
    Hidden,
}

/// One node of a laid-out frame.
#[derive(Debug)]
pub struct PlacedNode {
    id: Option<Id>,
    path: String,
    kind: NodeKind,
    depth: usize,
    /// One past the last descendant's index.
    pub(crate) end: usize,
    rect: Rect,
    pub(crate) border: Sides<f32>,
    pub(crate) padding: Sides<f32>,
    pub(crate) overflow: Point<Overflow>,
    pub(crate) flow: Flow,
    paint: BoxPaint,
    text: TextStyle,
    z: i16,
    inert: bool,
    pub(crate) interaction: Interaction,
    pub(crate) draw: Option<usize>,
}

impl PlacedNode {
    /// The egui id, for identified nodes.
    pub fn id(&self) -> Option<Id> {
        self.id
    }

    /// The readable key path. Anonymous nodes end in their kind, as in `/"props"/<text>`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// What kind of node this is.
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Distance from the root.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// The border box in absolute coordinates.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The node's box paint.
    pub fn paint(&self) -> &BoxPaint {
        &self.paint
    }

    /// The node's text style, resolved against its ancestors.
    pub fn text(&self) -> &TextStyle {
        &self.text
    }

    /// Sibling paint and hit order.
    pub fn z(&self) -> i16 {
        self.z
    }

    /// Whether the node was marked inert.
    pub fn is_inert(&self) -> bool {
        self.inert
    }

    /// Whether the node declares a sense, directly or through a variant, and
    /// so gets one hit test.
    pub fn senses(&self) -> bool {
        !self.interaction.sensing.is_none()
    }

    /// Whether the node carries a draw closure.
    pub fn has_draw(&self) -> bool {
        self.draw.is_some()
    }
}

/// A laid-out frame. Nodes are in pre-order: the root first, then each
/// node followed by its descendants.
pub struct Placed<A> {
    pub(crate) nodes: Vec<PlacedNode>,
    by_id: HashMap<Id, usize>,
    pub(crate) prims: Vec<Option<Prim>>,
    pub(crate) draws: Vec<Option<DrawFn<A>>>,
}

impl<A> Placed<A> {
    /// The rect of an identified node.
    pub fn rect(&self, id: Id) -> Option<Rect> {
        Some(self.nodes[*self.by_id.get(&id)?].rect)
    }

    /// The identified node with this id.
    pub fn node(&self, id: Id) -> Option<&PlacedNode> {
        Some(&self.nodes[*self.by_id.get(&id)?])
    }

    /// Every node, anonymous ones included, in pre-order.
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &PlacedNode> {
        self.nodes.iter()
    }

    /// The number of nodes, the root included.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Always false: the root is a node.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// One line per node: indentation by depth, the readable path, the kind and the rect.
    pub fn dump(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for node in &self.nodes {
            let Rect { min, max } = node.rect;
            writeln!(
                out,
                "{:indent$}{} {:?} [{:.1}, {:.1} .. {:.1}, {:.1}]",
                "",
                node.path,
                node.kind,
                min.x,
                min.y,
                max.x,
                max.y,
                indent = node.depth * 2,
            )
            .expect("writing to a String cannot fail");
        }
        out
    }
}
