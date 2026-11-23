//! The nested-closure builder.

use std::any::Any;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use egui::{Color32, Id, ImageSource, Painter, Rect};
use taffy::{NodeId, Size};

use crate::collapse::{Collapse, CollapseAxis, CollapsibleChain, Declared};
use crate::commands::{Bound, Commands, Within};
use crate::contract::Size as ControlSize;
use crate::dnd::{Activator, DragEnd, DragImage, Dragging, Drop, NodeDrag, Source};
use crate::draw::NodeUi;
use crate::key::{Key, Repeat, child_id};
use crate::measure::Measure;
use crate::mount::{Lensed, Mount, MountChain};
use crate::overlay::Band;
use crate::overlay_chain::{OverlayChain, Unplaced};
use crate::owned::Enablement;
use crate::pipeline::{
    BuildState, ColorSource, DismissFn, DrawKind, Frame, Identified, Identity, Interaction,
    MeasureFn, Node, NodeKind, Pass, ScrollNode, Sink, solve,
};
use crate::presence::{self, Entrance, Motion, Phase as Presence, Shown, Step};
use crate::prim::{Icon, Prim};
use crate::region::{RegionBody, RegionDecl};
use crate::roots::{Layer, LayerKind, OverlayDecl, Presentation, RootDecl, overlay_style};
use crate::scroll::TabRule;
use crate::scroll::{ScrollChain, ScrollOptions};
use crate::sizing::Width;
use crate::text::{TextStyle, VAlign};
use crate::tooltip::{Content, Tooltip};
use crate::tw::{Grouping, Observed, PointerEvents, Progress, Sensing, Tw, tw};
use crate::widget::Stock;

/// What a node is made of, besides its classes.
struct Parts {
    kind: NodeKind,
    id: Option<Id>,
    path: String,
    prim: Option<Prim>,
    measure: Option<usize>,
    collapse: Option<Collapse>,
}

impl Node {
    /// The node's drag declarations; none for an anonymous node.
    fn drag(&mut self) -> Option<&mut NodeDrag> {
        let identified = self.identity.identified_mut()?;
        Some(identified.drag.get_or_insert_with(Default::default))
    }
}

impl Parts {
    fn anonymous(kind: NodeKind, path: String) -> Self {
        Self {
            kind,
            id: None,
            path,
            prim: None,
            measure: None,
            collapse: None,
        }
    }

    fn with_prim(self, prim: Prim) -> Self {
        Self {
            prim: Some(prim),
            ..self
        }
    }
}

/// A key resolved against the builder that started a chain: its id and
/// readable name, fixed when the chain starts so the chain need not carry the
/// key's type.
///
/// `id` is `declared` unless the key repeats a path of the frame, which
/// debug builds give an id of its own.
pub(crate) struct Slot {
    pub(crate) declared: Id,
    pub(crate) id: Id,
    pub(crate) label: Label,
}

/// A key's `Debug` text, which debugs as itself.
pub(crate) struct Label(String);

impl Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Slot {
    /// The slot, debugging as `readable`.
    pub(crate) fn relabelled(self, readable: String) -> Self {
        Self {
            label: Label(readable),
            ..self
        }
    }

    pub(crate) fn readable(&self) -> &str {
        &self.label.0
    }
}

/// Builds one frame's tree. Nodes attach to the current parent.
///
/// A control's chain (`b.button(..)`) borrows the builder until its
/// terminating call (`.add()`, `.on_click(..)`, ...) adds it, so siblings
/// attach in the order they are written and a chain cannot outlive the body
/// that started it.
///
/// # Keys
///
/// An identified node's key is any [`Key`]: a name, a data key, or both
/// (`("pin", pin_key)`). Keys form a path through
/// [`scope`](Self::scope) and identified containers, and a node's egui
/// [`Id`] is the running hash of its path. Each key is hashed together with
/// its type name, so keys need only be unique among siblings **of the same
/// type**: variants of different enums, or `1u32` and a `BlockKey(1)` newtype,
/// never collide. Lists must use data keys, never indices.
///
/// In debug builds, two identified nodes with the same path in one frame
/// panic, naming the readable path.
pub struct Builder<'b, 'f, A: 'f> {
    st: &'b mut BuildState,
    frame: &'b mut dyn Sink<'f, A>,
    path: Id,
    kids: &'b mut Vec<NodeId>,
    inherited: &'b TextStyle,
    ambient: Ambient,
    style: &'b Arc<crate::Style>,
}

/// The leaf a virtual list's rows are built into, as its region needs it.
pub(crate) struct RowsLeaf {
    leaf: NodeId,
    readable: String,
    text: TextStyle,
    ambient: Ambient,
    style: Arc<crate::Style>,
}

/// What a subtree's nodes take from where it is built, beyond text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ambient {
    pub(crate) enablement: Enablement,
    pub(crate) surface: Surface,
    pub(crate) size: Option<ControlSize>,
    /// The direction of the flex container the nodes are built in, if it is
    /// one.
    pub(crate) flex: Option<taffy::FlexDirection>,
    /// The rules of the containers above, for the nodes built here.
    pub(crate) rules: crate::tw::Rules,
    /// The slot of the nearest part the nodes are built inside.
    pub(crate) part: Option<&'static str>,
}

impl Ambient {
    /// What a root of its own, an overlay or a window, takes from where it
    /// is declared: the rules of the containers it is not inside do not
    /// reach it.
    pub(crate) fn cut(&self) -> Self {
        Self {
            rules: crate::tw::Rules::NONE,
            part: None,
            ..*self
        }
    }

    pub(crate) const PAGE: Self = Self {
        enablement: Enablement::Enabled,
        surface: Surface::Page,
        size: None,
        flex: None,
        rules: crate::tw::Rules::NONE,
        part: None,
    };
}

/// What a node is drawn on, for the stock pieces whose look depends on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Surface {
    Page,
    Tooltip,
}

impl<'b, 'f, A> Builder<'b, 'f, A> {
    pub(crate) fn new(
        st: &'b mut BuildState,
        frame: &'b mut dyn Sink<'f, A>,
        path: Id,
        kids: &'b mut Vec<NodeId>,
        (inherited, ambient, style): (&'b TextStyle, Ambient, &'b Arc<crate::Style>),
    ) -> Self {
        Self {
            st,
            frame,
            path,
            kids,
            inherited,
            ambient,
            style,
        }
    }

    /// `own`, and disabled where [`enablement`](Self::enablement) is.
    pub(crate) fn enablement_of(&self, own: Enablement) -> Enablement {
        self.enablement().and(own)
    }

    /// The size a control draws at: its own, else [`control_size`](Self::control_size).
    pub(crate) fn size_of(&self, own: Option<ControlSize>) -> ControlSize {
        own.unwrap_or_else(|| self.control_size())
    }

    /// TODO(prose): the size a control built here draws at unless it sets
    /// its own: the one its subtree inherits from
    /// [`control_size`](Tw::control_size), else the style's default.
    ///
    /// ```
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::testing::Headless;
    /// use eggshell_ui::tw;
    ///
    /// const SM: Size = Size("sm");
    /// Headless::new(egui::vec2(100.0, 100.0)).layout(&(), |b, _| {
    ///     assert_eq!(b.control_size(), Size::DEFAULT);
    ///     b.div(tw().control_size(SM), |b| assert_eq!(b.control_size(), SM));
    /// });
    /// ```
    pub fn control_size(&self) -> ControlSize {
        self.ambient.size.unwrap_or(ControlSize::DEFAULT)
    }

    /// TODO(prose): whether controls built here take input: disabled inside
    /// an [`inert`](Tw::inert) subtree, as a `<fieldset disabled>` disables
    /// the controls inside it.
    ///
    /// ```
    /// use eggshell_ui::owned::Enablement;
    /// use eggshell_ui::testing::Headless;
    /// use eggshell_ui::tw;
    ///
    /// Headless::new(egui::vec2(100.0, 100.0)).layout(&(), |b, _| {
    ///     assert_eq!(b.enablement(), Enablement::Enabled);
    ///     b.div(tw().inert(), |b| assert_eq!(b.enablement(), Enablement::Disabled));
    /// });
    /// ```
    pub fn enablement(&self) -> Enablement {
        self.ambient.enablement
    }

    /// TODO(prose): the style the nodes built here are built, measured and
    /// drawn with: the layout's, or that of the [`styled`](Self::styled)
    /// scope around them.
    pub fn style(&self) -> &Arc<crate::Style> {
        self.style
    }

    /// TODO(prose): the classes of `part` for a node with `keys`: the style's sheet's, with `call_site`'s over them and the rules of the containers above between; the node says its ARIA and attribute keys, for the rules that match it. A variant or size the sheet leaves out is reported as [`Problem::MissingStyleEntry`](crate::Problem::MissingStyleEntry).
    #[track_caller]
    pub fn part<P: crate::sheet::Part>(
        &mut self,
        part: P,
        keys: &[crate::sheet::Key],
        call_site: Tw,
    ) -> Tw {
        self.part_over(part, keys, crate::sheet::Defaults(Tw::blank()), call_site)
    }

    /// TODO(prose): [`part`](Self::part), with `defaults`, the widget's own
    /// look for the part, under every layer of the sheet: the widgets'
    /// defaults, which a rule that replaces the part's lower layers, such as
    /// a preset's generated look, replaces too. The inspector shows them as
    /// the defaults layer, at the widget's own lines.
    ///
    /// ```
    /// use eggshell_ui::sheet::{Defaults, Part};
    /// use eggshell_ui::{Builder, tw};
    ///
    /// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum ChipPart {
    ///     Root,
    /// }
    ///
    /// impl Part for ChipPart {
    ///     const SOURCE: &'static str = "chip.tsx";
    ///     fn slot(self) -> &'static str {
    ///         "chip"
    ///     }
    /// }
    ///
    /// fn chip(b: &mut Builder<'_, '_, ()>) {
    ///     let classes = b.part_over(ChipPart::Root, &[], Defaults(tw().px(2.0)), tw());
    ///     b.div(classes, |_| {});
    /// }
    /// ```
    #[track_caller]
    pub fn part_over<P: crate::sheet::Part>(
        &mut self,
        part: P,
        keys: &[crate::sheet::Key],
        defaults: crate::sheet::Defaults,
        call_site: Tw,
    ) -> Tw {
        let sheet = self.resolved_part(part, keys);
        let base = match self.style.sheet.replaces(part, keys) {
            true => sheet,
            false => defaults
                .0
                .staged(crate::sheet::Layer::Defaults)
                .merge(sheet),
        };
        let said = keys
            .iter()
            .fold(Tw::over_sheet(base), |classes, key| match *key {
                crate::sheet::Key::Aria(state) => classes.aria(state),
                crate::sheet::Key::Attr(name, value) => classes.attr(name, value),
                _ => classes,
            });
        said.merge(call_site)
    }

    /// What the style's sheet gives `part` for a node with `keys`, on its own.
    #[track_caller]
    pub(crate) fn resolved_part<P: crate::sheet::Part>(
        &mut self,
        part: P,
        keys: &[crate::sheet::Key],
    ) -> Tw {
        let style = self.style;
        let sheet = &style.sheet;
        let (classes, fresh) = self.st.sheets.resolve(sheet, part, keys, &style.tokens);
        if fresh && let Some(missing) = sheet.unnamed(part, keys) {
            crate::report::report(
                self.frame.ctx,
                crate::Problem::MissingStyleEntry,
                Some(Id::NULL.with(("style-entry", format!("{part:?} {missing:?}")))),
                || {
                    format!(
                        "the style's sheet names no {missing:?} for {part:?}; drawing without it"
                    )
                },
            );
        }
        classes
    }

    /// TODO(prose): builds `body` in place with a style of its own: the style
    /// here, changed by `edit`. Every node `body` builds, and every control
    /// among them, is built, measured and drawn with it; nothing outside
    /// changes, and a scope inside starts from this one. No node is created.
    ///
    /// The fonts a style's [`faces`](crate::Style::faces) name are the
    /// context's, so a scope cannot change which faces exist.
    ///
    /// ```
    /// use eggshell_ui::{Builder, tw};
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>) {
    ///     b.styled(
    ///         |style| {
    ///             let mut table = style.table().clone();
    ///             table.cell_padding = 16.0;
    ///             style.set_table(table);
    ///         },
    ///         |b| {
    ///             b.text(tw(), "Tables built here are roomier.");
    ///         },
    ///     );
    /// }
    /// ```
    pub fn styled(
        &mut self,
        edit: impl FnOnce(&mut crate::Style),
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) {
        let mut style = crate::Style::clone(self.style);
        style.sheet.as_scope();
        edit(&mut style);
        self.with_style(&Arc::new(style), body);
    }

    /// Builds `body` in place with `style` for this scope's.
    pub(crate) fn with_style(
        &mut self,
        style: &Arc<crate::Style>,
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) {
        body(&mut Builder::new(
            self.st,
            self.frame,
            self.path,
            self.kids,
            (self.inherited, self.ambient, style),
        ));
    }

    /// Builds `body` with a key pushed onto the path. No node is created;
    /// the children attach to the current parent.
    ///
    /// `body` gets a [`Scope`], which components take: a component can only be
    /// placed where its caller named the instance.
    pub fn scope<K: Key>(&mut self, key: K, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        // A scope may reach into the subtree of a node already keyed `key`,
        // so a repeat of the key keeps its id.
        let declared = child_id(self.path, &key);
        let path = Slot {
            declared,
            id: declared,
            label: Label(format!("{key:?}")),
        };
        self.scope_with(&path, body);
    }

    /// Not a stable API: it exists for `eggshell-ui-extras`. Builds `body`
    /// with its nodes keyed under the node `parent` instead of this scope, so
    /// that their ids can be found from `parent` alone, wherever they sit in
    /// the tree.
    #[doc(hidden)]
    pub fn keyed_under(&mut self, parent: Id, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            parent,
            self.kids,
            (self.inherited, self.ambient, self.style),
        )));
    }

    pub(crate) fn scope_with(&mut self, slot: &Slot, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let mark = self.st.names.push(&slot.label);
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            slot.id,
            self.kids,
            (self.inherited, self.ambient, self.style),
        )));
        self.st.names.pop_to(mark);
    }

    /// The same builder, borrowed for the life of a chain.
    /// How many children this builder has added.
    pub(crate) fn added(&self) -> usize {
        self.kids.len()
    }

    /// Moves the child added last to `at` among its siblings, so that it is
    /// painted and hit-tested before those added from `at` on.
    pub(crate) fn move_last_to(&mut self, at: usize) {
        if let Some(last) = self.kids.pop() {
            self.kids.insert(at.min(self.kids.len()), last);
        }
    }

    pub(crate) fn reborrow(&mut self) -> Builder<'_, 'f, A> {
        Builder::new(
            self.st,
            self.frame,
            self.path,
            self.kids,
            (self.inherited, self.ambient, self.style),
        )
    }

    /// `key` resolved in this scope, for a chain that is finished later.
    pub(crate) fn slot<K: Key>(&self, key: &K) -> Slot {
        self.slot_of(child_id(self.path, key), format!("{key:?}"))
    }

    /// The slot of a node whose id, `declared`, is not a key's.
    pub(crate) fn slot_of(&self, declared: Id, readable: String) -> Slot {
        Slot {
            declared,
            id: self.st.seen.vacant(declared),
            label: Label(readable),
        }
    }

    pub(crate) fn kept(&self) -> &crate::kept::Handle {
        &self.st.kept
    }

    /// The state of the layout being built, for requests made while it is.
    pub fn shell(&self) -> crate::Shell {
        crate::Shell::new(self.frame.ctx, &self.st.kept)
    }

    /// The egui context the frame is built with, for reading the local state
    /// that draw closures keep in egui's memory under ids from
    /// [`id_of`](Self::id_of).
    pub fn ctx(&self) -> &egui::Context {
        self.frame.ctx
    }

    /// The egui viewport the node being built is drawn in.
    pub(crate) fn viewport(&self) -> egui::ViewportId {
        self.frame.ctx.viewport_id()
    }

    /// The id a node declared with `key` has in this scope.
    ///
    /// For overlay anchors and for reading local state at build time. Keep
    /// such keys in one place so a typo cannot split them.
    pub fn id_of<K: Key>(&self, key: K) -> Id {
        child_id(self.path, &key)
    }

    /// Whether the node declared with `key` in this scope has keyboard focus.
    ///
    /// Reads the focus egui held at the start of the frame, so it works before
    /// the node exists. A change of focus shows up in the layout one frame
    /// later, as a single relayout; layout never moves focus, so this cannot
    /// oscillate.
    pub fn has_focus<K: Key>(&self, key: K) -> bool {
        crate::focus::holds_focus(self.frame.ctx, self.id_of(key))
    }

    /// Whether the node declared with `key` in this scope, or a widget
    /// inside it, has keyboard focus. See [`Shell::focus_within`](crate::Shell::focus_within).
    pub fn has_focus_within<K: Key>(&self, key: K) -> bool {
        self.shell().focus_within(self.id_of(key))
    }

    /// An anonymous container. Its text classes are inherited by `body`.
    pub fn div(&mut self, tw: Tw, body: impl FnOnce(&mut Builder<'_, 'f, A>)) {
        let path = self.anonymous_path(NodeKind::Div);
        self.container(
            tw,
            Parts::anonymous(NodeKind::Div, path),
            self.path,
            |mut b| body(&mut b),
        );
    }

    /// An identified container. Its children are scoped under `key`.
    ///
    /// The returned [`NodeHandle`] attaches behaviour to the node.
    #[track_caller]
    pub fn div_id<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'_, 'f, A> {
        let slot = self.slot(&key);
        self.div_id_with(&slot, tw, body)
    }

    #[track_caller]
    pub(crate) fn div_id_with(
        &mut self,
        slot: &Slot,
        tw: Tw,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'_, 'f, A> {
        let path = self.identify(slot);
        let parts = Parts {
            id: Some(slot.id),
            ..Parts::anonymous(NodeKind::DivId, path)
        };
        let mark = self.st.names.push(&slot.label);
        let node = self.container(tw, parts, slot.id, |b| body(&mut Scope(b)));
        self.st.names.pop_to(mark);
        self.handle(node, slot.id)
    }

    /// TODO(prose): a view of the part of the application data `lens`
    /// reaches, built here from `data` once the chain is added.
    pub fn mount<K: Key, S: 'static, L, V>(
        &mut self,
        key: K,
        tw: Tw,
        data: &'f S,
        lens: L,
        view: V,
    ) -> MountChain<'_, 'f, A, S, L, V>
    where
        A: 'static,
        L: Fn(&mut A) -> &mut S + 'static,
        V: FnOnce(&mut Builder<'_, 'f, S>, &'f S),
    {
        MountChain::new(self.reborrow(), key, tw, (data, lens, view))
    }

    /// Where a mount keyed `slot` and built here is: its id and how deeply
    /// it is nested in other mounts.
    /// Declares the mount keyed `slot`, built here, and says whether it is
    /// selected.
    pub(crate) fn declare_mount(&mut self, slot: &Slot) -> crate::mount::Selected {
        let parent = self.frame.mount();
        self.frame.mounts.push(crate::mount::MountDecl {
            id: slot.id,
            parent,
        });
        if self.st.kept.with(|kept| kept.mounts.is_selected(slot.id)) {
            crate::mount::Selected::Yes
        } else {
            crate::mount::Selected::No
        }
    }

    pub(crate) fn mount_within(&self, slot: &Slot) -> Within {
        Within {
            mount: slot.id,
            depth: self.frame.depth(),
        }
    }

    /// Builds `body` here with a builder of `()`, for content that takes no
    /// input.
    pub(crate) fn unit<T>(&mut self, body: impl FnOnce(&mut Builder<'_, 'f, ()>) -> T) -> T
    where
        A: 'static,
    {
        let within = Within {
            mount: self.path,
            depth: self.frame.depth(),
        };
        // A `()` is never allocated, so leaking one leaks nothing.
        let lens: crate::mount::Lens<A, ()> = std::rc::Rc::new(|_| Box::leak(Box::new(())));
        self.within(&Mount::new(lens, within), body)
    }

    /// Builds `body` here with a builder of `mount`'s data.
    pub(crate) fn within<S: 'static, T>(
        &mut self,
        mount: &Mount<'f, A, S>,
        body: impl FnOnce(&mut Builder<'_, 'f, S>) -> T,
    ) -> T
    where
        A: 'static,
    {
        let mut lensed = Lensed::new(&mut *self.frame, mount.clone());
        body(&mut Builder::new(
            self.st,
            &mut lensed,
            self.path,
            self.kids,
            (self.inherited, self.ambient, self.style),
        ))
    }

    /// A scroll container: see [`scroll`](crate::scroll). Its children are
    /// scoped under `key`.
    pub fn scroll<K: Key>(&mut self, key: K, tw: Tw) -> ScrollChain<'_, 'f, A>
    where
        A: 'static,
    {
        ScrollChain::new(self.reborrow(), key, tw)
    }

    /// A virtual list: a scroll container of `rows` that builds only the rows
    /// in and near its viewport. See [`vlist`](crate::vlist).
    ///
    /// Each row is built under its own key (the row itself), so a row's egui
    /// state stays with it while it is scrolled away and back. The nodes a row
    /// builds sit under that key, so a test finds one with
    /// `placed.at(Part::List).at(item_key).at(Part::Row(item_key))`. `rows` and the
    /// rows' builder borrow application data for the frame (`'f`), so a
    /// function that declares a list names `'f`, as one that declares a
    /// [`region`](Self::region) does.
    ///
    /// ```
    /// # use eggshell_ui::{Builder, tw};
    /// #[derive(Clone, Copy, Debug, Hash)]
    /// struct Line(u32);
    ///
    /// fn log<'f>(b: &mut Builder<'_, 'f, ()>, lines: &'f [Line]) {
    ///     b.virtual_list("log", tw().h_px(200.0), lines)
    ///         .fixed_px(20.0)
    ///         .items(|s, line| s.text(tw(), format!("line {}", line.0)));
    /// }
    /// ```
    #[track_caller]
    pub fn virtual_list<K: Key, R: Key + Copy>(
        &mut self,
        key: K,
        tw: Tw,
        rows: impl Into<std::borrow::Cow<'f, [R]>>,
    ) -> crate::vlist::ListChain<'_, 'f, A, R>
    where
        A: 'static,
    {
        let slot = self.slot(&key);
        crate::vlist::ListChain::new(self.reborrow(), slot, tw, rows.into())
    }

    /// Declares the rows of the virtual list `list`, built into `leaf` by
    /// `run` once the frame is laid out.
    pub(crate) fn list_region(
        &mut self,
        RowsLeaf {
            leaf,
            readable,
            text,
            ambient,
            style,
        }: RowsLeaf,
        list: Id,
        run: crate::vlist::ListRun<'f, A>,
    ) {
        self.frame.add_region(RegionDecl {
            leaf,
            id: list,
            readable,
            text,
            ambient,
            style,
            body: RegionBody::List(run),
        });
    }

    /// A handle to `node`, a node this builder added.
    pub(crate) fn into_handle(self, node: NodeId, id: Id) -> NodeHandle<'b, 'f, A> {
        NodeHandle {
            b: self,
            node,
            id,
            kind: PhantomData,
        }
    }

    /// The text style in effect here.
    pub(crate) fn text_style(&self) -> &TextStyle {
        self.inherited
    }

    /// The leaf a virtual list's rows are built into once it is laid out:
    /// as wide as the content and `height` tall.
    pub(crate) fn rows_leaf(&mut self, height: f32) -> RowsLeaf {
        let slot = self.slot(&crate::vlist::RowsKey);
        let readable = self.identify(&slot);
        let id = slot.id;
        let tw = tw().w_full().h_px(height);
        let ambient = self.ambient_in(&tw);
        let leaf = self.leaf_node(
            tw,
            Parts {
                id: Some(id),
                ..Parts::anonymous(NodeKind::Region, readable.clone())
            },
        );
        let text = self.leaf_text(leaf);
        RowsLeaf {
            leaf,
            readable,
            text,
            ambient,
            style: self.style.clone(),
        }
    }

    /// Sizes the scroll container `node` to its content, `rows` tall inside
    /// the content's padding, unless its classes gave it a height, and no
    /// taller than its max height: the container's parents measure it by this
    /// height, not by the max.
    pub(crate) fn fit_height(&mut self, node: NodeId, rows: f32) {
        use taffy::{BoxSizing, CompactLength, Dimension};
        let mut style = self.st.tree.style(node).clone();
        if style.size.height != Dimension::auto() {
            return;
        }
        let padding = self.st.tree.scroll(node).map_or(0.0, |scroll| {
            block_padding(self.st.tree.style(scroll.content))
        });
        let border = block_border(&style);
        let insets = match style.box_sizing {
            BoxSizing::BorderBox => padding + border,
            BoxSizing::ContentBox => 0.0,
        };
        let wanted = rows + insets;
        let max = style.max_size.height.into_raw();
        let capped = if max.tag() == CompactLength::LENGTH_TAG {
            wanted.min(max.value())
        } else {
            wanted
        };
        style.size.height = Dimension::length(capped);
        self.st.tree.set_style(node, style);
    }

    /// How many roots have been declared so far this frame.
    pub(crate) fn roots_declared(&self) -> usize {
        self.frame.roots.len()
    }

    /// The overlays declared from the root at `from` on.
    pub(crate) fn overlays_from(&self, from: usize) -> impl Iterator<Item = Id> + '_ {
        self.frame
            .roots
            .iter()
            .skip(from)
            .filter_map(|root| Some(root.overlay()?.id))
    }

    /// The nodes the overlays declared so far are anchored to.
    pub(crate) fn anchored_to(&self) -> impl Iterator<Item = Id> + '_ {
        self.frame
            .roots
            .iter()
            .filter_map(|root| match root.overlay()?.presentation {
                Presentation::Floating {
                    anchor: crate::overlay::Anchor::Node(id),
                    ..
                } => Some(id),
                Presentation::Floating { .. } | Presentation::Modal(_) => None,
            })
    }

    /// Declares `region` in the root whose body is being built.
    /// A region: a leaf that only layout sizes, whose content `body` builds
    /// after the layout around it, in the same frame.
    ///
    /// `body` runs with the leaf's content-box size, and what it builds is
    /// laid out at exactly that size as a tree of its own, so what it builds
    /// can depend on the room it has. The leaf's size is its classes' alone
    /// (`flex_1`, a width, a height): it never depends on the content, as a
    /// CSS container query's container does not. Regions nest, an enclosing
    /// one's `body` running before those inside it; scroll containers and
    /// overlays declared in `body` follow. Text classes are inherited into
    /// `body`, and an overlay in it may be anchored to a node outside.
    ///
    /// `body` borrows application data for the frame (`'f`), so a function
    /// that declares a region names `'f`:
    ///
    /// ```
    /// # use eggshell_ui::{Builder, tw};
    /// struct App { title: String }
    ///
    /// fn canvas<'f>(app: &'f App, b: &mut Builder<'_, 'f, App>) {
    ///     b.region("canvas", tw().flex_1().min_w_0(), move |b, size| {
    ///         if size.x > 300.0 {
    ///             b.text(tw(), app.title.as_str());
    ///         }
    ///     });
    /// }
    /// ```
    ///
    /// It does not compile without it:
    ///
    /// ```compile_fail
    /// # use eggshell_ui::{Builder, tw};
    /// # struct App { title: String }
    /// fn canvas(app: &App, b: &mut Builder<'_, '_, App>) {
    ///     b.region("canvas", tw(), move |b, _| b.text(tw(), app.title.as_str()));
    /// }
    /// ```
    #[track_caller]
    pub fn region<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        body: impl FnOnce(&mut Builder<'_, 'f, A>, egui::Vec2) + 'f,
    ) {
        let slot = self.slot(&key);
        let readable = self.identify(&slot);
        let id = slot.id;
        let ambient = self.ambient_in(&tw);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::Region, readable.clone())
        };
        let leaf = self.leaf_node(tw, parts);
        let text = self.leaf_text(leaf);
        self.frame.add_region(RegionDecl {
            leaf,
            id,
            readable,
            text,
            ambient,
            style: self.style.clone(),
            body: RegionBody::Closure(Box::new(body)),
        });
    }

    /// Declares the commands of the root being built: the page, or the
    /// overlay whose body this is. See [`Commands`].
    ///
    /// `app` is passed here, once, because the builder holds no application
    /// data of its own: a command's [`enabled`](crate::Command::enabled) is
    /// read from it at build, so that a menu item for the command is
    /// disabled as it is built. Pass the `&'f A` the view was given.
    ///
    /// Under an open modal only the modal's own table runs; the page's is
    /// silent. A table whose keys or chords repeat, a bare Esc or Enter, and
    /// a root that declares its commands twice are reported (see
    /// [`Problem`](crate::Problem)).
    ///
    /// ```
    /// # use eggshell_ui::{Builder, Chord, Command, Commands};
    /// # use egui::{Key, Modifiers};
    /// #[derive(Hash, Debug)]
    /// enum Cmd { Save }
    /// #[derive(Default)]
    /// struct App { saved: u32, dirty: bool }
    ///
    /// fn commands() -> Commands<App> {
    ///     Commands::new().add(
    ///         Cmd::Save,
    ///         Command::new("Save", |app: &mut App| app.saved += 1)
    ///             .shortcut(const { Chord::new(Modifiers::COMMAND, Key::S).expect("not reserved") })
    ///             .enabled(|app| app.dirty),
    ///     )
    /// }
    ///
    /// fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    ///     b.commands(&commands(), app);
    /// }
    /// ```
    #[track_caller]
    pub fn commands(&mut self, table: &Commands<A>, app: &'f A) {
        table.validate(self.frame.ctx);
        if self.frame.declare_commands(table, app).is_err() {
            crate::report::report(self.frame.ctx, crate::Problem::CommandsTwice, None, || {
                "commands are declared twice in one root; the first table runs".to_owned()
            });
        }
    }

    /// The command `id` names, from the tables whose chords are live here.
    pub(crate) fn command<K: Key>(&self, id: &K) -> Option<Bound<A>> {
        self.frame.command(child_id(Id::NULL, id))
    }

    /// The width `candidate` has when it is laid out as wide as it wants: to
    /// choose, in a [`region`](Self::region), the first of several forms that
    /// fits the room.
    ///
    /// `candidate` is built into a tree of its own, with the text classes
    /// inherited here and under this scope's keys, and thrown away. Its keys
    /// are not registered, so it may be built again for real in the same
    /// frame, and neither the closures, the overlays and the regions it
    /// declares nor anything else it adds outlives the call. The cost is one
    /// more measure of the candidate.
    ///
    /// ```
    /// # use eggshell_ui::{Builder, Width, tw};
    /// fn title(b: &mut Builder<'_, '_, ()>, full: bool) {
    ///     b.text(tw(), if full { "A long title for a pane" } else { "Title" });
    /// }
    ///
    /// fn pick(b: &mut Builder<'_, '_, ()>, room: f32) {
    ///     let full = |b: &mut Builder<'_, '_, ()>| title(b, true);
    ///     if b.max_content_width(full) <= Width::new(room) {
    ///         full(b);
    ///     } else {
    ///         title(b, false);
    ///     }
    /// }
    /// ```
    pub fn max_content_width(&mut self, candidate: impl FnOnce(&mut Builder<'_, 'f, A>)) -> Width {
        let ctx = self.frame.ctx;
        let mut st = self.st.scratch();
        let mut frame = Frame::new(ctx);
        let mut kids = Vec::new();
        candidate(&mut Builder::new(
            &mut st,
            &mut frame,
            self.path,
            &mut kids,
            (self.inherited, self.ambient, self.style),
        ));
        let root = st.tree.new_node(
            overlay_style(),
            &kids,
            Node::bare(
                NodeKind::Root,
                String::new(),
                (self.inherited.clone(), self.style.clone()),
            ),
        );
        let loose = Size {
            width: taffy::AvailableSpace::MaxContent,
            height: taffy::AvailableSpace::MaxContent,
        };
        solve(ctx, &frame.measures, &mut st.tree, root, loose);
        Width::new(st.tree.layout(root).size.width)
    }

    /// A part of the page that opens and closes by its size, an accordion
    /// item or a collapsing sidebar. See [`CollapsibleChain`].
    ///
    /// While it is opening or closing the wrapper is a fraction of the size
    /// its content wants, so what follows it moves with it, and the content
    /// is clipped at its natural size. A closed region is not built.
    ///
    /// ```
    /// # use eggshell_ui::{Builder, tw};
    /// # use eggshell_ui::overlay::Shown;
    /// struct App { open: bool }
    ///
    /// fn item<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    ///     b.collapsible("details", Shown::from(app.open))
    ///         .content(tw().p(4.0))
    ///         .children(|b| b.text(tw(), "Only as tall as it needs"));
    /// }
    /// ```
    #[track_caller]
    pub fn collapsible<K: Key>(&mut self, key: K, shown: Shown) -> CollapsibleChain<'_, 'f, A> {
        let slot = self.slot(&key);
        CollapsibleChain::new(self.reborrow(), slot, shown)
    }

    /// Declares the collapsible `decl` and builds its content.
    #[track_caller]
    pub(crate) fn collapsible_with(
        mut self,
        slot: &Slot,
        decl: Declared<A>,
        header: impl FnOnce(&mut Scope<'_, 'f, A>, Progress),
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'b, 'f, A> {
        let Declared {
            shown,
            axis,
            transition,
            easing,
            classes,
            content,
            on_collapsed,
        } = decl;
        let motion = Motion {
            shown,
            enter: transition,
            exit: transition,
            easing,
            entrance: Entrance::Fade,
        };
        let step = presence::step(
            &self.st.kept,
            self.frame.ctx,
            slot.id,
            &motion,
            presence::First::Settled,
        );
        let open = match &step {
            Step::Closed { .. } => Progress::START,
            Step::Present(staged) => staged.progress,
        };
        header(&mut Scope(self.reborrow()), open);
        let wrapper = match axis {
            CollapseAxis::Height => classes.overflow_hidden().flex().flex_col(),
            CollapseAxis::Width => classes.overflow_hidden().flex().flex_row(),
        };
        let path = self.identify(slot);
        let mark = self.st.names.push(&slot.label);
        let parts = |collapse| Parts {
            id: Some(slot.id),
            collapse,
            ..Parts::anonymous(NodeKind::DivId, path.clone())
        };
        let node = match step {
            Step::Closed { exited } => {
                if let Some(then) = on_collapsed.filter(|_| exited) {
                    self.frame.on_exited(then);
                }
                let closed = match axis {
                    CollapseAxis::Height => wrapper.h_px(0.0),
                    CollapseAxis::Width => wrapper.w_px(0.0),
                };
                self.container(
                    closed,
                    parts(Some(Collapse {
                        axis,
                        motion: None,
                        flight: None,
                    })),
                    slot.id,
                    |_| {},
                )
            }
            Step::Present(staged) => {
                let moving = staged.phase != Presence::Open;
                self.frame.size_in_flight |= moving;
                let collapse = Collapse {
                    axis,
                    motion: moving.then_some(staged.progress),
                    flight: None,
                };
                let wrapper = if staged.is_exiting() {
                    wrapper.inert()
                } else {
                    wrapper
                };
                self.container(wrapper, parts(Some(collapse)), slot.id, |mut b| {
                    let inner = b.anonymous_path(NodeKind::Div);
                    let at = b.path;
                    b.container(
                        content.shrink_0(),
                        Parts::anonymous(NodeKind::Div, inner),
                        at,
                        |b| body(&mut Scope(b)),
                    );
                })
            }
        };
        self.st.names.pop_to(mark);
        NodeHandle {
            b: self,
            node,
            id: slot.id,
            kind: PhantomData,
        }
    }

    /// An overlay: a floating layer placed against an anchor. See
    /// [`overlay`](crate::overlay).
    #[track_caller]
    pub fn overlay<K: Key>(&mut self, key: K) -> OverlayChain<'_, 'f, A, Unplaced> {
        let slot = self.slot(&key);
        OverlayChain::new(self.reborrow(), slot)
    }

    /// TODO(prose): a subtree shown inline or, while popped out, in a window
    /// of its own. See [`crate::window`].
    pub fn pop_out<K: Key>(&mut self, key: K) -> crate::window::PopOutChain<'_, 'f, A>
    where
        A: 'static,
    {
        let slot = self.slot(&key);
        crate::window::PopOutChain::new(self.reborrow(), slot)
    }

    /// Notes the pop-out `id`, whatever it shows this frame.
    pub(crate) fn note_pop_out(&mut self, id: Id) {
        self.frame.pop_outs.push(id);
    }

    /// Declares the window `decl` of a popped-out subtree and builds `body`
    /// as its content, a root of its own, at the path the subtree has
    /// inline.
    #[track_caller]
    pub(crate) fn window_with(
        mut self,
        slot: &Slot,
        decl: crate::window::WindowDecl,
        hooks: crate::window::Hooks<A>,
        classes: Tw,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) {
        self.frame.window_hooks(slot.id, hooks);
        let path = self.identify(slot);
        let parent = self.frame.current;
        let ambient = self.ambient_in(&classes);
        let node = self.node(
            &classes,
            Parts {
                id: Some(slot.id),
                ..Parts::anonymous(NodeKind::Window, path)
            },
        );
        let style = self.resolved_style(&classes, &node);
        let text = node.text.clone();
        let root = self.st.tree.new_node(style, &[], node);
        let at = self.frame.roots.len();
        let mount = self.frame.mount();
        self.frame.roots.push(RootDecl {
            mount,
            ..RootDecl::layer(Layer {
                kind: LayerKind::Window(decl),
                parent,
                node: root,
            })
        });
        self.frame.current = at;
        let mark = self.st.names.push(&slot.label);
        let mut kids = Vec::new();
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            slot.id,
            &mut kids,
            (&text, ambient.cut(), self.style),
        )));
        self.st.names.pop_to(mark);
        self.frame.current = parent;
        self.st.tree.set_children(root, &kids);
    }

    /// Declares the overlay `decl` and builds `body` as its content, a root
    /// of its own. The slot is reserved before the body is built, so
    /// overlays declared in the body come after this one.
    pub(crate) fn overlay_with(
        mut self,
        slot: &Slot,
        decl: OverlayDecl,
        on_dismiss: Option<DismissFn<A>>,
        (motion, on_exited): (Option<Motion>, Option<DismissFn<A>>),
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) {
        let mut decl = decl;
        if let Some(motion) = motion {
            match presence::step(
                &self.st.kept,
                self.frame.ctx,
                slot.id,
                &motion,
                presence::First::Enters,
            ) {
                Step::Closed { exited } => {
                    if let Some(then) = on_exited.filter(|_| exited) {
                        self.frame.on_exited(then);
                    }
                    return;
                }
                Step::Present(staged) => decl.staged = staged,
            }
        }
        let band = decl.band;
        let classes = match decl.presentation {
            Presentation::Modal(_) => tw().bg(self.style.scrim),
            Presentation::Floating { .. } => tw(),
        };
        if let Some(on_dismiss) = on_dismiss {
            self.frame.on_dismiss(slot.id, on_dismiss);
        }
        let path = self.identify(slot);
        let parent = self.frame.current;
        let mut node = self.node(
            &classes,
            Parts {
                id: Some(slot.id),
                ..Parts::anonymous(NodeKind::Overlay, path)
            },
        );
        if band == Band::Tooltip {
            node.pointer_events = PointerEvents::None;
        }
        let text = node.text.clone();
        let root = self.st.tree.new_node(overlay_style(), &[], node);
        let at = self.frame.roots.len();
        let mount = self.frame.mount();
        self.frame.roots.push(RootDecl {
            mount,
            ..RootDecl::layer(Layer {
                kind: LayerKind::Overlay(decl),
                parent,
                node: root,
            })
        });
        self.frame.current = at;
        let mark = self.st.names.push(&slot.label);
        let mut kids = Vec::new();
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            slot.id,
            &mut kids,
            (&text, self.ambient.cut(), self.style),
        )));
        self.st.names.pop_to(mark);
        self.frame.current = parent;
        self.st.tree.set_children(root, &kids);
    }

    /// Adds a scroll container's leaf to the current parent, with the root of
    /// its content, built from `body`, a tree of its own.
    pub(crate) fn scroll_with<R>(
        mut self,
        slot: &Slot,
        mut classes: Tw,
        options: ScrollOptions,
        body: impl FnOnce(&mut Scope<'_, 'f, A>) -> R,
    ) -> (NodeHandle<'b, 'f, A>, R) {
        use taffy::{Overflow, Point};
        let ScrollOptions {
            axis,
            edge,
            bars,
            focusable,
            description,
        } = options;
        let path = self.identify(slot);
        self.apply_rules(&mut classes, NodeKind::Scroll);
        let bars = bars.settled(classes.scrollbar);
        let parts = Parts {
            id: Some(slot.id),
            ..Parts::anonymous(NodeKind::Scroll, path)
        };
        let mut leaf = self.node(&classes, parts);
        let mark = self.st.names.push(&slot.label);
        let content_path = self.anonymous_path(NodeKind::ScrollContent);
        let mut kids = Vec::new();
        let ambient = self.ambient_in(&classes);
        let built = body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            slot.id,
            &mut kids,
            (&leaf.text, ambient, self.style),
        )));
        self.st.names.pop_to(mark);
        let mut content_node = self.node(
            &tw(),
            Parts::anonymous(NodeKind::ScrollContent, content_path),
        );
        content_node.text = leaf.text.clone();
        let mut style = self.resolved_style(&classes, &leaf);
        let content_style = crate::scroll::take_content_layout(&mut style);
        let content = self.st.tree.new_node(content_style, &kids, content_node);
        let tab = TabRule::of(focusable, &classes, &self.st.tree, &kids);
        leaf.identity = Identity::Identified(Identified {
            id: slot.id,
            draw: None,
            drag: None,
            scroll: Some(ScrollNode {
                axis,
                edge,
                bars,
                tab,
                description,
                padding: classes.scroll_padding_points(self.style.tokens.spacing),
                content,
                size: egui::Vec2::ZERO,
            }),
        });
        style.overflow = Point {
            x: Overflow::Scroll,
            y: Overflow::Scroll,
        };
        let id = self.st.tree.new_node(style, &[], leaf);
        self.kids.push(id);
        self.frame.add_scroll(id);
        let handle = NodeHandle {
            b: self,
            node: id,
            id: slot.id,
            kind: PhantomData,
        };
        (handle, built)
    }

    /// An identified leaf whose size comes from `measure`.
    ///
    /// `measure` may borrow application data for the frame and is dropped when
    /// layout returns; [`fill`](crate::fill) sizes a leaf by classes.
    pub(crate) fn leaf<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f,
    ) -> NodeHandle<'_, 'f, A> {
        let slot = self.slot(&key);
        self.leaf_with(&slot, tw, measure)
    }

    #[track_caller]
    pub(crate) fn leaf_with(
        &mut self,
        slot: &Slot,
        tw: Tw,
        measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f,
    ) -> NodeHandle<'_, 'f, A> {
        let path = self.identify(slot);
        let measure: MeasureFn<'f> = Box::new(measure);
        let at = self.frame.measures.len();
        self.frame.measures.push(measure);
        let parts = Parts {
            id: Some(slot.id),
            measure: Some(at),
            ..Parts::anonymous(NodeKind::Leaf, path)
        };
        let node = self.leaf_node(tw, parts);
        self.handle(node, slot.id)
    }

    /// An identified leaf holding a stock egui widget, sized by the library
    /// from the widget's content (see [`widget`](crate::widget)).
    ///
    /// `draw` gets the widget back to `show` it with the application's data.
    #[track_caller]
    pub fn widget<K: Key, W: Stock>(
        &mut self,
        key: K,
        tw: Tw,
        widget: W,
        draw: impl FnOnce(&mut A, &mut egui::Ui, W) + 'static,
    ) where
        A: 'static,
    {
        let measured = widget.clone();
        self.leaf(key, tw, move |m| measured.measure(m))
            .draw_widgets(move |app, ui| draw(app, ui, widget));
    }

    /// An identified text leaf, measured and painted by the library.
    #[track_caller]
    pub fn span<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        text: impl Into<String>,
    ) -> NodeHandle<'_, 'f, A> {
        self.identified_leaf(key, tw, NodeKind::Span, Prim::Text(text.into()))
    }

    /// TODO(prose)
    ///
    /// ```
    /// use eggshell_ui::{Builder, Span, Weight, tw};
    ///
    /// fn view(b: &mut Builder<'_, '_, Vec<String>>) {
    ///     b.spans("intro", tw(), vec![
    ///         Span::new("Read "),
    ///         Span::new("the guide").link("#guide"),
    ///         Span::new(" or "),
    ///         Span::new("the source").link("https://github.com/samitbasu/eggshell"),
    ///         Span::new(", it is "),
    ///         Span::new("short").weight(Weight::Bold),
    ///         Span::new("."),
    ///     ])
    ///     .on_link(|followed, href| followed.push(href.to_owned()));
    /// }
    /// ```
    #[track_caller]
    pub fn spans<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        spans: impl Into<crate::Spans>,
    ) -> SpansHandle<'_, 'f, A> {
        SpansHandle(self.identified_leaf(key, tw, NodeKind::Span, Prim::Spans(spans.into())))
    }

    #[track_caller]
    fn identified_leaf<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        kind: NodeKind,
        prim: Prim,
    ) -> NodeHandle<'_, 'f, A> {
        let slot = self.slot(&key);
        let path = self.identify(&slot);
        let parts = Parts {
            id: Some(slot.id),
            ..Parts::anonymous(kind, path).with_prim(prim)
        };
        let node = self.leaf_node(tw, parts);
        self.handle(node, slot.id)
    }

    /// An identified icon, measured and painted by the library.
    #[track_caller]
    pub fn icon<K: Key>(&mut self, key: K, tw: Tw, icon: Icon) -> NodeHandle<'_, 'f, A> {
        self.identified_leaf(key, tw, NodeKind::Icon, Prim::Icon(icon))
    }

    /// Anonymous text.
    pub fn text(&mut self, tw: Tw, text: impl Into<String>) {
        self.display(tw, NodeKind::Text, Prim::Text(text.into()));
    }

    /// Anonymous text with egui's formatting: a [`RichText`](egui::RichText)'s
    /// colour, size, font family, style and decorations, or a laid-out
    /// [`LayoutJob`](egui::text::LayoutJob) or galley. Plain text is
    /// [`text`](Self::text). What the text does not set comes from the node's
    /// text style; egui's `strong` and `weak` colours are the node's colour,
    /// as egui has no bold.
    pub fn rich(&mut self, tw: Tw, text: impl Into<egui::WidgetText>) {
        match text.into() {
            egui::WidgetText::Text(text) => self.text(tw, text),
            rich => self.display(tw, NodeKind::Text, Prim::Rich(rich)),
        }
    }

    /// A key chip: `keys`, a key or a chord, in a small box, as HTML's
    /// `<kbd>`. Inside a tooltip it takes the look the style gives a chip on
    /// a tooltip ([`KbdStyle`](crate::contract::KbdStyle)).
    pub fn kbd(&mut self, keys: impl Into<String>) {
        let style = self.style.clone();
        let kbd = &style.kbd;
        let colors = match self.surface() {
            Surface::Page => kbd.page,
            Surface::Tooltip => kbd.tooltip,
        };
        let classes = tw()
            .flex()
            .items_center()
            .justify_center()
            .shrink_0()
            .h_px(kbd.height)
            .min_w_px(kbd.height)
            .px_px(kbd.padding_x)
            .border(kbd.border_width)
            .border_color(colors.border)
            .rounded_px(f32::from(kbd.radius))
            .bg(colors.fill)
            .text(colors.text)
            .whitespace_nowrap()
            .slot("kbd");
        let classes = match kbd.text.clone() {
            Some(spec) => classes.role(spec),
            None => classes,
        };
        let keys = keys.into();
        self.div(classes, |b| b.text(tw(), keys));
    }

    /// An anonymous icon, one em square unless sized by classes.
    pub fn glyph(&mut self, tw: Tw, icon: Icon) {
        self.display(tw, NodeKind::Glyph, Prim::Icon(icon));
    }

    /// An anonymous image. Its size is its texture's when egui knows it, and
    /// otherwise comes from classes.
    pub fn image(&mut self, tw: Tw, src: ImageSource<'static>) {
        self.display(tw, NodeKind::Image, Prim::Image(src));
    }

    /// An empty anonymous box. Orientation and colour come from `tw`.
    pub fn rule(&mut self, tw: Tw) {
        let path = self.anonymous_path(NodeKind::Rule);
        self.leaf_node(tw, Parts::anonymous(NodeKind::Rule, path));
    }

    /// Custom painting, run at draw time with the node's content rect and its
    /// text colour, which a `hover:` or `focus:` variant of the node or an
    /// ancestor may have changed.
    pub fn paint(&mut self, tw: Tw, paint: impl Fn(&Painter, Rect, Color32) + 'static) {
        self.display(tw, NodeKind::Paint, Prim::Paint(Box::new(paint)));
    }

    fn display(&mut self, tw: Tw, kind: NodeKind, prim: Prim) {
        let path = self.anonymous_path(kind);
        self.leaf_node(tw, Parts::anonymous(kind, path).with_prim(prim));
    }

    /// Registers an identified chain's own key, so that a duplicate is
    /// reported at the call that finishes the chain and not in a closure of
    /// the library.
    #[track_caller]
    pub(crate) fn claim(&mut self, slot: &Slot) {
        self.identify(slot);
    }

    #[track_caller]
    fn identify(&mut self, slot: &Slot) -> String {
        let mark = self.st.names.push(&slot.label);
        let path = self.st.names.as_str().to_owned();
        self.st.names.pop_to(mark);
        if self.st.pass == Pass::Scratch {
            return path;
        }
        if let Some(repeat) = self.st.seen.register(slot.declared, slot.id, &path) {
            crate::report::report(
                self.frame.ctx,
                repeat.problem(),
                Some(slot.declared),
                || match &repeat {
                    Repeat::Duplicate => {
                        format!("duplicate key path {path}; this one is given an id of its own")
                    }
                    Repeat::Collision { first } => format!(
                        "key paths {first} and {path} have the same id; the second is given an \
                         id of its own"
                    ),
                },
            );
        }
        #[cfg(debug_assertions)]
        if self.path == self.st.root {
            crate::key::claims::top_level(
                self.frame.ctx,
                slot.id,
                &path,
                &self.st.claimant(),
                self.st.built_at,
            );
        }
        path
    }

    fn anonymous_path(&mut self, kind: NodeKind) -> String {
        let mark = self
            .st
            .names
            .push_label(&format!("{kind:?}").to_lowercase());
        let path = self.st.names.as_str().to_owned();
        self.st.names.pop_to(mark);
        path
    }

    fn handle(&mut self, node: NodeId, id: Id) -> NodeHandle<'_, 'f, A> {
        NodeHandle {
            b: self.reborrow(),
            node,
            id,
            kind: PhantomData,
        }
    }

    fn node(&self, tw: &Tw, parts: Parts) -> Node {
        let subject = parts.id.unwrap_or_else(|| Id::NULL.with(&parts.path));
        let mut tw = Cow::Borrowed(tw);
        if parts.id.is_none() && tw.needs_identity() {
            crate::report::report(
                self.frame.ctx,
                crate::Problem::NeedsIdentity,
                Some(subject),
                || {
                    format!(
                        "{}: only identified nodes (div_id, leaf, span, icon) may sense, group or \
                         have hover, active, focus, drag, transition or animate_layout variants; \
                         they are ignored",
                        parts.path
                    )
                },
            );
            tw = Cow::Owned(tw.into_owned().without_identity());
        }
        if tw.grouping == Grouping::Group && tw.sensing.is_none() {
            crate::report::report(
                self.frame.ctx,
                crate::Problem::GroupSensesNothing,
                Some(subject),
                || {
                    format!(
                        "{}: a group must sense something to be hovered; it senses hover",
                        parts.path
                    )
                },
            );
            tw = Cow::Owned(tw.into_owned().sense_hover());
        }
        if tw.lengths == crate::tw::Lengths::Rejected {
            crate::report::report(
                self.frame.ctx,
                crate::Problem::NonFiniteLength,
                Some(subject),
                || {
                    format!(
                        "{}: a class given a length that is infinite or not a number was ignored",
                        parts.path
                    )
                },
            );
        }
        let color_source = if tw.text.color.is_some() {
            ColorSource::Own
        } else {
            ColorSource::Inherited
        };
        Node {
            identity: Identity::of(parts.id),
            path: parts.path,
            kind: parts.kind,
            paint: tw.paint,
            text: tw.text.over(self.inherited),
            style: self.style.clone(),
            text_valign: VAlign::of(&tw.style),
            prim: parts.prim,
            measure: parts.measure,
            draw_kind: DrawKind::Paint,
            z: tw.z.unwrap_or(0),
            inert: tw.inert,
            pointer_events: tw.pointer_events,
            visibility: tw.visibility,
            region: None,
            collapse: parts.collapse,
            description: None,
            announced: None,
            action: None,
            open_focus: crate::pipeline::OpenFocus::Elsewhere,
            pane: None,
            group: None,
            selection: None,
            #[cfg(debug_assertions)]
            origins: tw.origins.clone(),
            interaction: Interaction {
                sensing: tw.sensing,
                focusability: tw.focusability,
                grouping: tw.grouping,
                variants: tw.variants.clone(),
                color_source,
                hover_by: crate::pipeline::HoverBy::Hit,
                tiling: crate::tiling::Tiling::Untiled,
            },
        }
    }

    /// `tw`'s layout style in points, at the style's spacing.
    pub(crate) fn layout_of(&self, tw: &Tw) -> taffy::Style {
        tw.style_at(self.style.tokens.spacing)
    }

    /// The node's layout style, its widths in `ch` and its reservation
    /// measured in its resolved text style.
    fn resolved_style(&self, tw: &Tw, node: &Node) -> taffy::Style {
        let ctx = self.frame.ctx;
        let mut style = self.layout_of(tw);
        let font = node.text.spec.font_id(ctx);
        let mut reserved = None;
        tw.ch.resolve(
            &mut style,
            || {
                ctx.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap("0".to_owned(), font, Color32::PLACEHOLDER)
                        .size()
                        .x
                })
            },
            |reserve| {
                let width = crate::measure::reserved_width(ctx, reserve, &node.text);
                reserved = Some(width);
                width
            },
        );
        #[cfg(debug_assertions)]
        if let (Some(reserve), Some(reserved)) = (tw.ch.reserve(), reserved) {
            report_outgrown(ctx, node, reserve, reserved);
        }
        style
    }

    /// Adds a leaf to the current parent.
    fn leaf_node(&mut self, mut tw: Tw, parts: Parts) -> NodeId {
        self.apply_rules(&mut tw, parts.kind);
        let observed = self.observation(&tw, &parts);
        let node = self.node(&tw, parts);
        let style = self.resolved_style(&tw, &node);
        let id = self.st.tree.new_node(style, &[], node);
        self.kids.push(id);
        self.observe(id, observed, Vec::new());
        id
    }

    /// `tw` as its `has` rules make it once the children `kids` are built,
    /// and what was observed of each child and what it holds.
    fn closed(&mut self, tw: Tw, node: &mut Node, kids: &[NodeId]) -> (Tw, Vec<Vec<Observed>>) {
        let seen: Vec<Vec<Observed>> = match self.st.observed.is_empty() {
            true => Vec::new(),
            false => kids
                .iter()
                .filter_map(|kid| self.st.observed.remove(kid))
                .collect(),
        };
        let Some(applied) = tw.with_has(&seen) else {
            return (tw, seen);
        };
        node.paint = applied.paint;
        node.text_valign = VAlign::of(&applied.style);
        #[cfg(debug_assertions)]
        {
            node.origins = applied.origins.clone();
        }
        (applied, seen)
    }

    /// What the container these nodes are built in learns of the node a `tw`
    /// and `parts` make, if it has `has` rules.
    fn observation(&self, tw: &Tw, parts: &Parts) -> Option<Observed> {
        self.ambient.rules.observed.then(|| Observed {
            kind: parts.kind,
            facts: tw.facts().clone(),
            focused: parts.id.is_some_and(|id| self.shell().focus_within(id)),
        })
    }

    /// Keeps what is observed of `node` and what it holds, `below`, for the
    /// containers above.
    fn observe(&mut self, node: NodeId, observed: Option<Observed>, below: Vec<Vec<Observed>>) {
        if let Some(observed) = observed {
            let mut seen = vec![observed];
            seen.extend(below.into_iter().flatten());
            self.st.observed.insert(node, seen);
        }
    }

    #[track_caller]
    fn leaf_text(&self, leaf: NodeId) -> TextStyle {
        self.st
            .tree
            .node(leaf)
            .map_or_else(|| self.inherited.clone(), |node| node.text.clone())
    }

    /// Gives `tw` the classes the rules of the containers above make it.
    #[inline(always)]
    fn apply_rules(&self, tw: &mut Tw, kind: NodeKind) {
        if tw.is_layered() {
            let own = std::mem::replace(tw, Tw::blank());
            *tw = self.ambient.rules.apply_layered(&self.st.links, own, kind);
        } else if !self.ambient.rules.is_empty() {
            *tw = self.ambient.rules.apply(&self.st.links, tw, kind);
        }
    }

    /// What the controls inside a node with `classes` inherit.
    fn ambient_in(&mut self, classes: &Tw) -> Ambient {
        Ambient {
            enablement: if classes.inert {
                Enablement::Disabled
            } else {
                self.ambient.enablement
            },
            size: classes.control_size.or(self.ambient.size),
            flex: (classes.style.display == taffy::Display::Flex)
                .then_some(classes.style.flex_direction),
            rules: self.ambient.rules.within(classes, &mut self.st.links),
            part: classes.facts().said_slot().or(self.ambient.part),
            ..self.ambient
        }
    }

    /// The slot of the nearest part this builds inside: a field inside an
    /// input group is its control.
    pub(crate) fn within_part(&self) -> Option<&'static str> {
        self.ambient.part
    }

    pub(crate) fn flex(&self) -> Option<taffy::FlexDirection> {
        self.ambient.flex
    }

    /// Declares that a size changes this frame by a gesture that moves it
    /// smoothly, so layout moves start no slides.
    pub(crate) fn size_in_flight(&mut self) {
        self.frame.size_in_flight = true;
    }

    /// Builds `body` in place, its nodes drawn on `surface`.
    pub(crate) fn on_surface(
        &mut self,
        surface: Surface,
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) {
        body(&mut Builder::new(
            self.st,
            self.frame,
            self.path,
            self.kids,
            (
                self.inherited,
                Ambient {
                    surface,
                    ..self.ambient
                },
                self.style,
            ),
        ));
    }

    pub(crate) fn build_built(&self) -> Option<crate::tooltip::BuildBuilt<A>> {
        self.frame.build_built()
    }

    pub(crate) fn set_build_built(&mut self, build: crate::tooltip::BuildBuilt<A>) {
        self.frame.set_build_built(build);
    }

    /// Whether this builds inside a hover card's content, or inside an
    /// overlay nested in one.
    pub(crate) fn in_hover_card(&self) -> bool {
        let layer = |at: usize| self.frame.roots.get(at)?.layer.as_ref();
        std::iter::successors(layer(self.frame.current), |found| layer(found.parent)).any(|layer| {
            match &layer.kind {
                LayerKind::Overlay(decl) => decl.behavior.hover.is_some(),
                LayerKind::Window(_) => false,
            }
        })
    }

    /// TODO(prose): while assistive technology is on, the text `body` shows,
    /// its pieces joined by spaces in the order they are drawn: the
    /// accessible description of content shown elsewhere, such as a hover
    /// card's. `None` while it is off, without building `body`. Its nodes
    /// are built under `salt` and left out of the frame, so nothing is laid
    /// out or drawn, and no handler it registers runs.
    ///
    /// ```
    /// use eggshell_ui::{Builder, tw};
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>) {
    ///     let description = b.description_of("card", |c| {
    ///         c.text(tw(), "@nextjs");
    ///         c.text(tw(), "The React Framework");
    ///     });
    ///     if let Some(text) = description {
    ///         assert_eq!(text, "@nextjs The React Framework");
    ///     }
    /// }
    /// ```
    pub fn description_of(
        &mut self,
        salt: impl std::hash::Hash + Debug,
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) -> Option<String> {
        accesskit_is_on(self.ctx()).then(|| self.text_of(salt, body))
    }

    /// The text `body` shows, its pieces joined by spaces, in the order
    /// they are drawn. Its nodes are built under `salt` and left out of the
    /// frame, so nothing is laid out or drawn.
    pub(crate) fn text_of(
        &mut self,
        salt: impl std::hash::Hash + Debug,
        body: impl FnOnce(&mut Builder<'_, 'f, A>),
    ) -> String {
        let mut kids = Vec::new();
        let mark = self.st.names.push_label("text-of");
        body(&mut Builder::new(
            self.st,
            self.frame,
            self.path.with(("eggshell-text-of", salt)),
            &mut kids,
            (self.inherited, self.ambient, self.style),
        ));
        self.st.names.pop_to(mark);
        let tree = &self.st.tree;
        let mut pieces = Vec::new();
        let mut stack: Vec<NodeId> = kids.into_iter().rev().collect();
        while let Some(node) = stack.pop() {
            if let Some(text) = tree.node(node).and_then(|node| node.prim.as_ref()?.text()) {
                pieces.push(text.into_owned());
            }
            stack.extend(tree.children(node).into_iter().rev());
        }
        pieces.join(" ")
    }

    /// What is drawn here: the page, or a tooltip's box.
    pub(crate) fn surface(&self) -> Surface {
        self.ambient.surface
    }

    /// Builds `body` as the children of a new container whose children's key
    /// path starts at `path`.
    fn container(
        &mut self,
        mut tw: Tw,
        parts: Parts,
        path: Id,
        body: impl FnOnce(Builder<'_, 'f, A>),
    ) -> NodeId {
        self.apply_rules(&mut tw, parts.kind);
        let ambient = self.ambient_in(&tw);
        let observed = self.observation(&tw, &parts);
        let mut node = self.node(&tw, parts);
        let text = node.text.clone();
        let mut kids = Vec::new();
        body(Builder::new(
            self.st,
            self.frame,
            path,
            &mut kids,
            (&text, ambient, self.style),
        ));
        let (tw, below) = self.closed(tw, &mut node, &kids);
        let style = self.resolved_style(&tw, &node);
        let id = self.st.tree.new_node(style, &kids, node);
        self.kids.push(id);
        self.observe(id, observed, below);
        id
    }
}

impl<A: 'static> Builder<'_, '_, A> {
    /// What is being dragged, if it is a `T`: for the view to show a gap or a
    /// placeholder. See [`Dragging`].
    ///
    /// It is `None` in the frame the button is released or Esc is pressed,
    /// so that frame is not built as held, and also `None` for the frame a
    /// drag starts in: a lift is seen from the next build on. Files dragged
    /// in from outside are a drag of [`Files`](crate::Files).
    pub fn dragging<T: Any + Send + Sync>(&self) -> Option<Dragging<T>> {
        crate::dnd::dragging(self.frame.ctx, &self.st.kept)
    }

    /// Calls `handler` once when a drag of a `T` ends, with the payload and
    /// how it ended.
    ///
    /// This is declared on the builder, not on the source node, so that it is
    /// delivered also when the source is gone
    /// ([`Cause::SourceGone`](crate::Cause::SourceGone)). Declare it in every
    /// frame, as everything else.
    pub fn on_drag_end<T: Any + Send + Sync>(
        &mut self,
        handler: impl FnOnce(&mut A, Arc<T>, DragEnd) + 'static,
    ) {
        self.frame.add_end(crate::dnd::EndDecl::new(handler));
    }

    /// Runs `draw` after the node's existing draw closure, if it has one.
    fn attach_draw(&mut self, node: NodeId, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) {
        let Some(context) = self.st.tree.node_mut(node) else {
            return;
        };
        let Some(identified) = context.identity.identified_mut() else {
            crate::report::report(
                self.frame.ctx,
                crate::Problem::Internal("draw on an anonymous node"),
                None,
                || {
                    format!(
                        "{}: a draw closure on an anonymous node is dropped",
                        context.path
                    )
                },
            );
            return;
        };
        identified.draw = Some(self.frame.attach_draw(identified.draw, Box::new(draw)));
    }
}

/// The node an identified-node method just added, for attaching behaviour to
/// it. It borrows the builder for the statement, so the next sibling attaches
/// after it. Dropping it attaches nothing.
///
/// ```
/// # use eggshell_ui::tw;
/// # struct App { saves: u32 }
/// # fn demo(b: &mut eggshell_ui::Builder<'_, '_, App>) {
/// b.span("save", tw(), "Save").on_click(|app| app.saves += 1);
/// # }
/// ```
///
/// Handlers compose: [`draw`](Self::draw), [`on_click`](Self::on_click) and
/// [`on_response`](Self::on_response) may each be called any number of times
/// and run in call order, in the node's own `Ui`.
pub struct NodeHandle<'b, 'f, A, Kind = AnyNode> {
    b: Builder<'b, 'f, A>,
    node: NodeId,
    id: Id,
    kind: PhantomData<Kind>,
}

/// TODO(prose): a [`NodeHandle`] on any node.
///
/// ```
/// use eggshell_ui::{AnyNode, Builder, NodeHandle, tw};
///
/// fn card<'b, 'f>(b: &'b mut Builder<'_, 'f, ()>) -> NodeHandle<'b, 'f, (), AnyNode> {
///     b.div_id("card", tw(), |_| {})
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct AnyNode;

/// TODO(prose): a [`NodeHandle`] on a drag source, which alone takes
/// [`drag_handle`](NodeHandle::drag_handle).
///
/// ```
/// use eggshell_ui::{Builder, tw};
///
/// fn row(b: &mut Builder<'_, '_, ()>) {
///     b.div_id("row", tw(), |b| {
///         b.div_id("grip", tw().size_px(10.0), |_| {});
///     })
///     .drag_source(1_u32)
///     .drag_handle("grip");
/// }
/// ```
///
/// A handle on a node that is not a drag source does not compile:
///
/// ```compile_fail
/// use eggshell_ui::{Builder, tw};
///
/// fn row(b: &mut Builder<'_, '_, ()>) {
///     b.div_id("row", tw(), |b| {
///         b.div_id("grip", tw().size_px(10.0), |_| {});
///     })
///     .drag_handle("grip");
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DragSource;

impl<'b, 'f, A, K> NodeHandle<'b, 'f, A, K> {
    pub(crate) fn node(&self) -> NodeId {
        self.node
    }

    pub(crate) fn into_builder(self) -> Builder<'b, 'f, A> {
        self.b
    }

    pub(crate) fn id(&self) -> Id {
        self.id
    }
}

impl<'b, 'f, A: 'static, K> NodeHandle<'b, 'f, A, K> {
    /// Runs `draw` at draw time in the node's own `Ui`, which is limited to
    /// the node's content rect. See [`NodeUi`](crate::NodeUi).
    pub fn draw(mut self, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) -> Self {
        self.b.attach_draw(self.node, draw);
        self
    }

    /// Like [`draw`](Self::draw), for a closure that adds focusable or
    /// accessible egui widgets: a text field, a button. The closure still runs
    /// while the node is scrolled out of a scroll container's view, with
    /// nothing painted, so that Tab can reach the widgets and assistive
    /// technology can read them. A closure that only paints is [`draw`](Self::draw):
    /// it does not run there.
    pub fn draw_widgets(mut self, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) -> Self {
        self.edit(|context| context.draw_kind = DrawKind::Widgets);
        self.draw(draw)
    }

    /// Makes the node a drag source carrying `payload`, and makes it sense
    /// drags and show a grab cursor (see [`drag_handle`](Self::drag_handle)
    /// to lift it from one part only). The node must keep its key across rebuilds for as long as it is
    /// dragged, so a reorder that moves it keeps the drag; if it leaves the
    /// tree the drag is cancelled
    /// ([`Cause::SourceGone`](crate::Cause::SourceGone)). See [`dnd`](crate).
    pub fn drag_source<T: Any + Send + Sync>(
        mut self,
        payload: T,
    ) -> NodeHandle<'b, 'f, A, DragSource> {
        self.edit(|context| {
            let declared = context.interaction.sensing;
            context.interaction.sensing = declared.or(Sensing::Drag);
            if let Some(drag) = context.drag() {
                drag.source = Some(Source {
                    payload: Arc::new(payload),
                    activator: Activator::Itself,
                    declared,
                });
            }
        });
        let Self {
            b,
            node,
            id,
            kind: _,
        } = self;
        NodeHandle {
            b,
            node,
            id,
            kind: PhantomData,
        }
    }

    /// How the node looks while it is dragged. [`DragImage::Live`] unless
    /// set.
    pub fn drag_image(mut self, image: DragImage) -> Self {
        self.edit_drag(|drag| drag.image = image);
        self
    }

    /// The image of the drag while this node is the source: `build` makes
    /// any subtree of it. It wins over [`DragImage::Live`], and the source
    /// stays in its slot, where its [`drag_lifted`](crate::Tw::drag_lifted)
    /// look applies.
    ///
    /// `build` runs only while this source is the one lifted, so nothing is
    /// built for a drag that is not happening, and the preview first shows
    /// the build after the lift. The subtree goes under a node the library
    /// makes: out of flow, so it changes neither the layout of the siblings
    /// nor the source's `index_hint` items. Its shapes are painted on the
    /// drag layer, unclipped, with its top-left a [`Shell::drag_grab`](crate::Shell::drag_grab)
    /// behind the pointer: the live image's anchor, resolved against this
    /// frame's layout. The node is a sibling of the source, under an id
    /// derived from the source's.
    pub fn drag_preview(mut self, build: impl FnOnce(&mut Scope<'_, 'f, A>)) -> Self {
        let (source, path) = (self.id, self.path());
        self.edit_drag(|drag| drag.previewed = true);
        if crate::dnd::lifted_source(self.b.frame.ctx, &self.b.st.kept) == Some(source) {
            let slot = self.b.slot_of(
                crate::dnd::preview_id(source),
                format!("drag preview of {path}"),
            );
            self.b
                .div_id_with(&slot, tw().absolute().pointer_events_none(), build);
        }
        self
    }

    /// Makes the node take payloads of type `T`, calling `handler` when one is
    /// dropped on it. Of the targets that take a drag and hold the pointer,
    /// the innermost wins. A target needs no sense of its own.
    pub fn drop_target<T: Any + Send + Sync>(
        self,
        handler: impl FnOnce(&mut A, Drop<T>) + 'static,
    ) -> Self {
        self.drop_target_if(|_: &T| true, handler)
    }

    /// [`drop_target`](Self::drop_target) that takes only the payloads
    /// `accept` says yes to: files by extension, say.
    pub fn drop_target_if<T: Any + Send + Sync>(
        mut self,
        accept: impl Fn(&T) -> bool + 'static,
        handler: impl FnOnce(&mut A, Drop<T>) + 'static,
    ) -> Self {
        let decl = self
            .b
            .frame
            .add_drop(crate::dnd::DropDecl::new(accept, handler));
        self.edit_drag(|drag| drag.drops.push(decl));
        self
    }

    /// Makes the node take drops that arrive with no position, so long as it
    /// takes the payload: files dropped from outside on a platform that gives
    /// no pointer position. Of several nodes, the topmost wins. Without one,
    /// such a drop is refused.
    pub fn drop_fallback(mut self) -> Self {
        self.edit_drag(|drag| drag.fallback = true);
        self
    }

    /// The node's path; empty if it is missing.
    #[track_caller]
    fn path(&mut self) -> String {
        self.edit(|context| context.path.clone())
            .unwrap_or_default()
    }

    #[track_caller]
    fn edit_drag(&mut self, edit: impl FnOnce(&mut NodeDrag)) {
        self.edit(|context| context.drag().map(edit));
    }

    #[track_caller]
    fn edit<R>(&mut self, edit: impl FnOnce(&mut Node) -> R) -> Option<R> {
        self.b.st.tree.node_mut(self.node).map(edit)
    }

    /// Runs `then` with the node's response each frame it is drawn: the escape
    /// hatch for what the builder does not offer, such as hover.
    pub fn on_response(self, then: impl FnOnce(&mut A, &egui::Response) + 'static) -> Self {
        self.draw(move |app, ui| then(app, &ui.node_response()))
    }

    /// Shows `text` in a tooltip overlay while the node is hovered, also when
    /// it is disabled, as the owned widgets' `tooltip` does. The node senses
    /// hover if it senses nothing else.
    pub fn tooltip(self, text: impl Into<egui::WidgetText>) -> Self {
        self.tipped(Content::Text(text.into()))
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](Builder::kbd) chip. The
    /// content takes no input, and its text describes the node to assistive
    /// technology.
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use eggshell_ui::{Builder, tw};
    ///
    /// struct App { saves: u32 }
    ///
    /// fn save_tip(s: &mut Builder<'_, '_, ()>) {
    ///     s.text(tw(), "Save");
    ///     s.kbd("Ctrl+S");
    /// }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>) {
    ///     b.span("label", tw(), "Saves").tooltip_with(save_tip);
    ///     b.button("save", "Save")
    ///         .tooltip_with(save_tip)
    ///         .on_click(|app: &mut App| app.saves += 1);
    /// }
    /// ```
    ///
    /// The content is built over `()`, so it cannot reach the application:
    ///
    /// ```compile_fail
    /// use eggshell_ui::{Builder, tw};
    ///
    /// struct App { saves: u32 }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>) {
    ///     b.span("label", tw(), "Saves").tooltip_with(|s| {
    ///         s.button("save", "Save")
    ///             .on_click(|app: &mut App| app.saves += 1);
    ///     });
    /// }
    /// ```
    pub fn tooltip_with(
        self,
        build: impl Fn(&mut Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.tipped(Content::built(build))
    }

    /// Shows the node's whole text in a tooltip while the node is hovered,
    /// on the frames its text is drawn cut short, as
    /// [`truncate`](crate::Tw::truncate) and
    /// [`line_clamp`](crate::Tw::line_clamp) cut it. Text that fits shows
    /// none. The node must be a text node, such as [`span`](Builder::span)
    /// makes; it senses hover if it senses nothing else. Its accessible name
    /// is its whole text already, so the tooltip does not describe it.
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use eggshell_ui::{Builder, tw};
    ///
    /// fn view(b: &mut Builder<'_, '_, ()>) {
    ///     b.div(tw().w_px(80.0), |b| {
    ///         b.span("note", tw().truncate(), "A note longer than its box")
    ///             .tooltip_when_truncated();
    ///     });
    /// }
    /// ```
    pub fn tooltip_when_truncated(mut self) -> Self {
        self.senses_hover_for_tip();
        self.draw(|_, ui| {
            if let Some(text) = crate::draw::elided_text(ui) {
                let tip = Some(Tooltip::undescribed(egui::WidgetText::from(&*text)));
                crate::owned::tipped(ui, ui.node_response(), &tip);
            }
        })
    }

    fn tipped(mut self, content: Content) -> Self {
        let tip = Tooltip::new(&mut self.b, self.node, content);
        self.tip(tip)
    }

    /// Shows `tip` on the node, as [`tooltip`](Self::tooltip) does.
    pub(crate) fn tip(mut self, tip: Tooltip) -> Self {
        self.senses_hover_for_tip();
        let tip = Some(tip);
        self.draw(move |_, ui| {
            crate::owned::tipped(ui, ui.node_response(), &tip);
        })
    }

    fn senses_hover_for_tip(&mut self) {
        self.edit(|context| {
            let interaction = &mut context.interaction;
            if interaction.sensing == Sensing::None {
                interaction.focusability = crate::tw::Focusability::Skip;
            }
            interaction.sensing = interaction.sensing.or(Sensing::Hover);
        });
    }

    /// Announces the node to assistive technology as a `role`. A node
    /// with an item role (`MenuItem`, `MenuItemCheckBox`, `MenuItemRadio`,
    /// `ListBoxOption`, `GridCell`) is an item of the menu, listbox or grid
    /// it is in, for [`Nav`](crate::overlay::Nav). The node must sense; give it a name with
    /// [`named`](Self::named).
    pub fn role(mut self, role: egui::Role) -> Self {
        self.edit(|context| context.description.get_or_insert_default().role = Some(role));
        self
    }

    /// Puts the node inside `parent` for assistive technology: a table's row
    /// inside the table, or inside its row group.
    pub(crate) fn accessibility_parent(mut self, parent: Id) -> Self {
        self.edit(|context| context.description.get_or_insert_default().parent = Some(parent));
        self
    }

    /// Hovers the node whenever the pointer is over its rect, also where a
    /// sibling drawn over it takes the pointer.
    pub(crate) fn hover_by_rect(mut self) -> Self {
        self.edit(|context| context.interaction.hover_by = crate::pipeline::HoverBy::Rect);
        self
    }

    /// Lets the node's sensing children tile its padding box for the
    /// pointer: a press in a gap or in the padding goes to the nearest.
    pub(crate) fn tile_hits(mut self) -> Self {
        self.edit(|context| context.interaction.tiling = crate::tiling::Tiling::Tiled);
        self
    }

    /// Gives the node focus as the overlay it is in opens, instead of the
    /// overlay's first focusable: the current colour of a swatch picker. For
    /// an overlay that takes focus in ([`Focus::Enter`](crate::overlay::Focus));
    /// the node, or a widget inside it, must take focus. If two nodes of one overlay are marked, the
    /// first takes focus and it is reported as
    /// [`FocusOnOpenTwice`](crate::Problem::FocusOnOpenTwice).
    pub fn focus_on_open(self) -> Self {
        self.open_focus(crate::pipeline::OpenFocus::Here)
    }

    pub(crate) fn open_focus(mut self, open_focus: crate::pipeline::OpenFocus) -> Self {
        self.edit(|context| context.open_focus = open_focus);
        self
    }

    pub(crate) fn sizes_pane(mut self, side: crate::owned::Side) -> Self {
        self.edit(|context| context.pane = Some(side));
        self
    }

    /// Opens a context menu, which `items` fills, at the pointer when the
    /// node is secondary-clicked. The menu is shown while it is open, and
    /// the library owns whether it is. The node must be identified; if it
    /// does not sense clicks it starts to, and stays out of the Tab order.
    #[track_caller]
    pub fn context_menu(
        mut self,
        items: impl for<'m> FnOnce(&mut crate::owned::Menu<'m, 'f, A>),
    ) -> Self {
        let (owner, path) = (self.id, self.path());
        self.edit(|context| {
            let interaction = &mut context.interaction;
            if interaction.sensing == Sensing::None || interaction.sensing == Sensing::Hover {
                interaction.focusability = crate::tw::Focusability::Skip;
            }
            interaction.sensing = interaction.sensing.or(Sensing::Click);
        });
        let kept = self.b.kept().clone();
        let mut this =
            self.draw(move |_, ui| crate::owned::open_on_secondary_click(ui, &kept, owner));
        crate::owned::context(this.b.reborrow(), owner, &path, items);
        this
    }

    /// Makes the node the button that Return or Esc presses.
    pub(crate) fn action(mut self, action: crate::actions::Action) -> Self {
        self.edit(|context| context.action = Some(action));
        self
    }

    /// Notes that the node's draw closure announces it as `role`, which
    /// decides what the node does with keys before it is drawn.
    pub(crate) fn announces(mut self, role: egui::Role) -> Self {
        self.edit(|context| context.announced = Some(role));
        self
    }

    /// Announces whether what the node controls is open, the `aria-expanded`
    /// of the web: for the header of a [`collapsible`](Builder::collapsible),
    /// with the [`Button`](egui::Role::Button) role.
    pub fn expanded(self, open: bool) -> Self {
        self.draw(move |_, ui| {
            let id = ui.node_response().id;
            ui.ctx()
                .accesskit_node_builder(id, |node| node.set_expanded(open));
        })
    }

    /// Makes the node a live region, the `aria-live` of the web: assistive
    /// technology announces changes to its content, waiting for a pause in
    /// speech when it is [`Polite`](egui::accesskit::Live::Polite). Give it a
    /// [`role`](Self::role), such as [`Status`](egui::Role::Status), and a
    /// name.
    pub fn live(self, live: egui::accesskit::Live) -> Self {
        self.draw(move |_, ui| {
            let id = ui.node_response().id;
            ui.ctx()
                .accesskit_node_builder(id, |node| node.set_live(live));
        })
    }

    /// TODO(prose): makes the node a group of items the arrow keys move focus through as `nav` says, one Tab stop (the focused item, else the [`selected`](Self::selected) one, else the first enabled); a `Listbox` or `Submenu` nav is reported as [`NavNeedsOverlay`](crate::Problem::NavNeedsOverlay).
    ///
    /// ```
    /// use eggshell_ui::overlay::{Nav, Wrap};
    /// use eggshell_ui::owned::Mark;
    /// use eggshell_ui::{Builder, tw};
    /// use egui::Role;
    ///
    /// struct App { page: usize }
    ///
    /// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    ///     b.div_id("pages", tw().flex(), |b| {
    ///         for (page, label) in ["One", "Two"].into_iter().enumerate() {
    ///             b.div_id(page, tw().sense_click(), |b| b.text(tw(), label))
    ///                 .role(Role::Tab)
    ///                 .named(label)
    ///                 .selected(Mark::from(app.page == page))
    ///                 .on_click(move |app: &mut App| app.page = page);
    ///         }
    ///     })
    ///     .role(Role::TabList)
    ///     .named("Pages")
    ///     .nav(Nav::Row(Wrap::Around))
    ///     .selection_follows_focus();
    /// }
    /// ```
    #[track_caller]
    pub fn nav(mut self, nav: crate::overlay::Nav) -> Self {
        if matches!(
            nav,
            crate::overlay::Nav::Listbox(_) | crate::overlay::Nav::Submenu(_)
        ) {
            let path = self.path();
            crate::report::report_at(
                self.b.frame.ctx,
                crate::Problem::NavNeedsOverlay,
                Some(self.id),
                std::panic::Location::caller(),
                || format!("{path}: nav({nav:?}) is an overlay's; the node is no group"),
            );
        }
        let group = nav.on_page().then(|| crate::roving::Group::new(nav));
        self.edit(|context| context.group = group);
        self
    }

    /// TODO(prose): makes the node an item of the [`nav`](Self::nav) group it is in whatever its role: a button that is one of an accordion's headers among the buttons in its content.
    pub fn nav_item(mut self) -> Self {
        self.edit(|context| {
            context.description.get_or_insert_default().member = crate::scroll::Member::Marked;
        });
        self
    }

    /// TODO(prose): an arrow key that moves focus to an item of this [`nav`](Self::nav) group also activates it, as a click would.
    pub fn selection_follows_focus(mut self) -> Self {
        self.edit(|context| {
            if let Some(group) = context.group.as_mut() {
                group.follows = crate::roving::Follows::Selection;
            }
        });
        self
    }

    /// TODO(prose): whether the node is the chosen item of its [`nav`](Self::nav) group, its Tab stop while focus is elsewhere; announced as selected, or as checked on a radio button.
    pub fn selected(mut self, mark: crate::owned::Mark) -> Self {
        self.edit(|context| context.selection = Some(mark));
        self.draw(move |_, ui| {
            let id = ui.node_response().id;
            let on = mark == crate::owned::Mark::On;
            ui.ctx()
                .accesskit_node_builder(id, |node| match node.role() {
                    egui::accesskit::Role::RadioButton => node.set_toggled(if on {
                        egui::accesskit::Toggled::True
                    } else {
                        egui::accesskit::Toggled::False
                    }),
                    _ => node.set_selected(on),
                });
        })
    }

    /// Announces that the node controls the node `target`, the
    /// `aria-controls` of the web: a section header the region it opens, a
    /// tab its panel. `target` is that node's id, from
    /// [`Builder::id_of`]. The node must sense; the relation is announced
    /// only in a frame where `target` is in the accessibility tree too, which
    /// a node that does not sense, or is not built, is not.
    ///
    /// ```
    /// # use eggshell_ui::{Builder, tw};
    /// # use eggshell_ui::overlay::Shown;
    /// struct App { open: bool }
    ///
    /// fn item<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    ///     let region = b.id_of("details");
    ///     b.div_id("head", tw().sense_click(), |b| b.text(tw(), "Details"))
    ///         .role(egui::Role::Button)
    ///         .expanded(app.open)
    ///         .controls(region)
    ///         .on_click(|app: &mut App| app.open = !app.open);
    ///     b.collapsible("details", Shown::from(app.open))
    ///         .classes(tw().sense_hover())
    ///         .children(|b| b.text(tw(), "Only as tall as it needs"))
    ///         .role(egui::Role::Region)
    ///         .named("Details");
    /// }
    /// ```
    pub fn controls(mut self, target: Id) -> Self {
        self.edit(|context| context.description.get_or_insert_default().controls = Some(target));
        self
    }

    /// The node's accessible name, the `aria-label` of the web.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.edit(|context| context.description.get_or_insert_default().name = Some(name));
        self
    }

    /// The node's accessible description, the `aria-description` of the
    /// web: read after its name, such as a dialog's explanatory sentence.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        let description = description.into();
        self.edit(|context| {
            context.description.get_or_insert_default().description = Some(description);
        });
        self
    }

    /// Runs `act` when the node is clicked. The node must sense clicks.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) -> Self {
        self.draw(move |app, ui| {
            if ui.node_response().clicked()
                || ui
                    .node_id()
                    .is_some_and(|id| crate::nav::take_activation(ui, id))
            {
                act(app);
            }
        })
    }
}

impl<'f, A: 'static> NodeHandle<'_, 'f, A, DragSource> {
    /// Makes the node of this source's subtree declared with `key` its
    /// handle: only a drag that starts on the handle lifts the source. The
    /// rest of the source keeps its own pointer behaviour: its buttons click,
    /// its text selects, and a node in it that senses drags gets them.
    /// Everything else of the drag is the source's: its payload, its image,
    /// its place among a target's items, and the grab offset, measured from
    /// the source's top-left. The handle senses drags, is focusable, and shows
    /// a grab cursor; give it a name ([`named`](Self::named)) that says what
    /// it moves.
    ///
    /// `key` is resolved as [`id_of`](Builder::id_of) resolves it in the
    /// source's body, so anonymous containers between the two do not count,
    /// but a handle inside another identified container cannot be named.
    ///
    /// A `key` that names no node of the subtree, or names a drag source, is
    /// reported as [`DragHandle`](crate::Problem::DragHandle), and the whole
    /// source lifts.
    #[track_caller]
    pub fn drag_handle<K: Key>(mut self, key: K) -> Self {
        let (source, path) = (self.id, self.path());
        let ctx = self.b.frame.ctx;
        let handle = child_id(source, &key);
        let at = std::panic::Location::caller();
        let refused = |why: &str| {
            crate::report::report_at(ctx, crate::Problem::DragHandle, Some(handle), at, || {
                format!("{path}: drag_handle({key:?}) {why}; the whole source lifts")
            });
        };
        let Some(found) = self.descendant(handle) else {
            refused("names no node in the source's subtree");
            return self;
        };
        let handle_is_source = self
            .b
            .st
            .tree
            .node(found)
            .and_then(|node| node.identity.drag())
            .is_some_and(|drag| drag.source.is_some());
        if handle_is_source {
            refused("names a drag source; a handle is not one");
            return self;
        }
        self.edit(|context| {
            let declared = context.drag().and_then(|drag| {
                let source = drag.source.as_mut()?;
                source.activator = Activator::Handle(handle);
                Some(source.declared)
            });
            if let Some(declared) = declared {
                context.interaction.sensing = declared;
            }
        });
        if let Some(handle_node) = self.b.st.tree.node_mut(found) {
            handle_node.interaction.sensing = handle_node.interaction.sensing.or(Sensing::Drag);
        }
        self
    }

    /// The node with `id` in this node's subtree, not the node itself.
    fn descendant(&self, id: Id) -> Option<NodeId> {
        let tree = &self.b.st.tree;
        let mut pending = tree.children(self.node);
        while let Some(node) = pending.pop() {
            if tree.node(node).and_then(|context| context.identity.id()) == Some(id) {
                return Some(node);
            }
            pending.extend(tree.children(node));
        }
        None
    }
}

/// TODO(prose): the node [`spans`](Builder::spans) adds.
pub struct SpansHandle<'b, 'f, A>(NodeHandle<'b, 'f, A>);

impl<'b, 'f, A: 'static> SpansHandle<'b, 'f, A> {
    /// TODO(prose)
    pub fn on_link(self, on_link: impl FnOnce(&mut A, &str) + 'static) -> NodeHandle<'b, 'f, A> {
        self.0.draw(move |app, ui| {
            if let Some(href) = crate::draw::node_id_or_report(ui, "following a link")
                .and_then(|id| crate::spans::take_followed(ui.ctx(), id))
            {
                on_link(app, &href);
            }
        })
    }

    /// TODO(prose)
    pub fn node(self) -> NodeHandle<'b, 'f, A> {
        self.0
    }
}

/// A [`Builder`] inside a keyed scope: what [`Builder::scope`] and
/// [`Builder::div_id`] hand to their bodies.
///
/// It has the whole builder API (it dereferences to [`Builder`]), and a
/// `&mut Scope` can be passed wherever a `&mut Builder` is wanted. The
/// reverse is the point: a component function that takes `&mut Scope` cannot
/// be called with a plain builder, so every use of a component sits under a
/// key its caller chose, and two uses in one parent need two keys.
///
/// ```
/// use eggshell_ui::{Scope, tw};
///
/// #[derive(Hash, Debug)]
/// enum Field {
///     Label,
/// }
///
/// fn labelled(s: &mut Scope<'_, '_, ()>, label: &str) {
///     s.span(Field::Label, tw(), label);
/// }
///
/// # fn demo(b: &mut eggshell_ui::Builder<'_, '_, ()>) {
/// b.scope("name", |s| labelled(s, "Name"));
/// b.scope("email", |s| labelled(s, "Email"));
/// # }
/// ```
///
/// A component can also hand back a control's chain for its caller to finish,
/// so that the caller adds the click handler:
///
/// ```
/// use eggshell_ui::Scope;
/// use eggshell_ui::owned::ButtonChain;
///
/// struct App { saves: u32 }
///
/// #[derive(Hash, Debug)]
/// enum Part { Save }
///
/// fn save<'a, 'f>(s: &'a mut Scope<'_, 'f, App>, key: Part) -> ButtonChain<'a, 'f, App> {
///     s.button(key, "Save").tooltip("Saves")
/// }
///
/// # fn demo(b: &mut eggshell_ui::Builder<'_, '_, App>) {
/// b.scope("form", |s| save(s, Part::Save).on_click(|app| app.saves += 1));
/// # }
/// ```
///
/// A chain borrows its builder until it is finished, so it cannot outlive the
/// body that started it:
///
/// ```compile_fail
/// use eggshell_ui::Builder;
/// use eggshell_ui::owned::ButtonChain;
///
/// fn view(b: &mut Builder<'_, '_, ()>) {
///     let mut later: Option<ButtonChain<'_, '_, ()>> = None;
///     b.div(eggshell_ui::tw(), |b| later = Some(b.button("go", "Go")));
///     later.expect("started").add();
/// }
/// ```
pub struct Scope<'b, 'f, A>(Builder<'b, 'f, A>);

impl<A> Scope<'_, '_, A> {
    /// TODO(prose): whether this scope is the highlighted item of a listbox.
    pub fn is_active(&self) -> bool {
        crate::nav::is_highlighted(self.0.kept(), self.0.path)
    }
}

impl<'b, 'f, A> Deref for Scope<'b, 'f, A> {
    type Target = Builder<'b, 'f, A>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> DerefMut for Scope<'_, '_, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// egui has no public query for this; its root node is made only while
/// accesskit is on.
fn accesskit_is_on(ctx: &egui::Context) -> bool {
    ctx.has_accesskit_node(egui::accesskit_root_id())
}

/// Reports a text leaf whose text is wider than the widest template it
/// reserves, `reserved` points: the leaf grew past its reservation.
#[cfg(debug_assertions)]
fn report_outgrown(ctx: &egui::Context, node: &Node, reserve: &crate::Reserve, reserved: f32) {
    let Some(prim) = &node.prim else {
        return;
    };
    let Some(leaf) = crate::measure::TextLeaf::of(ctx, prim, (&node.text, &node.style.rich)) else {
        return;
    };
    let galley = ctx.fonts_mut(|fonts| leaf.unwrapped(fonts));
    let width = galley.size().x;
    if width <= reserved + 0.5 {
        return;
    }
    let subject = node
        .identity
        .id()
        .unwrap_or_else(|| Id::NULL.with(&node.path));
    crate::report::report(ctx, crate::Problem::OutgrewReserve, Some(subject), || {
        let templates: Vec<_> = reserve.templates().map(|t| format!("{t:?}")).collect();
        format!(
            "{}: {:?} is {width:.1} points wide, wider than the {reserved:.1} its node reserves \
             for the widest of [{}]; the node grew, so its neighbours moved",
            node.path,
            galley.text(),
            templates.join(", ")
        )
    });
}

/// The points `side` is, when it is a length; a percentage or `auto` counts
/// as none.
fn length_px(side: taffy::CompactLength) -> f32 {
    match side {
        side if side.tag() == taffy::CompactLength::LENGTH_TAG => side.value(),
        _ => 0.0,
    }
}

/// A box's top and bottom padding, in points.
fn block_padding(style: &taffy::Style) -> f32 {
    length_px(style.padding.top.into_raw()) + length_px(style.padding.bottom.into_raw())
}

/// A box's top and bottom border, in points.
fn block_border(style: &taffy::Style) -> f32 {
    length_px(style.border.top.into_raw()) + length_px(style.border.bottom.into_raw())
}

/// What a box's padding and border take of its height, in points.
pub(crate) fn block_insets(style: &taffy::Style) -> f32 {
    block_padding(style) + block_border(style)
}
