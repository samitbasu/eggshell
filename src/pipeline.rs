//! The frame pipeline: [`LayoutState::build`] gives [`Built`], whose
//! [`layout`](Built::layout) gives [`Placed`].

use std::collections::{HashMap, HashSet};

use egui::{Id, Rect, Role, vec2};
use taffy::{
    AvailableSpace, Baselines, Dimension, Display, LayoutInput, LayoutOutput, LengthPercentageAuto,
    NodeId, Overflow, Point, Position, ResolveOrZero, Size, Style, TaffyTree, compute_leaf_layout,
    style_helpers::length,
};

use crate::actions::Action;
use crate::builder::Builder;
use crate::debug::{Debug, Phase, Timer};
use crate::dismiss::Opening;
use crate::dnd::{Decls, EdgeScroll, NodeDrag};
use crate::focus;
use crate::key::{ReadablePath, Seen, root_id};
use crate::measure::{Content, Measure, measure_content};
use crate::overlay::{Anchor, AnchorLookup, Anchored, Focus, MatchWidth, place};
use crate::owned::Enablement;
use crate::prim::Prim;
use crate::roots::{
    ById, Form, PlacedOverlay, PlacedRoot, PlacedRootKind, Presentation, RootDecl, RootKind,
    modal_style, root_of,
};
use crate::scroll::{Axis, Bars, Description, Moved, ScrollPlan, Settle};
use crate::stack::Stack;
use crate::text::{TextStyle, VAlign, WhiteSpace};
use crate::tw::{
    BoxPaint, Focusability, Grouping, PointerEvents, RingPlacement, Sensing, Sides, Variants,
};

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
    /// The root of an overlay, laid out on its own and placed against its
    /// anchor.
    Overlay,
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
    pub draw_kind: DrawKind,
    pub z: i16,
    pub inert: bool,
    pub pointer_events: PointerEvents,
    pub interaction: Interaction,
    pub drag: Option<Box<NodeDrag>>,
    pub scroll: Option<ScrollNode>,
    pub description: Option<Box<Description>>,
    pub action: Option<Action>,
}

/// Whether a node with this `interaction` and draw is a Tab stop by what its
/// classes declare, or its draw closure adds widgets that may be.
pub(crate) fn takes_tab(interaction: &Interaction, draw_kind: DrawKind) -> bool {
    draw_kind == DrawKind::Widgets
        || interaction
            .focusability
            .sense(interaction.sensing)
            .is_focusable()
}

/// What a node's draw closure does, for a node scrolled out of view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DrawKind {
    /// It paints. It does not run while the node is culled.
    #[default]
    Paint,
    /// It adds focusable or accessible widgets. It still runs while the node
    /// is culled, without painting.
    Widgets,
}

/// What a scroll container's leaf carries until layout.
pub(crate) struct ScrollNode {
    pub axis: Axis,
    pub edge: Option<EdgeScroll>,
    pub bars: Bars,
    pub tab: crate::scroll::TabRule,
    pub description: crate::scroll::Description,
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
pub(crate) type DismissFn<A> = Box<dyn FnOnce(&mut A)>;

/// Build data that persists across frames so allocations are reused.
pub(crate) struct BuildState {
    /// Names this state's overlay stack in the egui context.
    pub scope: Id,
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
    /// The roots, the base first and each overlay in the order it was
    /// declared: a slot is reserved before a body is built, so an overlay
    /// declared in a body comes after the overlay it is declared in.
    pub roots: Vec<RootDecl>,
    /// What to run when an overlay is dismissed, by the overlay's id.
    pub dismiss: HashMap<Id, DismissFn<A>>,
    /// The root whose body is being built.
    pub current: usize,
}

impl<A> Frame<'_, A> {
    /// Records `leaf` as a scroll container of the root being built.
    pub(crate) fn add_scroll(&mut self, leaf: NodeId) {
        self.roots[self.current].scrolls.push(leaf);
    }
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

/// A fresh name for the overlay stack of a [`LayoutState`].
fn stack_scope() -> Id {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    root_id().with((
        "overlay-stack",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ))
}

impl LayoutState {
    /// An empty state.
    pub fn new() -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            st: BuildState {
                scope: stack_scope(),
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
            roots: vec![RootDecl::base()],
            dismiss: HashMap::new(),
            current: 0,
        };
        let mut kids = Vec::new();
        f(&mut Builder::new(
            st,
            &mut frame,
            root_id(),
            &mut kids,
            (&base, Enablement::Enabled),
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
                    draw_kind: DrawKind::Paint,
                    z: 0,
                    inert: false,
                    pointer_events: PointerEvents::Auto,
                    interaction: Interaction::default(),
                    drag: None,
                    scroll: None,
                    description: None,
                    action: None,
                }),
            )
            .expect("root was just created");
        frame.roots[0].node = Some(root);
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
            roots: decls,
            dismiss,
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
        let mut decls = decls.into_iter();
        let base = decls.next().expect("a frame has a base root");
        solve(
            &mut st.tree,
            root,
            Size {
                width: AvailableSpace::Definite(area.width()),
                height: AvailableSpace::Definite(area.height()),
            },
        );
        for leaf in base.scrolls.into_iter().rev() {
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
        let mut by_id = ById::new();
        index_ids(&nodes, 0, &mut by_id);
        let mut roots = vec![PlacedRoot {
            first: 0,
            parent: None,
            kind: PlacedRootKind::Base,
        }];
        let scope = st.scope;
        let mut stack = Stack::load(ctx, scope);
        let mut present = HashSet::new();
        for decl in decls {
            let RootDecl {
                kind,
                parent,
                node,
                scrolls,
            } = decl;
            let node = node.expect("a root's body was built");
            let overlay = match kind {
                RootKind::Base => unreachable!("only the first root is the base"),
                RootKind::Overlay(overlay) => overlay,
            };
            let (open, opening) = stack.enter(overlay.id, overlay.band);
            if opening == Opening::Opened && overlay.behavior.focus != Focus::None {
                open.restore = ctx.memory(|memory| memory.focused());
            }
            let (anchor_decl, anchor, anchored, lookup, rect, form) = match overlay.presentation {
                Presentation::Floating {
                    anchor: anchor_decl,
                    placement,
                    match_width,
                } => {
                    let (anchor, anchored, lookup) =
                        resolve_anchor(anchor_decl, area, &nodes, &by_id, &roots, || {
                            *open.pointer.get_or_insert_with(|| {
                                ctx.input(|input| input.pointer.latest_pos())
                                    .unwrap_or(area.min)
                            })
                        });
                    if lookup == AnchorLookup::Missing && opening == Opening::Opened {
                        #[cfg(debug_assertions)]
                        {
                            let path = st
                                .tree
                                .get_node_context(node)
                                .map_or_else(String::new, |context| context.path.clone());
                            let Anchor::Node(id) = anchor_decl else {
                                unreachable!("only a node anchor can be missing")
                            };
                            panic!(
                                "overlay {path} is anchored to a node that doesn't exist in this frame \
                                 ({id:?}) (declared at {})",
                                overlay.declared_at
                            );
                        }
                    }
                    let mut style = st.tree.style(node).expect("node exists").clone();
                    style.min_size.width = match match_width {
                        MatchWidth::Anchor => LengthPercentageAuto::length(anchor.width()),
                        MatchWidth::Content => LengthPercentageAuto::auto(),
                    };
                    st.tree.set_style(node, style.clone()).expect("node exists");
                    let loose = Size {
                        width: AvailableSpace::MaxContent,
                        height: AvailableSpace::MaxContent,
                    };
                    solve(&mut st.tree, node, loose);
                    let natural = laid_size(&st.tree, node);
                    let mut spot = place(anchor, natural, &placement, area);
                    let room = Size {
                        width: (natural.x > spot.available.x).then_some(spot.available.x),
                        height: (natural.y > spot.available.y).then_some(spot.available.y),
                    };
                    if room.width.is_some() || room.height.is_some() {
                        style.size = Size {
                            width: room.width.map_or(Dimension::auto(), Dimension::length),
                            height: room.height.map_or(Dimension::auto(), Dimension::length),
                        };
                        st.tree.set_style(node, style).expect("node exists");
                        let definite = |room: Option<f32>| {
                            room.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite)
                        };
                        solve(
                            &mut st.tree,
                            node,
                            Size {
                                width: definite(room.width),
                                height: definite(room.height),
                            },
                        );
                        spot = spot.refit(anchor, laid_size(&st.tree, node), &placement, area);
                    }
                    (
                        anchor_decl,
                        anchor,
                        anchored.and(spot.anchored),
                        lookup,
                        spot.rect,
                        Form::Floating(spot.side),
                    )
                }
                Presentation::Modal(modal) => {
                    let screen = ctx.content_rect();
                    st.tree
                        .set_style(node, modal_style(modal, screen))
                        .expect("node exists");
                    solve(
                        &mut st.tree,
                        node,
                        Size {
                            width: AvailableSpace::Definite(screen.width()),
                            height: AvailableSpace::Definite(screen.height()),
                        },
                    );
                    (
                        Anchor::Screen,
                        screen,
                        Anchored::Attached,
                        AnchorLookup::Found,
                        screen,
                        Form::Modal(modal),
                    )
                }
            };
            present.insert(overlay.id);
            for leaf in scrolls.into_iter().rev() {
                solve_content(&mut st.tree, leaf, &solve);
            }
            let first = nodes.len();
            collect(
                &mut st.tree,
                node,
                rect.min,
                0,
                probe.as_ref(),
                &mut settle,
                &mut nodes,
                &mut prims,
            );
            index_ids(&nodes, first, &mut by_id);
            let parent = parent.expect("an overlay is declared in a root");
            let declared_in = roots[parent]
                .overlay()
                .map_or(Anchored::Attached, |parent| parent.anchored);
            roots.push(PlacedRoot {
                first,
                parent: Some(parent),
                kind: PlacedRootKind::Overlay(PlacedOverlay {
                    id: overlay.id,
                    band: overlay.band,
                    form,
                    anchored: anchored.and(declared_in),
                    anchor,
                    anchor_decl,
                    opening,
                    lookup,
                    behavior: overlay.behavior,
                    stack_index: 0,
                    description: overlay.description,
                }),
            });
        }
        settle.finish();
        focus::restore(ctx, &stack.keep_only(&present));
        for root in &mut roots {
            if let PlacedRootKind::Overlay(overlay) = &mut root.kind {
                overlay.stack_index = stack
                    .index_of(overlay.id)
                    .expect("an open overlay is in the stack");
            }
        }
        stack.store(ctx);
        drop(measures);
        let draws = draws.into_iter().map(Some).collect();
        Placed {
            ctx: ctx.clone(),
            stack: scope,
            nodes,
            by_id,
            prims,
            draws,
            drag,
            dismiss,
            roots,
        }
    }
}

fn laid_size(tree: &TaffyTree<Node>, node: NodeId) -> egui::Vec2 {
    let size = tree.layout(node).expect("node exists").size;
    vec2(size.width, size.height)
}

/// Notes the identified nodes from `from` on.
fn index_ids(nodes: &[PlacedNode], from: usize, by_id: &mut ById) {
    by_id.extend(
        nodes
            .iter()
            .enumerate()
            .skip(from)
            .filter_map(|(index, node)| Some((node.id()?, index))),
    );
}

/// The rect `anchor` stands for among the nodes collected so far, and
/// whether it can be seen: a node's rect cut by the clips and scroll
/// viewports above it. An anchor that is not among them (declared in a later
/// overlay, or not at all) cannot be seen.
fn resolve_anchor(
    anchor: Anchor,
    area: Rect,
    nodes: &[PlacedNode],
    by_id: &ById,
    roots: &[PlacedRoot],
    pointer: impl FnOnce() -> egui::Pos2,
) -> (Rect, Anchored, AnchorLookup) {
    let point = |at: egui::Pos2| {
        (
            Rect::from_min_max(at, at),
            Anchored::Attached,
            AnchorLookup::Found,
        )
    };
    match anchor {
        Anchor::Node(id) => {
            let Some(&index) = by_id.get(&id) else {
                return (area, Anchored::Detached, AnchorLookup::Missing);
            };
            let root = &roots[root_of(roots, index)];
            let seen = reachable(nodes, root.first, index, area);
            let under_hidden = root
                .overlay()
                .is_some_and(|overlay| overlay.anchored == Anchored::Detached);
            let sees = seen.min.x <= seen.max.x && seen.min.y <= seen.max.y;
            let anchored = if sees && !under_hidden {
                Anchored::Attached
            } else {
                Anchored::Detached
            };
            (nodes[index].rect(), anchored, AnchorLookup::Found)
        }
        Anchor::Point(at) => point(at),
        Anchor::Rect(rect) => (rect, Anchored::Attached, AnchorLookup::Found),
        Anchor::Pointer => point(pointer()),
        Anchor::Screen => (area, Anchored::Attached, AnchorLookup::Found),
    }
}

/// The part of the node at `index` that lies within the clips of its
/// ancestors in the root at `root`, starting from `clip`: its rect cut by
/// every `overflow` clip and scroll viewport above it. Empty (negative) when
/// the pointer cannot reach any of it.
pub(crate) fn reachable(nodes: &[PlacedNode], root: usize, index: usize, clip: Rect) -> Rect {
    let mut at = root;
    let mut clip = clip;
    while at != index {
        clip = nodes[at].clip_for_children(clip);
        at = crate::draw::children_of(nodes, at)
            .into_iter()
            .find(|&child| child <= index && index < nodes[child].end)
            .expect("the node is inside the root");
    }
    nodes[index].rect().intersect(clip)
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
    let (axis, root, gutter) = {
        let scroll = tree
            .get_node_context(leaf)
            .and_then(|node| node.scroll.as_ref())
            .expect("a scroll leaf has a scroll node");
        (scroll.axis, scroll.content, scroll.bars.gutter)
    };
    let viewport = Size {
        width: (own.size.width - own.border.left - own.border.right - gutter.x).max(0.0),
        height: (own.size.height - own.border.top - own.border.bottom - gutter.y).max(0.0),
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

/// Measures the application's leaves again at their final content width, for
/// the debug overlay: their measure closures end with the layout, so this is
/// the only time they can be asked. The library's own leaves are measured on
/// demand, by [`wanted`].
struct Probe<'a, 'f> {
    ctx: &'a egui::Context,
    measures: &'a [MeasureFn<'f>],
}

impl Probe<'_, '_> {
    fn wanted(&self, node: &Node, content: Size<f32>) -> Option<egui::Vec2> {
        match node_content(node, self.measures) {
            Content::Leaf(measure) => wanted(self.ctx, Content::Leaf(measure), &node.text, content),
            _ => None,
        }
    }
}

/// The size a leaf's content wants in a content box `content` wide, or `None`
/// for a leaf with no content. Truncated text is cut to the box by design, so
/// it never wants more than the box's width.
pub(crate) fn wanted(
    ctx: &egui::Context,
    content_of: Content<'_>,
    text: &TextStyle,
    content: Size<f32>,
) -> Option<egui::Vec2> {
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
    let size = measure_content(ctx, content_of, text, &input).size;
    let width = match text.white_space {
        WhiteSpace::Truncate => size.width.min(content.width),
        _ => size.width,
    };
    Some(vec2(width, size.height))
}

/// How far past a node's border box its own painting reaches: its shadow and
/// ring, and a little for the offsets that hover and press variants apply.
fn paint_reach(paint: &BoxPaint) -> f32 {
    const OFFSETS: f32 = 16.0;
    let shadow = paint.shadow.map_or(0.0, |shadow| {
        let offset = shadow.offset[0].abs().max(shadow.offset[1].abs());
        f32::from(offset) + f32::from(shadow.blur) + f32::from(shadow.spread)
    });
    let ring = match paint.ring_placement {
        RingPlacement::Outside => paint.ring.map_or(0.0, |ring| ring.width),
        RingPlacement::Inside => 0.0,
    };
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
        pointer_events: node.pointer_events,
        interaction: node.interaction,
        drag: node.drag.take(),
        draw: node.draw,
        draw_kind: node.draw_kind,
        scroll: None,
        description: node.description.take(),
        action: node.action,
        measured,
    });
    for child in tree.children(id).expect("node exists") {
        let at = nodes.len();
        collect(tree, child, min, depth + 1, probe, settle, nodes, prims);
        let reach = nodes[index].reach.union(nodes[at].reach);
        nodes[index].reach = reach;
    }
    if let Some(scroll) = scroll {
        let padding_box = crate::draw::inset(rect, border);
        let viewport = crate::scroll::viewport_in(padding_box, scroll.bars.gutter);
        let offset = settle.offset(
            nodes[index].id.expect("a scroll container is identified"),
            scroll.axis,
            scroll.size,
            viewport.size(),
        );
        nodes[index].scroll = Some(ScrollPlan {
            axis: scroll.axis,
            edge: scroll.edge,
            bars: scroll.bars,
            tab: scroll.tab,
            description: scroll.description,
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
    pub(crate) text: TextStyle,
    text_valign: VAlign,
    z: i16,
    inert: bool,
    pointer_events: PointerEvents,
    pub(crate) interaction: Interaction,
    pub(crate) drag: Option<Box<NodeDrag>>,
    pub(crate) draw: Option<usize>,
    draw_kind: DrawKind,
    pub(crate) scroll: Option<ScrollPlan>,
    description: Option<Box<Description>>,
    action: Option<Action>,
    /// What the content of an application-measured leaf wants at its final
    /// width; recorded only while [`Debug::Outlines`] is set. The library's
    /// own leaves are measured when asked ([`Placed::overflowing`]).
    pub(crate) measured: Option<egui::Vec2>,
}

impl PlacedNode {
    /// Moves the node by `by`: the content of a scroll container that was
    /// laid out at another offset.
    pub(crate) fn translate(&mut self, by: egui::Vec2) {
        self.rect = self.rect.translate(by);
        self.reach = self.reach.translate(by);
    }

    /// The clip the node's descendants are painted and reached within, given
    /// the clip of the node itself: its `overflow` and, for a scroll
    /// container, its viewport.
    pub(crate) fn clip_for_children(&self, clip: Rect) -> Rect {
        let padding = crate::draw::inset(self.rect, self.border);
        let inner = crate::draw::content_clip(clip, padding, self.overflow);
        match self.scroll_viewport() {
            Some(viewport) => inner.intersect(viewport),
            None => inner,
        }
    }

    /// Whether the node, by itself, lets the pointer through.
    pub(crate) fn pointer_events(&self) -> PointerEvents {
        self.pointer_events
    }

    /// Whether the node takes the pointer from the sensing nodes it covers:
    /// it paints a background or a border, or it is the root of an overlay,
    /// and it did not ask to let the pointer through.
    ///
    /// This is the one definition of a blocking box: the draw registers a
    /// blocker for such a node, and [`Placed::root_at`] asks it of the roots.
    pub(crate) fn blocks_pointer(&self) -> bool {
        let paints_border = [
            (self.paint.border_colors.top, self.border.top),
            (self.paint.border_colors.right, self.border.right),
            (self.paint.border_colors.bottom, self.border.bottom),
            (self.paint.border_colors.left, self.border.left),
        ]
        .into_iter()
        .any(|(color, width)| color.is_some() && width > 0.0);
        self.pointer_events == PointerEvents::Auto
            && (self.kind == NodeKind::Overlay || self.paint.background.is_some() || paints_border)
    }

    /// For a scroll container, its viewport: the padding box minus the
    /// gutter its bars reserve. The content is laid out in it and clipped to
    /// it; the gutter is where the bars draw.
    pub(crate) fn scroll_viewport(&self) -> Option<Rect> {
        let plan = self.scroll.as_ref()?;
        Some(crate::scroll::viewport_in(
            crate::draw::inset(self.rect, self.border),
            plan.bars.gutter,
        ))
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

    /// How the node is announced to assistive technology, if it was given a
    /// role or a name with [`NodeHandle::role`](crate::NodeHandle::role).
    pub(crate) fn description(&self) -> Option<&Description> {
        self.description.as_deref()
    }

    /// The key the node answers, if it is a default or cancel button.
    pub(crate) fn action(&self) -> Option<Action> {
        self.action
    }

    /// Whether the node is an item of a menu or a listbox: it has an item
    /// role.
    pub(crate) fn is_item(&self) -> bool {
        self.description()
            .and_then(|description| description.role)
            .is_some_and(|role| {
                matches!(
                    role,
                    Role::MenuItem
                        | Role::MenuItemCheckBox
                        | Role::MenuItemRadio
                        | Role::ListBoxOption
                )
            })
    }

    /// Whether the node must register its widgets while it is scrolled out
    /// of view: Tab can reach it and assistive technology can read it. A
    /// scroll container does, if it turns out to be focusable.
    pub(crate) fn registers(&self) -> bool {
        self.takes_tab() || self.scroll.is_some() || (self.senses() && self.draw.is_some())
    }

    /// Whether the node is a Tab stop by what its classes declare, or its
    /// draw closure adds widgets that may be.
    pub(crate) fn takes_tab(&self) -> bool {
        takes_tab(&self.interaction, self.draw_kind)
    }

    /// Whether the node's draw closure adds widgets: it runs while the node
    /// is culled.
    pub(crate) fn draws_widgets(&self) -> bool {
        self.draw.is_some() && self.registers()
    }

    /// Whether the node carries a draw closure.
    pub fn has_draw(&self) -> bool {
        self.draw.is_some()
    }
}

/// A laid-out frame. Nodes are in pre-order: the root first, then each
/// node followed by its descendants.
pub struct Placed<A> {
    pub(crate) ctx: egui::Context,
    pub(crate) stack: Id,
    pub(crate) nodes: Vec<PlacedNode>,
    pub(crate) by_id: HashMap<Id, usize>,
    pub(crate) prims: Vec<Option<Prim>>,
    pub(crate) draws: Vec<Option<DrawFn<A>>>,
    pub(crate) drag: Decls<A>,
    pub(crate) dismiss: HashMap<Id, DismissFn<A>>,
    /// The base root, then the overlays in the order they were declared.
    pub(crate) roots: Vec<PlacedRoot>,
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
    ///
    /// Independent of the overlay. A normal frame pays nothing for it; asking
    /// measures the text, icons and images of every leaf once more at its
    /// final width (about the cost of the layout's text measuring), so call
    /// it from tests and tools, not every frame. Leaves measured by an
    /// application closure ([`Builder::leaf`](crate::Builder)) cannot be
    /// measured after the layout; they are included only while
    /// [`Debug::Outlines`] is set.
    pub fn overflowing(&self) -> Vec<&str> {
        let overflows = crate::debug::overflow(&self.ctx, &self.nodes, &self.prims);
        (0..self.nodes.len())
            .filter(|&index| overflows[index])
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
