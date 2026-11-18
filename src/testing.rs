//! Testing an eggshell UI: headless layout, a harness that drives the real
//! path, continuity recording, and the roles to query controls by.
//!
//! This is ordinary code, not `cfg(test)`, so the tests of a crate that uses
//! eggshell can use it. It needs nothing beyond egui, so it is not behind a
//! feature.
//!
//! There are two ways in:
//!
//! - [`Headless`] builds and lays a view out without a window, a GPU or a
//!   draw, and answers where its nodes are. Use it for layout: sizes,
//!   positions, what a change of state moves ([`Placed::diff`],
//!   [`Change::moved`]). Nothing is drawn, so no input is handled.
//! - An [`egui_kittest`](https://docs.rs/egui_kittest) `Harness` runs whole
//!   frames: build, layout, draw, and the input of each frame. Use it for
//!   behaviour: clicks, keys, focus, overlays, motion.
//!
//! # Headless
//!
//! ```
//! use eggshell_ui::testing::Headless;
//! use eggshell_ui::{Builder, tw};
//! use egui::vec2;
//!
//! #[derive(Hash, Debug)]
//! enum Part { Card, Name }
//!
//! fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
//!     b.div_id(Part::Card, tw().w_px(200.0).p(4.0), |b| {
//!         b.span(Part::Name, tw(), "Mono");
//!     });
//! }
//!
//! let placed = Headless::new(vec2(400.0, 300.0)).layout(&(), view);
//! let card = placed.at(Part::Card);
//! assert_eq!(card.rect().width(), 200.0);
//! assert_eq!(card.text("Mono").rect(), card.at(Part::Name).rect());
//! assert!(card.try_at(Part::Card).is_none());
//!
//! let wider = Headless::new(vec2(400.0, 300.0)).layout(&(), |b, _| {
//!     b.div_id(Part::Card, tw().w_px(250.0).p(4.0), |b| {
//!         b.span(Part::Name, tw(), "Mono");
//!     });
//! });
//! let changes = placed.diff(&wider);
//! assert!(changes.iter().any(|change| change.path() == "/Card"));
//! ```
//!
//! # A harness
//!
//! A harness frame builds, lays out and draws a [`LayoutState`] as an app
//! does. Set the context up in `with_context_setup`: kittest draws a first
//! frame inside `build_ui_state`, so a style installed after it is missing
//! from that frame, and recording started after it misses it.
//!
//! ```
//! use eggshell_ui::{Builder, LayoutState, Style, tw};
//! use egui_kittest::Harness;
//! use egui_kittest::kittest::Queryable as _;
//!
//! #[derive(Default)]
//! struct App { clicks: u32 }
//!
//! fn view(b: &mut Builder<'_, '_, App>, _: &App) {
//!     b.div(tw().p(4.0), |b| {
//!         b.button("save", "Save").on_click(|app: &mut App| app.clicks += 1);
//!     });
//! }
//!
//! let mut harness = Harness::builder()
//!     .with_size(egui::vec2(300.0, 200.0))
//!     .with_context_setup(|ctx| {
//!         Style::default().install(ctx);
//!         eggshell_ui::testing::record(ctx);
//!     })
//!     .build_ui_state(
//!         |ui, (layout, app): &mut (LayoutState, App)| {
//!             let placed = layout.build(ui.ctx(), |b| view(b, app)).layout(ui.max_rect());
//!             placed.draw(app, ui);
//!         },
//!         (LayoutState::new(), App::default()),
//!     );
//!
//! harness.get_by_role_and_label(egui::Role::Button, "Save").click();
//! harness.run();
//! assert_eq!(harness.state().1.clicks, 1);
//! ```
//!
//! Three things about kittest's defaults:
//!
//! - **Motion.** kittest sets egui's `animation_time` to zero, and eggshell
//!   takes a zero as reduced motion: transitions, slides and overlay motion
//!   jump to their end. A test of motion sets it back in the context setup
//!   (`ctx.global_style_mut(|style| style.animation_time = 1.0 / 12.0)`).
//! - **Settling.** `harness.run()` steps until nothing asks for a repaint, at
//!   most `max_steps` times, 4 by default, and panics past that. With motion
//!   on, a transition takes several frames to settle, so raise it
//!   (`.with_max_steps(..)`) or step a known number of frames with
//!   `run_steps`.
//! - **The step.** The default step is 0.25 s, so a double click sent as a
//!   press and release one step apart falls outside egui's 0.3 s window and
//!   never registers. A test of a double click builds its harness with
//!   `with_step_dt(1.0 / 60.0)`.
//!
//! # Querying by role
//!
//! Query a control by its accessibility role and label
//! (`get_by_role_and_label`): a label alone is ambiguous when a heading
//! repeats a button's name. Each builder method names its control's role:
//!
//! | Control | Role |
//! |---|---|
//! | [`button`](crate::Builder::button), [`toggle`](crate::Builder::toggle), a menu's trigger | `Button` |
//! | [`toggle_item`](crate::Builder::toggle_item), each option of [`radio_group`](crate::Builder::radio_group) and [`segmented`](crate::Builder::segmented) | `RadioButton` |
//! | [`checkbox`](crate::Builder::checkbox) | `CheckBox` |
//! | [`switch`](crate::Builder::switch) | `Switch` |
//! | [`slider`](crate::Builder::slider) | `Slider` |
//! | [`select`](crate::Builder::select): trigger, list, option | `ComboBox`, `ListBox`, `ListBoxOption` |
//! | [`dropdown_menu`](crate::Builder::dropdown_menu): menu, items | `Menu`, `MenuItem`, `MenuItemCheckBox`, `MenuItemRadio` |
//! | [`stepper`](crate::Builder::stepper): field, buttons | `SpinButton`, `Button` ("Increase …", "Decrease …") |
//! | [`progress`](crate::Builder::progress) | `ProgressIndicator` |
//! | [`text_field`](crate::Builder::text_field) | `TextInput` |
//! | [`table`](crate::Builder::table): table, rows, header cells, cells | `Table`, `Row`, `ColumnHeader`, `Cell` |
//!
//! A node of your own gets a role and a name with
//! [`NodeHandle::role`](crate::NodeHandle::role) and
//! [`named`](crate::NodeHandle::named).
//!
//! # Continuity
//!
//! [`record`] collects every node each frame draws, and [`Timeline`] checks
//! the frames for what a user would see as a glitch: a node that jumps
//! instead of sliding, a gap of a frame, a double paint, a layout still
//! moving at the end of the run, an overlay off screen. Start recording in
//! the context setup, as above, so the first frame is in it; drive the
//! harness; then check:
//!
//! ```
//! # use eggshell_ui::{Builder, LayoutState, Style, tw};
//! # use egui_kittest::Harness;
//! # let mut harness = Harness::builder()
//! #     .with_context_setup(|ctx| {
//! #         Style::default().install(ctx);
//! #         eggshell_ui::testing::record(ctx);
//! #     })
//! #     .build_ui_state(
//! #         |ui, layout: &mut LayoutState| {
//! #             let view = |b: &mut Builder<'_, '_, ()>| { b.div_id("card", tw().w_px(80.0).h_px(40.0), |_| {}); };
//! #             layout.build(ui.ctx(), view).layout(ui.max_rect()).draw(&mut (), ui);
//! #         },
//! #         LayoutState::new(),
//! #     );
//! harness.run_steps(10);
//! eggshell_ui::testing::Timeline::recorded(&harness.ctx).assert_continuous();
//! ```
//!
//! The tolerances ([`Timeline::max_step`], [`Timeline::settle_window`], …)
//! and the exceptions ([`Timeline::allow_jump`]) are set on the timeline
//! before it is checked.
//!
//! # Virtual lists
//!
//! The rows of a [`virtual_list`](crate::Builder::virtual_list) are addressed
//! by the keys the application gave them, as if the list held them directly:
//! `placed.at("list").at(Row(2))`. A row outside the viewport and its overscan
//! is not built, so [`try_at`](Query::try_at) is `None` for it: a missing row
//! is one scrolled out of reach, not a wrong key, until the list is scrolled
//! to it.
//!
//! Each row sits under its item key, and the nodes the row builder makes sit
//! under that: `placed.at(Region).at(Part::List).at(item_key).at(Part::Row(item_key))`.

#[cfg(all(feature = "screenshots", target_os = "linux"))]
pub mod screenshot;

use std::collections::HashMap;

use egui::{Id, Rect, Theme, Vec2};

pub use crate::continuity::{
    Interrupted, Motion, NodeFrame, NodeRef, OverlayFrame, Points, Slide, SlideId, Timeline,
    Violation, ViolationKind, laid_out_rect, leaves_bounds, record,
};
use crate::key::{Key, child_id};
use crate::nodes::Nodes;
pub use crate::pipeline::Placed;
use crate::pipeline::{LayoutState, PlacedNode};
use crate::prim::Prim;
use crate::{Builder, OverlayRef, Problem, Report, TextStyle};

type Prepare = Box<dyn FnOnce(&egui::Context)>;

/// An egui context that lays views out without drawing them.
///
/// ```
/// use eggshell_ui::testing::Headless;
/// use eggshell_ui::{Builder, tw};
/// use egui::{Theme, vec2};
///
/// let mut headless = Headless::new(vec2(400.0, 300.0)).theme(Theme::Light);
/// let view = |b: &mut Builder<'_, '_, ()>, _: &()| {
///     b.div_id("box", tw().w_px(80.0).h_px(20.0), |_| {});
/// };
/// let first = headless.layout(&(), view);
/// let second = headless.layout(&(), view);
/// assert_eq!(first.at("box").rect().width(), 80.0);
/// assert!(first.diff(&second).is_empty());
/// ```
///
/// It owns its [`egui::Context`]. The first [`layout`](Self::layout) runs the
/// [`prepare`](Self::prepare) closures, then one warm-up pass so that fonts
/// exist; each `layout` then runs one pass that builds and lays out at the
/// given size, [theme](Self::theme) and [scale](Self::pixels_per_point).
pub struct Headless {
    ctx: egui::Context,
    size: Vec2,
    base_text: Option<TextStyle>,
    state: LayoutState,
    theme: Option<Theme>,
    pixels_per_point: PixelsPerPoint,
    pending: Vec<Prepare>,
    warm: bool,
}

impl Headless {
    /// A headless context whose layouts fill `size`, in points.
    pub fn new(size: Vec2) -> Self {
        Self {
            ctx: egui::Context::default(),
            size,
            base_text: None,
            state: LayoutState::new(),
            theme: None,
            pixels_per_point: PixelsPerPoint::default(),
            pending: Vec::new(),
            warm: false,
        }
    }

    /// Runs `setup` on the context once, at the first [`layout`](Self::layout)
    /// after the theme is set and before the warm-up pass, so fonts it sets apply to every layout.
    pub fn prepare(mut self, setup: impl FnOnce(&egui::Context) + 'static) -> Self {
        self.pending.push(Box::new(setup));
        self
    }

    /// The root text style of every layout, as [`LayoutState::with_base_text`].
    pub fn base_text(mut self, base: TextStyle) -> Self {
        self.base_text = Some(base);
        self
    }

    /// Names the layout's [`LayoutState`], for a context that other named
    /// layouts share.
    pub fn named(mut self, name: &'static str) -> Self {
        self.state = LayoutState::named(name);
        self
    }

    /// Lays views out under this theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// TODO(prose): measures text as a display of `scale` draws it.
    pub fn pixels_per_point(mut self, scale: PixelsPerPoint) -> Self {
        self.pixels_per_point = scale;
        self
    }

    /// The context the layouts run on.
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    /// TODO(prose): the reports of the context, as [`reports`].
    ///
    /// ```
    /// use eggshell_ui::testing::Headless;
    /// use egui::vec2;
    ///
    /// assert!(Headless::new(vec2(10.0, 10.0)).reports().is_empty());
    /// ```
    pub fn reports(&self) -> Vec<Report> {
        reports(&self.ctx)
    }

    /// Builds and lays out `view` of `app`; nothing is drawn.
    ///
    /// `view` gets `app` for the frame's lifetime `'f`, as an application's
    /// own root does, so it may hand borrows of `app` to the nodes it builds.
    pub fn layout<A>(
        &mut self,
        app: &A,
        view: impl for<'f> FnOnce(&mut Builder<'_, 'f, A>, &'f A),
    ) -> Placed<A> {
        self.warm_up();
        let area = Rect::from_min_size(egui::Pos2::ZERO, self.size);
        let Self {
            ctx,
            size,
            base_text,
            state,
            theme,
            pixels_per_point,
            ..
        } = self;
        in_pass(ctx, *size, *theme, *pixels_per_point, |ctx| {
            state.set_base_text(base_text.clone());
            state.build(ctx, |b| view(b, app)).layout(area)
        })
    }

    fn warm_up(&mut self) {
        if std::mem::replace(&mut self.warm, true) {
            return;
        }
        if let Some(theme) = self.theme {
            self.ctx.set_theme(theme);
        }
        for setup in std::mem::take(&mut self.pending) {
            setup(&self.ctx);
        }
        in_pass(
            &self.ctx,
            self.size,
            self.theme,
            self.pixels_per_point,
            |_| {},
        );
    }
}

/// TODO(prose): the reports kept in `ctx`, oldest first.
///
/// ```
/// use eggshell_ui::testing::reports;
///
/// assert!(reports(&egui::Context::default()).is_empty());
/// ```
pub fn reports(ctx: &egui::Context) -> Vec<Report> {
    crate::report::reports(ctx)
}

/// TODO(prose): the reports kept in `ctx`, leaving none.
///
/// ```
/// use eggshell_ui::testing::{reports, take_reports};
///
/// let ctx = egui::Context::default();
/// assert!(take_reports(&ctx).is_empty());
/// assert!(reports(&ctx).is_empty());
/// ```
pub fn take_reports(ctx: &egui::Context) -> Vec<Report> {
    crate::report::take(ctx)
}

/// TODO(prose): the first report of `problem` in `ctx`; panics if there is none.
///
/// ```
/// use eggshell_ui::Problem;
/// use eggshell_ui::testing::{Headless, assert_reported};
/// use eggshell_ui::{Builder, tw};
/// use egui::vec2;
///
/// let mut headless = Headless::new(vec2(100.0, 100.0));
/// headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
///     b.div_id("a", tw(), |_| {});
///     b.div_id("a", tw(), |_| {});
/// });
/// let report = assert_reported(headless.ctx(), Problem::DuplicateKey);
/// assert!(report.message.contains("/\"a\""));
/// ```
#[track_caller]
#[expect(clippy::panic, reason = "an assertion fails the test")]
pub fn assert_reported(ctx: &egui::Context, problem: Problem) -> Report {
    let reports = reports(ctx);
    match reports.iter().find(|report| report.problem == problem) {
        Some(report) => report.clone(),
        None => panic!("expected a {problem:?} report, got {reports:#?}"),
    }
}

/// TODO(prose): panics if `ctx` keeps any report.
///
/// ```
/// use eggshell_ui::testing::assert_no_reports;
///
/// assert_no_reports(&egui::Context::default());
/// ```
#[track_caller]
#[expect(clippy::panic, reason = "an assertion fails the test")]
pub fn assert_no_reports(ctx: &egui::Context) {
    let reports = reports(ctx);
    if !reports.is_empty() {
        panic!("expected no reports, got {reports:#?}");
    }
}

fn in_pass<T>(
    ctx: &egui::Context,
    size: Vec2,
    theme: Option<Theme>,
    pixels_per_point: PixelsPerPoint,
    run: impl FnOnce(&egui::Context) -> T,
) -> T {
    if let Some(theme) = theme {
        ctx.set_theme(theme);
    }
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..egui::RawInput::default()
    };
    input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(pixels_per_point.into());
    ctx.begin_pass(input);
    let out = run(ctx);
    ctx.end_pass().drop_without_applying_deltas();
    out
}

/// TODO(prose): the physical pixels a point spans on a display: finite and positive.
///
/// ```
/// use eggshell_ui::testing::PixelsPerPoint;
///
/// assert_eq!(f32::from(PixelsPerPoint::new(2.0)), 2.0);
/// assert_eq!(f32::from(PixelsPerPoint::default()), 1.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PixelsPerPoint(f32);

impl PixelsPerPoint {
    /// TODO(prose): a display where a point spans `scale` pixels, or one pixel
    /// if `scale` is not finite and positive.
    pub fn new(scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            Self(scale)
        } else {
            // TODO(#279): report a scale that is not finite and positive.
            Self::default()
        }
    }
}

impl Default for PixelsPerPoint {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<PixelsPerPoint> for f32 {
    fn from(scale: PixelsPerPoint) -> f32 {
        scale.0
    }
}

#[derive(Clone, Copy)]
struct Layout<'p> {
    nodes: &'p Nodes,
    prims: &'p [Option<Prim>],
    by_id: &'p HashMap<Id, usize>,
}

/// A node that shows text, its index and the text.
type Shown<'p> = (&'p PlacedNode, usize, std::borrow::Cow<'p, str>);

impl<'p> Layout<'p> {
    fn node(&self, index: usize) -> Option<&'p PlacedNode> {
        self.nodes.get(index)
    }
}

/// A place in a laid-out frame, reached by chaining [`at`](Self::at) with the
/// keys the view was built with. Each key is hashed exactly as the builder
/// hashes it, so a query finds the node the builder made.
///
/// A key that names a [`Builder::scope`] rather than a node is passed
/// through; only [`rect`](Self::rect) and [`id`](Self::id) need a node.
#[derive(Clone)]
pub struct Query<'p> {
    layout: Layout<'p>,
    path: String,
    id: Option<Id>,
    index: Option<usize>,
}

impl<A> Placed<A> {
    fn layout_view(&self) -> Layout<'_> {
        Layout {
            nodes: &self.nodes,
            prims: &self.prims,
            by_id: &self.by_id,
        }
    }

    fn root_query(&self) -> Query<'_> {
        Query {
            layout: self.layout_view(),
            path: String::new(),
            id: Some(self.root),
            index: Some(0),
        }
    }

    /// The node declared with `key` at the top of the view.
    ///
    /// # Panics
    ///
    /// If there is none. The message names the path and what exists beside it.
    #[track_caller]
    pub fn at<K: Key>(&self, key: K) -> Query<'_> {
        self.root_query().at(key)
    }

    /// [`at`](Self::at), or `None` when nothing is there.
    pub fn try_at<K: Key>(&self, key: K) -> Option<Query<'_>> {
        self.root_query().try_at(key)
    }

    /// TODO(prose): the overlay declared with `key` at the top of the view; panics if it is not open.
    ///
    /// ```
    /// use eggshell_ui::overlay::Anchor;
    /// use eggshell_ui::testing::Headless;
    /// use eggshell_ui::{Builder, tw};
    /// use egui::{pos2, vec2};
    ///
    /// let placed = Headless::new(vec2(200.0, 200.0)).layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
    ///     b.overlay("menu").anchor(Anchor::Point(pos2(10.0, 10.0))).children(|s| {
    ///         s.div_id("item", tw().w_px(40.0).h_px(20.0), |_| {});
    ///     });
    /// });
    /// assert_eq!(placed.expect_overlay("menu").size(), vec2(40.0, 20.0));
    /// ```
    #[track_caller]
    #[expect(clippy::panic, reason = "a test lookup fails the test")]
    pub fn expect_overlay<K: Key>(&self, key: K) -> OverlayRef<'_> {
        let open = || list(self.overlays().iter().map(OverlayRef::path));
        self.overlay_with(child_id(self.root, &key))
            .unwrap_or_else(|| panic!("no overlay {key:?}; the open overlays are: {}", open()))
    }

    /// The nodes whose border box differs between `self` and `other`, or that
    /// exist in only one of them.
    ///
    /// Identified nodes are matched by id; anonymous nodes by readable path,
    /// in order when a path repeats. Rects compare exactly: layout is exact,
    /// only drawing rounds.
    pub fn diff(&self, other: &Self) -> Vec<Change> {
        #[derive(Hash, PartialEq, Eq)]
        enum Identity<'p> {
            Id(Id),
            Anonymous(&'p str, usize),
        }
        fn identities(nodes: &Nodes) -> Vec<(Identity<'_>, &PlacedNode)> {
            let mut seen: HashMap<&str, usize> = HashMap::new();
            nodes
                .iter()
                .map(|node| {
                    let identity = node.id().map_or_else(
                        || {
                            let nth = seen.entry(node.path()).or_default();
                            *nth += 1;
                            Identity::Anonymous(node.path(), *nth)
                        },
                        Identity::Id,
                    );
                    (identity, node)
                })
                .collect()
        }
        let before = identities(&self.nodes);
        let mut after: HashMap<_, _> = identities(&other.nodes).into_iter().collect();
        let mut changes = Vec::new();
        for (identity, node) in &before {
            let counterpart = after.remove(identity);
            let rect = counterpart.map(PlacedNode::rect);
            if rect != Some(node.rect()) {
                changes.push(Change {
                    path: node.path().to_owned(),
                    before: Some(node.rect()),
                    after: rect,
                });
            }
        }
        let mut added: Vec<_> = after.into_values().collect();
        added.sort_by_key(|node| node.path().to_owned());
        changes.extend(added.into_iter().map(|node| Change {
            path: node.path().to_owned(),
            before: None,
            after: Some(node.rect()),
        }));
        changes
    }
}

impl<'p> Query<'p> {
    /// The node (or scope) declared with `key` below this one.
    ///
    /// # Panics
    ///
    /// If there is none. The message names the path asked for and the paths
    /// of the nodes below the last one that exists.
    #[track_caller]
    #[expect(clippy::panic, reason = "a test lookup fails the test")]
    pub fn at<K: Key>(&self, key: K) -> Query<'p> {
        self.child(&key).unwrap_or_else(|| {
            panic!(
                "no node {}/{key:?}{}; below {} there are: {}",
                self.path,
                self.not_built_hint(),
                self.display_path(),
                list(
                    self.children()
                        .into_iter()
                        .filter_map(|index| self.layout.node(index))
                        .map(PlacedNode::path)
                ),
            )
        })
    }

    /// What to add to the message for a missing node when this one is a
    /// virtual list, which builds only the rows in and near its viewport.
    fn not_built_hint(&self) -> &'static str {
        let rows = format!("{}/<rows>", self.path);
        if self.layout.nodes.iter().any(|node| node.path() == rows) {
            " (a virtual list builds only the rows in and near its viewport: scroll to the row, or use try_at)"
        } else {
            ""
        }
    }

    /// [`at`](Self::at), or `None` when nothing is there.
    pub fn try_at<K: Key>(&self, key: K) -> Option<Query<'p>> {
        self.child(&key)
    }

    fn child<K: Key>(&self, key: &K) -> Option<Query<'p>> {
        let id = child_id(self.id?, key);
        let path = format!("{}/{key:?}", self.path);
        let index = self.layout.by_id.get(&id).copied();
        let is_scope = || {
            let prefix = format!("{path}/");
            self.layout
                .nodes
                .iter()
                .any(|node| node.path().starts_with(&prefix))
        };
        (index.is_some() || is_scope()).then_some(Query {
            layout: self.layout,
            path,
            id: Some(id),
            index,
        })
    }

    /// The unkeyed or keyed text node below this one whose text is `text`.
    ///
    /// # Panics
    ///
    /// Unless exactly one node below this one shows `text`. The message lists
    /// the candidates.
    #[track_caller]
    #[expect(clippy::panic, reason = "a test lookup fails the test")]
    pub fn text(&self, text: &str) -> Query<'p> {
        let texts: Vec<Shown<'p>> = self
            .descendants()
            .filter_map(|index| {
                let shown = self.layout.prims.get(index)?.as_ref()?.text()?;
                Some((self.layout.node(index)?, index, shown))
            })
            .collect();
        let describe = |shown: &mut dyn Iterator<Item = &Shown<'_>>| {
            list(shown.map(|(node, _, shown)| format!("{shown:?} at {}", node.path())))
        };
        let matching: Vec<_> = texts.iter().filter(|(_, _, shown)| shown == text).collect();
        match matching[..] {
            [&(node, index, _)] => Query {
                layout: self.layout,
                path: node.path().to_owned(),
                id: node.id(),
                index: Some(index),
            },
            [] => panic!(
                "no text {text:?} below {}; text below it: {}",
                self.display_path(),
                describe(&mut texts.iter()),
            ),
            _ => panic!(
                "{} nodes show {text:?} below {}: {}",
                matching.len(),
                self.display_path(),
                describe(&mut matching.iter().copied()),
            ),
        }
    }

    /// The readable key path, as in [`PlacedNode::path`].
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The egui id the builder gave the node, if it has one.
    pub fn id(&self) -> Option<Id> {
        self.id
    }

    /// The placed node, if this is a node and not a scope.
    pub fn node(&self) -> Option<&'p PlacedNode> {
        self.layout.node(self.index?)
    }

    /// The node's border box in absolute coordinates.
    ///
    /// # Panics
    ///
    /// If this names a scope, which has no box.
    #[track_caller]
    #[expect(clippy::panic, reason = "a test lookup fails the test")]
    pub fn rect(&self) -> Rect {
        self.node()
            .unwrap_or_else(|| panic!("{} is a scope, not a node: it has no rect", self.path))
            .rect()
    }

    fn display_path(&self) -> &str {
        if self.path.is_empty() {
            "<root>"
        } else {
            &self.path
        }
    }

    /// Every node below this one, in pre-order.
    fn descendants(&self) -> impl Iterator<Item = usize> + '_ {
        let (from, to, prefix) = match self
            .index
            .and_then(|index| Some((index, self.layout.node(index)?)))
        {
            Some((index, node)) => (index + 1, node.end, String::new()),
            None => (0, self.layout.nodes.len(), format!("{}/", self.path)),
        };
        (from..to).filter(move |&index| {
            self.layout
                .node(index)
                .is_some_and(|node| node.path().starts_with(&prefix))
        })
    }

    /// The identified nodes nearest below this one.
    fn children(&self) -> Vec<usize> {
        let named: Vec<(usize, &PlacedNode)> = self
            .descendants()
            .filter_map(|index| Some((index, self.layout.node(index)?)))
            .filter(|(_, node)| node.id().is_some())
            .collect();
        let nearest = named.iter().map(|(_, node)| node.depth()).min();
        named
            .into_iter()
            .filter(|(_, node)| Some(node.depth()) == nearest)
            .map(|(index, _)| index)
            .collect()
    }
}

fn list<S: AsRef<str>>(items: impl Iterator<Item = S>) -> String {
    let items: Vec<S> = items.collect();
    if items.is_empty() {
        "nothing".to_owned()
    } else {
        items
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>()
            .join(", ")
    }
}

/// A node whose border box differs between two layouts, or that exists in
/// only one.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    path: String,
    before: Option<Rect>,
    after: Option<Rect>,
}

impl Change {
    /// The readable key path of the node.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Its border box in the first layout, or `None` if absent there.
    pub fn before(&self) -> Option<Rect> {
        self.before
    }

    /// Its border box in the second layout, or `None` if absent there.
    pub fn after(&self) -> Option<Rect> {
        self.after
    }

    /// Whether the node is in both layouts and its top-left corner moved:
    /// the shift a test usually forbids, as opposed to a node that only
    /// appeared, disappeared or changed size in place.
    pub fn moved(&self) -> bool {
        matches!((self.before, self.after), (Some(before), Some(after)) if before.min != after.min)
    }
}

/// A clipboard for tests, where there is no display: install it with
/// [`LayoutState::with_clipboard`](crate::LayoutState::with_clipboard). Clones
/// share one state, so a test keeps a clone to set what is on the clipboard
/// and to read back what was written.
#[derive(Clone, Default)]
pub struct FakeClipboard(std::sync::Arc<std::sync::Mutex<FakeState>>);

#[derive(Default)]
struct FakeState {
    held: crate::Clipboard,
    written: Vec<egui::ColorImage>,
    refusal: Option<crate::ClipboardError>,
}

impl FakeClipboard {
    fn state(&self) -> std::sync::MutexGuard<'_, FakeState> {
        crate::sync::lock(&self.0)
    }

    /// Puts `text` on the clipboard, leaving an image there as it is.
    pub fn set_text(&self, text: impl Into<String>) {
        self.state().held.text = Some(text.into());
    }

    /// Puts `image` on the clipboard, leaving text there as it is.
    pub fn set_image(&self, image: egui::ColorImage) {
        self.state().held.image = Some(image);
    }

    /// Empties the clipboard.
    pub fn clear(&self) {
        self.state().held = crate::Clipboard::default();
    }

    /// Makes every read and write fail with `error`, or with `None` stops.
    pub fn refuse(&self, error: Option<crate::ClipboardError>) {
        self.state().refusal = error;
    }

    /// The images written, oldest first.
    pub fn written(&self) -> Vec<egui::ColorImage> {
        self.state().written.clone()
    }
}

impl crate::ClipboardSource for FakeClipboard {
    fn read(&self) -> Result<crate::Clipboard, crate::ClipboardError> {
        let state = self.state();
        match &state.refusal {
            Some(error) => Err(error.clone()),
            None => Ok(state.held.clone()),
        }
    }

    fn write_image(&self, image: &egui::ColorImage) -> Result<(), crate::ClipboardError> {
        let mut state = self.state();
        if let Some(error) = &state.refusal {
            return Err(error.clone());
        }
        state.held.image = Some(image.clone());
        state.written.push(image.clone());
        Ok(())
    }
}
