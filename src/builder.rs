//! The nested-closure builder.

use std::ops::{Deref, DerefMut};

use egui::{Color32, Id, ImageSource, Painter, Rect};
use taffy::{NodeId, Size};

use crate::key::{Key, child_id};
use crate::measure::Measure;
use crate::pipeline::{
    BuildState, ColorSource, DrawFn, Frame, Interaction, MeasureFn, Node, NodeKind,
};
use crate::prim::{Icon, Prim};
use crate::text::{TextStyle, VAlign};
use crate::tw::{Grouping, Tw};
use crate::widget::Stock;

/// What a node is made of, besides its classes.
struct Parts {
    kind: NodeKind,
    id: Option<Id>,
    path: String,
    prim: Option<Prim>,
    measure: Option<usize>,
    draw: Option<usize>,
}

impl Parts {
    fn anonymous(kind: NodeKind, path: String) -> Self {
        Self {
            kind,
            id: None,
            path,
            prim: None,
            measure: None,
            draw: None,
        }
    }

    fn with_prim(self, prim: Prim) -> Self {
        Self {
            prim: Some(prim),
            ..self
        }
    }
}

/// Builds one frame's tree. Nodes attach to the current parent.
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
        let mark = self.st.names.push(&key);
        body(&mut Scope(Builder::new(
            self.st,
            self.frame,
            child_id(self.path, &key),
            self.kids,
            self.inherited,
        )));
        self.st.names.pop_to(mark);
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

    /// An identified container with a draw closure. Its children are scoped under `key`.
    pub fn div_id<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static,
        body: impl FnOnce(&mut Scope<'_, 'f, A>),
    ) {
        let (id, path) = self.identify(&key);
        let draw = self.push_draw(draw);
        let parts = Parts {
            id: Some(id),
            draw: Some(draw),
            ..Parts::anonymous(NodeKind::DivId, path)
        };
        let mark = self.st.names.push(&key);
        self.container(tw, parts, id, |b| body(&mut Scope(b)));
        self.st.names.pop_to(mark);
    }

    /// An identified leaf whose size comes from `measure` and which draws itself.
    ///
    /// `measure` may borrow application data for the frame and is dropped when
    /// layout returns; [`fill`](crate::fill) sizes a leaf by classes.
    pub(crate) fn leaf<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        measure: impl Fn(&Measure<'_>) -> Size<f32> + 'f,
        draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static,
    ) {
        let (id, path) = self.identify(&key);
        let measure: MeasureFn<'f> = Box::new(measure);
        self.frame.measures.push(measure);
        let parts = Parts {
            id: Some(id),
            measure: Some(self.frame.measures.len() - 1),
            draw: Some(self.push_draw(draw)),
            ..Parts::anonymous(NodeKind::Leaf, path)
        };
        self.leaf_node(tw, parts);
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
    ) {
        let measured = widget.clone();
        self.leaf(
            key,
            tw,
            move |m| measured.measure(m),
            move |app, ui| draw(app, ui, widget),
        );
    }

    /// An identified text leaf, measured and painted by the library.
    pub fn span<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        text: impl Into<String>,
        draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static,
    ) {
        let (id, path) = self.identify(&key);
        let parts = Parts {
            id: Some(id),
            draw: Some(self.push_draw(draw)),
            ..Parts::anonymous(NodeKind::Span, path).with_prim(Prim::Text(text.into()))
        };
        self.leaf_node(tw, parts);
    }

    /// An identified icon, measured and painted by the library.
    pub fn icon<K: Key>(
        &mut self,
        key: K,
        tw: Tw,
        icon: Icon,
        draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static,
    ) {
        let (id, path) = self.identify(&key);
        let parts = Parts {
            id: Some(id),
            draw: Some(self.push_draw(draw)),
            ..Parts::anonymous(NodeKind::Icon, path).with_prim(Prim::Icon(icon))
        };
        self.leaf_node(tw, parts);
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

    fn identify<K: Key>(&mut self, key: &K) -> (Id, String) {
        let id = child_id(self.path, key);
        let mark = self.st.names.push(key);
        let path = self.st.names.as_str().to_owned();
        self.st.names.pop_to(mark);
        self.st.seen.register(id, &path);
        (id, path)
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

    fn push_draw(&mut self, draw: impl FnOnce(&mut A, &mut egui::Ui) + 'static) -> usize {
        let draw: DrawFn<A> = Box::new(draw);
        self.frame.draws.push(draw);
        self.frame.draws.len() - 1
    }

    fn node(&self, tw: &Tw, parts: Parts) -> Node {
        debug_assert!(
            parts.id.is_some() || !tw.needs_identity(),
            "{}: only identified nodes (div_id, leaf, span, icon) may sense, group or have hover, active or transition variants",
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
            draw: parts.draw,
            z: tw.z.unwrap_or(0),
            inert: tw.inert,
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
/// use bullseye::{Scope, tw};
///
/// #[derive(Hash, Debug)]
/// enum Field {
///     Label,
/// }
///
/// fn labelled(s: &mut Scope<'_, '_, ()>, label: &str) {
///     s.span(Field::Label, tw(), label, bullseye::none);
/// }
///
/// # fn demo(b: &mut bullseye::Builder<'_, '_, ()>) {
/// b.scope("name", |s| labelled(s, "Name"));
/// b.scope("email", |s| labelled(s, "Email"));
/// # }
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

/// A draw for identified nodes the application does not draw, such as anchors.
pub fn none<A>(_: &mut A, _: &mut egui::Ui) {}
