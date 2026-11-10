//! Headless layout for unit tests: build and lay a view out without a window,
//! a GPU or a draw, then ask where its nodes are.
//!
//! This is ordinary code, not `cfg(test)`, so the tests of a crate that uses
//! bullseye can use it. It needs nothing beyond egui, so it is not behind a
//! feature.
//!
//! ```
//! use bullseye::testing::Headless;
//! use bullseye::{Builder, tw};
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

use std::collections::HashMap;
use std::fmt::Write as _;

use egui::{Id, Rect, Theme, Vec2};

use crate::key::{Key, child_id, root_id};
pub use crate::pipeline::Placed;
use crate::pipeline::{LayoutState, PlacedNode};
use crate::prim::Prim;
use crate::{Builder, TextStyle};

type Prepare = Box<dyn FnOnce(&egui::Context)>;

/// An egui context that lays views out without drawing them.
///
/// ```
/// use bullseye::testing::Headless;
/// use bullseye::{Builder, tw};
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
/// given size and [theme](Self::theme).
pub struct Headless {
    ctx: egui::Context,
    size: Vec2,
    base_text: Option<TextStyle>,
    theme: Option<Theme>,
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
            theme: None,
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

    /// Lays views out under this theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// The context the layouts run on.
    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
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
        let mut view = Some(view);
        let mut placed = None;
        self.pass(|ctx| {
            let view = view.take().expect("a layout runs one pass");
            let mut state = LayoutState::new();
            state.set_base_text(self.base_text.clone());
            placed = Some(state.build(ctx, |b| view(b, app)).layout(area));
        });
        placed.expect("a pass runs its closure")
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
        self.pass(|_| {});
    }

    fn pass(&self, run: impl FnMut(&egui::Context)) {
        if let Some(theme) = self.theme {
            self.ctx.set_theme(theme);
        }
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            ..egui::RawInput::default()
        };
        self.ctx.run_pass(input, run).drop_without_applying_deltas();
    }
}

#[derive(Clone, Copy)]
struct Layout<'p> {
    nodes: &'p [PlacedNode],
    prims: &'p [Option<Prim>],
    by_id: &'p HashMap<Id, usize>,
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
            id: Some(root_id()),
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
        fn identities(nodes: &[PlacedNode]) -> Vec<(Identity<'_>, &PlacedNode)> {
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
    pub fn at<K: Key>(&self, key: K) -> Query<'p> {
        self.child(&key).unwrap_or_else(|| {
            panic!(
                "no node {}/{key:?}; below {} there are: {}",
                self.path,
                self.display_path(),
                list(
                    self.children()
                        .iter()
                        .map(|&index| self.layout.nodes[index].path())
                ),
            )
        })
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
    pub fn text(&self, text: &str) -> Query<'p> {
        let texts: Vec<(usize, &str)> = self
            .descendants()
            .filter_map(|index| match &self.layout.prims[index] {
                Some(Prim::Text(shown)) => Some((index, shown.as_str())),
                _ => None,
            })
            .collect();
        let matching: Vec<usize> = texts
            .iter()
            .filter(|(_, shown)| *shown == text)
            .map(|&(index, _)| index)
            .collect();
        let [index] = matching[..] else {
            let describe = |indices: &mut dyn Iterator<Item = (usize, &str)>| {
                list(indices.map(|(index, shown)| {
                    format!("{shown:?} at {}", self.layout.nodes[index].path())
                }))
            };
            if matching.is_empty() {
                panic!(
                    "no text {text:?} below {}; text below it: {}",
                    self.display_path(),
                    describe(&mut texts.iter().copied()),
                )
            }
            panic!(
                "{} nodes show {text:?} below {}: {}",
                matching.len(),
                self.display_path(),
                describe(&mut texts.iter().copied().filter(|(_, shown)| *shown == text)),
            )
        };
        Query {
            layout: self.layout,
            path: self.layout.nodes[index].path().to_owned(),
            id: self.layout.nodes[index].id(),
            index: Some(index),
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
        Some(&self.layout.nodes[self.index?])
    }

    /// The node's border box in absolute coordinates.
    ///
    /// # Panics
    ///
    /// If this names a scope, which has no box.
    #[track_caller]
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
        let (from, to, prefix) = match self.index {
            Some(index) => (index + 1, self.layout.nodes[index].end, String::new()),
            None => (0, self.layout.nodes.len(), format!("{}/", self.path)),
        };
        (from..to).filter(move |&index| self.layout.nodes[index].path().starts_with(&prefix))
    }

    /// The identified nodes nearest below this one.
    fn children(&self) -> Vec<usize> {
        let named: Vec<usize> = self
            .descendants()
            .filter(|&index| self.layout.nodes[index].id().is_some())
            .collect();
        let nearest = named
            .iter()
            .map(|&index| self.layout.nodes[index].depth())
            .min();
        named
            .into_iter()
            .filter(|&index| Some(self.layout.nodes[index].depth()) == nearest)
            .collect()
    }
}

fn list<S: AsRef<str>>(items: impl Iterator<Item = S>) -> String {
    let mut out = String::new();
    for item in items {
        if !out.is_empty() {
            out.push_str(", ");
        }
        write!(out, "{}", item.as_ref()).expect("writing to a String cannot fail");
    }
    if out.is_empty() {
        "nothing".to_owned()
    } else {
        out
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
}
