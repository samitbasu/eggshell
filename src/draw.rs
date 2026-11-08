//! The draw phase: [`Placed::draw`](crate::Placed::draw) walks the laid-out
//! tree in pre-order and paints box, primitive and draw closure per node.

use egui::epaint::{RectShape, StrokeKind};
use std::borrow::Cow;

use egui::{
    Align, Color32, Frame, Id, Image, Layout, Painter, Pos2, Rect, Response, Sense, Stroke, Ui,
    UiBuilder, UiKind, UiStackInfo, Vec2, pos2, vec2,
};
use taffy::Overflow;

use crate::focus;
use crate::measure::TextLeaf;
use crate::pipeline::{ColorSource, DrawFn, Flow, Placed, PlacedNode};
use crate::prim::{Icon, Prim};
use crate::text::{TextAlign, TextStyle};
use crate::tw::{BoxPaint, Flip, Grouping, Look, Opacity, Presence, Progress, Sides, State};

/// Rounds coordinates to whole physical pixels.
#[derive(Clone, Copy)]
struct PixelGrid {
    pixels_per_point: f32,
}

impl PixelGrid {
    fn coord(self, value: f32) -> f32 {
        (value * self.pixels_per_point).round() / self.pixels_per_point
    }

    fn rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            pos2(self.coord(rect.min.x), self.coord(rect.min.y)),
            pos2(self.coord(rect.max.x), self.coord(rect.max.y)),
        )
    }

    /// `width` as a whole number of pixels, at least one.
    fn line_width(self, width: f32) -> f32 {
        (width * self.pixels_per_point).round().max(1.0) / self.pixels_per_point
    }
}

/// Whether the subtree can be interacted with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inertness {
    Live,
    Inert,
}

impl From<bool> for Inertness {
    fn from(inert: bool) -> Self {
        if inert { Self::Inert } else { Self::Live }
    }
}

impl Inertness {
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Live, Self::Live) => Self::Live,
            _ => Self::Inert,
        }
    }
}

/// Whether the nearest `group` ancestor is hovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hovering {
    Over,
    Away,
}

impl Hovering {
    fn of(over: bool) -> Self {
        if over { Self::Over } else { Self::Away }
    }
}

/// What a node passes down to its subtree.
#[derive(Clone, Copy)]
struct Inherited {
    clip: Rect,
    opacity: Opacity,
    inertness: Inertness,
    group: Option<Hovering>,
    /// A text colour a variant of an ancestor set, for descendants that
    /// inherit their colour.
    text_color: Option<Color32>,
    /// The backgrounds of the node and its ancestors, composited. egui widgets
    /// that fade toward `ui.stack().bg_color()` read it from the `Ui`'s stack
    /// info, as bullseye paints backgrounds without egui `Frame`s.
    backdrop: Color32,
}

/// A node's boxes, snapped to physical pixels.
struct Boxes {
    border: Rect,
    padding: Rect,
    content: Rect,
    /// The unsnapped content width. Text wraps at this width, the one it was
    /// measured at: egui breaks a word wrapped a fraction narrower than itself.
    wrap_width: f32,
}

fn inset(rect: Rect, sides: Sides<f32>) -> Rect {
    let min = rect.min + vec2(sides.left, sides.top);
    let max = rect.max - vec2(sides.right, sides.bottom);
    Rect::from_min_max(min, max.max(min))
}

fn add(a: Sides<f32>, b: Sides<f32>) -> Sides<f32> {
    Sides {
        top: a.top + b.top,
        right: a.right + b.right,
        bottom: a.bottom + b.bottom,
        left: a.left + b.left,
    }
}

impl Boxes {
    fn of(node: &PlacedNode, grid: PixelGrid) -> Self {
        let rect = node.rect();
        let content = inset(rect, add(node.border, node.padding));
        Self {
            border: grid.rect(rect),
            padding: grid.rect(inset(rect, node.border)),
            content: grid.rect(content),
            wrap_width: content.width(),
        }
    }
}

/// The clip a node's content gets: the padding box on each axis that
/// `overflow` clips.
fn content_clip(clip: Rect, padding: Rect, overflow: taffy::Point<Overflow>) -> Rect {
    let clips = |overflow| overflow != Overflow::Visible;
    let mut clip = clip;
    if clips(overflow.x) {
        clip.min.x = clip.min.x.max(padding.min.x);
        clip.max.x = clip.max.x.min(padding.max.x);
    }
    if clips(overflow.y) {
        clip.min.y = clip.min.y.max(padding.min.y);
        clip.max.y = clip.max.y.min(padding.max.y);
    }
    clip
}

/// The one rounded stroke a border of equal declared widths and colours is
/// drawn as. Decided from the declared widths: snapping each edge on its own
/// can make the snapped ones differ, which must not turn a rounded border
/// into square strips.
fn uniform_border(
    declared: Sides<f32>,
    colors: Sides<Option<Color32>>,
    grid: PixelGrid,
) -> Option<Stroke> {
    match (colors.top, colors.right, colors.bottom, colors.left) {
        (Some(t), Some(r), Some(b), Some(l))
            if t == r
                && r == b
                && b == l
                && declared.top > 0.0
                && declared.top == declared.right
                && declared.right == declared.bottom
                && declared.bottom == declared.left =>
        {
            Some(Stroke::new(grid.line_width(declared.top), t))
        }
        _ => None,
    }
}

fn paint_box(
    painter: &Painter,
    paint: &BoxPaint,
    boxes: &Boxes,
    declared_border: Sides<f32>,
    grid: PixelGrid,
) {
    let Boxes { border, .. } = *boxes;
    if let Some(shadow) = paint.shadow {
        paint_shadow(painter, shadow, border, paint);
    }
    let uniform = uniform_border(declared_border, paint.border_colors, grid);
    if paint.background.is_some() || uniform.is_some() {
        painter.add(RectShape::new(
            border,
            paint.radius,
            paint.background.unwrap_or(Color32::TRANSPARENT),
            uniform.unwrap_or(Stroke::NONE),
            StrokeKind::Inside,
        ));
    }
    if uniform.is_none() {
        paint_side_borders(painter, boxes, paint.border_colors);
    }
    if let Some(ring) = paint.ring {
        painter.add(RectShape::stroke(
            border,
            paint.radius,
            Stroke::new(ring.width, ring.color),
            StrokeKind::Outside,
        ));
    }
}

/// CSS clips a box-shadow to outside the border box, so a box that does not
/// cover it (no background, or a translucent one) must not show it inside.
/// The shadow is painted in the four strips around the border box; the
/// rounded corners' cut-outs are inside the box and stay clean.
fn paint_shadow(painter: &Painter, shadow: egui::Shadow, border: Rect, paint: &BoxPaint) {
    let shape = shadow.as_shape(border, paint.radius);
    if paint.background.is_some_and(|fill| fill.is_opaque()) {
        painter.add(shape);
        return;
    }
    let clip = painter.clip_rect();
    let strips = [
        Rect::from_min_max(clip.min, pos2(clip.max.x, border.min.y)),
        Rect::from_min_max(pos2(clip.min.x, border.max.y), clip.max),
        Rect::from_min_max(
            pos2(clip.min.x, border.min.y),
            pos2(border.min.x, border.max.y),
        ),
        Rect::from_min_max(
            pos2(border.max.x, border.min.y),
            pos2(clip.max.x, border.max.y),
        ),
    ];
    for strip in strips.into_iter().filter(|strip| strip.is_positive()) {
        painter.with_clip_rect(strip).add(shape.clone());
    }
}

fn paint_side_borders(painter: &Painter, boxes: &Boxes, colors: Sides<Option<Color32>>) {
    let Boxes {
        border: b,
        padding: p,
        ..
    } = *boxes;
    let strips = [
        (
            colors.top,
            Rect::from_min_max(b.min, pos2(b.max.x, p.min.y)),
        ),
        (
            colors.bottom,
            Rect::from_min_max(pos2(b.min.x, p.max.y), b.max),
        ),
        (
            colors.left,
            Rect::from_min_max(pos2(b.min.x, p.min.y), pos2(p.min.x, p.max.y)),
        ),
        (
            colors.right,
            Rect::from_min_max(pos2(p.max.x, p.min.y), pos2(b.max.x, p.max.y)),
        ),
    ];
    for (color, strip) in strips {
        if let Some(color) = color.filter(|_| strip.is_positive()) {
            painter.rect_filled(strip, 0.0, color);
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn span(self, rect: Rect) -> (f32, f32) {
        match self {
            Self::X => (rect.min.x, rect.max.x),
            Self::Y => (rect.min.y, rect.max.y),
        }
    }

    fn other(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    fn rect(self, along: (f32, f32), across: (f32, f32)) -> Rect {
        match self {
            Self::X => Rect::from_min_max(pos2(along.0, across.0), pos2(along.1, across.1)),
            Self::Y => Rect::from_min_max(pos2(across.0, along.0), pos2(across.1, along.1)),
        }
    }
}

/// The hairline between two neighbours along `axis`, if they sit one after
/// the other there and share some extent across it.
fn divider(prev: Rect, next: Rect, axis: Axis, width: f32, grid: PixelGrid) -> Option<Rect> {
    const TOUCH: f32 = 0.5;
    let (prev_span, next_span) = (axis.span(prev), axis.span(next));
    let gap_centre = if next_span.0 >= prev_span.1 - TOUCH {
        (prev_span.1 + next_span.0) / 2.0
    } else if next_span.1 <= prev_span.0 + TOUCH {
        (next_span.1 + prev_span.0) / 2.0
    } else {
        return None;
    };
    let (a, b) = (axis.other().span(prev), axis.other().span(next));
    let across = (a.0.min(b.0), a.1.max(b.1));
    if a.0.max(b.0) >= a.1.min(b.1) {
        return None;
    }
    let width = grid.line_width(width);
    let start = grid.coord(gap_centre - width / 2.0);
    Some(axis.rect((start, start + width), across))
}

fn text_overrides(ui: &mut Ui, text: &TextStyle) {
    ui.visuals_mut().override_text_color = Some(text.color);
    ui.style_mut().override_font_id = Some(text.spec.font_id());
}

/// Runs `add_contents` in a child `Ui` that is not allocated in `parent`.
fn node_ui(
    parent: &mut Ui,
    builder: UiBuilder,
    inherited: Inherited,
    text: Option<&TextStyle>,
    add_contents: impl FnOnce(&mut Ui),
) {
    let stack_info =
        UiStackInfo::new(UiKind::GenericArea).with_frame(Frame::NONE.fill(inherited.backdrop));
    let mut child = parent.new_child(
        builder
            .layout(Layout::top_down(Align::Min))
            .ui_stack_info(stack_info),
    );
    child.set_clip_rect(inherited.clip);
    if inherited.inertness == Inertness::Inert {
        child.disable();
    }
    let parent_opacity = parent.opacity();
    child.set_opacity(parent_opacity * f32::from(inherited.opacity));
    if let Some(text) = text {
        text_overrides(&mut child, text);
    }
    add_contents(&mut child);
}

fn image_for(icon: Icon, tint: Color32) -> Image<'static> {
    let Icon { src, rotate, flip } = icon;
    let (u, v) = match flip {
        Flip::None => ((0.0, 1.0), (0.0, 1.0)),
        Flip::Horizontal => ((1.0, 0.0), (0.0, 1.0)),
        Flip::Vertical => ((0.0, 1.0), (1.0, 0.0)),
        Flip::Both => ((1.0, 0.0), (1.0, 0.0)),
    };
    let uv = Rect::from_min_max(pos2(u.0, v.0), pos2(u.1, v.1));
    Image::new(src)
        .tint(tint)
        .uv(uv)
        .rotate(rotate.into(), Vec2::splat(0.5))
}

struct Walk<'a, A> {
    app: &'a mut A,
    ui: &'a mut Ui,
    nodes: &'a [PlacedNode],
    prims: Vec<Option<Prim>>,
    draws: Vec<Option<DrawFn<A>>>,
    grid: PixelGrid,
    subtrees: Vec<(Id, Box<[Id]>)>,
}

impl<A> Walk<'_, A> {
    fn painter(&self, clip: Rect, opacity: Opacity) -> Painter {
        let mut painter = self.ui.painter().with_clip_rect(clip);
        painter.multiply_opacity(opacity.into());
        painter
    }

    fn children(&self, index: usize) -> Vec<usize> {
        let end = self.nodes[index].end;
        let mut children = Vec::new();
        let mut next = index + 1;
        while next < end {
            children.push(next);
            next = self.nodes[next].end;
        }
        children
    }

    /// The node's one hit test, made before anything is painted so that
    /// hover styling and clicks agree. Registration order is egui's hit
    /// order: later wins, and the walk is pre-order with siblings by `z`.
    fn interact(&mut self, node: &PlacedNode, boxes: &Boxes, inherited: Inherited) -> Response {
        let id = node.id().expect("a sensing node is identified");
        let sense = node
            .interaction
            .focusability
            .sense(node.interaction.sensing);
        let mut response = None;
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(boxes.border),
            inherited,
            None,
            |ui| response = Some(ui.interact(boxes.border, id, sense)),
        );
        response.expect("node_ui runs its contents")
    }

    /// Hovering the node or any sensing descendant hovers the node, as
    /// `:hover` does in CSS.
    fn hovering(&self, index: usize, own: &Response) -> Hovering {
        let ctx = self.ui.ctx();
        let node = &self.nodes[index];
        let descendants = &self.nodes[index + 1..node.end];
        Hovering::of(
            own.hovered()
                || descendants
                    .iter()
                    .filter(|descendant| descendant.senses())
                    .filter_map(PlacedNode::id)
                    .any(|id| ctx.read_response(id).is_some_and(|r| r.hovered())),
        )
    }

    /// How far each state variant has taken hold, animated when the node has
    /// a `transition`.
    fn presence(
        &self,
        node: &PlacedNode,
        response: Option<&Response>,
        hovering: Option<Hovering>,
        group: Option<Hovering>,
    ) -> Presence {
        let variants = &node.interaction.variants;
        let progress = |state: State| {
            if variants.get(state).is_none() {
                return Progress::START;
            }
            let on = match state {
                State::GroupHover => {
                    debug_assert!(
                        group.is_some(),
                        "{}: group_hover needs an ancestor marked group()",
                        node.path()
                    );
                    group == Some(Hovering::Over)
                }
                State::Hover => hovering == Some(Hovering::Over),
                State::Focus => response.is_some_and(Response::has_focus),
                State::FocusWithin => node
                    .id()
                    .is_some_and(|id| focus::focus_within(self.ui.ctx(), id)),
                State::Active => {
                    response.is_some_and(|r| r.hovered() && r.is_pointer_button_down_on())
                }
            };
            let target = if on { Progress::END } else { Progress::START };
            match (variants.transition, node.id()) {
                (Some(duration), Some(id)) => Progress::new(self.ui.ctx().animate_value_with_time(
                    id.with(("bullseye-state", state as u8)),
                    target.into(),
                    duration.as_secs_f32(),
                )),
                _ => target,
            }
        };
        Presence {
            group_hover: progress(State::GroupHover),
            hover: progress(State::Hover),
            focus_within: progress(State::FocusWithin),
            focus: progress(State::Focus),
            active: progress(State::Active),
        }
    }

    /// Visits the node and its subtree, noting which focusable widgets the
    /// subtree registered if the node is identified.
    fn visit(&mut self, index: usize, parent: Inherited) {
        let layer = self.ui.layer_id();
        let before = self.nodes[index]
            .id()
            .map(|id| (id, focus::registered(self.ui.ctx(), layer)));
        self.visit_node(index, parent);
        if let Some((id, from)) = before {
            let widgets = focus::focusable_since(self.ui.ctx(), layer, from);
            self.subtrees.push((id, widgets));
        }
    }

    fn visit_node(&mut self, index: usize, parent: Inherited) {
        let nodes = self.nodes;
        let node = &nodes[index];
        if node.flow == Flow::Hidden {
            return;
        }
        let boxes = Boxes::of(node, self.grid);
        let inertness = parent.inertness.or(node.is_inert().into());
        let interaction = &node.interaction;

        let response = node.senses().then(|| {
            self.interact(
                node,
                &boxes,
                Inherited {
                    inertness,
                    ..parent
                },
            )
        });
        let needs_hovering = interaction.grouping == Grouping::Group
            || interaction.variants.get(State::Hover).is_some();
        let hovering = response
            .as_ref()
            .filter(|_| needs_hovering)
            .map(|own| self.hovering(index, own));
        let presence = self.presence(node, response.as_ref(), hovering, parent.group);
        let group = match interaction.grouping {
            Grouping::Group => hovering,
            Grouping::Ungrouped => parent.group,
        };

        let base_color = match (interaction.color_source, parent.text_color) {
            (ColorSource::Inherited, Some(inherited)) => inherited,
            _ => node.text().color,
        };
        let look = interaction.variants.resolve(
            Look {
                paint: *node.paint(),
                text: base_color,
                decoration: node.text().decoration,
            },
            presence,
        );
        let text = if look.text == node.text().color && look.decoration == node.text().decoration {
            Cow::Borrowed(node.text())
        } else {
            Cow::Owned(TextStyle {
                color: look.text,
                decoration: look.decoration,
                ..node.text().clone()
            })
        };

        let opacity = parent.opacity * look.paint.opacity;
        paint_box(
            &self.painter(parent.clip, opacity),
            &look.paint,
            &boxes,
            node.border,
            self.grid,
        );

        let painted_background = look.paint.background.map_or(Color32::TRANSPARENT, |fill| {
            fill.gamma_multiply(f32::from(opacity))
        });
        let inherited = Inherited {
            backdrop: parent.backdrop.blend(painted_background),
            clip: content_clip(parent.clip, boxes.padding, node.overflow),
            opacity,
            inertness,
            group,
            text_color: if interaction.variants.sets_text_color() {
                Some(look.text)
            } else if interaction.color_source == ColorSource::Inherited {
                parent.text_color
            } else {
                None
            },
        };
        if let Some(prim) = self.prims[index].take() {
            self.paint_prim(prim, &text, &boxes, inherited);
        }
        if let Some(draw) = node.draw.and_then(|slot| self.draws[slot].take()) {
            let id = node.id().expect("a node with a draw closure is identified");
            let builder = UiBuilder::new()
                .max_rect(boxes.content)
                .scope_id(id.with(DRAW_SCOPE));
            let app = &mut *self.app;
            node_ui(self.ui, builder, inherited, Some(&text), |ui| {
                ui.data_mut(|data| data.insert_temp(ui.unique_id(), NodeIdTag(id)));
                draw(app, ui)
            });
        }

        let in_layout_order = self.children(index);
        let mut in_paint_order = in_layout_order.clone();
        in_paint_order.sort_by_key(|&child| self.nodes[child].z());
        for child in in_paint_order {
            self.visit(child, inherited);
        }
        self.paint_dividers(index, &in_layout_order, inherited);
    }

    fn paint_prim(&mut self, prim: Prim, text: &TextStyle, boxes: &Boxes, inherited: Inherited) {
        let content = boxes.content;
        match prim {
            Prim::Text(string) => {
                let leaf = TextLeaf {
                    text: &string,
                    style: text,
                };
                let galley = self
                    .ui
                    .ctx()
                    .fonts_mut(|fonts| leaf.drawn_galley(fonts, boxes.wrap_width));
                let anchor: Pos2 = match text.align {
                    TextAlign::Left => content.left_top(),
                    TextAlign::Center => content.center_top(),
                    TextAlign::Right => content.right_top(),
                };
                self.painter(inherited.clip, inherited.opacity)
                    .galley(anchor, galley, text.color);
            }
            Prim::Icon(icon) => self.paint_image(image_for(icon, text.color), content, inherited),
            Prim::Image(src) => self.paint_image(Image::new(src), content, inherited),
            Prim::Paint(paint) => paint(&self.painter(inherited.clip, inherited.opacity), content),
        }
    }

    fn paint_image(&mut self, image: Image<'static>, content: Rect, inherited: Inherited) {
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(content),
            inherited,
            None,
            |ui| image.paint_at(ui, content),
        );
    }

    fn paint_dividers(&self, index: usize, children: &[usize], inherited: Inherited) {
        let paint = self.nodes[index].paint();
        let painter = self.painter(inherited.clip, inherited.opacity);
        let in_flow: Vec<Rect> = children
            .iter()
            .map(|&child| &self.nodes[child])
            .filter(|node| node.flow == Flow::Normal)
            .map(|node| self.grid.rect(node.rect()))
            .collect();
        for (divide, axis) in [(paint.divide_x, Axis::X), (paint.divide_y, Axis::Y)] {
            let Some(divide) = divide else { continue };
            for pair in in_flow.windows(2) {
                if let Some(line) = divider(pair[0], pair[1], axis, divide.width, self.grid) {
                    painter.rect_filled(line, 0.0, divide.color);
                }
            }
        }
    }
}

impl<A> Placed<A> {
    /// Paints the frame into `ui`.
    ///
    /// Nodes are visited in pre-order. A node that declares a sense (see
    /// [`sense_click`](crate::Tw::sense_click)) is first given its one
    /// `ui.interact` over its border box, so egui's hit order is the paint
    /// order: pre-order, siblings by [`z`](crate::PlacedNode::z), later on
    /// top. An [`inert`](crate::Tw::inert) subtree is interacted with as
    /// disabled. The node's `hover`, `active` and `group_hover` variants are
    /// then resolved, animated if the node has a `transition`, and used for
    /// everything painted for the node. For each node the library paints, in
    /// order: shadow, background, borders and ring (clipped by the ancestors'
    /// `overflow`); then its text, icon, image or custom paint; then the draw
    /// closure of an identified node; then its children stable-sorted by
    /// [`z`](crate::PlacedNode::z); and last the `divide_x` / `divide_y`
    /// hairlines between its in-flow children.
    ///
    /// Each draw closure runs with `app` in its own child [`Ui`] whose
    /// [`scope_id`](Ui::scope_id) is derived from the node's [`Id`](egui::Id)
    /// ([`NodeUi::node_id`]), whatever `ui` is, and whose rect is the node's content box. The `Ui` carries the node's
    /// text colour and font as egui overrides, the clip of the ancestors'
    /// `overflow`, the multiplied opacity of the node and its ancestors, and is
    /// disabled below an [`inert`](crate::Tw::inert) node.
    ///
    /// Rects are rounded to physical pixels here; layout itself is exact.
    pub fn draw(self, app: &mut A, ui: &mut Ui) {
        let Self {
            nodes,
            prims,
            draws,
            ..
        } = self;
        let grid = PixelGrid {
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let root = Inherited {
            clip: ui.clip_rect(),
            opacity: Opacity::OPAQUE,
            inertness: Inertness::Live,
            group: None,
            text_color: None,
            backdrop: Color32::TRANSPARENT,
        };
        let mut walk = Walk {
            app,
            ui,
            nodes: &nodes,
            prims,
            draws,
            grid,
            subtrees: Vec::new(),
        };
        walk.visit(0, root);
        focus::store(walk.ui.ctx(), walk.subtrees);
    }
}

/// Salt of a draw closure's [`Ui`] ids. The `Ui` registers a widget under its
/// own id, which must not be the node's, whose widget is the border box.
const DRAW_SCOPE: &str = "bullseye-draw";

#[derive(Clone, Copy)]
struct NodeIdTag(egui::Id);

/// Reads the response the library gave the node whose draw closure is
/// running.
pub trait NodeUi {
    /// The node's response: clicks, drags, hover, pointer position, tooltips.
    ///
    /// The library made the node's one hit test before painting it, with its
    /// border box, so hover styling and this response agree. A node that
    /// declares no sense gets a response that senses nothing. Call it on the
    /// [`Ui`] the draw closure is given, not on a nested one, and never
    /// interact with the node's own id yourself.
    fn node_response(&self) -> Response;

    /// The node's egui id: its key path, the one `Builder::id_of` reports.
    /// The [`scope_id`](Ui::scope_id) of the draw closure's `Ui` is derived
    /// from it, so ids made with `make_persistent_id` are stable per node.
    fn node_id(&self) -> egui::Id;

    /// Whether the node, or a widget inside it, has keyboard focus. See
    /// [`focus_within`](crate::focus_within).
    fn has_focus_within(&self) -> bool;
}

impl NodeUi for Ui {
    fn node_id(&self) -> egui::Id {
        self.data(|data| data.get_temp::<NodeIdTag>(self.unique_id()))
            .map(|tag| tag.0)
            .expect("node_response and node_id are called on the Ui a draw closure is given")
    }

    fn has_focus_within(&self) -> bool {
        focus::focus_within(self.ctx(), self.node_id())
    }

    fn node_response(&self) -> Response {
        let id = self.node_id();
        self.ctx()
            .read_response(id)
            .unwrap_or_else(|| self.interact(self.max_rect(), id, Sense::hover()))
    }
}
