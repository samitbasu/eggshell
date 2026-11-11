//! The frame pipeline: [`LayoutState::build`] gives [`Built`], whose
//! [`layout`](Built::layout) gives [`Placed`].

use std::collections::HashMap;

use egui::{Id, Rect, vec2};
use taffy::{
    AvailableSpace, Baselines, Dimension, Display, LayoutInput, LayoutOutput, NodeId, Overflow,
    Point, Position, ResolveOrZero, Size, Style, TaffyTree, compute_leaf_layout,
    style_helpers::length,
};

use crate::builder::Builder;
use crate::debug::{Debug, Phase, Timer};
use crate::dnd::{Decls, NodeDrag};
use crate::key::{ReadablePath, Seen, root_id};
use crate::measure::{Content, Measure, measure_content};
use crate::prim::Prim;
use crate::scroll::{Axis, Moved, ScrollPlan, Settle};
use crate::text::{TextStyle, VAlign, WhiteSpace};
use crate::tw::{BoxPaint, Focusability, Grouping, Sensing, Sides, Variants};

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
    /// An identified scroll container: a leaf of its tree, with the root of
    /// its content as its only child.
    Scroll,
    /// The root of a scroll container's content, laid out on its own.
    ScrollContent,
}

/// taffy's per-node context.
pub(crate) struct Node {
    pub id: Option<Id>,
    pub path: String,
    pub kind: NodeKind,
    pub paint: BoxPaint,
    pub text: TextStyle,
    pub text_valign: VAlign,
    pub prim: Option<Prim>,
    pub measure: Option<usize>,
    pub draw: Option<usize>,
    pub z: i16,
    pub inert: bool,
    pub interaction: Interaction,
    pub drag: Option<Box<NodeDrag>>,
    pub scroll: Option<ScrollNode>,
}

/// What a scroll container's leaf carries until layout.
pub(crate) struct ScrollNode {
    pub axis: Axis,
    /// The root of the content, laid out on its own.
    pub content: NodeId,
    /// The content root's size, once laid out.
    pub size: egui::Vec2,
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
    pub focusability: Focusability,
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
    pub ctx: &'f egui::Context,
    pub measures: Vec<MeasureFn<'f>>,
    pub draws: Vec<DrawFn<A>>,
    pub drag: Decls<A>,
    /// The scroll containers' leaves, an enclosing container after those
    /// it encloses.
    pub scrolls: Vec<NodeId>,
}

/// Owns the state a frame is built in; reuse one across frames.
///
/// The root's text style defaults to [`TextStyle::from_egui`], recomputed on
/// every build so theme changes apply. [`with_base_text`](Self::with_base_text)
/// replaces it, and [`set_base_text`](Self::set_base_text) changes or clears
/// that between builds. Any subtree can still override fields with text classes on a
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

    /// Sets, or with `None` clears, the root text style of the builds that
    /// follow. Without one each build follows egui's style.
    pub fn set_base_text(&mut self, base: Option<TextStyle>) {
        self.base_text = base;
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
        let _timer = Timer::start(ctx, Phase::Build);
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
            ctx,
            measures: Vec::new(),
            draws: Vec::new(),
            drag: Decls::default(),
            scrolls: Vec::new(),
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
                    text_valign: VAlign::Top,
                    prim: None,
                    measure: None,
                    draw: None,
                    z: 0,
                    inert: false,
                    interaction: Interaction::default(),
                    drag: None,
                    scroll: None,
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
        let _timer = Timer::start(ctx, Phase::Layout);
        let Frame {
            measures,
            draws,
            drag,
            scrolls,
            ..
        } = frame;
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
        let solve = |tree: &mut TaffyTree<Node>, root: NodeId, available: Size<AvailableSpace>| {
            tree.compute_layout_with_measure(root, available, |input, _, node, style| {
                let Some(node) = node else {
                    return leaf_layout(input, style, |_| {
                        LayoutOutput::from_outer_size(Size::ZERO)
                    });
                };
                leaf_layout(input, style, |input| {
                    measure_content(ctx, node_content(node, &measures), &node.text, &input)
                })
            })
            .expect("tree is well formed");
        };
        solve(
            &mut st.tree,
            root,
            Size {
                width: AvailableSpace::Definite(area.width()),
                height: AvailableSpace::Definite(area.height()),
            },
        );
        for leaf in scrolls.into_iter().rev() {
            solve_content(&mut st.tree, leaf, &solve);
        }

        let mut nodes = Vec::new();
        let mut prims = Vec::new();
        let probe = (Debug::of(ctx) == Debug::Outlines).then_some(Probe {
            ctx,
            measures: &measures,
        });
        let mut settle = Settle::new(ctx);
        collect(
            &mut st.tree,
            root,
            area.min,
            0,
            probe.as_ref(),
            &mut settle,
            &mut nodes,
            &mut prims,
        );
        settle.finish();
        drop(measures);
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
            drag,
        }
    }
}

/// Lays out the content of the scroll container `leaf`, whose own size its
/// tree has solved: at the viewport's size across the axes that do not
/// scroll and as large as it needs along those that do. Content smaller than
/// the viewport is laid out again at the viewport's size, so that it can
/// fill it.
fn solve_content(
    tree: &mut TaffyTree<Node>,
    leaf: NodeId,
    solve: &impl Fn(&mut TaffyTree<Node>, NodeId, Size<AvailableSpace>),
) {
    let own = *tree.layout(leaf).expect("node exists");
    let viewport = Size {
        width: (own.size.width - own.border.left - own.border.right).max(0.0),
        height: (own.size.height - own.border.top - own.border.bottom).max(0.0),
    };
    let (axis, root) = {
        let scroll = tree
            .get_node_context(leaf)
            .and_then(|node| node.scroll.as_ref())
            .expect("a scroll leaf has a scroll node");
        (scroll.axis, scroll.content)
    };
    let scrolls = axis.enabled();
    let mut fit = Size {
        width: (!scrolls.x).then_some(viewport.width),
        height: (!scrolls.y).then_some(viewport.height),
    };
    let mut size = Size::ZERO;
    for attempt in 0..2 {
        let mut style = tree.style(root).expect("node exists").clone();
        style.size = Size {
            width: fit.width.map_or(Dimension::auto(), length),
            height: fit.height.map_or(Dimension::auto(), length),
        };
        tree.set_style(root, style).expect("node exists");
        let available =
            |fit: Option<f32>| fit.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite);
        solve(
            tree,
            root,
            Size {
                width: available(fit.width),
                height: available(fit.height),
            },
        );
        let laid = tree.layout(root).expect("node exists").size;
        size = laid;
        let short = Size {
            width: fit.width.is_none() && laid.width < viewport.width,
            height: fit.height.is_none() && laid.height < viewport.height,
        };
        if attempt == 1 || !(short.width || short.height) {
            break;
        }
        fit = Size {
            width: fit.width.or(short.width.then_some(viewport.width)),
            height: fit.height.or(short.height.then_some(viewport.height)),
        };
    }
    tree.get_node_context_mut(leaf)
        .and_then(|node| node.scroll.as_mut())
        .expect("a scroll leaf has a scroll node")
        .size = egui::vec2(size.width, size.height);
}

/// A leaf's layout: taffy's own, which adds padding and border around what
/// `measure` reports for the content box, with the content's first baseline
/// moved down by the top inset.
///
/// Taffy hands the measure the content-box size of the space on offer but the
/// border-box size of a dimension layout has fixed (a stretched flex item);
/// both reach `measure` as content-box here, so a text leaf wraps at the
/// width it is drawn at.
fn leaf_layout(
    input: LayoutInput,
    style: &Style,
    measure: impl FnOnce(LayoutInput) -> LayoutOutput,
) -> LayoutOutput {
    let resolve = |length: taffy::LengthPercentage| {
        length.resolve_or_zero(input.parent_size.width, |_, _| 0.0)
    };
    let inset = Size {
        width: resolve(style.padding.left)
            + resolve(style.padding.right)
            + resolve(style.border.left)
            + resolve(style.border.right),
        height: resolve(style.padding.top)
            + resolve(style.padding.bottom)
            + resolve(style.border.top)
            + resolve(style.border.bottom),
    };
    let mut baselines = Baselines::NONE;
    let mut output = compute_leaf_layout(
        input,
        style,
        |_, _| 0.0,
        |known_dimensions, available_space| {
            let measured = measure(LayoutInput {
                known_dimensions: Size {
                    width: known_dimensions
                        .width
                        .map(|width| (width - inset.width).max(0.0)),
                    height: known_dimensions
                        .height
                        .map(|height| (height - inset.height).max(0.0)),
                },
                available_space,
                ..input
            });
            baselines = measured.baselines;
            measured.size
        },
    );
    let inset_top = resolve(style.padding.top) + resolve(style.border.top);
    output.baselines = Baselines {
        first: baselines.first.map(|first| first + inset_top),
        last: None,
    };
    output
}

fn node_content<'a>(node: &'a Node, measures: &'a [MeasureFn<'_>]) -> Content<'a> {
    match (&node.prim, node.measure) {
        (Some(prim), _) => Content::Prim(prim),
        (None, Some(index)) => Content::Leaf(&*measures[index]),
        (None, None) => Content::Empty,
    }
}

/// Measures leaves again at their final content width, for the debug overlay.
struct Probe<'a, 'f> {
    ctx: &'a egui::Context,
    measures: &'a [MeasureFn<'f>],
}

impl Probe<'_, '_> {
    /// The size the leaf's content wants in a content box `content` wide, or
    /// `None` for a leaf with no content. Truncated text is cut to the box
    /// by design, so it never wants more than the box's width.
    fn wanted(&self, node: &Node, content: Size<f32>) -> Option<egui::Vec2> {
        let content_of = node_content(node, self.measures);
        if matches!(content_of, Content::Empty) {
            return None;
        }
        let input = LayoutInput {
            known_dimensions: Size::NONE,
            available_space: Size {
                width: AvailableSpace::Definite(content.width),
                height: AvailableSpace::MaxContent,
            },
            ..LayoutInput::HIDDEN
        };
        let size = measure_content(self.ctx, content_of, &node.text, &input).size;
        let width = match node.text.white_space {
            WhiteSpace::Truncate => size.width.min(content.width),
            _ => size.width,
        };
        Some(vec2(width, size.height))
    }
}

/// How far past a node's border box its own painting reaches: its shadow and
/// ring, and a little for the offsets that hover and press variants apply.
fn paint_reach(paint: &BoxPaint) -> f32 {
    const OFFSETS: f32 = 16.0;
    let shadow = paint.shadow.map_or(0.0, |shadow| {
        let offset = shadow.offset[0].abs().max(shadow.offset[1].abs());
        f32::from(offset) + f32::from(shadow.blur) + f32::from(shadow.spread)
    });
    let ring = paint.ring.map_or(0.0, |ring| ring.width);
    OFFSETS + shadow.max(ring)
}

#[allow(clippy::too_many_arguments, reason = "the walk's state, passed down")]
fn collect(
    tree: &mut TaffyTree<Node>,
    id: NodeId,
    parent_origin: egui::Pos2,
    depth: usize,
    probe: Option<&Probe<'_, '_>>,
    settle: &mut Settle,
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
    let is_leaf = tree.children(id).expect("node exists").is_empty();
    let node = tree
        .get_node_context_mut(id)
        .expect("every node has a context");
    let measured = probe
        .filter(|_| is_leaf && flow != Flow::Hidden)
        .and_then(|probe| {
            let inset = Size {
                width: layout.padding.left
                    + layout.padding.right
                    + layout.border.left
                    + layout.border.right,
                height: layout.padding.top
                    + layout.padding.bottom
                    + layout.border.top
                    + layout.border.bottom,
            };
            probe.wanted(
                node,
                Size {
                    width: (layout.size.width - inset.width).max(0.0),
                    height: (layout.size.height - inset.height).max(0.0),
                },
            )
        });
    prims.push(node.prim.take());
    let scroll = node.scroll.take();
    let index = nodes.len();
    nodes.push(PlacedNode {
        id: node.id,
        path: std::mem::take(&mut node.path),
        kind: node.kind,
        depth,
        end: index + 1,
        rect,
        reach: rect.expand(paint_reach(&node.paint)),
        border,
        padding,
        overflow,
        flow,
        paint: node.paint,
        text: node.text.clone(),
        text_valign: node.text_valign,
        z: node.z,
        inert: node.inert,
        interaction: node.interaction,
        drag: node.drag.take(),
        draw: node.draw,
        scroll: None,
        measured,
    });
    for child in tree.children(id).expect("node exists") {
        let at = nodes.len();
        collect(tree, child, min, depth + 1, probe, settle, nodes, prims);
        let reach = nodes[index].reach.union(nodes[at].reach);
        nodes[index].reach = reach;
    }
    if let Some(scroll) = scroll {
        let viewport = crate::draw::inset(rect, border);
        let offset = settle.offset(
            nodes[index].id.expect("a scroll container is identified"),
            scroll.axis,
            scroll.size,
            viewport.size(),
        );
        nodes[index].scroll = Some(ScrollPlan {
            axis: scroll.axis,
            content: scroll.size,
            offset,
            moved: Moved::No,
        });
        collect(
            tree,
            scroll.content,
            viewport.min - offset,
            depth + 1,
            probe,
            settle,
            nodes,
            prims,
        );
        settle.settle(nodes, index);
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
    /// Where the node and its descendants may paint: for culling.
    pub(crate) reach: Rect,
    pub(crate) border: Sides<f32>,
    pub(crate) padding: Sides<f32>,
    pub(crate) overflow: Point<Overflow>,
    pub(crate) flow: Flow,
    paint: BoxPaint,
    text: TextStyle,
    text_valign: VAlign,
    z: i16,
    inert: bool,
    pub(crate) interaction: Interaction,
    pub(crate) drag: Option<Box<NodeDrag>>,
    pub(crate) draw: Option<usize>,
    pub(crate) scroll: Option<ScrollPlan>,
    /// What the content of a leaf wants at its final width; recorded only
    /// while [`Debug::Outlines`] is set.
    pub(crate) measured: Option<egui::Vec2>,
}

impl PlacedNode {
    /// Moves the node by `by`: the content of a scroll container that was
    /// laid out at another offset.
    pub(crate) fn translate(&mut self, by: egui::Vec2) {
        self.rect = self.rect.translate(by);
        self.reach = self.reach.translate(by);
    }

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

    /// Where the node's own text sits in a box taller than it.
    pub(crate) fn text_valign(&self) -> VAlign {
        self.text_valign
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
    pub(crate) by_id: HashMap<Id, usize>,
    pub(crate) prims: Vec<Option<Prim>>,
    pub(crate) draws: Vec<Option<DrawFn<A>>>,
    pub(crate) drag: Decls<A>,
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

    /// The paths of the nodes whose content does not fit them: the nodes the
    /// debug overlay outlines in red.
    pub fn overflowing(&self) -> Vec<&str> {
        (0..self.nodes.len())
            .filter(|&index| crate::debug::overflows(&self.nodes, index))
            .map(|index| self.nodes[index].path())
            .collect()
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
