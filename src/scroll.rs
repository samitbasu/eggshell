//! Scroll containers: [`Builder::scroll`](crate::Builder::scroll).
//!
//! A scroll container is a leaf of the tree it sits in, sized by its classes
//! like any other leaf. What it scrolls is a separate root, laid out in the
//! same frame as the tree around it: at the viewport's width when it scrolls
//! [`Axis::Y`], at its height when it scrolls [`Axis::X`], and unconstrained
//! along the axis it scrolls. The content's size is therefore right in the
//! first frame, with nothing remembered from the last one.
//!
//! egui draws it. The content is shown inside a [`egui::ScrollArea`] keyed by
//! the node's id, so egui owns the offset, the wheel, the scroll bars and the
//! momentum, and the offset survives rebuilds. The content is laid out at the
//! offset egui is at (clamped to the content's size), so the content root's
//! rect moves as the container scrolls and the rects of the nodes inside are
//! screen rects at every moment. A node's position within its parent does not
//! change, which is what `animate_layout` compares: scrolling alone starts
//! no slide.
//!
//! The viewport clips both painting and hit testing, and the draw does not
//! paint nodes that are entirely outside it: their draw closures do not run.
//! A scrolled-out node that is focusable or has an accessible role or name
//! still registers its widget at its real rect, painting nothing: Tab
//! reaches it, scrolls it into view, and assistive technology reads it. That
//! is the node's one hit test and, if it adds widgets, its draw closure with
//! the opacity at zero ([`NodeHandle::draw_widgets`](crate::NodeHandle::draw_widgets));
//! a node that only paints costs a counter lookup. Text selection does not
//! extend into culled text. A node that holds the keyboard focus, or the
//! drag that is in progress, is always visited.
//!
//! # The keyboard
//!
//! A focused container scrolls with ↑ ↓ ← → (one line: the container's text
//! line height), PageUp / PageDown (the viewport less one line), Home / End
//! and Space / Shift+Space, as a browser does; the target is the innermost
//! container that holds the focus, or, with nothing focused, the one last
//! pressed in. A focused control that uses a key (a text field, a slider)
//! has it first, and a key never chains to the container around at an edge.
//! A container is a Tab stop, as Chrome's keyboard-focusable scrollers, when
//! its content can scroll (it is larger than the viewport on an axis it
//! scrolls) and nothing inside it is focusable, or always with
//! [`Focusable::Always`]; [`ScrollChain::role`] and [`ScrollChain::named`]
//! describe it to assistive technology. Whether something inside is
//! focusable follows from the classes of what is inside, when the container
//! is built (a widget added by a draw closure counts only if it was added
//! with [`NodeHandle::draw_widgets`](crate::NodeHandle::draw_widgets)); the
//! overflow is the layout's, so a container becomes a stop as its content
//! grows. Being a stop affects no layout. A focused container paints the
//! style's focus ring inside its border box, **last**: over its content, bars
//! and fades, as a browser's outline, taking no space and following the
//! container's corner radius. A container whose classes set a `focus:` look
//! keeps that look and gets no ring. Focus that moves to a node
//! scrolls it into view.
//!
//! # Scroll bars
//!
//! There are no overlay bars. A container always reserves a **gutter** on the
//! axis it scrolls (the width for `Axis::Y`, the height for `X`, both for
//! `Both`), whether or not its content overflows, as CSS
//! `scrollbar-gutter: stable`, and lays the content out in the viewport
//! without it. Layout is therefore single-pass: nothing depends on overflow,
//! and content that grows past the viewport does not reflow. The bar is drawn
//! in the gutter only on an axis where the content is larger than the
//! viewport; otherwise the gutter is empty (a track is drawn only if the style
//! gives it a colour, and only beside a bar). [`Viewport::rect`] is the viewport
//! without the gutter. `overflow-y-auto` and `overflow-y-scroll` mean this and
//! nothing else, and a container's own overflow classes are replaced by it.
//!
//! The bar is egui's, which handles its drag, painted with the colours of
//! [`ScrollStyle`] ([`Style::scroll`](crate::contract::Style::scroll); the
//! shadcn preset gives shadcn's `ScrollArea`). [`ScrollChain::scrollbar_style`]
//! changes it for one container.
//!
//! # The wheel
//!
//! The innermost container that can still move takes the wheel; at its edge
//! egui lets the wheel chain to the container around it. The wheel works
//! while another widget is being dragged.
//!
//! # Drags
//!
//! While a drag is live ([`drag_pointer`]) and the
//! pointer is near the edge of a container, the container scrolls by
//! [`EdgeScroll`]'s velocity and the frame is repainted, without any code of
//! the application. [`ScrollChain::edge_scroll`] changes the band and the
//! speed and [`ScrollChain::no_edge_scroll`] turns it off. Of nested
//! containers, the innermost that can move in the direction wins.
//!
//! # Programmatic scrolling
//!
//! [`scroll_by`], [`scroll_to_rect`] and [`scroll_into_view`] ask for a
//! scroll. The request is applied by the next layout, to that frame's rects,
//! so a node added in the same frame can be scrolled to. They move at once,
//! without egui's animation. [`viewport`] reads where the viewport and the
//! content are.

use egui::{Align, Context, Id, Pos2, Rect, Role, Vec2, Vec2b};

use crate::builder::{Builder, NodeHandle, Scope, Slot};
use crate::contract::Style;
pub use crate::contract::{EdgeFade, ScrollStyle};
use crate::dnd::{EdgeScroll, drag_pointer};
use crate::key::Key;
use crate::pipeline::{Node, PlacedNode, takes_tab};
use crate::tw::{Focusability, Tw};

/// Along which axes a scroll container scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Vertically: the content is as wide as the viewport and as tall as it
    /// needs.
    #[default]
    Y,
    /// Horizontally: the content is as tall as the viewport and as wide as
    /// it needs.
    X,
    /// Both ways: the content is as large as it needs in both.
    Both,
}

impl Axis {
    pub(crate) fn enabled(self) -> Vec2b {
        match self {
            Self::Y => Vec2b::new(false, true),
            Self::X => Vec2b::new(true, false),
            Self::Both => Vec2b::TRUE,
        }
    }
}

/// Whether a scroll container is a Tab stop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Focusable {
    /// Only when its content can scroll and nothing inside it is a Tab stop,
    /// as Chrome's keyboard-focusable scrollers: plain scrolling text is
    /// reachable with Tab, a list of controls is reached through them, and
    /// content that fits is no stop.
    #[default]
    Auto,
    /// Always, before the controls inside it, as a transcript with
    /// `tabindex="0"`.
    Always,
}

/// When a scroll container is a Tab stop, as decided when it is built from
/// its classes and the classes of what is inside it. It has no effect on
/// layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TabRule {
    #[default]
    Never,
    Always,
    /// Only while the content overflows the viewport on an axis that scrolls.
    WhenScrollable,
}

impl TabRule {
    /// The rule of a container built from `classes`, with the nodes `kids`
    /// as its content.
    pub(crate) fn of(
        focusable: Focusable,
        classes: &Tw,
        tree: &taffy::TaffyTree<Node>,
        kids: &[taffy::NodeId],
    ) -> Self {
        let takes = classes.focusability == Focusability::Tab && !classes.inert;
        match focusable {
            _ if !takes => Self::Never,
            Focusable::Always => Self::Always,
            Focusable::Auto if holds_tab_stop(tree, kids) => Self::Never,
            Focusable::Auto => Self::WhenScrollable,
        }
    }
}

/// Whether one of `nodes`, or of their descendants, is a Tab stop by what its
/// classes declare. A nested scroll container counts: it is one or holds
/// one. Inert and hidden subtrees do not.
fn holds_tab_stop(tree: &taffy::TaffyTree<Node>, nodes: &[taffy::NodeId]) -> bool {
    nodes.iter().any(|&id| {
        let hidden = tree
            .style(id)
            .is_ok_and(|style| style.display == taffy::Display::None);
        match tree.get_node_context(id) {
            Some(node) if node.inert || hidden => false,
            Some(node) if node.scroll.is_some() => true,
            Some(node) if takes_tab(&node.interaction, node.draw_kind) => true,
            _ => holds_tab_stop(tree, &tree.children(id).unwrap_or_default()),
        }
    })
}

/// A scroll container, started with [`Builder::scroll`] and finished with
/// [`children`](Self::children).
///
/// The container is a leaf of the tree it sits in: give it a size with its
/// classes (`h_full`, `grow`, `h_px(200.0)`). Its children are scoped under
/// its key.
///
/// As in CSS, the container is the flex or grid container of its children
/// and the whole thing scrolls. The classes split in two:
///
/// - **the content's**, which scroll with it: padding and everything that
///   lays out the children: `display` (`flex`, `grid`, `block`), `direction`,
///   `text-align`, flex direction and wrap, `gap`, `align-items`,
///   `align-content`, `justify-content`, `justify-items`, and the grid
///   templates (rows, columns, areas, line names) and auto tracks and flow;
/// - **the frame's**, which stay put: size, min/max size, aspect ratio,
///   margin, border, background, radius, shadow,
///   opacity, and the container's own placement in its parent (`self-*`,
///   `grow`, `shrink`, `basis`, position and inset, grid placement).
///
/// `hidden` hides the container, content and all. The split is
/// `take_content_layout` in this module, the one list of it.
///
/// ```
/// use eggshell_ui::scroll::Axis;
/// use eggshell_ui::{Builder, tw};
///
/// fn log(b: &mut Builder<'_, '_, ()>, lines: &[String]) {
///     b.scroll("log", tw().h_px(200.0).p(2.0))
///         .axis(Axis::Y)
///         .children(|s| {
///             for (n, line) in lines.iter().enumerate() {
///                 s.div_id(n, tw(), |s| s.text(tw(), line.clone()));
///             }
///         });
/// }
/// ```
#[must_use = "a scroll container is not added until you call `.children(..)`"]
pub struct ScrollChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: Tw,
    axis: Axis,
    edge: Option<EdgeScroll>,
    bars: Option<ScrollStyle>,
    fade: Option<EdgeFade>,
    focusable: Focusable,
    role: Option<Role>,
    name: Option<String>,
}

impl<'a, 'f, A: 'static> ScrollChain<'a, 'f, A> {
    pub(crate) fn new<K: Key>(b: Builder<'a, 'f, A>, key: K, classes: Tw) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            classes,
            axis: Axis::default(),
            edge: Some(EdgeScroll::default()),
            bars: None,
            fade: None,
            focusable: Focusable::default(),
            role: None,
            name: None,
        }
    }

    /// Whether the container is a Tab stop. [`Focusable::Auto`] unless set.
    /// A focused container scrolls with the keys (arrows, PageUp/PageDown,
    /// Home/End, Space); a focused control inside it gets them first.
    pub fn focusable(mut self, focusable: Focusable) -> Self {
        self.focusable = focusable;
        self
    }

    /// The container's accessible role, such as [`Role::Log`] for a
    /// transcript. [`Role::ScrollView`] unless set.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// The container's accessible name, the `aria-label` of the web.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Which way the container scrolls. [`Axis::Y`] unless set.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// The container's scroll bar, instead of the style's
    /// ([`Style::scroll`](crate::contract::Style::scroll)): its width is the
    /// gutter the container reserves.
    pub fn scrollbar_style(mut self, bars: ScrollStyle) -> Self {
        self.bars = Some(bars);
        self
    }

    /// Fades the container's edges by `fade` where more content lies beyond
    /// them, for a carousel or a tab strip. Containers do not fade unless
    /// this is set ([`ScrollStyle::fade`] is off).
    pub fn fade_edges(mut self, fade: EdgeFade) -> Self {
        self.fade = Some(fade);
        self
    }

    /// Scrolls the container by `edge` when a drag is near its edge. The
    /// default is [`EdgeScroll::default`].
    pub fn edge_scroll(mut self, edge: EdgeScroll) -> Self {
        self.edge = Some(edge);
        self
    }

    /// Does not scroll the container when a drag is near its edge.
    pub fn no_edge_scroll(mut self) -> Self {
        self.edge = None;
        self
    }

    /// Builds `body` as the content and adds the container. The returned
    /// handle attaches behaviour to the container's node.
    #[track_caller]
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) -> NodeHandle<'a, 'f, A> {
        let Self {
            b,
            slot,
            classes,
            axis,
            edge,
            bars,
            fade,
            focusable,
            role,
            name,
        } = self;
        let mut style = bars.unwrap_or_else(|| Style::of(b.ctx()).scroll);
        if let Some(fade) = fade {
            style.fade = fade;
        }
        let bars = Bars::new(style, axis);
        let options = ScrollOptions {
            axis,
            edge,
            bars,
            focusable,
            description: Description { role, name },
        };
        b.scroll_with(slot.id, &slot.label, classes, options, body)
    }
}

/// Where a scroll container's viewport and content are in screen space, as
/// the last layout of the container put them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// The visible part: the inside of the container's border.
    pub rect: Rect,
    /// The content's root. Its top-left is the origin of the content's
    /// coordinates: a point `p` in the content is at `content.min + p` on
    /// the screen.
    pub content: Rect,
}

impl Viewport {
    /// How far the content is scrolled: the content coordinates of the
    /// viewport's top-left.
    pub fn offset(&self) -> Vec2 {
        self.rect.min - self.content.min
    }

    /// `rect`, a screen rect, in content coordinates.
    pub fn to_content(&self, rect: Rect) -> Rect {
        rect.translate(-self.content.min.to_vec2())
    }

    /// `rect`, in content coordinates, as a screen rect.
    pub fn to_screen(&self, rect: Rect) -> Rect {
        rect.translate(self.content.min.to_vec2())
    }
}

/// The viewport of the scroll container `scroll`, as the last layout put it,
/// or `None` if none was laid out. A draw closure sees its own frame's; a
/// build sees the previous frame's.
pub fn viewport(ctx: &Context, scroll: Id) -> Option<Viewport> {
    ctx.data(|data| data.get_temp(viewport_key(scroll)))
}

/// Scrolls the container `scroll` by `delta`: positive y shows what is
/// further down, positive x what is further right. The offset stays within
/// the content.
pub fn scroll_by(ctx: &Context, scroll: Id, delta: Vec2) {
    submit(
        ctx,
        Request {
            container: Some(scroll),
            wish: Wish::By(delta),
            ..Request::new(ctx)
        },
    );
}

/// Scrolls the container `scroll` to show `rect`, a rect in the content's
/// coordinates ([`Viewport::to_content`]). With an `align`, the rect is
/// aligned in the viewport that way; without, the content moves the least
/// that shows it, and a rect larger than the viewport shows its start.
pub fn scroll_to_rect(ctx: &Context, scroll: Id, rect: Rect, align: Option<Align>) {
    submit(
        ctx,
        Request {
            container: Some(scroll),
            wish: Wish::Rect { rect, align },
            ..Request::new(ctx)
        },
    );
}

/// Scrolls every scroll container that holds the node `node` to show it,
/// the innermost first: the focused control of a nested list is shown in
/// each of its ancestors. `align` as in [`scroll_to_rect`].
pub fn scroll_into_view(ctx: &Context, node: Id, align: Option<Align>) {
    submit(
        ctx,
        Request {
            container: None,
            wish: Wish::Node { node, align },
            ..Request::new(ctx)
        },
    );
}

/// A request is kept this many passes for a layout to apply it.
const REQUEST_LIFE: u64 = 2;

#[derive(Clone, Debug)]
enum Wish {
    By(Vec2),
    Rect { rect: Rect, align: Option<Align> },
    Node { node: Id, align: Option<Align> },
}

#[derive(Clone, Debug)]
struct Request {
    pass: u64,
    container: Option<Id>,
    wish: Wish,
    applied: Applied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Applied {
    No,
    Yes,
}

impl Request {
    fn new(ctx: &Context) -> Self {
        Self {
            pass: ctx.cumulative_pass_nr(),
            container: None,
            wish: Wish::By(Vec2::ZERO),
            applied: Applied::No,
        }
    }
}

#[derive(Clone, Default)]
struct Pending(Vec<Request>);

fn pending_key() -> Id {
    crate::key::root_id().with("scroll-requests")
}

fn viewport_key(scroll: Id) -> Id {
    scroll.with("eggshell-scroll-viewport")
}

fn submit(ctx: &Context, request: Request) {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Pending>(pending_key())
            .0
            .push(request);
    });
    ctx.request_repaint();
}

/// The salt of the `ScrollArea`'s id.
pub(crate) const AREA_SALT: &str = "area";

/// The scope of the `Ui` a container's `ScrollArea` is shown in.
pub(crate) fn area_scope(node: Id) -> Id {
    node.with("eggshell-scroll")
}

/// The id egui keeps the container's scroll state under: the one
/// `ScrollArea::id_salt` makes in `area_scope`.
pub(crate) fn state_id(node: Id) -> Id {
    area_scope(node).with(egui::IdSalt::new(AREA_SALT))
}

/// Splits a scroll container's `frame` style in two. The returned style is the
/// content root's: every property that lays out the container's children,
/// moved out of `frame`. What stays in `frame` is the container's box.
///
/// This is the one list of the split; [`ScrollChain`]'s docs describe it.
/// A `display: none` container stays hidden and its content is a block.
pub(crate) fn take_content_layout(frame: &mut taffy::Style) -> taffy::Style {
    use std::mem::replace;
    use taffy::Display;
    let fresh = taffy::Style::default();
    let display = match frame.display {
        Display::None => Display::Block,
        _ => std::mem::replace(&mut frame.display, Display::Block),
    };
    taffy::Style {
        display,
        direction: replace(&mut frame.direction, fresh.direction),
        text_align: replace(&mut frame.text_align, fresh.text_align),
        padding: replace(&mut frame.padding, fresh.padding),
        flex_direction: replace(&mut frame.flex_direction, fresh.flex_direction),
        flex_wrap: replace(&mut frame.flex_wrap, fresh.flex_wrap),
        gap: replace(&mut frame.gap, fresh.gap),
        align_items: replace(&mut frame.align_items, fresh.align_items),
        align_content: replace(&mut frame.align_content, fresh.align_content),
        justify_content: replace(&mut frame.justify_content, fresh.justify_content),
        justify_items: replace(&mut frame.justify_items, fresh.justify_items),
        grid_template_rows: replace(&mut frame.grid_template_rows, fresh.grid_template_rows),
        grid_template_columns: replace(
            &mut frame.grid_template_columns,
            fresh.grid_template_columns,
        ),
        grid_template_areas: replace(&mut frame.grid_template_areas, fresh.grid_template_areas),
        grid_template_row_names: replace(
            &mut frame.grid_template_row_names,
            fresh.grid_template_row_names,
        ),
        grid_template_column_names: replace(
            &mut frame.grid_template_column_names,
            fresh.grid_template_column_names,
        ),
        grid_auto_rows: replace(&mut frame.grid_auto_rows, fresh.grid_auto_rows),
        grid_auto_columns: replace(&mut frame.grid_auto_columns, fresh.grid_auto_columns),
        grid_auto_flow: replace(&mut frame.grid_auto_flow, fresh.grid_auto_flow),
        ..taffy::Style::default()
    }
}

/// What a scroll container is built with.
pub(crate) struct ScrollOptions {
    pub(crate) axis: Axis,
    pub(crate) edge: Option<EdgeScroll>,
    pub(crate) bars: Bars,
    pub(crate) focusable: Focusable,
    pub(crate) description: Description,
}

/// How a container is announced by assistive technology, if it is.
#[derive(Clone, Debug, Default)]
pub(crate) struct Description {
    pub(crate) role: Option<Role>,
    pub(crate) name: Option<String>,
}

impl Description {
    pub(crate) fn is_set(&self) -> bool {
        self.role.is_some() || self.name.is_some()
    }
}

/// The viewport of a container whose padding box is `padding` and whose
/// bars take `gutter`: the one rect the content is laid out in, clipped to and
/// hit-tested in.
pub(crate) fn viewport_in(padding: Rect, gutter: Vec2) -> Rect {
    Rect::from_min_size(padding.min, (padding.size() - gutter).max(Vec2::ZERO))
}

/// The regions the bars reserve in `padding`: beside the viewport on the
/// right and below it, and the corner where they meet.
pub(crate) fn gutters(padding: Rect, gutter: Vec2) -> Vec<Rect> {
    let view = viewport_in(padding, gutter);
    let mut found = Vec::new();
    if gutter.x > 0.0 {
        found.push(Rect::from_min_max(
            Pos2::new(view.max.x, padding.min.y),
            Pos2::new(padding.max.x, view.max.y),
        ));
    }
    if gutter.y > 0.0 {
        found.push(Rect::from_min_max(
            Pos2::new(padding.min.x, view.max.y),
            Pos2::new(view.max.x, padding.max.y),
        ));
    }
    if gutter.x > 0.0 && gutter.y > 0.0 {
        found.push(Rect::from_min_max(view.max, padding.max));
    }
    found
}

/// The bars of a container: their style, and the gutter they take.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bars {
    pub(crate) style: ScrollStyle,
    /// What the bars take from the viewport: the width of the vertical bar
    /// and the height of the horizontal one.
    pub(crate) gutter: Vec2,
}

impl Bars {
    fn new(style: ScrollStyle, axis: Axis) -> Self {
        let (width, enabled) = (style.gutter(), axis.enabled());
        let gutter = Vec2::new(
            if enabled.y { width } else { 0.0 },
            if enabled.x { width } else { 0.0 },
        );
        Self { style, gutter }
    }

    /// egui's scroll style for these bars: solid, and `style`'s size.
    pub(crate) fn egui(&self, base: &egui::style::ScrollStyle) -> egui::style::ScrollStyle {
        egui::style::ScrollStyle {
            floating: false,
            bar_width: self.style.gutter(),
            bar_inner_margin: 0.0,
            bar_outer_margin: 0.0,
            handle_min_length: self.style.min_thumb_length,
            foreground_color: false,
            fade: egui::style::ScrollFadeStyle {
                strength: self.style.fade.strength.into(),
                size: self.style.fade.size,
            },
            ..*base
        }
    }

    /// Paints the bar with `style`'s colours: egui paints it from the
    /// widget visuals of the `Ui` it is shown in.
    pub(crate) fn paint_in(&self, visuals: &mut egui::Visuals) {
        let style = &self.style;
        let widgets = &mut visuals.widgets;
        widgets.inactive.bg_fill = style.thumb;
        widgets.hovered.bg_fill = style.thumb_hover;
        widgets.active.bg_fill = style.thumb_active;
        for widget in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            widget.corner_radius = style.radius;
        }
        visuals.extreme_bg_color = style.track;
    }
}

/// What a container knows about itself once it is laid out.
#[derive(Clone, Debug)]
pub(crate) struct ScrollPlan {
    pub(crate) axis: Axis,
    pub(crate) edge: Option<EdgeScroll>,
    pub(crate) bars: Bars,
    pub(crate) tab: TabRule,
    pub(crate) description: Description,
    /// The content root's size.
    pub(crate) content: Vec2,
    /// How far the content is scrolled.
    pub(crate) offset: Vec2,
    /// Whether layout moved the offset away from egui's, which then has to
    /// be told.
    pub(crate) moved: Moved,
}

impl ScrollPlan {
    /// Whether the content is larger than a viewport of size `viewport` on an
    /// axis that scrolls.
    pub(crate) fn overflows(&self, viewport: Vec2) -> Vec2b {
        let enabled = self.axis.enabled();
        Vec2b::new(
            enabled.x && self.content.x > viewport.x,
            enabled.y && self.content.y > viewport.y,
        )
    }

    /// Whether the container is a Tab stop in this layout.
    pub(crate) fn is_tab_stop(&self, viewport: Vec2) -> bool {
        match self.tab {
            TabRule::Never => false,
            TabRule::Always => true,
            TabRule::WhenScrollable => self.overflows(viewport).any(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Moved {
    No,
    Yes,
}

/// The farthest the content can be scrolled along each axis.
fn limit(content: Vec2, viewport: Vec2) -> Vec2 {
    (content - viewport).max(Vec2::ZERO)
}

/// The offset egui keeps for the container, within the content's limit
/// unless egui is deliberately past it (rubber-banding).
pub(crate) fn stored_offset(
    ctx: &Context,
    node: Id,
    axis: Axis,
    content: Vec2,
    viewport: Vec2,
) -> Vec2 {
    let Some(state) = egui::scroll_area::State::load(ctx, state_id(node)) else {
        return Vec2::ZERO;
    };
    let offset = state.unclamped_offset();
    let offset = if state.overscroll() == Vec2::ZERO {
        offset.clamp(Vec2::ZERO, limit(content, viewport))
    } else {
        offset
    };
    masked(offset, axis)
}

/// `vector` with the components along axes that do not scroll zeroed.
fn masked(vector: Vec2, axis: Axis) -> Vec2 {
    let enabled = axis.enabled();
    Vec2::new(
        if enabled.x { vector.x } else { 0.0 },
        if enabled.y { vector.y } else { 0.0 },
    )
}

/// How far to move the offset along one axis to show `item` in `view`, both
/// as `(start, end)`. With an `align` the item is aligned that way; without,
/// it moves the least, and an item larger than the view shows its start.
fn reveal(view: (f32, f32), item: (f32, f32), align: Option<Align>) -> f32 {
    match align {
        Some(align) => {
            let at = align.to_factor();
            let lerp = |(start, end): (f32, f32)| start + (end - start) * at;
            lerp(item) - lerp(view)
        }
        None if item.0 < view.0 => item.0 - view.0,
        None if item.1 > view.1 => (item.1 - view.1).min(item.0 - view.0),
        None => 0.0,
    }
}

/// Whether a drag has already scrolled a container this layout.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Taken {
    Free,
    Scrolled,
}

/// The scrolling one layout does: the requests made since the last, and the
/// drag's pull at the edges.
pub(crate) struct Settle {
    ctx: Context,
    pending: Vec<Request>,
    pointer: Option<Pos2>,
    dt: f32,
    edge: Taken,
}

impl Settle {
    pub(crate) fn new(ctx: &Context) -> Self {
        let pass = ctx.cumulative_pass_nr();
        let pending = ctx
            .data_mut(|data| {
                std::mem::take(&mut data.get_temp_mut_or_default::<Pending>(pending_key()).0)
            })
            .into_iter()
            .filter(|request| pass <= request.pass + REQUEST_LIFE)
            .collect();
        Self {
            ctx: ctx.clone(),
            pending,
            pointer: drag_pointer(ctx),
            dt: ctx.input(|input| input.stable_dt).min(0.1),
            edge: Taken::Free,
        }
    }

    pub(crate) fn offset(&self, node: Id, axis: Axis, content: Vec2, viewport: Vec2) -> Vec2 {
        stored_offset(&self.ctx, node, axis, content, viewport)
    }

    /// Applies what is wanted of the container at `nodes[at]` to its content,
    /// the nodes after it, which are laid out at the plan's offset: moves
    /// them if the offset changes, and publishes the viewport.
    pub(crate) fn settle(&mut self, nodes: &mut [PlacedNode], at: usize) {
        let (head, content) = nodes.split_at_mut(at + 1);
        let node = &mut head[at];
        let id = node.id().expect("a scroll container is identified");
        let viewport = node
            .scroll_viewport()
            .expect("a scroll container has a plan");
        let plan = node.scroll.as_mut().expect("a scroll container has a plan");
        let start = plan.offset;
        let max = limit(plan.content, viewport.size());
        let enabled = plan.axis.enabled();
        let origin = |offset: Vec2| viewport.min.to_vec2() - offset;

        let mut want = start;
        for request in &mut self.pending {
            let item = match &request.wish {
                Wish::By(delta) if request.container == Some(id) => {
                    want += *delta;
                    request.applied = Applied::Yes;
                    continue;
                }
                Wish::Rect { rect, align } if request.container == Some(id) => {
                    Some((rect.translate(origin(want)), *align))
                }
                Wish::Node { node, align } => content
                    .iter()
                    .find(|candidate| candidate.id() == Some(*node))
                    .map(|found| (found.rect().translate(origin(want) - origin(start)), *align)),
                _ => None,
            };
            if let Some((item, align)) = item {
                request.applied = Applied::Yes;
                if enabled.x {
                    want.x += reveal(
                        (viewport.min.x, viewport.max.x),
                        (item.min.x, item.max.x),
                        align,
                    );
                }
                if enabled.y {
                    want.y += reveal(
                        (viewport.min.y, viewport.max.y),
                        (item.min.y, item.max.y),
                        align,
                    );
                }
            }
        }
        if want != start {
            want = masked(want, plan.axis).clamp(Vec2::ZERO, max);
        }
        if let (Some(edge), Some(pointer), Taken::Free) = (plan.edge, self.pointer, self.edge)
            && viewport.expand(f32::from(edge.band)).contains(pointer)
        {
            let step = masked(edge.velocity(viewport, pointer) * self.dt, plan.axis);
            let next = (want + step).clamp(Vec2::ZERO, max);
            if next != want {
                want = next;
                self.edge = Taken::Scrolled;
                self.ctx.request_repaint();
            }
        }

        if want != start {
            let by = start - want;
            for node in content.iter_mut() {
                node.translate(by);
            }
            plan.offset = want;
            plan.moved = Moved::Yes;
        }
        let published = Viewport {
            rect: viewport,
            content: content[0].rect(),
        };
        self.ctx
            .data_mut(|data| data.insert_temp(viewport_key(id), published));
    }

    /// Keeps the requests no container has applied yet, for the next layout.
    pub(crate) fn finish(self) {
        let kept: Vec<Request> = self
            .pending
            .into_iter()
            .filter(|request| request.applied == Applied::No)
            .collect();
        self.ctx.data_mut(|data| {
            let pending = data.get_temp_mut_or_default::<Pending>(pending_key());
            pending.0.splice(0..0, kept);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_moves_the_least_that_shows_the_item() {
        let view = (100.0, 200.0);
        assert_eq!(reveal(view, (120.0, 180.0), None), 0.0, "already visible");
        assert_eq!(
            reveal(view, (50.0, 90.0), None),
            -50.0,
            "above: its top to the top"
        );
        assert_eq!(
            reveal(view, (210.0, 260.0), None),
            60.0,
            "below: its bottom to the bottom"
        );
    }

    #[test]
    fn reveal_shows_the_start_of_an_item_larger_than_the_view() {
        let view = (100.0, 200.0);
        assert_eq!(reveal(view, (150.0, 400.0), None), 50.0);
        assert_eq!(reveal(view, (20.0, 400.0), None), -80.0);
    }

    #[test]
    fn reveal_aligns_the_item_when_asked() {
        let view = (100.0, 200.0);
        let item = (300.0, 340.0);
        assert_eq!(reveal(view, item, Some(Align::Min)), 200.0);
        assert_eq!(reveal(view, item, Some(Align::Center)), 170.0);
        assert_eq!(reveal(view, item, Some(Align::Max)), 140.0);
    }

    #[test]
    fn the_limit_is_zero_for_content_that_fits() {
        assert_eq!(
            limit(Vec2::new(50.0, 80.0), Vec2::new(100.0, 100.0)),
            Vec2::ZERO
        );
        assert_eq!(
            limit(Vec2::new(150.0, 300.0), Vec2::new(100.0, 100.0)),
            Vec2::new(50.0, 200.0)
        );
    }

    #[test]
    fn axes_that_do_not_scroll_keep_no_offset() {
        let offset = Vec2::new(5.0, 7.0);
        assert_eq!(masked(offset, Axis::Y), Vec2::new(0.0, 7.0));
        assert_eq!(masked(offset, Axis::X), Vec2::new(5.0, 0.0));
        assert_eq!(masked(offset, Axis::Both), offset);
    }
}
