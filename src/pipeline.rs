//! The frame pipeline: [`LayoutState::build`] gives [`Built`], whose
//! [`layout`](Built::layout) gives [`Placed`].

use std::collections::{HashMap, HashSet};

use egui::{Id, Rect, Role, Vec2, ViewportId, vec2};
use taffy::{
    AvailableSpace, Baselines, Dimension, Display, LayoutInput, LayoutOutput, LengthPercentageAuto,
    NodeId, Overflow, Point, Position, ResolveOrZero, Size, Style, TaffyTree, compute_leaf_layout,
    style_helpers::length,
};

use crate::actions::Action;
use crate::builder::Builder;
use crate::collapse::{Collapse, Flight};
use crate::commands::{Commands, Declared};
use crate::debug::{Debug, Phase, Timer};
use crate::dismiss::Opening;
use crate::dnd::{Decls, EdgeScroll, NodeDrag};
use crate::finite;
use crate::focus;
use crate::kept::Handle;
use crate::key::{ReadablePath, Seen, layout_root, root_id};
use crate::measure::{Content, Measure, measure_content};
use crate::overlay::{Anchor, AnchorLookup, Anchored, Focus, MatchWidth, Placement, place};
use crate::owned::Side;
use crate::prim::Prim;
use crate::region::{self, RegionDecl};
use crate::roots::{
    ById, Form, Home, PlacedOverlay, PlacedRoot, PlacedRootKind, Presentation, RootDecl, RootKind,
    modal_style, root_of,
};
use crate::scroll::{Axis, Bars, Description, Moved, ScrollPlan, Settle};
use crate::shell::Shell;
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
    /// TODO(prose): the root of a popped-out subtree, laid out in its window.
    Window,
    /// An identified region: a leaf that only layout sizes, with the root of
    /// its content built after the layout around it.
    Region,
    /// The root of a region's content, laid out at its leaf's content box.
    RegionContent,
}

/// Whether focus goes to a node as the overlay it is in opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OpenFocus {
    /// To the overlay's first focusable, unless another node is chosen.
    #[default]
    Elsewhere,
    /// To this node: the current colour of a swatch picker.
    Here,
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
    /// For a region's leaf, the root of its content, once built.
    pub region: Option<NodeId>,
    /// For a collapsible's wrapper that is opening or closing.
    pub collapse: Option<Collapse>,
    pub description: Option<Box<Description>>,
    pub action: Option<Action>,
    pub open_focus: OpenFocus,
    /// For a splitter, the neighbour it sizes.
    pub pane: Option<Side>,
}

impl Node {
    /// A node with no identity, paint or behaviour.
    pub(crate) fn bare(kind: NodeKind, path: String, text: TextStyle) -> Self {
        Self {
            id: None,
            path,
            kind,
            paint: BoxPaint::default(),
            text,
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
            region: None,
            collapse: None,
            description: None,
            action: None,
            open_focus: OpenFocus::Elsewhere,
            pane: None,
        }
    }
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

/// Whether a node is hovered by the hit test, or whenever the pointer is
/// over its rect: a table row stays hovered under a button in one of its
/// cells, which lies over it as a sibling, not inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum HoverBy {
    #[default]
    Hit,
    Rect,
}

/// What a node does when the pointer meets it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Interaction {
    pub sensing: Sensing,
    pub focusability: Focusability,
    pub grouping: Grouping,
    pub variants: Variants,
    pub color_source: ColorSource,
    pub hover_by: HoverBy,
}

pub(crate) type MeasureFn<'f> = Box<dyn Fn(&Measure<'_>) -> Size<f32> + 'f>;
pub(crate) type DrawFn<A> = Box<dyn FnOnce(&mut A, &mut egui::Ui)>;
pub(crate) type DismissFn<A> = Box<dyn FnOnce(&mut A)>;

/// Build data that persists across frames so allocations are reused.
pub(crate) struct BuildState {
    /// Names this state among those sharing a context: the owner a drag's
    /// gesture is tagged with.
    pub scope: Id,
    /// The id every node's path derives from.
    pub root: Id,
    pub name: Option<&'static str>,
    pub alive: std::sync::Arc<()>,
    /// Where the pass being built was started.
    pub built_at: &'static std::panic::Location<'static>,
    pub kept: Handle,
    pub tree: TaffyTree<Node>,
    pub seen: Seen,
    pub names: ReadablePath,
    pub pass: Pass,
}

/// Whether what is being built is the frame, or a candidate measured and
/// thrown away ([`Builder::max_content_width`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pass {
    Live,
    Scratch,
}

impl BuildState {
    /// A state to build a candidate in: its own tree, its own memory, and no
    /// claim on any key, so that building it leaves nothing behind.
    pub(crate) fn scratch(&self) -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            scope: self.scope,
            root: self.root,
            name: self.name,
            alive: self.alive.clone(),
            built_at: self.built_at,
            kept: Handle::default(),
            tree,
            seen: Seen::default(),
            names: ReadablePath::default(),
            pass: Pass::Scratch,
        }
    }
}

#[cfg(debug_assertions)]
impl BuildState {
    pub(crate) fn claimant(&self) -> crate::key::claims::Layout {
        crate::key::claims::Layout {
            scope: self.scope,
            alive: self.alive.clone(),
        }
    }
}

/// What a frame's closures are built around, whatever application data they
/// take. A mount's builder shares it with the page's.
pub(crate) struct FrameCore<'f> {
    pub ctx: &'f egui::Context,
    pub measures: Vec<MeasureFn<'f>>,
    /// The roots, the base first and each overlay in the order it was
    /// declared: a slot is reserved before a body is built, so an overlay
    /// declared in a body comes after the overlay it is declared in.
    pub roots: Vec<RootDecl>,
    /// Every pop-out declared, inline or popped out.
    pub pop_outs: Vec<Id>,
    /// The root whose body is being built.
    pub current: usize,
    /// Whether a collapsible is opening or closing in this frame.
    pub size_in_flight: bool,
}

/// One frame's closures. `'f` is how long the build may borrow application data.
pub(crate) struct Frame<'f, A> {
    pub core: FrameCore<'f>,
    /// The draw closures of each slot a node names, run in order.
    pub draws: Vec<Vec<DrawFn<A>>>,
    pub drag: Decls<A>,
    /// The regions declared, by the index of the root each was declared in.
    pub regions: Vec<(usize, RegionDecl<'f, A>)>,
    /// What to run when an overlay is dismissed, by the overlay's id.
    pub dismiss: HashMap<Id, DismissFn<A>>,
    /// What to run when a pop-out's window is closed or moved, by its id.
    pub windows: HashMap<Id, crate::window::Hooks<A>>,
    /// What to run for the overlays that finished leaving this frame.
    pub exited: Vec<DismissFn<A>>,
    pub build_built: Option<crate::tooltip::BuildBuilt<A>>,
    /// The command tables each root declared, by the root's index.
    pub tables: Vec<(usize, Commands<A>)>,
    /// The application data the commands' enablement is read from.
    pub app: Option<&'f A>,
    /// What runs the chords of every table declared, the page's and the
    /// mounts'.
    pub commands: Vec<Declared<A>>,
}

impl<'f, A> std::ops::Deref for Frame<'f, A> {
    type Target = FrameCore<'f>;

    fn deref(&self) -> &FrameCore<'f> {
        &self.core
    }
}

impl<'f, A> std::ops::DerefMut for Frame<'f, A> {
    fn deref_mut(&mut self) -> &mut FrameCore<'f> {
        &mut self.core
    }
}

impl<'f> FrameCore<'f> {
    /// Takes the declaration of root `at` for layout, leaving an empty one
    /// that the closures of its regions declare into.
    pub(crate) fn take_decl(&mut self, at: usize) -> RootDecl {
        std::mem::replace(&mut self.roots[at], RootDecl::base())
    }

    /// Records `leaf` as a scroll container of the root being built.
    pub(crate) fn add_scroll(&mut self, leaf: NodeId) {
        self.roots[self.current].scrolls.push(leaf);
    }
}

impl<'f, A> Frame<'f, A> {
    pub(crate) fn new(ctx: &'f egui::Context) -> Self {
        Self {
            core: FrameCore {
                ctx,
                measures: Vec::new(),
                roots: vec![RootDecl::base()],
                pop_outs: Vec::new(),
                current: 0,
                size_in_flight: false,
            },
            draws: Vec::new(),
            drag: Decls::default(),
            regions: Vec::new(),
            dismiss: HashMap::new(),
            windows: HashMap::new(),
            exited: Vec::new(),
            build_built: None,
            tables: Vec::new(),
            app: None,
            commands: Vec::new(),
        }
    }

    /// Takes the regions declared in root `at`, in the order they were
    /// declared.
    pub(crate) fn take_regions(&mut self, at: usize) -> Vec<RegionDecl<'f, A>> {
        let (taken, kept) = std::mem::take(&mut self.regions)
            .into_iter()
            .partition(|(root, _)| *root == at);
        self.regions = kept;
        taken.into_iter().map(|(_, region)| region).collect()
    }
}

/// Where a builder puts what it declares that takes application data: the
/// page's [`Frame`], or a mount's, which lifts each closure through its lens
/// into the frame around it.
pub(crate) trait Sink<'f, A>: std::ops::DerefMut<Target = FrameCore<'f>> {
    /// Runs `draw` after the draw closure in `slot`, if there is one, and
    /// returns the slot that runs both.
    fn attach_draw(&mut self, slot: Option<usize>, draw: DrawFn<A>) -> usize;
    fn add_drop(&mut self, decl: crate::dnd::DropDecl<A>) -> usize;
    fn add_end(&mut self, decl: crate::dnd::EndDecl<A>);
    /// Declares a region in the root being built.
    fn add_region(&mut self, region: RegionDecl<'f, A>);
    fn on_dismiss(&mut self, overlay: Id, then: DismissFn<A>);
    fn on_exited(&mut self, then: DismissFn<A>);
    fn window_hooks(&mut self, window: Id, hooks: crate::window::Hooks<A>);
    fn build_built(&self) -> Option<crate::tooltip::BuildBuilt<A>>;
    fn set_build_built(&mut self, build: crate::tooltip::BuildBuilt<A>);
    /// Declares the command table of the root being built, or returns it if
    /// that root has one.
    fn declare_commands(&mut self, table: &Commands<A>, app: &'f A) -> Result<(), Twice>;
    /// The command `id` names, from the tables whose chords are live in the
    /// root being built, and whether it is enabled.
    fn command(&self, id: Id) -> Option<(crate::Command<A>, bool)>;
    /// Adds what runs a table's chords.
    fn dispatch(&mut self, declared: Declared<A>);
    /// How many mounts deep the builder is.
    fn depth(&self) -> usize;
}

/// A root's command table, declared again in the same frame.
pub(crate) struct Twice;

impl<'f, A> Sink<'f, A> for Frame<'f, A> {
    fn attach_draw(&mut self, slot: Option<usize>, draw: DrawFn<A>) -> usize {
        if let Some(slot) = slot
            && let Some(at) = self.draws.get_mut(slot)
        {
            at.push(draw);
            return slot;
        }
        self.draws.push(vec![draw]);
        self.draws.len() - 1
    }

    fn add_drop(&mut self, decl: crate::dnd::DropDecl<A>) -> usize {
        self.drag.push_drop(decl)
    }

    fn add_end(&mut self, decl: crate::dnd::EndDecl<A>) {
        self.drag.push_end(decl);
    }

    fn add_region(&mut self, region: RegionDecl<'f, A>) {
        self.regions.push((self.core.current, region));
    }

    fn on_dismiss(&mut self, overlay: Id, then: DismissFn<A>) {
        self.dismiss.insert(overlay, then);
    }

    fn on_exited(&mut self, then: DismissFn<A>) {
        self.exited.push(then);
    }

    fn window_hooks(&mut self, window: Id, hooks: crate::window::Hooks<A>) {
        self.windows.insert(window, hooks);
    }

    fn build_built(&self) -> Option<crate::tooltip::BuildBuilt<A>> {
        self.build_built
    }

    fn set_build_built(&mut self, build: crate::tooltip::BuildBuilt<A>) {
        self.build_built = Some(build);
    }

    fn declare_commands(&mut self, table: &Commands<A>, app: &'f A) -> Result<(), Twice> {
        let root = self.core.current;
        if self.tables.iter().any(|(at, _)| *at == root) {
            return Err(Twice);
        }
        self.app = Some(app);
        self.tables.push((root, table.clone()));
        self.commands.push(Declared {
            root,
            within: None,
            table: crate::commands::Table::Own(table.clone()),
        });
        Ok(())
    }

    fn command(&self, id: Id) -> Option<(crate::Command<A>, bool)> {
        crate::commands::look_up(&self.core, &self.tables, self.app?, id)
    }

    fn dispatch(&mut self, declared: Declared<A>) {
        self.commands.push(declared);
    }

    fn depth(&self) -> usize {
        0
    }
}

/// Owns the state a frame is built in; reuse one across frames.
///
/// The root's text style defaults to the installed [`Style`](crate::Style)'s
/// text, else [`TextStyle::from_egui`], recomputed on every build so theme
/// changes apply. [`with_base_text`](Self::with_base_text)
/// replaces it, and [`set_base_text`](Self::set_base_text) changes or clears
/// that between builds. Any subtree can still override fields with text classes on a
/// div.
///
/// Eggshell's own state between frames (the overlay stack, the listbox
/// highlight, the layout animation) lives here, not in the egui context, so an
/// app that lays out a bar and a page in one context keeps them apart. egui
/// keeps egui's own (focus, text-edit memory, the clock), and diagnostics are
/// context-wide.
pub struct LayoutState {
    st: BuildState,
    base_text: Option<TextStyle>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::new()
    }
}

/// A fresh name for a [`LayoutState`].
fn fresh_scope() -> Id {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    root_id().with((
        "layout-state",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ))
}

/// The room an overlay would have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Room {
    /// It fits as it is, or it is not known where it would go.
    Enough,
    /// Taller than the room it would have: it must scroll.
    Short,
}

/// Whether an overlay of the `natural` size anchored to `anchor` gets the
/// room it wants under `placement`, as the last layout of the state placed
/// things. Asks [`place`] itself, so that a build-time decision to scroll
/// agrees with the clamp the layout applies.
pub(crate) fn room_for(
    ctx: &egui::Context,
    (kept, viewport): (&Handle, egui::ViewportId),
    anchor: Id,
    placement: &Placement,
    natural: Vec2,
) -> Room {
    let area = kept.viewport(viewport, |here| here.area);
    let rect = |id| ctx.read_response(id).map(|response| response.rect);
    let placement = match placement.boundary_node() {
        Some(node) => rect(node).map(|rect| placement.with_node_rect(rect)),
        None => Some(*placement),
    };
    match (area, rect(anchor), placement) {
        (Some(area), Some(anchor), Some(placement))
            if place(anchor, natural, &placement, area).available.y < natural.y =>
        {
            Room::Short
        }
        _ => Room::Enough,
    }
}

impl LayoutState {
    pub(crate) fn kept(&self) -> &Handle {
        &self.st.kept
    }

    /// This state, reached from where only `ctx` is at hand.
    pub fn shell(&self, ctx: &egui::Context) -> crate::Shell {
        crate::Shell::new(ctx, &self.st.kept)
    }

    /// An empty state.
    ///
    /// Node ids derive from a root shared by every unnamed state, so two
    /// unnamed states in one egui context must not use the same top-level
    /// key; [`named`](Self::named) keeps their ids apart.
    pub fn new() -> Self {
        Self::with_name(None)
    }

    /// An empty state whose node ids are salted by `name`.
    ///
    /// Use it for each of several layouts that share an egui context (a bar
    /// and a page): their top-level keys then cannot collide in egui's
    /// per-id memory (focus, scroll offsets, text edits). The salt is a hash
    /// of the name, so the ids, and the state egui persists under them, are
    /// the same on every run.
    ///
    /// In debug builds, another state of the context built with the same
    /// name in the same pass is reported as
    /// [`Problem::LayoutNameClash`](crate::Problem::LayoutNameClash).
    pub fn named(name: &'static str) -> Self {
        Self::with_name(Some(name))
    }

    fn with_name(name: Option<&'static str>) -> Self {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        Self {
            st: BuildState {
                scope: fresh_scope(),
                root: layout_root(name),
                name,
                alive: std::sync::Arc::default(),
                built_at: std::panic::Location::caller(),
                kept: Handle::default(),
                tree,
                seen: Seen::default(),
                names: ReadablePath::default(),
                pass: Pass::Live,
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

    /// Uses `source` for the clipboard reads and writes of this layout, in
    /// place of the system's: a test's [`FakeClipboard`](crate::testing::FakeClipboard).
    pub fn with_clipboard(self, source: std::sync::Arc<dyn crate::ClipboardSource>) -> Self {
        self.set_clipboard(Some(source));
        self
    }

    /// Sets, or with `None` clears, the clipboard source of this layout.
    pub fn set_clipboard(&self, source: Option<std::sync::Arc<dyn crate::ClipboardSource>>) {
        self.st.kept.with(|kept| kept.clipboard = source);
    }

    /// Sets, or with `None` clears, the root text style of the builds that
    /// follow. Without one each build follows egui's style.
    pub fn set_base_text(&mut self, base: Option<TextStyle>) {
        self.base_text = base;
    }

    /// Builds one frame. This is the only way to obtain a [`Builder`].
    ///
    /// The root inherits the override from
    /// [`with_base_text`](Self::with_base_text) if set, else the text of the
    /// [`Style`](crate::Style) installed in `ctx`, else
    /// [`TextStyle::from_egui`] of `ctx`. `ctx` is also used by
    /// [`Built::layout`] to measure text. `'f` covers this borrow, `ctx`, and
    /// the application data that the frame's measure closures capture.
    #[track_caller]
    pub fn build<'f, A>(
        &'f mut self,
        ctx: &'f egui::Context,
        f: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) -> Built<'f, A> {
        let _timer = Timer::start(ctx, Phase::Build);
        crate::symbols::install(ctx);
        crate::icons::install(ctx);
        let base = self
            .base_text
            .clone()
            .unwrap_or_else(|| TextStyle::default_for(ctx));
        let st = &mut self.st;
        st.built_at = std::panic::Location::caller();
        #[cfg(debug_assertions)]
        if st.name.is_some() {
            crate::key::claims::name(ctx, st.root, &st.claimant(), st.built_at);
        }
        st.tree.clear();
        st.tree.disable_rounding();
        st.seen.clear();
        st.names.clear();
        let mut frame = Frame::new(ctx);
        let mut kids = Vec::new();
        let mut builder = Builder::new(
            st,
            &mut frame,
            st.root,
            &mut kids,
            (&base, crate::builder::Ambient::PAGE),
        );
        f(&mut builder);
        crate::tooltip::declare(&mut builder);
        let root = st
            .tree
            .new_with_children(Style::default(), &kids)
            .expect("children were created in this tree");
        st.tree
            .set_node_context(
                root,
                Some(Node::bare(NodeKind::Root, "<root>".to_owned(), base)),
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
            mut frame,
            root,
        } = self;
        let _timer = Timer::start(ctx, Phase::Layout);
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
        let base = frame.take_decl(0);
        solve(
            ctx,
            &frame.measures,
            &mut st.tree,
            root,
            Size {
                width: AvailableSpace::Definite(area.width()),
                height: AvailableSpace::Definite(area.height()),
            },
        );
        let regions = frame.take_regions(0);
        settle_below(ctx, st, &mut frame, 0, base.scrolls, regions);

        let mut nodes = Vec::new();
        let mut prims = Vec::new();
        let outlines = Debug::of(ctx) == Debug::Outlines;
        let mut settle = Settle::new(Shell::new(ctx, &st.kept));
        collect(
            &mut st.tree,
            root,
            area.min,
            0,
            outlines
                .then_some(Probe {
                    ctx,
                    measures: &frame.measures,
                })
                .as_ref(),
            &mut settle,
            &mut nodes,
            &mut prims,
        );
        let mut by_id = ById::new();
        index_ids(&nodes, 0, &mut by_id);
        let mut roots = vec![PlacedRoot {
            first: 0,
            parent: None,
            home: Home::Page,
            kind: PlacedRootKind::Base,
        }];
        let scope = st.scope;
        let root = st.root;
        let page = ctx.viewport_id();
        st.kept.viewport(page, |here| here.area = Some(area));
        let mut stacks: HashMap<ViewportId, (Stack, HashSet<Id>)> = HashMap::new();
        // Regions may declare overlays, which come after those declared in
        // the build.
        let mut at = 1;
        while at < frame.roots.len() {
            let RootDecl {
                kind,
                parent,
                node,
                scrolls,
            } = frame.take_decl(at);
            let regions = frame.take_regions(at);
            let node = node.expect("a root's body was built");
            let Some(parent) = parent else {
                at += 1;
                continue;
            };
            let overlay = match kind {
                RootKind::Base => unreachable!("only the first root is the base"),
                RootKind::Window(decl) => {
                    let window = place_window(ctx, st, &frame.measures, node, decl, area);
                    let home = match window.presentation {
                        crate::window::Presentation::Native => Home::Window(roots.len()),
                        crate::window::Presentation::Embedded => Home::of(&roots, parent),
                    };
                    settle_below(ctx, st, &mut frame, at, scrolls, regions);
                    let first = nodes.len();
                    collect(
                        &mut st.tree,
                        node,
                        window.content.min,
                        0,
                        outlines
                            .then_some(Probe {
                                ctx,
                                measures: &frame.measures,
                            })
                            .as_ref(),
                        &mut settle,
                        &mut nodes,
                        &mut prims,
                    );
                    index_ids(&nodes, first, &mut by_id);
                    roots.push(PlacedRoot {
                        first,
                        parent: Some(parent),
                        home,
                        kind: PlacedRootKind::Window(window),
                    });
                    at += 1;
                    continue;
                }
                RootKind::Overlay(overlay) => overlay,
            };
            let home = Home::of(&roots, parent);
            let (viewport, area) = home
                .window(&roots)
                .map_or((page, area), |window| (window.viewport(), window.content));
            let (stack, present) = stacks
                .entry(viewport)
                .or_insert_with(|| (Stack::load(&st.kept, viewport), HashSet::new()));
            let (open, opening) = stack.enter(overlay.id, overlay.band);
            if opening == Opening::Opened
                && overlay.behavior.focus != Focus::None
                && home == Home::Page
            {
                open.restore = ctx.memory(|memory| memory.focused());
            }
            let (anchor_decl, anchor, anchored, lookup, rect, form, bounds, natural) = match overlay
                .presentation
            {
                Presentation::Floating {
                    anchor: anchor_decl,
                    placement,
                    match_width,
                } => {
                    let (anchor, anchored, lookup) =
                        resolve_anchor(anchor_decl, area, &nodes, &by_id, &roots, || {
                            *open.pointer.get_or_insert_with(|| {
                                ctx.input_for(viewport, |input| input.pointer.latest_pos())
                                    .unwrap_or(area.min)
                            })
                        });
                    let boundary = placement
                        .boundary_node()
                        .map(|id| by_id.get(&id).map(|&index| nodes[index].rect()).ok_or(id));
                    let (placement, lookup) = match boundary {
                        Some(Ok(rect)) => (placement.with_node_rect(rect), lookup),
                        Some(Err(_)) => (placement, AnchorLookup::Missing),
                        None => (placement, lookup),
                    };
                    if lookup == AnchorLookup::Missing
                        && overlay.band == crate::overlay::Band::Tooltip
                    {
                        // A tooltip opens a frame after its trigger asked,
                        // and the trigger may be gone by then.
                        if let Anchor::Node(trigger) = anchor_decl {
                            st.kept
                                .viewport(ctx.viewport_id(), |here| here.tooltips.forget(trigger));
                        }
                    } else if lookup == AnchorLookup::Missing && opening == Opening::Opened {
                        let missing = match (anchor_decl, boundary) {
                            (Anchor::Node(id), _) if !by_id.contains_key(&id) => {
                                Some(("anchored to", id))
                            }
                            (_, Some(Err(id))) => Some(("bounded by", id)),
                            _ => None,
                        };
                        if let Some((what, id)) = missing {
                            let path = st
                                .tree
                                .get_node_context(node)
                                .map_or_else(String::new, |context| context.path.clone());
                            crate::report::report_at(
                                ctx,
                                crate::Problem::MissingAnchor,
                                Some(overlay.id),
                                overlay.declared_at,
                                || {
                                    format!(
                                        "overlay {path} is {what} a node that doesn't exist in \
                                         this frame ({id:?}); it is hidden and dismissed"
                                    )
                                },
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
                    solve(ctx, &frame.measures, &mut st.tree, node, loose);
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
                            ctx,
                            &frame.measures,
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
                        placement.region(area),
                        natural,
                    )
                }
                Presentation::Modal(modal) => {
                    let screen = match home {
                        Home::Page => ctx.content_rect(),
                        Home::Window(_) => area,
                    };
                    st.tree
                        .set_style(node, modal_style(modal, screen))
                        .expect("node exists");
                    solve(
                        ctx,
                        &frame.measures,
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
                        screen,
                        screen.size(),
                    )
                }
            };
            present.insert(overlay.id);
            settle_below(ctx, st, &mut frame, at, scrolls, regions);
            let first = nodes.len();
            collect(
                &mut st.tree,
                node,
                rect.min,
                0,
                outlines
                    .then_some(Probe {
                        ctx,
                        measures: &frame.measures,
                    })
                    .as_ref(),
                &mut settle,
                &mut nodes,
                &mut prims,
            );
            index_ids(&nodes, first, &mut by_id);
            let declared_in = roots[parent]
                .overlay()
                .map_or(Anchored::Attached, |parent| parent.anchored);
            roots.push(PlacedRoot {
                first,
                parent: Some(parent),
                home,
                kind: PlacedRootKind::Overlay(PlacedOverlay {
                    id: overlay.id,
                    band: overlay.band,
                    form,
                    anchored: anchored.and(declared_in),
                    anchor,
                    bounds,
                    natural,
                    anchor_decl,
                    opening,
                    lookup,
                    behavior: overlay.behavior,
                    stack_index: 0,
                    description: overlay.description,
                    staged: overlay.staged,
                }),
            });
            at += 1;
        }
        settle.finish();
        stacks
            .entry(page)
            .or_insert_with(|| (Stack::load(&st.kept, page), HashSet::new()));
        let viewport_of = |roots: &[PlacedRoot], home: Home| {
            home.window(roots)
                .map_or(page, crate::window::PlacedWindow::viewport)
        };
        for (&viewport, (stack, present)) in &mut stacks {
            let closed = stack.keep_only(present);
            if viewport == page {
                focus::restore(ctx, &st.kept, &closed);
            } else {
                st.kept
                    .viewport(viewport, |here| here.focus.closed.extend(closed));
            }
        }
        let viewports: Vec<ViewportId> = roots
            .iter()
            .map(|root| viewport_of(&roots, root.home))
            .collect();
        for (root, viewport) in roots.iter_mut().zip(viewports) {
            if let PlacedRootKind::Overlay(overlay) = &mut root.kind {
                overlay.stack_index = stacks[&viewport]
                    .0
                    .index_of(overlay.id)
                    .expect("an open overlay is in the stack");
            }
        }
        for (viewport, (stack, _)) in stacks {
            stack.store(&st.kept, viewport);
        }
        st.kept.with(|kept| {
            kept.windows.retain(|id, _| {
                roots
                    .iter()
                    .any(|root| root.window().is_some_and(|window| window.id == *id))
            });
            kept.carried.retain(|id, _| frame.pop_outs.contains(id));
            kept.inline.retain(|id, _| frame.pop_outs.contains(id));
            for id in &frame.pop_outs {
                let popped = roots
                    .iter()
                    .any(|root| root.window().is_some_and(|window| window.id == *id));
                let inline = by_id.get(id).and_then(|&index| nodes.get(index));
                if let (false, Some(node)) = (popped, inline) {
                    kept.inline.insert(*id, node.rect().size());
                }
            }
        });
        st.kept.with(|kept| {
            kept.presences.sweep();
            let gone = kept.lists.sweep(ctx.cumulative_pass_nr());
            crate::nav::forget_lists(kept, &gone);
        });
        let Frame {
            core:
                FrameCore {
                    measures,
                    pop_outs,
                    size_in_flight,
                    ..
                },
            draws,
            drag,
            dismiss,
            windows,
            exited,
            commands,
            ..
        } = frame;
        drop(measures);
        Placed {
            ctx: ctx.clone(),
            scope,
            root,
            kept: st.kept.clone(),
            nodes,
            by_id,
            prims,
            draws,
            drag,
            dismiss,
            windows,
            pop_outs,
            exited,
            roots,
            commands,
            size_in_flight,
        }
    }
}

/// The most of the page a window that was never laid out inline opens at.
const FIRST_OPEN_SHARE: f32 = 0.75;

/// Lays out the root `node` of a popped-out subtree at the size its window
/// had when it was last drawn, or, when it opens, at the size the
/// application's frame gives it, else the size the subtree had inline, else
/// its max-content size within [`FIRST_OPEN_SHARE`] of the page; never below
/// its minimum. `area` is where an embedded window opens on the page.
fn place_window(
    ctx: &egui::Context,
    st: &mut BuildState,
    measures: &[MeasureFn<'_>],
    node: NodeId,
    decl: crate::window::WindowDecl,
    area: Rect,
) -> crate::window::PlacedWindow {
    use crate::window::{MinSize, Presentation};
    let presentation = if ctx.embed_viewports() {
        Presentation::Embedded
    } else {
        Presentation::Native
    };
    let mut style = st.tree.style(node).cloned().unwrap_or_default();
    let mut sized = |size: Option<Vec2>, available: AvailableSpace| {
        style.size = Size {
            width: size.map_or(Dimension::auto(), |size| Dimension::length(size.x)),
            height: size.map_or(Dimension::auto(), |size| Dimension::length(size.y)),
        };
        if st.tree.set_style(node, style.clone()).is_ok() {
            let available = Size {
                width: size.map_or(available, |size| AvailableSpace::Definite(size.x)),
                height: size.map_or(available, |size| AvailableSpace::Definite(size.y)),
            };
            solve(ctx, measures, &mut st.tree, node, available);
        }
        laid_size(&st.tree, node)
    };
    let min = match decl.window.min_size {
        MinSize::Content => sized(None, AvailableSpace::MinContent),
        MinSize::Points(size) => size,
    };
    let (seen, inline, chrome) = st.kept.with(|kept| {
        (
            kept.windows.get(&decl.id).and_then(|seen| seen.content),
            kept.inline.get(&decl.id).copied(),
            kept.chrome,
        )
    });
    let size = seen
        .map(|content| content.size())
        .or_else(|| decl.window.frame.map(|frame| frame.size))
        .or(inline)
        .unwrap_or_else(|| {
            sized(None, AvailableSpace::MaxContent).min(area.size() * FIRST_OPEN_SHARE)
        })
        .max(min);
    let origin = match presentation {
        Presentation::Native => egui::Pos2::ZERO,
        Presentation::Embedded => seen.map_or_else(
            || {
                let offset = chrome.map_or(Vec2::ZERO, |chrome| chrome.offset);
                decl.window
                    .frame
                    .map_or(area.center() - size / 2.0, |frame| frame.position + offset)
            },
            |content| content.min,
        ),
    };
    sized(Some(size), AvailableSpace::MaxContent);
    crate::window::PlacedWindow {
        id: decl.id,
        viewport: decl.viewport(),
        window: decl.window,
        presentation,
        content: Rect::from_min_size(origin, size),
        min,
    }
}

/// Solves the tree under `root` in the room `available`, calling the
/// application leaves' measures.
pub(crate) fn solve(
    ctx: &egui::Context,
    measures: &[MeasureFn<'_>],
    tree: &mut TaffyTree<Node>,
    root: NodeId,
    available: Size<AvailableSpace>,
) {
    crate::collapse::solve_in_motion(tree, root, |tree| {
        tree.compute_layout_with_measure(root, available, |input, _, node, style| {
            let Some(node) = node else {
                return leaf_layout(input, style, |_| LayoutOutput::from_outer_size(Size::ZERO));
            };
            leaf_layout(input, style, |input| {
                measure_content(ctx, node_content(node, measures), &node.text, &input)
            })
        })
        .expect("tree is well formed");
    });
}

/// Lays out what hangs off a root that has been solved: the content of its
/// scroll containers, an enclosing one first, and then its regions, each built
/// at its leaf's size and solved before the regions and scroll containers that
/// it declared in turn.
fn settle_below<'f, A>(
    ctx: &egui::Context,
    st: &mut BuildState,
    frame: &mut Frame<'f, A>,
    at: usize,
    scrolls: Vec<NodeId>,
    regions: Vec<RegionDecl<'f, A>>,
) {
    for leaf in scrolls.into_iter().rev() {
        solve_content(&mut st.tree, leaf, &|tree, root, available| {
            solve(ctx, &frame.measures, tree, root, available);
        });
    }
    for region in regions {
        let (scrolls, regions) = region::run(ctx, st, frame, at, region);
        settle_below(ctx, st, frame, at, scrolls, regions);
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
/// for a leaf with no content or an icon, which is rasterised to whatever box
/// it is given. Truncated text is cut to the box by design, so it never wants
/// more than the box's width.
pub(crate) fn wanted(
    ctx: &egui::Context,
    content_of: Content<'_>,
    text: &TextStyle,
    content: Size<f32>,
) -> Option<egui::Vec2> {
    if matches!(content_of, Content::Empty | Content::Prim(Prim::Icon(_))) {
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
    let rect = finite::rect_or_empty(
        Rect::from_min_size(
            parent_origin + vec2(layout.location.x, layout.location.y),
            vec2(layout.size.width, layout.size.height),
        ),
        parent_origin,
    );
    let min = rect.min;
    let style = tree.style(id).expect("node exists");
    let flow = match (style.display, style.position) {
        (Display::None, _) => Flow::Hidden,
        (_, Position::Absolute) => Flow::Absolute,
        _ => Flow::Normal,
    };
    let overflow = style.overflow;
    let edges = |rect: taffy::Rect<f32>| Sides {
        top: finite::length_or_zero(rect.top),
        right: finite::length_or_zero(rect.right),
        bottom: finite::length_or_zero(rect.bottom),
        left: finite::length_or_zero(rect.left),
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
    let region = node.region.take();
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
        collapsible: node.collapse.is_some(),
        flight: node.collapse.and_then(|collapse| collapse.flight),
        pointer_events: node.pointer_events,
        interaction: node.interaction,
        drag: node.drag.take(),
        draw: node.draw,
        draw_kind: node.draw_kind,
        scroll: None,
        description: node.description.take(),
        action: node.action,
        open_focus: node.open_focus,
        pane: node.pane,
        measured,
    });
    for child in tree.children(id).expect("node exists") {
        let at = nodes.len();
        collect(tree, child, min, depth + 1, probe, settle, nodes, prims);
        let reach = nodes[index].reach.union(nodes[at].reach);
        nodes[index].reach = reach;
    }
    if let Some(content) = region {
        let (inset, _) = region::content_box(tree, id);
        let at = nodes.len();
        collect(
            tree,
            content,
            min + inset,
            depth + 1,
            probe,
            settle,
            nodes,
            prims,
        );
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
    /// Whether the node is a collapsible's wrapper.
    pub(crate) collapsible: bool,
    flight: Option<Flight>,
    pointer_events: PointerEvents,
    pub(crate) interaction: Interaction,
    pub(crate) drag: Option<Box<NodeDrag>>,
    pub(crate) draw: Option<usize>,
    draw_kind: DrawKind,
    pub(crate) scroll: Option<ScrollPlan>,
    description: Option<Box<Description>>,
    action: Option<Action>,
    open_focus: OpenFocus,
    pub(crate) pane: Option<Side>,
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

    /// Whether the node paints a background or a border: it is a box, which
    /// the pointer cannot pass through unless it asked to.
    pub(crate) fn paints_box(&self) -> bool {
        let paints_border = [
            (self.paint.border_colors.top, self.border.top),
            (self.paint.border_colors.right, self.border.right),
            (self.paint.border_colors.bottom, self.border.bottom),
            (self.paint.border_colors.left, self.border.left),
        ]
        .into_iter()
        .any(|(color, width)| color.is_some() && width > 0.0);
        self.paint.background.is_some() || paints_border
    }

    /// Whether the node takes the pointer from what lies beneath it: it paints
    /// a box, or it is the root of an overlay, and it did not ask to let the
    /// pointer through.
    ///
    /// The draw registers a hit-test widget for such a node, and
    /// [`Placed::root_at`] asks it of the roots.
    pub(crate) fn blocks_pointer(&self) -> bool {
        self.pointer_events == PointerEvents::Auto
            && (matches!(self.kind, NodeKind::Overlay | NodeKind::Window) || self.paints_box())
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

    pub(crate) fn is_drag_source(&self) -> bool {
        self.drag.as_ref().is_some_and(|drag| drag.source.is_some())
    }

    /// For a collapsible's wrapper that is opening or closing, where it is
    /// going and where it is.
    pub fn size_flight(&self) -> Option<Flight> {
        self.flight
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

    /// Whether the overlay the node is in gives it focus as it opens.
    pub(crate) fn focuses_on_open(&self) -> bool {
        self.open_focus == OpenFocus::Here
    }

    /// Whether the node is an item of a menu, a listbox or a grid: it has an
    /// item role.
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
                        | Role::GridCell
                )
            })
    }

    /// Whether the node is an option of a listbox, which may be a row of a
    /// virtual list rather than an item of an overlay.
    pub(crate) fn is_listbox_option(&self) -> bool {
        self.description().and_then(|description| description.role) == Some(Role::ListBoxOption)
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
    pub(crate) scope: Id,
    pub(crate) root: Id,
    pub(crate) kept: Handle,
    pub(crate) nodes: Vec<PlacedNode>,
    pub(crate) by_id: HashMap<Id, usize>,
    pub(crate) prims: Vec<Option<Prim>>,
    pub(crate) draws: Vec<Vec<DrawFn<A>>>,
    pub(crate) drag: Decls<A>,
    pub(crate) dismiss: HashMap<Id, DismissFn<A>>,
    /// What to run when a pop-out's window is closed or moved, by its id.
    pub(crate) windows: HashMap<Id, crate::window::Hooks<A>>,
    /// Every pop-out declared, inline or popped out.
    pub(crate) pop_outs: Vec<Id>,
    pub(crate) exited: Vec<DismissFn<A>>,
    /// The base root, then the overlays in the order they were declared.
    pub(crate) roots: Vec<PlacedRoot>,
    /// The command tables declared, the page's and the mounts'.
    pub(crate) commands: Vec<Declared<A>>,
    /// Whether a collapsible is opening or closing in this frame.
    pub(crate) size_in_flight: bool,
}

impl<A> Placed<A> {
    /// The state of the layout this was placed by.
    pub fn shell(&self) -> Shell {
        Shell::new(&self.ctx, &self.kept)
    }

    /// The rect of an identified node.
    pub fn rect(&self, id: Id) -> Option<Rect> {
        self.node(id).map(|node| node.rect)
    }

    /// The identified node with this id.
    pub fn node(&self, id: Id) -> Option<&PlacedNode> {
        self.nodes.get(*self.by_id.get(&id)?)
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
        self.nodes
            .iter()
            .zip(overflows)
            .filter(|(_, overflows)| *overflows)
            .map(|(node, _)| node.path())
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
        let mut out = String::new();
        for node in &self.nodes {
            let Rect { min, max } = node.rect;
            out += &format!(
                "{:indent$}{} {:?} [{:.1}, {:.1} .. {:.1}, {:.1}]\n",
                "",
                node.path,
                node.kind,
                min.x,
                min.y,
                max.x,
                max.y,
                indent = node.depth * 2,
            );
        }
        out
    }
}
