//! The nested-closure builder.

use std::any::Any;
use std::fmt::{self, Debug};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use egui::{Color32, Id, ImageSource, Painter, Rect};
use taffy::{NodeId, Size};

use crate::dnd::{DragEnd, DragImage, Dragging, Drop, NodeDrag};
use crate::draw::NodeUi;
use crate::key::{Key, child_id};
use crate::measure::Measure;
use crate::pipeline::{BuildState, ColorSource, Frame, Interaction, MeasureFn, Node, NodeKind};
use crate::prim::{Icon, Prim};
use crate::text::{TextStyle, VAlign};
use crate::tw::{Grouping, Sensing, Tw};
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
}

impl<'b, 'f, A> Builder<'b, 'f, A> {
    pub(crate) fn new(
        st: &'b mut BuildState,
        frame: &'b mut Frame<'f, A>,
        path: Id,
        kids: &'b mut Vec<NodeId>,
        inherited: &'b TextStyle,
    ) -> Self {
        Self {
            st,
            frame,
            path,
            kids,
            inherited,
        }
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
            self.inherited,
        )));
        self.st.names.pop_to(mark);
    }

    /// The same builder, borrowed for the life of a chain.
    pub(crate) fn reborrow(&mut self) -> Builder<'_, 'f, A> {
        Builder::new(self.st, self.frame, self.path, self.kids, self.inherited)
    }

    /// `key` resolved in this scope, for a chain that is finished later.
    pub(crate) fn slot<K: Key>(&self, key: &K) -> Slot {
        Slot {
            id: child_id(self.path, key),
            label: Label(format!("{key:?}")),
        }
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
    pub fn div_id<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) -> NodeHandle<'_, 'f, A> {
        self.div_id_with(child_id(self.path, &key), &key, tw, body)
    }

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
            .draw(move |app, ui| draw(app, ui, widget));
    }

    /// An identified text leaf, measured and painted by the library.
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

    fn identify(&mut self, id: Id, key: &dyn Debug) -> String {
        let mark = self.st.names.push(key);
        let path = self.st.names.as_str().to_owned();
        self.st.names.pop_to(mark);
        self.st.seen.register(id, &path);
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
            z: tw.z.unwrap_or(0),
            inert: tw.inert,
            drag: None,
            interaction: Interaction {
                sensing: tw.sensing,
                focusability: tw.focusability,
                grouping: tw.grouping,
                variants: tw.variants,
                color_source,
            },
        }
    }

    fn leaf_node(&mut self, tw: Tw, parts: Parts) {
        let node = self.node(&tw, parts);
        let id = self
            .st
            .tree
            .new_leaf_with_context(tw.style, node)
            .expect("taffy accepts a new leaf");
        self.kids.push(id);
    }

    /// Builds `body` as the children of a new container whose children's key
    /// path starts at `path`.
    fn container(&mut self, tw: Tw, parts: Parts, path: Id, body: impl FnOnce(Builder<'_, 'f, A>)) {
        let node = self.node(&tw, parts);
        let text = node.text.clone();
        let mut kids = Vec::new();
        body(Builder::new(self.st, self.frame, path, &mut kids, &text));
        let id = self
            .st
            .tree
            .new_with_children(tw.style, &kids)
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

impl<A: 'static> NodeHandle<'_, '_, A> {
    /// Runs `draw` at draw time in the node's own `Ui`, which is limited to
    /// the node's content rect. See [`NodeUi`](crate::NodeUi).
    pub fn draw(mut self, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) -> Self {
        self.b.attach_draw(self.node, draw);
        self
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

    /// Shows `text` while the node is hovered, also when it is disabled, as
    /// the owned widgets' `tooltip` does.
    pub fn tooltip(self, text: impl Into<egui::WidgetText>) -> Self {
        let tip = Some(text.into());
        self.draw(move |_, ui| {
            crate::owned::tipped(ui.node_response(), &tip);
        })
    }

    /// Runs `act` when the node is clicked. The node must sense clicks.
    pub fn on_click(self, act: impl FnOnce(&mut A) + 'static) -> Self {
        self.draw(move |app, ui| {
            if ui.node_response().clicked() {
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
