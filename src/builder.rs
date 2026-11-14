//! The nested-closure builder.

use std::any::Any;
use std::fmt::{self, Debug};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use egui::{Color32, Id, ImageSource, Painter, Rect};
use taffy::{NodeId, Size};

use crate::contract::Style;
use crate::dnd::{DragEnd, DragImage, Dragging, Drop, NodeDrag};
use crate::draw::NodeUi;
use crate::key::{Key, child_id};
use crate::measure::Measure;
use crate::overlay::Band;
use crate::overlay_chain::{OverlayChain, Unplaced};
use crate::owned::Enablement;
use crate::pipeline::{
    BuildState, ColorSource, DismissFn, DrawKind, Frame, Interaction, MeasureFn, Node, NodeKind,
    Pass, ScrollNode, solve,
};
use crate::presence::{self, Motion, Step};
use crate::prim::{Icon, Prim};
use crate::region::{RegionBody, RegionDecl};
use crate::roots::{OverlayDecl, Presentation, RootDecl, RootKind, overlay_style};
use crate::scroll::TabRule;
use crate::scroll::{ScrollChain, ScrollOptions};
use crate::sizing::Width;
use crate::text::{TextStyle, VAlign};
use crate::tooltip::{Content, Tooltip};
use crate::tw::{Grouping, PointerEvents, Sensing, Tw, tw};
use crate::widget::Stock;

/// What a node is made of, besides its classes.
struct Parts {
    kind: NodeKind,
    id: Option<Id>,
    path: String,
    prim: Option<Prim>,
    measure: Option<usize>,
}

impl Node {
    fn drag(&mut self) -> &mut NodeDrag {
        self.drag.get_or_insert_with(Default::default)
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
pub(crate) struct Slot {
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
    /// A slot with `id` that debugs as `readable`.
    pub(crate) fn labelled(id: Id, readable: String) -> Self {
        Self {
            id,
            label: Label(readable),
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
pub struct Builder<'b, 'f, A> {
    st: &'b mut BuildState,
    frame: &'b mut Frame<'f, A>,
    path: Id,
    kids: &'b mut Vec<NodeId>,
    inherited: &'b TextStyle,
    ambient: Ambient,
}

/// The leaf a virtual list's rows are built into, as its region needs it.
pub(crate) struct RowsLeaf {
    leaf: NodeId,
    readable: String,
    text: TextStyle,
    ambient: Ambient,
}

/// What a subtree's nodes take from where it is built, beyond text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ambient {
    pub(crate) enablement: Enablement,
    pub(crate) surface: Surface,
}

impl Ambient {
    pub(crate) const PAGE: Self = Self {
        enablement: Enablement::Enabled,
        surface: Surface::Page,
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
        frame: &'b mut Frame<'f, A>,
        path: Id,
        kids: &'b mut Vec<NodeId>,
        (inherited, ambient): (&'b TextStyle, Ambient),
    ) -> Self {
        Self {
            st,
            frame,
            path,
            kids,
            inherited,
            ambient,
        }
    }

    /// `own`, and disabled when the control is inside an
    /// [`inert`](Tw::inert) subtree, as a `<fieldset disabled>` disables the
    /// controls inside it.
    pub(crate) fn enablement_of(&self, own: Enablement) -> Enablement {
        self.ambient.enablement.and(own)
    }

    /// Builds `body` with a key pushed onto the path. No node is created;
    /// the children attach to the current parent.
    ///
    /// `body` gets a [`Scope`], which components take: a component can only be
    /// placed where its caller named the instance.
    pub fn scope<K: Key>(&mut self, key: K, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        self.scope_with(child_id(self.path, &key), &key, body);
    }

    pub(crate) fn scope_with(
        &mut self,
        id: Id,
        key: &dyn Debug,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) {
        let mark = self.st.names.push(key);
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            id,
            self.kids,
            (self.inherited, self.ambient),
        )));
        self.st.names.pop_to(mark);
    }

    /// The same builder, borrowed for the life of a chain.
    pub(crate) fn reborrow(&mut self) -> Builder<'_, 'f, A> {
        Builder::new(
            self.st,
            self.frame,
            self.path,
            self.kids,
            (self.inherited, self.ambient),
        )
    }

    /// `key` resolved in this scope, for a chain that is finished later.
    pub(crate) fn slot<K: Key>(&self, key: &K) -> Slot {
        Slot {
            id: child_id(self.path, key),
            label: Label(format!("{key:?}")),
        }
    }

    /// Whether an overlay `natural` points big, anchored to the node `anchor`
    /// and placed by `placement`, has the room it wants.
    pub(crate) fn room_for(
        &self,
        anchor: Id,
        placement: &crate::overlay::Placement,
        natural: egui::Vec2,
    ) -> crate::pipeline::Room {
        crate::pipeline::room_for(self.frame.ctx, &self.st.kept, anchor, placement, natural)
    }

    pub(crate) fn kept(&self) -> &crate::kept::Handle {
        &self.st.kept
    }

    /// The egui context the frame is built with, for reading the local state
    /// that draw closures keep in egui's memory under ids from
    /// [`id_of`](Self::id_of).
    pub fn ctx(&self) -> &egui::Context {
        self.frame.ctx
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
    /// inside it, has keyboard focus. See [`focus_within`](crate::focus_within).
    pub fn has_focus_within<K: Key>(&self, key: K) -> bool {
        crate::focus::focus_within(self.frame.ctx, self.id_of(key))
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
        self.div_id_with(child_id(self.path, &key), &key, tw, body)
    }

    #[track_caller]
    pub(crate) fn div_id_with(
        &mut self,
        id: Id,
        key: &dyn Debug,
        tw: Tw,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'_, 'f, A> {
        let path = self.identify(id, key);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::DivId, path)
        };
        let mark = self.st.names.push(key);
        self.container(tw, parts, id, |b| body(&mut Scope(b)));
        self.st.names.pop_to(mark);
        self.handle()
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
    /// state stays with it while it is scrolled away and back. `rows` and the
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
        rows: &'f [R],
    ) -> crate::vlist::ListChain<'_, 'f, A, R>
    where
        A: 'static,
    {
        let slot = self.slot(&key);
        crate::vlist::ListChain::new(self.reborrow(), slot, tw, rows)
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
        }: RowsLeaf,
        list: Id,
        run: crate::vlist::ListRun<'f, A>,
    ) {
        self.frame.roots[self.frame.current]
            .regions
            .push(RegionDecl {
                leaf,
                id: list,
                readable,
                text,
                ambient,
                body: RegionBody::List(run),
            });
    }

    /// A handle to `node`, a node this builder added.
    pub(crate) fn into_handle(self, node: NodeId) -> NodeHandle<'b, 'f, A> {
        NodeHandle { b: self, node }
    }

    /// The text style in effect here.
    pub(crate) fn text_style(&self) -> &TextStyle {
        self.inherited
    }

    /// The leaf a virtual list's rows are built into once it is laid out:
    /// as wide as the content and `height` tall.
    pub(crate) fn rows_leaf(&mut self, height: f32) -> RowsLeaf {
        let id = child_id(self.path, &crate::vlist::RowsKey);
        let readable = self.identify(id, &crate::vlist::RowsKey);
        let tw = tw().w_full().h_px(height);
        let ambient = self.ambient_in(&tw);
        self.leaf_node(
            tw,
            Parts {
                id: Some(id),
                ..Parts::anonymous(NodeKind::Region, readable.clone())
            },
        );
        let leaf = *self.kids.last().expect("the leaf was just added");
        let text = self
            .st
            .tree
            .get_node_context(leaf)
            .expect("the leaf has a context")
            .text
            .clone();
        RowsLeaf {
            leaf,
            readable,
            text,
            ambient,
        }
    }

    /// How many roots have been declared so far this frame.
    pub(crate) fn roots_declared(&self) -> usize {
        self.frame.roots.len()
    }

    /// The overlays declared from the root at `from` on.
    pub(crate) fn overlays_from(&self, from: usize) -> impl Iterator<Item = Id> + '_ {
        self.frame.roots[from..]
            .iter()
            .filter_map(|root| match &root.kind {
                RootKind::Overlay(overlay) => Some(overlay.id),
                RootKind::Base => None,
            })
    }

    /// The nodes the overlays declared so far are anchored to.
    pub(crate) fn anchored_to(&self) -> impl Iterator<Item = Id> + '_ {
        self.frame.roots.iter().filter_map(|root| match &root.kind {
            RootKind::Overlay(overlay) => match overlay.presentation {
                Presentation::Floating {
                    anchor: crate::overlay::Anchor::Node(id),
                    ..
                } => Some(id),
                _ => None,
            },
            RootKind::Base => None,
        })
    }

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
        let id = child_id(self.path, &key);
        let readable = self.identify(id, &key);
        let ambient = self.ambient_in(&tw);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::Region, readable.clone())
        };
        self.leaf_node(tw, parts);
        let leaf = *self.kids.last().expect("the leaf was just added");
        let text = self
            .st
            .tree
            .get_node_context(leaf)
            .expect("the leaf has a context")
            .text
            .clone();
        self.frame.roots[self.frame.current]
            .regions
            .push(RegionDecl {
                leaf,
                id,
                readable,
                text,
                ambient,
                body: RegionBody::Closure(Box::new(body)),
            });
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
            (self.inherited, self.ambient),
        ));
        let root = st
            .tree
            .new_with_children(overlay_style(), &kids)
            .expect("children were created in this tree");
        let loose = Size {
            width: taffy::AvailableSpace::MaxContent,
            height: taffy::AvailableSpace::MaxContent,
        };
        solve(ctx, &frame.measures, &mut st.tree, root, loose);
        Width::new(st.tree.layout(root).expect("node exists").size.width)
    }

    /// An overlay: a floating layer placed against an anchor. See
    /// [`overlay`](crate::overlay).
    #[track_caller]
    pub fn overlay<K: Key>(&mut self, key: K) -> OverlayChain<'_, 'f, A, Unplaced> {
        let slot = self.slot(&key);
        OverlayChain::new(self.reborrow(), slot)
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
            match presence::step(&self.st.kept, self.frame.ctx, slot.id, &motion) {
                Step::Closed { exited } => {
                    if let Some(then) = on_exited.filter(|_| exited) {
                        self.frame.exited.push(then);
                    }
                    return;
                }
                Step::Present(staged) => decl.staged = staged,
            }
        }
        let band = decl.band;
        let classes = match decl.presentation {
            Presentation::Modal(_) => tw().bg(Style::of(self.ctx()).scrim),
            Presentation::Floating { .. } => tw(),
        };
        if let Some(on_dismiss) = on_dismiss {
            self.frame.dismiss.insert(slot.id, on_dismiss);
        }
        let path = self.identify(slot.id, &slot.label);
        let parent = self.frame.current;
        self.frame.roots.push(RootDecl::overlay(decl, parent));
        let at = self.frame.roots.len() - 1;
        self.frame.current = at;
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
        let mark = self.st.names.push(&slot.label);
        let mut kids = Vec::new();
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            slot.id,
            &mut kids,
            (&text, self.ambient),
        )));
        self.st.names.pop_to(mark);
        self.frame.current = parent;
        let root = self
            .st
            .tree
            .new_with_children(overlay_style(), &kids)
            .expect("children were created in this tree");
        self.st
            .tree
            .set_node_context(root, Some(node))
            .expect("node was just created");
        self.frame.roots[at].node = Some(root);
    }

    /// Adds a scroll container's leaf to the current parent, with the root of
    /// its content, built from `body`, a tree of its own.
    pub(crate) fn scroll_with(
        mut self,
        id: Id,
        key: &dyn Debug,
        classes: Tw,
        options: ScrollOptions,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'b, 'f, A> {
        use taffy::{Overflow, Point};
        let ScrollOptions {
            axis,
            edge,
            bars,
            focusable,
            description,
        } = options;
        let path = self.identify(id, key);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::Scroll, path)
        };
        let mut leaf = self.node(&classes, parts);
        let mark = self.st.names.push(key);
        let content_path = self.anonymous_path(NodeKind::ScrollContent);
        let mut kids = Vec::new();
        let ambient = self.ambient_in(&classes);
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            id,
            &mut kids,
            (&leaf.text, ambient),
        )));
        self.st.names.pop_to(mark);
        let mut content_node = self.node(
            &tw(),
            Parts::anonymous(NodeKind::ScrollContent, content_path),
        );
        content_node.text = leaf.text.clone();
        let mut style = classes.style.clone();
        let content_style = crate::scroll::take_content_layout(&mut style);
        let content = self
            .st
            .tree
            .new_with_children(content_style, &kids)
            .expect("children were created in this tree");
        self.st
            .tree
            .set_node_context(content, Some(content_node))
            .expect("node was just created");
        let tab = TabRule::of(focusable, &classes, &self.st.tree, &kids);
        leaf.scroll = Some(ScrollNode {
            axis,
            edge,
            bars,
            tab,
            description,
            content,
            size: egui::Vec2::ZERO,
        });
        style.overflow = Point {
            x: Overflow::Scroll,
            y: Overflow::Scroll,
        };
        let id = self
            .st
            .tree
            .new_leaf_with_context(style, leaf)
            .expect("taffy accepts a new leaf");
        self.kids.push(id);
        self.frame.add_scroll(id);
        NodeHandle { b: self, node: id }
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
        self.leaf_with(child_id(self.path, &key), &key, tw, measure)
    }

    #[track_caller]
    pub(crate) fn leaf_with(
        &mut self,
        id: Id,
        key: &dyn Debug,
        tw: Tw,
        measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f,
    ) -> NodeHandle<'_, 'f, A> {
        let path = self.identify(id, key);
        let measure: MeasureFn<'f> = Box::new(measure);
        self.frame.measures.push(measure);
        let parts = Parts {
            id: Some(id),
            measure: Some(self.frame.measures.len() - 1),
            ..Parts::anonymous(NodeKind::Leaf, path)
        };
        self.leaf_node(tw, parts);
        self.handle()
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
        let id = child_id(self.path, &key);
        let path = self.identify(id, &key);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::Span, path).with_prim(Prim::Text(text.into()))
        };
        self.leaf_node(tw, parts);
        self.handle()
    }

    /// An identified icon, measured and painted by the library.
    #[track_caller]
    pub fn icon<K: Key>(&mut self, key: K, tw: Tw, icon: Icon) -> NodeHandle<'_, 'f, A> {
        let id = child_id(self.path, &key);
        let path = self.identify(id, &key);
        let parts = Parts {
            id: Some(id),
            ..Parts::anonymous(NodeKind::Icon, path).with_prim(Prim::Icon(icon))
        };
        self.leaf_node(tw, parts);
        self.handle()
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
        let style = crate::Style::of(self.frame.ctx);
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
            .px(kbd.padding_x / 4.0)
            .border(kbd.border_width)
            .border_color(colors.border)
            .rounded_px(f32::from(kbd.radius))
            .bg(colors.fill)
            .text(colors.text)
            .whitespace_nowrap();
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

    /// Registers an identified chain's own key, so that a duplicate panics at
    /// the call that finishes the chain and not in a closure of the library.
    #[track_caller]
    pub(crate) fn claim(&mut self, slot: &Slot) {
        self.identify(slot.id, &slot.label);
    }

    #[track_caller]
    fn identify(&mut self, id: Id, key: &dyn Debug) -> String {
        let mark = self.st.names.push(key);
        let path = self.st.names.as_str().to_owned();
        self.st.names.pop_to(mark);
        if self.st.pass == Pass::Scratch {
            return path;
        }
        self.st.seen.register(id, &path);
        #[cfg(debug_assertions)]
        if self.path == self.st.root {
            crate::key::claims::top_level(
                self.frame.ctx,
                id,
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

    /// A handle on the node just added to the current parent.
    fn handle(&mut self) -> NodeHandle<'_, 'f, A> {
        let node = *self.kids.last().expect("a node was just added");
        NodeHandle {
            b: self.reborrow(),
            node,
        }
    }

    fn node(&self, tw: &Tw, parts: Parts) -> Node {
        debug_assert!(
            parts.id.is_some() || !tw.needs_identity(),
            "{}: only identified nodes (div_id, leaf, span, icon) may sense, group or have hover, active, transition or animate_layout variants",
            parts.path,
        );
        debug_assert!(
            tw.grouping != Grouping::Group || !tw.sensing.is_none(),
            "{}: a group must sense something to be hovered",
            parts.path,
        );
        let color_source = if tw.text.color.is_some() {
            ColorSource::Own
        } else {
            ColorSource::Inherited
        };
        Node {
            id: parts.id,
            path: parts.path,
            kind: parts.kind,
            paint: tw.paint,
            text: tw.text.over(self.inherited),
            text_valign: VAlign::of(&tw.style),
            prim: parts.prim,
            measure: parts.measure,
            draw: None,
            draw_kind: DrawKind::Paint,
            z: tw.z.unwrap_or(0),
            inert: tw.inert,
            pointer_events: tw.pointer_events,
            drag: None,
            scroll: None,
            region: None,
            description: None,
            action: None,
            interaction: Interaction {
                sensing: tw.sensing,
                focusability: tw.focusability,
                grouping: tw.grouping,
                variants: tw.variants,
                color_source,
            },
        }
    }

    /// The node's layout style, its widths in `ch` measured in its resolved font.
    fn resolved_style(&self, tw: Tw, node: &Node) -> taffy::Style {
        let mut style = tw.style;
        let font = node.text.spec.font_id();
        tw.ch.resolve(&mut style, || {
            self.frame.ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap("0".to_owned(), font, Color32::PLACEHOLDER)
                    .size()
                    .x
            })
        });
        style
    }

    fn leaf_node(&mut self, tw: Tw, parts: Parts) {
        let node = self.node(&tw, parts);
        let style = self.resolved_style(tw, &node);
        let id = self
            .st
            .tree
            .new_leaf_with_context(style, node)
            .expect("taffy accepts a new leaf");
        self.kids.push(id);
    }

    /// What the controls inside a node with `classes` inherit.
    fn ambient_in(&self, classes: &Tw) -> Ambient {
        if classes.inert {
            Ambient {
                enablement: Enablement::Disabled,
                ..self.ambient
            }
        } else {
            self.ambient
        }
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
            ),
        ));
    }

    /// The text `body` shows, its pieces joined by spaces, in the order
    /// they are drawn. Its nodes are built under `salt` and left out of the
    /// frame, so nothing is laid out or drawn.
    pub(crate) fn build_built(&self) -> Option<crate::tooltip::BuildBuilt<A>> {
        self.frame.build_built
    }

    pub(crate) fn set_build_built(&mut self, build: crate::tooltip::BuildBuilt<A>) {
        self.frame.build_built = Some(build);
    }

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
            (self.inherited, self.ambient),
        ));
        self.st.names.pop_to(mark);
        let tree = &self.st.tree;
        let mut pieces = Vec::new();
        let mut stack: Vec<NodeId> = kids.into_iter().rev().collect();
        while let Some(node) = stack.pop() {
            if let Some(text) = tree
                .get_node_context(node)
                .and_then(|node| node.prim.as_ref()?.text())
            {
                pieces.push(text.to_owned());
            }
            stack.extend(tree.children(node).into_iter().flatten().rev());
        }
        pieces.join(" ")
    }

    /// What is drawn here: the page, or a tooltip's box.
    pub(crate) fn surface(&self) -> Surface {
        self.ambient.surface
    }

    /// Builds `body` as the children of a new container whose children's key
    /// path starts at `path`.
    fn container(&mut self, tw: Tw, parts: Parts, path: Id, body: impl FnOnce(Builder<'_, 'f, A>)) {
        let ambient = self.ambient_in(&tw);
        let node = self.node(&tw, parts);
        let style = self.resolved_style(tw, &node);
        let text = node.text.clone();
        let mut kids = Vec::new();
        body(Builder::new(
            self.st,
            self.frame,
            path,
            &mut kids,
            (&text, ambient),
        ));
        let id = self
            .st
            .tree
            .new_with_children(style, &kids)
            .expect("children were created in this tree");
        self.st
            .tree
            .set_node_context(id, Some(node))
            .expect("node was just created");
        self.kids.push(id);
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
        crate::dnd::dragging(self.frame.ctx)
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
        self.frame.drag.add_end(handler);
    }

    /// Runs `draw` after the node's existing draw closure, if it has one.
    fn attach_draw(&mut self, node: NodeId, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) {
        let context = self
            .st
            .tree
            .get_node_context_mut(node)
            .expect("the handle's node was added to this tree");
        let draws = &mut self.frame.draws;
        match context.draw {
            Some(slot) => {
                let first = std::mem::replace(&mut draws[slot], Box::new(|_, _| {}));
                draws[slot] = Box::new(move |app, ui| {
                    first(app, ui);
                    draw(app, ui);
                });
            }
            None => {
                context.draw = Some(draws.len());
                draws.push(Box::new(draw));
            }
        }
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
pub struct NodeHandle<'b, 'f, A> {
    b: Builder<'b, 'f, A>,
    node: NodeId,
}

impl<A> NodeHandle<'_, '_, A> {
    pub(crate) fn node(&self) -> NodeId {
        self.node
    }

    /// The node's id; it is identified.
    pub(crate) fn id(&self) -> Id {
        self.b
            .st
            .tree
            .get_node_context(self.node)
            .and_then(|node| node.id)
            .expect("the node is identified")
    }
}

impl<'f, A: 'static> NodeHandle<'_, 'f, A> {
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
    /// drags. The node must keep its key across rebuilds for as long as it is
    /// dragged, so a reorder that moves it keeps the drag; if it leaves the
    /// tree the drag is cancelled
    /// ([`Cause::SourceGone`](crate::Cause::SourceGone)). See [`dnd`](crate).
    pub fn drag_source<T: Any + Send + Sync>(mut self, payload: T) -> Self {
        self.edit(|context| {
            context.interaction.sensing = context.interaction.sensing.or(Sensing::Drag);
            context.drag().source = Some(Arc::new(payload));
        });
        self
    }

    /// How the node looks while it is dragged. [`DragImage::Live`] unless
    /// set.
    pub fn drag_image(mut self, image: DragImage) -> Self {
        self.edit(|context| context.drag().image = image);
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
    /// drag layer, unclipped, with its top-left a [`drag_grab`](crate::drag_grab)
    /// behind the pointer: the live image's anchor, resolved against this
    /// frame's layout. The node is a sibling of the source, under an id
    /// derived from the source's.
    pub fn drag_preview(mut self, build: impl FnOnce(&mut Scope<'_, 'f, A>)) -> Self {
        let (source, path) = {
            let context = self.node_context();
            (context.id, context.path.clone())
        };
        self.edit(|context| context.drag().previewed = true);
        if let Some(source) =
            source.filter(|&id| crate::dnd::lifted_source(self.b.frame.ctx) == Some(id))
        {
            let label = format!("drag preview of {path}");
            self.b.div_id_with(
                crate::dnd::preview_id(source),
                &label,
                tw().absolute().pointer_events_none(),
                build,
            );
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
        let decl = self.b.frame.drag.add_drop(accept, handler);
        self.edit(|context| context.drag().drops.push(decl));
        self
    }

    /// Makes the node take drops that arrive with no position, so long as it
    /// takes the payload: files dropped from outside on a platform that gives
    /// no pointer position. Of several nodes, the topmost wins. Without one,
    /// such a drop is refused.
    pub fn drop_fallback(mut self) -> Self {
        self.edit(|context| context.drag().fallback = true);
        self
    }

    fn node_context(&mut self) -> &mut Node {
        self.b
            .st
            .tree
            .get_node_context_mut(self.node)
            .expect("the handle's node was added to this tree")
    }

    fn edit(&mut self, edit: impl FnOnce(&mut Node)) {
        let context = self
            .b
            .st
            .tree
            .get_node_context_mut(self.node)
            .expect("the handle's node was added to this tree");
        edit(context);
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
    pub fn tooltip_with(
        self,
        build: impl Fn(&mut Builder<'_, '_, A>) + Send + Sync + 'static,
    ) -> Self {
        self.tipped(Content::built(build))
    }

    fn tipped(mut self, content: Content) -> Self {
        self.edit(|context| {
            let interaction = &mut context.interaction;
            if interaction.sensing == Sensing::None {
                interaction.focusability = crate::tw::Focusability::Skip;
            }
            interaction.sensing = interaction.sensing.or(Sensing::Hover);
        });
        let tip = Some(Tooltip::new(&mut self.b, self.node, content));
        self.draw(move |_, ui| {
            crate::owned::tipped(ui, ui.node_response(), &tip);
        })
    }

    /// Announces the node to assistive technology as a `role`. A node
    /// with an item role (`MenuItem`, `MenuItemCheckBox`, `MenuItemRadio`,
    /// `ListBoxOption`) is an item of the menu or listbox it is in, for
    /// [`Nav`](crate::overlay::Nav). The node must sense; give it a name with
    /// [`named`](Self::named).
    pub fn role(mut self, role: egui::Role) -> Self {
        self.edit(|context| context.description.get_or_insert_default().role = Some(role));
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
        let (owner, path) = {
            let context = self.node_context();
            (
                context.id.expect("a context menu needs an identified node"),
                context.path.clone(),
            )
        };
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

    /// The node's accessible name, the `aria-label` of the web.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.edit(|context| context.description.get_or_insert_default().name = Some(name));
        self
    }

    /// Runs `act` when the node is clicked. The node must sense clicks.
    #[track_caller]
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) -> Self {
        self.draw(move |app, ui| {
            if ui.node_response().clicked() || crate::nav::take_activation(ui.ctx(), ui.node_id()) {
                act(app);
            }
        })
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
///     s.button(key, "Save").named("Save the form").tooltip("Saves")
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
