//! The draw phase: [`Placed::draw`](crate::Placed::draw) walks the laid-out
//! tree in pre-order and paints box, primitive and draw closure per node.

use egui::emath::TSTransform;
use egui::epaint::{Mesh, RectShape, Shape, StrokeKind, Tessellator};
use egui::layers::ShapeIdx;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use egui::response::Flags;
use egui::text_selection::LabelSelectionState;
use egui::{
    Align, Color32, Frame, Galley, Id, Image, Layout, Painter, Pos2, Rect, Response, Role, Sense,
    Stroke, Ui, UiBuilder, UiKind, UiStackInfo, Vec2, WidgetInfo, pos2, vec2,
};
use taffy::Overflow;

use crate::commands::{Declared, Visit};
use crate::continuity::{self, Motion, NodeFrame, OverlayFrame, RecordedPass};
use crate::contract::{Selection, Style};
use crate::debug::{self, Debug, Phase, Timer};
use crate::dnd::{self, Decls, Stakes};
use crate::focus;
use crate::kept::Handle;
use crate::measure::TextLeaf;
use crate::nav;
use crate::nodes::Nodes;
use crate::overlay::Anchored;
use crate::overlay_input::Blur;
use crate::overlay_input::{self, press_origin};
use crate::owned::Side;
use crate::pipeline::{
    ColorSource, DismissFn, DrawFn, Flow, NodeKind, Placed, PlacedFrame, PlacedNode, ScrollRef,
};
use crate::plane::Axis;
use crate::presence::Staged;
use crate::prim::{Icon, Prim};
use crate::roots::{
    Form, Here, Home, PlacedRoot, covers_screen, root_and_node, root_at, root_of, stacked_top_down,
};
use crate::scroll;
use crate::scroll_keys;
use crate::shadow_corners::{Corner, arc_segments, clip_mesh_to_triangle, outside_corner_fan};
use crate::spans;
use crate::stack::{self, Drawn};
use crate::text::{Interactive, TextAlign, TextStyle};
use crate::tooltip;
use crate::transition::{self, LayoutAnimation};
use crate::tw::{
    BoxPaint, Grouping, Look, Mirror, Opacity, PointerEvents, Presence, Progress, RingPlacement,
    Sides, State, Visibility,
};
use crate::window;

/// Rounds coordinates to whole physical pixels.
#[derive(Clone, Copy)]
pub(crate) struct PixelGrid {
    pub(crate) pixels_per_point: f32,
}

impl PixelGrid {
    pub(crate) fn coord(self, value: f32) -> f32 {
        (value * self.pixels_per_point).round() / self.pixels_per_point
    }

    pub(crate) fn rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            pos2(self.coord(rect.min.x), self.coord(rect.min.y)),
            pos2(self.coord(rect.max.x), self.coord(rect.max.y)),
        )
    }

    /// `width` as a whole number of pixels, at least one.
    pub(crate) fn line_width(self, width: f32) -> f32 {
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

/// Whether a box takes drags from the widgets beneath it. Beneath a node that
/// takes drags itself, or a scroll container whose content drags to scroll
/// (egui's `ScrollSource`: on a touch screen), they are the ancestor's, and
/// the box lets them through to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drags {
    Block,
    PassThrough,
}

impl Drags {
    fn sense(self) -> Sense {
        match self {
            Self::Block => Sense::CLICK | Sense::DRAG,
            Self::PassThrough => Sense::CLICK,
        }
    }
}

/// What a node passes down to its subtree.
#[derive(Clone, Copy)]
struct Inherited {
    drags: Drags,
    /// Whether nodes outside the clip are left out of the draw.
    culls: bool,
    clip: Rect,
    opacity: Opacity,
    inertness: Inertness,
    visibility: Visibility,
    group: Option<Hovering>,
    /// A text colour a variant of an ancestor set, for descendants that
    /// inherit their colour.
    text_color: Option<Color32>,
    interactive: Interactive,
    /// The backgrounds of the node and its ancestors, composited. egui widgets
    /// that fade toward `ui.stack().bg_color()` read it from the `Ui`'s stack
    /// info, as eggshell paints backgrounds without egui `Frame`s.
    backdrop: Color32,
    /// Whether the boxes of the subtree block the pointer.
    pointer_events: PointerEvents,
    /// The layout position of the node's parent.
    origin: Pos2,
    /// How far the ancestors' paint offsets have moved this subtree.
    shift: Vec2,
    /// How far the scroll containers around this subtree have moved it.
    scrolled: Vec2,
}

impl Inherited {
    /// What the root of a tree passes down: the clip it is painted within and
    /// the layout position it is at.
    fn root(clip: Rect, origin: Pos2) -> Self {
        Self {
            drags: Drags::Block,
            culls: false,
            clip,
            opacity: Opacity::OPAQUE,
            inertness: Inertness::Live,
            visibility: Visibility::Visible,
            group: None,
            text_color: None,
            interactive: Interactive::Outside,
            backdrop: Color32::TRANSPARENT,
            pointer_events: PointerEvents::Auto,
            origin,
            shift: Vec2::ZERO,
            scrolled: Vec2::ZERO,
        }
    }
}

/// A node's one hit test: its id and what it senses.
#[derive(Clone, Copy)]
struct HitTest {
    id: Id,
    sense: Sense,
}

/// The node painted lifted, at a screen point.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lift {
    /// The node's index in the frame.
    pub(crate) index: usize,
    /// Where the top-left of the node's border box is painted, in screen
    /// space.
    pub(crate) min: Pos2,
}

/// The layer lifted nodes are painted on: above every other, so a lifted
/// node is neither clipped by its ancestors nor covered by anything.
pub(crate) fn drag_layer() -> egui::LayerId {
    egui::LayerId::new(egui::Order::Tooltip, Id::NULL.with("eggshell-drag-layer"))
}

/// A node's boxes, snapped to physical pixels.
struct Boxes {
    border: Rect,
    /// Where the node takes the pointer: see [`PlacedNode::hit`].
    hit: Rect,
    padding: Rect,
    content: Rect,
    /// The unsnapped content width. Text wraps at this width, the one it was
    /// measured at: egui breaks a word wrapped a fraction narrower than itself.
    wrap_width: f32,
}

pub(crate) fn inset(rect: Rect, sides: Sides<f32>) -> Rect {
    let min = rect.min + vec2(sides.left, sides.top);
    let max = rect.max - vec2(sides.right, sides.bottom);
    Rect::from_min_max(min, max.max(min))
}

pub(crate) fn add(a: Sides<f32>, b: Sides<f32>) -> Sides<f32> {
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
            hit: grid.rect(node.hit),
            padding: grid.rect(inset(rect, node.border)),
            content: grid.rect(content),
            wrap_width: content.width(),
        }
    }
}

/// The clip a node's content gets: the padding box on each axis that
/// `overflow` clips.
pub(crate) fn content_clip(clip: Rect, padding: Rect, overflow: taffy::Point<Overflow>) -> Rect {
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
            match paint.ring_placement {
                RingPlacement::Outside => StrokeKind::Outside,
                RingPlacement::Inside => StrokeKind::Inside,
            },
        ));
    }
}

/// CSS paints a box-shadow everywhere outside the rounded border box and
/// nowhere inside it, so a box that does not cover the shadow (no background,
/// or a translucent one) must not show it inside. The shadow is painted in the
/// four rect strips around the border box, plus the cut-out of each rounded
/// corner, which no rect clip can express: the tessellated shadow is clipped
/// to a fan of triangles between the rect's corner and the arc.
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
    let max_radius = border.width().min(border.height()) / 2.0;
    let radii = Corner::ALL.map(|corner| {
        (
            corner,
            f32::from(corner.radius(paint.radius)).min(max_radius),
        )
    });
    if radii.iter().all(|&(_, radius)| radius <= 0.0) {
        return;
    }
    let pixels_per_point = painter.pixels_per_point();
    let mut mesh = Mesh::default();
    let mut tessellator = Tessellator::new(
        pixels_per_point,
        painter.ctx().tessellation_options(|options| *options),
        painter.fonts(|fonts| fonts.font_image_size()),
        Vec::new(),
    );
    tessellator.tessellate_shape(shape.into(), &mut mesh);
    for (corner, radius) in radii.into_iter().filter(|&(_, radius)| radius > 0.0) {
        let segments = arc_segments(radius, pixels_per_point);
        let mut cut_out = Mesh::with_texture(mesh.texture_id);
        for triangle in outside_corner_fan(border, corner, radius, segments) {
            cut_out.append(clip_mesh_to_triangle(&mesh, triangle));
        }
        if !cut_out.is_empty() {
            painter.add(cut_out);
        }
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
    ui.style_mut().override_font_id = Some(text.spec.font_id(ui.ctx()));
}

/// Runs `add_contents` in a child `Ui` that is not allocated in `parent`.
fn node_ui<R>(
    parent: &mut Ui,
    builder: UiBuilder,
    inherited: Inherited,
    text: Option<&TextStyle>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
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
    add_contents(&mut child)
}

fn image_for(icon: Icon, tint: Color32) -> Image<'static> {
    let Icon {
        src,
        rotate,
        mirror,
    } = icon;
    let (u, v) = match mirror {
        Mirror::None => ((0.0, 1.0), (0.0, 1.0)),
        Mirror::Horizontal => ((1.0, 0.0), (0.0, 1.0)),
        Mirror::Vertical => ((0.0, 1.0), (1.0, 0.0)),
        Mirror::Both => ((1.0, 0.0), (1.0, 0.0)),
    };
    let uv = Rect::from_min_max(pos2(u.0, v.0), pos2(u.1, v.1));
    Image::new(src)
        .tint(tint)
        .uv(uv)
        .rotate(rotate.into(), Vec2::splat(0.5))
}

/// The square an icon is painted in within `content`: the largest that fits,
/// centred, as an inline SVG's default `xMidYMid meet` does.
fn meet(content: Rect) -> Rect {
    Rect::from_center_size(
        content.center(),
        Vec2::splat(content.width().min(content.height())),
    )
}

/// `rect` scaled by `scale` about its centre.
fn scaled_about_centre(rect: Rect, scale: f32) -> Rect {
    if scale == 1.0 {
        rect
    } else {
        Rect::from_center_size(rect.center(), rect.size() * scale)
    }
}

/// What the walk of a frame accumulates, whichever `Ui` it is in.
struct Shared<A> {
    prims: Vec<Option<Prim>>,
    draws: Vec<Vec<DrawFn<A>>>,
    recording: Option<Vec<NodeFrame>>,
    /// While recording, the identified nodes left out as culled.
    culled: Vec<Id>,
    /// The id of each node's box widget.
    box_ids: Vec<Id>,
    /// The nodes of overlays that are entering or leaving, by index.
    stages: HashMap<usize, Stage>,
}

/// What one pass of a frame keeps while it draws: what belongs to the input
/// of the viewport it draws in.
struct Pass {
    subtrees: Vec<(Id, Box<[Id]>)>,
    animation: LayoutAnimation,
    /// The boxes the pointer is on or pressing, from the last pass.
    hot: Vec<Hot>,
    /// The items listboxes highlight, which look hovered.
    highlighted: Vec<Id>,
    /// Why a field losing focus in this pass does, told to its hooks.
    blur: Blur,
    /// The items of groups on the page that are not their group's Tab stop.
    off_tab: std::collections::HashSet<Id>,
    /// The relations between the pass's nodes, announced once it has drawn.
    relations: Vec<crate::relation::Relation>,
}

/// The id of the blocker of the node at `index`: the widget an overlay's
/// root is described on to assistive technology.
pub(crate) fn blocker_id(index: usize) -> Id {
    crate::key::root_id().with("blocker").with(index)
}

/// How an entering or leaving overlay is painted at one of its nodes.
#[derive(Clone, Copy)]
enum Stage {
    /// The root of a modal: its own fill is the scrim, which fades with the
    /// overlay's progress, while the box in it has the overlay's entrance.
    Scrim(Opacity),
    /// A box that scales and moves with the overlay's progress.
    Box { staged: Staged, form: Form },
}

struct Walk<'a, A> {
    app: &'a mut A,
    ui: &'a mut Ui,
    nodes: &'a Nodes,
    /// For each node, how many text nodes of the same path come before it.
    ordinals: &'a [u32],
    /// `registrants[i]` counts the nodes before `i` that register widgets
    /// while culled.
    registrants: &'a [u32],
    shared: &'a mut Shared<A>,
    pass: &'a mut Pass,
    grid: PixelGrid,
    lift: Option<Lift>,
    drag: &'a dnd::Draw,
    roots: &'a [PlacedRoot],
    shell: &'a crate::Shell,
}

impl<A> Walk<'_, A> {
    /// The same walk in another `Ui`: another layer.
    fn in_layer<'b>(&'b mut self, ui: &'b mut Ui) -> Walk<'b, A> {
        Walk {
            app: &mut *self.app,
            ui,
            nodes: self.nodes,
            ordinals: self.ordinals,
            registrants: self.registrants,
            shared: &mut *self.shared,
            pass: &mut *self.pass,
            grid: self.grid,
            lift: self.lift,
            drag: self.drag,
            roots: self.roots,
            shell: self.shell,
        }
    }

    fn painter(&self, clip: Rect, opacity: Opacity) -> Painter {
        let mut painter = self.ui.painter().with_clip_rect(clip);
        painter.multiply_opacity(opacity.into());
        painter
    }

    /// The node's one hit test, made before anything is painted so that
    /// hover styling and clicks agree. Registration order is egui's hit
    /// order: later wins, and the walk is pre-order with siblings by `z`.
    ///
    /// A node that senses hover only takes clicks too, as any box does.
    fn interact(
        &mut self,
        index: usize,
        node: &PlacedNode,
        HitTest { id, sense }: HitTest,
        boxes: &Boxes,
        inherited: Inherited,
    ) -> (Response, Merged) {
        let sense = if node.senses() && !sense.interactive() {
            sense | Sense::CLICK
        } else {
            sense
        };
        let had_focus = self.ui.ctx().memory(|memory| memory.has_focus(id));
        let builder = UiBuilder::new().max_rect(boxes.hit);
        let builder = match node
            .description()
            .and_then(|description| description.parent)
        {
            Some(parent) => builder.accessibility_parent(parent),
            None => builder,
        };
        let mut response = node_ui(self.ui, builder, inherited, None, |ui| {
            let own = ui.interact(boxes.hit, id, sense);
            if let Some(plan) = node.scroll().filter(|_| !node.senses()) {
                plan.description
                    .announce(&own, ui.is_enabled().into(), Role::ScrollView);
            } else if let Some(description) = node.description().filter(|d| d.is_set()) {
                description.announce(&own, ui.is_enabled().into(), Role::GenericContainer);
            }
            own
        });
        if let Some(target) = node.description().and_then(|d| d.controls) {
            self.pass
                .relations
                .push(crate::relation::Relation::Controls { node: id, target });
        }
        let merged = self.merge_hot(index, &mut response);
        let lost_focus = had_focus && !self.ui.ctx().memory(|memory| memory.has_focus(id));
        if lost_focus && merged == Merged::Bubbled && response.flags.intersects(Hot::PRESS) {
            // egui took the focus because the click was not on the node's own
            // widget, which a box in its subtree covers.
            response.request_focus();
        }
        (response, merged)
    }

    /// Adds to the response of the sensing node at `index` what the boxes of
    /// its subtree won: they cover the node's own widget, so a press on one
    /// is the node's unless a nearer node would take it.
    fn merge_hot(&self, index: usize, response: &mut Response) -> Merged {
        let subtree = self.nodes.span(index);
        let mut merged = Merged::Own;
        for hot in self
            .pass
            .hot
            .iter()
            .filter(|hot| index < hot.index && subtree.contains(&hot.index))
        {
            response.flags |= hot.flags & Hot::HOVER;
            if nearest_clicker(self.nodes, hot.index) == Some(index) {
                response.flags |= hot.flags & Hot::PRESS;
                if let Some(at) = hot.at.filter(|_| response.interact_pointer_pos().is_none()) {
                    response.interact_pointer_pos_or_nan = at;
                }
            }
            merged = Merged::Bubbled;
        }
        merged
    }

    /// Registers the node's box, if it paints one and does not sense: a
    /// widget over its border box that takes clicks, so that the nodes
    /// registered before it, whatever they are, do not. A press it wins is
    /// the nearest sensing ancestor's (see [`Hot`]). Egui's hit test is the
    /// only one: a box registered after a widget covers it, as in CSS.
    ///
    /// The root of an overlay or a pop-out's window gets a blocker that also
    /// takes drags, since what lies beneath it is another layer.
    fn cover(&mut self, index: usize, boxes: &Boxes, inherited: Inherited) {
        let Some(node) = self.nodes.node(index) else {
            return;
        };
        if inherited.pointer_events != PointerEvents::Auto {
            return;
        }
        if matches!(node.kind(), NodeKind::Overlay | NodeKind::Window) {
            self.block(index, boxes, inherited);
        } else if node.paints_box() && !node.senses() && inherited.inertness == Inertness::Live {
            // Not `node_ui`: a child `Ui` gets an accessibility node.
            let Some(&id) = self.shared.box_ids.get(index) else {
                return;
            };
            let clip = self.ui.clip_rect();
            self.ui.set_clip_rect(inherited.clip);
            self.ui.interact(boxes.border, id, inherited.drags.sense());
            self.ui.set_clip_rect(clip);
        }
    }

    /// The node's blocker: a widget over its border box that takes clicks and
    /// drags, registered before the node's descendants so that they win.
    fn block(&mut self, index: usize, boxes: &Boxes, inherited: Inherited) {
        let id = blocker_id(index);
        let overlay = self
            .roots
            .iter()
            .find(|root| root.first == index)
            .and_then(PlacedRoot::overlay);
        let modal = overlay.is_some_and(|overlay| matches!(overlay.form, Form::Modal(_)));
        let described = overlay
            .map(|overlay| &overlay.description)
            .filter(|description| description.is_set());
        // An overlay that is leaving is inert, but its blocker keeps the page
        // from taking the pointer until it is gone.
        let inherited = match overlay {
            Some(_) => Inherited {
                inertness: Inertness::Live,
                ..inherited
            },
            None => inherited,
        };
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(boxes.border),
            inherited,
            None,
            |ui| {
                let response = ui.interact(boxes.border, id, Sense::CLICK | Sense::DRAG);
                if let Some(description) = described {
                    description.announce(&response, ui.is_enabled().into(), Role::GenericContainer);
                }
                if modal {
                    ui.ctx()
                        .accesskit_node_builder(response.id, |node| node.set_modal());
                }
            },
        );
    }

    /// What the node's one hit test senses, if it gets one: what its classes
    /// declare, or, for a scroll container with nothing focusable inside, a
    /// place in the Tab order.
    fn sense_of(&self, index: usize) -> Option<HitTest> {
        let node = self.nodes.node(index)?;
        let id = node.id()?;
        let sense = if node.senses() {
            let interaction = &node.interaction;
            let sense = interaction.focusability.sense(interaction.sensing);
            if self.pass.off_tab.contains(&id) {
                sense - Sense::FOCUSABLE
            } else {
                sense
            }
        } else if self.scroller_takes_focus(index) {
            Sense::focusable_noninteractive()
        } else {
            let plan = node.scroll()?;
            plan.description.is_set().then(Sense::hover)?
        };
        Some(HitTest { id, sense })
    }

    /// Whether the scroll container at `index` is a Tab stop in this layout.
    fn scroller_takes_focus(&self, index: usize) -> bool {
        let Some(node) = self.nodes.node(index) else {
            return false;
        };
        let Some(viewport) = node.scroll_viewport() else {
            return false;
        };
        node.scroll()
            .is_some_and(|plan| plan.is_tab_stop(viewport.size()))
            && !node.is_inert()
    }

    /// Hovering the node or any sensing descendant hovers the node, as
    /// `:hover` does in CSS. An item of a listbox is hovered only by its
    /// highlight, which the pointer moves (see [`nav`]): the pointer and the
    /// keys never leave two items looking hovered.
    fn hovering(&self, index: usize, node: &PlacedNode, own: &Response) -> Hovering {
        let ctx = self.ui.ctx();
        let descendants = self.nodes.descendants(index);
        let highlighted = node.id().is_some_and(|id| {
            self.pass.highlighted.contains(&id)
                || (node.hovers_when_focused() && focus::holds_focus(ctx, id))
        });
        let by_rect = node.interaction.hover_by == crate::pipeline::HoverBy::Rect
            && ctx.rect_contains_pointer(self.ui.layer_id(), own.interact_rect);
        let pointed = own.hovered()
            || by_rect
            || descendants
                .iter()
                .filter(|descendant| descendant.senses())
                .filter_map(PlacedNode::id)
                .any(|id| ctx.read_response(id).is_some_and(|r| r.hovered()));
        let in_listbox = node.is_item()
            && root_of(self.roots, index)
                .and_then(|root| self.roots.get(root))
                .and_then(PlacedRoot::overlay)
                .map_or(node.is_listbox_option(), |overlay| {
                    overlay.behavior.nav.is_listbox()
                });
        Hovering::of(highlighted || (pointed && !in_listbox))
    }

    /// How far each state variant has taken hold, animated when the node has
    /// a `transition`.
    fn presence(
        &self,
        node: &PlacedNode,
        response: Option<&Response>,
        hovering: Option<Hovering>,
        group: Option<Hovering>,
        stakes: Stakes,
    ) -> Presence {
        let variants = &node.interaction.variants;
        let progress = |state: State| {
            if variants.get(state).is_none() {
                return Progress::START;
            }
            let on = match state {
                State::GroupHover => {
                    if group.is_none() {
                        crate::report::report(
                            self.ui.ctx(),
                            crate::Problem::GroupHoverWithoutGroup,
                            Some(node.id().unwrap_or_else(|| Id::NULL.with(node.path()))),
                            || {
                                format!(
                                    "{}: group_hover needs an ancestor marked group(); it never \
                                     shows",
                                    node.path()
                                )
                            },
                        );
                    }
                    group == Some(Hovering::Over)
                }
                State::Hover => hovering == Some(Hovering::Over),
                State::Focus => response.is_some_and(Response::has_focus),
                State::FocusWithin => node.id().is_some_and(|id| self.shell.focus_within(id)),
                State::Active => {
                    response.is_some_and(|r| r.hovered() && r.is_pointer_button_down_on())
                }
                State::DragAccepts => stakes.accepts,
                State::DragOver => stakes.over,
                State::DragLifted => stakes.lifted,
            };
            let target = if on { Progress::END } else { Progress::START };
            match (variants.transition(), node.id()) {
                (Some(duration), Some(id)) => transition::animate(
                    self.ui.ctx(),
                    id.with(("eggshell-state", state as u8)),
                    target,
                    duration,
                    variants.easing(),
                ),
                _ => target,
            }
        };
        Presence {
            group_hover: progress(State::GroupHover),
            hover: progress(State::Hover),
            focus_within: progress(State::FocusWithin),
            focus: progress(State::Focus),
            active: progress(State::Active),
            drag_accepts: progress(State::DragAccepts),
            drag_over: progress(State::DragOver),
            drag_lifted: progress(State::DragLifted),
        }
    }

    /// Visits the node and its subtree, noting which focusable widgets the
    /// subtree registered if the node is identified.
    fn visit(&mut self, index: usize, parent: Inherited) {
        if parent.culls && self.culled(index, parent.clip) {
            if self.shared.recording.is_some() {
                let subtree = self.nodes.subtree(index);
                self.shared
                    .culled
                    .extend(subtree.iter().filter_map(PlacedNode::id));
            }
            self.register_culled(index, parent);
            return;
        }
        self.noting(index, |walk| walk.visit_node(index, parent));
    }

    /// Runs `visit` and notes which focusable widgets it registered, if the
    /// node is identified.
    fn noting(&mut self, index: usize, visit: impl FnOnce(&mut Self)) {
        let layer = self.ui.layer_id();
        let before = self
            .nodes
            .node(index)
            .and_then(PlacedNode::id)
            .map(|id| (id, focus::registered(self.ui.ctx(), layer)));
        visit(self);
        if let Some((id, from)) = before {
            let widgets = focus::focusable_since(self.ui.ctx(), layer, from);
            self.pass.subtrees.push((id, widgets));
        }
    }

    /// Registers the widgets of a culled subtree at their real rects, painting
    /// nothing: Tab reaches them and assistive technology reads them. Nodes
    /// that only paint are left out, and a subtree with none costs one lookup.
    ///
    /// A node costs its one hit test (a widget entry and an accessibility
    /// node), and, if it adds widgets, its draw closure with the opacity at
    /// zero, so that what it paints is dropped by egui's painter.
    fn register_culled(&mut self, index: usize, parent: Inherited) {
        let nodes = self.nodes;
        let Some(node) = nodes.node(index) else {
            return;
        };
        if node.flow == Flow::Hidden
            || self.registrants.get(node.end) == self.registrants.get(index)
        {
            return;
        }
        let inertness = parent.inertness.or(node.is_inert().into());
        let visibility = node.visibility().unwrap_or(parent.visibility);
        let hidden = Inherited {
            inertness,
            visibility,
            opacity: Opacity::TRANSPARENT,
            ..parent
        };
        self.noting(index, |walk| {
            let boxes = Boxes::of(node, walk.grid);
            if visibility == Visibility::Visible {
                if let Some(test) = walk.sense_of(index) {
                    walk.interact(index, node, test, &boxes, hidden);
                }
                if node.draws_widgets() {
                    walk.run_draw(index, &boxes, hidden, node.text(), (None, None));
                }
            }
            for child in nodes.in_paint_order(index) {
                walk.register_culled(child, hidden);
            }
        });
    }

    /// Whether the node and its subtree can be left out: nothing of it
    /// reaches `clip`, no slide in flight can bring it there, and it holds
    /// neither the keyboard focus nor the drag.
    fn culled(&self, index: usize, clip: Rect) -> bool {
        let Some(node) = self.nodes.node(index) else {
            return false;
        };
        let within = self.nodes.span(index);
        let ctx = self.ui.ctx();
        node.flow != Flow::Hidden
            && !node
                .reach
                .intersects(clip.expand(self.pass.animation.slide_reach()))
            && !self.drag.involves(&within)
            && !(ctx.memory(|memory| memory.focused()).is_some()
                && self
                    .nodes
                    .subtree(index)
                    .iter()
                    .filter_map(PlacedNode::id)
                    .any(|id| self.shell.focus_within(id)))
    }

    fn visit_node(&mut self, index: usize, parent: Inherited) {
        let nodes = self.nodes;
        let Some(node) = nodes.node(index) else {
            return;
        };
        if node.flow == Flow::Hidden {
            return;
        }
        let boxes = Boxes::of(node, self.grid);
        let inertness = parent.inertness.or(node.is_inert().into());
        let visibility = node.visibility().unwrap_or(parent.visibility);
        let shown = visibility == Visibility::Visible;
        if !shown && node.as_scroll().is_some() {
            return;
        }
        let interaction = &node.interaction;

        let pointer_events = parent.pointer_events.merge(node.pointer_events());
        if shown {
            self.cover(
                index,
                &boxes,
                Inherited {
                    inertness,
                    pointer_events,
                    ..parent
                },
            );
        }

        let (response, merged) = match self.sense_of(index).filter(|_| shown) {
            Some(test) => {
                let (response, merged) = self.interact(
                    index,
                    node,
                    test,
                    &boxes,
                    Inherited {
                        inertness,
                        ..parent
                    },
                );
                (Some(response), merged)
            }
            None => (None, Merged::Own),
        };
        if let Some(response) = &response {
            dnd::show_grab(self.shell, nodes, index, response);
        }
        let needs_hovering = interaction.grouping == Grouping::Group
            || interaction.variants.get(State::Hover).is_some();
        let hovering = response
            .as_ref()
            .filter(|_| needs_hovering)
            .map(|own| self.hovering(index, node, own));
        let presence = self.presence(
            node,
            response.as_ref(),
            hovering,
            parent.group,
            self.drag.stakes(index),
        );
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
                translate: Vec2::ZERO,
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

        let stage = self.shared.stages.get(&index).copied();
        let (staged_offset, staged_scale, scrim) = match stage {
            Some(Stage::Box { staged, form }) => (
                staged.offset(node.rect().size(), form),
                staged.scale(),
                Opacity::OPAQUE,
            ),
            Some(Stage::Scrim(scrim)) => (Vec2::ZERO, 1.0, scrim),
            None => (Vec2::ZERO, 1.0, Opacity::OPAQUE),
        };
        let opacity = parent.opacity * look.paint.opacity;
        let own_opacity = opacity * scrim;
        let lifted = self.lift.filter(|lift| lift.index == index);
        let slide = match (interaction.variants.animate_layout(), node.id(), lifted) {
            (Some(duration), Some(id), None) => self.pass.animation.offset(
                id,
                node.rect().min - parent.origin,
                duration,
                interaction.variants.easing(),
            ),
            _ => Vec2::ZERO,
        };
        // The anchor is screen space. The shapes leave the layer before the
        // ancestors move what they painted there, so their offsets do not
        // carry the node, and the recording leaves them out as well as the
        // anchor: where the pointer carries a node is input, not motion.
        let (translate, shift) = match lifted {
            Some(lift) => (lift.min - node.rect().min, Vec2::ZERO),
            None => {
                let translate = look.translate + slide + staged_offset;
                (translate, parent.shift + translate)
            }
        };
        if let (Some(frame), Some(id)) = (&mut self.shared.recording, node.id()) {
            frame.push(NodeFrame {
                id,
                path: node.path().to_owned(),
                layout_rect: node.rect(),
                painted_rect: scaled_about_centre(node.rect(), staged_scale).translate(shift),
                ancestors: if lifted.is_some() {
                    Vec2::ZERO
                } else {
                    parent.shift
                },
                scrolled: parent.scrolled,
                opacity: own_opacity,
                motion: match (interaction.variants.animate_layout(), lifted, stage) {
                    (_, None, Some(_)) => Motion::Presents,
                    _ if node.collapsible => Motion::Resizes,
                    (Some(_), None, None) => Motion::Slides,
                    _ => Motion::Snaps,
                },
                slide: self
                    .pass
                    .animation
                    .slide(id, interaction.variants.easing(), translate),
            });
        }
        let moved_from = (translate != Vec2::ZERO || lifted.is_some() || staged_scale != 1.0)
            .then(|| self.next_shape());
        if shown {
            paint_box(
                &self.painter(parent.clip, own_opacity),
                &look.paint,
                &boxes,
                node.border,
                self.grid,
            );
        }

        let painted_background = look.paint.background.map_or(Color32::TRANSPARENT, |fill| {
            fill.gamma_multiply(f32::from(opacity))
        });
        let inherited = Inherited {
            // A lifted node is painted where the pointer carries it, not where
            // layout put it.
            culls: lifted.is_none() && parent.culls,
            backdrop: parent.backdrop.blend(painted_background),
            clip: content_clip(parent.clip, boxes.padding, node.overflow),
            opacity,
            inertness,
            visibility,
            pointer_events,
            drags: if interaction.sensing.drags()
                || (node.scroll().is_some() && scroll::drag_scrolls(self.ui.ctx()))
            {
                Drags::PassThrough
            } else {
                parent.drags
            },
            origin: node.rect().min,
            shift,
            scrolled: parent.scrolled,
            group,
            text_color: if interaction.variants.sets_text_color() {
                Some(look.text)
            } else if interaction.color_source == ColorSource::Inherited {
                parent.text_color
            } else {
                None
            },
            interactive: if node.senses() {
                Interactive::Within
            } else {
                parent.interactive
            },
        };
        let mut elided = None;
        if let Some(prim) = self.shared.prims.get_mut(index).and_then(Option::take) {
            if shown {
                elided = self.paint_prim(&prim, index, node, &text, &boxes, inherited);
            }
            if let Some(slot) = self.shared.prims.get_mut(index) {
                *slot = Some(prim);
            }
        }
        if shown {
            let bubbled = response.as_ref().filter(|_| merged == Merged::Bubbled);
            self.run_draw(index, &boxes, inherited, &text, (bubbled, elided));
        }

        if let Some(scroll) = node.as_scroll() {
            self.scroll_content(index, scroll, &boxes, inherited);
        } else {
            for child in nodes.in_paint_order(index) {
                self.visit(child, inherited);
            }
            if shown {
                self.paint_dividers(node, index, inherited);
            }
        }
        if self.scroller_takes_focus(index)
            && !interaction.variants.has_focus()
            && response.as_ref().is_some_and(Response::has_focus)
        {
            let ring = node.style.tokens.focus;
            self.painter(parent.clip, opacity).add(RectShape::stroke(
                boxes.border,
                look.paint.radius,
                Stroke::new(ring.width, ring.color),
                StrokeKind::Inside,
            ));
        }
        if let Some(first) = moved_from {
            let about = node.rect().center().to_vec2() * (1.0 - staged_scale);
            self.transform_shapes_since(first, TSTransform::new(about + translate, staged_scale));
            if lifted.is_some() {
                self.lift_shapes_since(first);
            }
        }
    }

    /// Runs the node's draw closure in its own `Ui`, if it has one.
    fn run_draw(
        &mut self,
        index: usize,
        boxes: &Boxes,
        inherited: Inherited,
        text: &TextStyle,
        (bubbled, elided): (Option<&Response>, Option<Elided>),
    ) {
        let Some(node) = self.nodes.node(index) else {
            return;
        };
        let Some((id, slot)) = node.draw_with_id() else {
            return;
        };
        let draws = self
            .shared
            .draws
            .get_mut(slot)
            .map(std::mem::take)
            .filter(|draws| !draws.is_empty());
        if let Some(draws) = draws {
            let builder = UiBuilder::new()
                .max_rect(boxes.content)
                .scope_id(id.with(DRAW_SCOPE));
            let app = &mut *self.app;
            let blur = self.pass.blur;
            let pane = node
                .pane
                .and_then(|side| neighbour(self.nodes, index, side));
            node_ui(self.ui, builder, inherited, Some(text), |ui| {
                let scope = ui.unique_id();
                ui.data_mut(|data| {
                    data.insert_temp(scope, NodeIdTag(id));
                    data.insert_temp(scope, PaneTag(pane));
                    data.insert_temp(scope, BlurTag(blur));
                    data.insert_temp(scope, TextTag(text.clone()));
                    data.insert_temp(scope, StyleTag(node.style.clone()));
                    if let Some(response) = bubbled {
                        data.insert_temp(scope, BubbledTag(response.clone()));
                    } else {
                        data.remove::<BubbledTag>(scope);
                    }
                    if let Some(elided) = elided {
                        data.insert_temp(scope, elided);
                    } else {
                        data.remove::<Elided>(scope);
                    }
                });
                for draw in draws {
                    draw(app, ui);
                }
            });
        }
    }

    /// Shows the content of the scroll container at `index` in an egui
    /// `ScrollArea` over its viewport: egui owns the offset, the wheel and
    /// the bars, and the content, laid out at that offset, is walked inside
    /// it with the nodes outside the viewport left out.
    fn scroll_content(
        &mut self,
        index: usize,
        ScrollRef { id, plan, viewport }: ScrollRef<'_>,
        boxes: &Boxes,
        inherited: Inherited,
    ) {
        let nodes = self.nodes;
        let wheel = self
            .shell
            .mount_of(nodes, index)
            .map(|mount| self.shell.mount_selected(mount));
        let source = match wheel {
            Some(false) => egui::scroll_area::ScrollSource {
                mouse_wheel: false,
                ..scroll::source()
            },
            Some(true) | None => scroll::source(),
        };
        let area = egui::ScrollArea::new(plan.overflows(boxes.padding.size() - plan.bars.gutter))
            .id_salt(scroll::AREA_SALT)
            .auto_shrink(false)
            .scroll_source(source)
            // egui inflates a viewport smaller than 64 points to 64 and clamps
            // the offset to that, so a short container stopped short of its end
            // and then drifted back.
            .min_scrolled_width(0.0)
            .min_scrolled_height(0.0)
            .content_margin(egui::Margin::ZERO)
            .scroll_bar_visibility(plan.bars.visibility.egui());
        let area = if plan.moved {
            area.scroll_offset(plan.offset)
        } else {
            area
        };
        let Self {
            app,
            ui,
            ordinals,
            registrants,
            shared,
            pass,
            grid,
            lift,
            drag,
            roots,
            shell,
            ..
        } = self;
        // The viewport clips the content, whatever egui reserves for its bars.
        let content = Inherited {
            clip: inherited.clip.intersect(grid.rect(viewport)),
            culls: true,
            scrolled: inherited.scrolled - plan.offset,
            ..inherited
        };
        node_ui(
            ui,
            UiBuilder::new()
                .max_rect(boxes.padding)
                .scope_id(scroll::area_scope(id)),
            inherited,
            None,
            |ui| {
                let outer = ui.style().clone();
                ui.spacing_mut().scroll = plan.bars.egui(&outer.spacing.scroll, viewport.size());
                // egui bounds the offset by the last frame's content, and
                // springs one past them back, carrying the spring's velocity
                // into the next frames: an offset set at the end of content
                // that grew this frame would creep back short of it.
                if plan.moved {
                    ui.spacing_mut().scroll.kinetic.rubber_band = false;
                }
                plan.bars.paint_in(ui.visuals_mut());
                let shown = area.show_viewport(ui, |ui, _| {
                    // The bar's colours are for the bar, not for the content.
                    ui.set_style(outer);
                    ui.allocate_space(plan.content);
                    Walk {
                        app: &mut **app,
                        ui,
                        nodes,
                        ordinals,
                        registrants,
                        shared: &mut **shared,
                        pass,
                        grid: *grid,
                        lift: *lift,
                        drag,
                        roots,
                        shell,
                    }
                    .visit(index.saturating_add(1), content);
                });
                scroll::note_bar_hold(shell, id, shown.id);
                // A selected mount's scroll container keeps what the wheel
                // gave it past its end: none of it reaches the page.
                if wheel == Some(true) && ui.rect_contains_pointer(shown.inner_rect) {
                    ui.ctx()
                        .input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
                }
            },
        );
    }

    /// Moves what the subtree painted since `first` to the drag layer, with
    /// the screen as its clip.
    fn lift_shapes_since(&self, first: ShapeIdx) {
        let layer = self.ui.layer_id();
        let screen = self.ui.ctx().content_rect();
        self.ui.ctx().graphics_mut(|graphics| {
            let lifted: Vec<Shape> = graphics
                .entry(layer)
                .all_entries()
                .skip(first.0)
                .map(|clipped| clipped.shape.clone())
                .collect();
            let source = graphics.entry(layer);
            for at in first.0..first.0 + lifted.len() {
                source.reset_shape(ShapeIdx(at));
            }
            let target = graphics.entry(drag_layer());
            for shape in lifted {
                target.add(screen, shape);
            }
        });
    }

    fn next_shape(&self) -> ShapeIdx {
        let layer = self.ui.layer_id();
        self.ui
            .ctx()
            .graphics_mut(|graphics| graphics.entry(layer).next_idx())
    }

    /// Moves what the subtree painted since `first`, and the clips it was
    /// painted with, as a CSS transform does: layout and hit rects stay.
    fn transform_shapes_since(&self, first: ShapeIdx, by: TSTransform) {
        let layer = self.ui.layer_id();
        self.ui.ctx().graphics_mut(|graphics| {
            let list = graphics.entry(layer);
            let end = list.next_idx();
            list.transform_range(first, end, by);
        });
    }

    fn paint_prim(
        &mut self,
        prim: &Prim,
        index: usize,
        node: &PlacedNode,
        text: &TextStyle,
        boxes: &Boxes,
        inherited: Inherited,
    ) -> Option<Elided> {
        let content = boxes.content;
        if let Some(leaf) = TextLeaf::of(self.ui.ctx(), prim, (text, &node.style.rich)) {
            return self.paint_text(&leaf, index, node, boxes, inherited);
        }
        match prim {
            Prim::Icon(icon) => self.paint_image(
                image_for(icon.clone(), text.color),
                meet(content),
                inherited,
            ),
            Prim::Image(src) => self.paint_image(Image::new(src.clone()), content, inherited),
            Prim::Paint(paint) => paint(
                &self.painter(inherited.clip, inherited.opacity),
                content,
                text.color,
            ),
            Prim::Text(_) | Prim::Rich(_) | Prim::Spans(_) => {}
        }
        None
    }

    fn paint_text(
        &mut self,
        leaf: &TextLeaf<'_>,
        index: usize,
        node: &PlacedNode,
        boxes: &Boxes,
        inherited: Inherited,
    ) -> Option<Elided> {
        let content = boxes.content;
        let text = leaf.style;
        let (galley, underlines) = self
            .ui
            .ctx()
            .fonts_mut(|fonts| leaf.drawn_galley(fonts, boxes.wrap_width));
        let elided = galley.elided.then(|| Elided(galley.text().into()));
        let top = node
            .text_valign()
            .offset(content.height() - galley.size().y);
        let anchor: Pos2 = match text.align {
            TextAlign::Left => content.left_top(),
            TextAlign::Center => content.center_top(),
            TextAlign::Right => content.right_top(),
        } + vec2(0.0, top);
        let lines = spans::underlines(&galley, &underlines, anchor);
        let marks = leaf.marks.as_deref();
        let runs = marks.map_or_else(Vec::new, |marks| spans::runs(&galley, marks));
        if let Some(marks) = marks {
            let rich = &node.style.rich;
            let painter = self.painter(inherited.clip, inherited.opacity);
            for tint in spans::code_tints(&runs, marks, rich, anchor) {
                painter.rect_filled(tint, rich.code.radius, rich.code.fill);
            }
        }
        let id = self.selection_id(index, node);
        let links = marks.filter(|marks| spans::has_links(marks));
        let response = if text.select.selects(inherited.interactive) {
            Some(self.select_text(
                id,
                (galley, anchor),
                content,
                (text, node.style.selection),
                inherited,
            ))
        } else {
            self.painter(inherited.clip, inherited.opacity)
                .galley(anchor, galley, text.color);
            links.map(|_| self.sense_links(id, content, inherited))
        };
        let painter = self.painter(inherited.clip, inherited.opacity);
        for (line, stroke) in lines {
            let color = match stroke.color {
                Color32::PLACEHOLDER => text.color,
                color => color,
            };
            painter.line_segment(line, Stroke::new(stroke.width, color));
        }
        // TODO(#279): report text with links that has no id, which `spans` never adds.
        if let (Some(marks), Some(response), Some(node)) = (links, response, node.id()) {
            spans::follow(&response, &runs, marks, anchor, node);
        }
        elided
    }

    fn sense_links(&mut self, id: Id, content: Rect, inherited: Inherited) -> Response {
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(content),
            inherited,
            None,
            |ui| ui.interact(content, id, Sense::click() - Sense::FOCUSABLE),
        )
    }

    /// Paints `galley` through egui's label selection, which draws the
    /// highlight, follows drags across labels and copies on Ctrl+C. The
    /// galley and its position are the ones plain painting uses, so layout
    /// and pixels are the same without a selection. The whole content box
    /// takes the pointer, so a drag may end in the blank beside the text.
    /// Where drags pass through, as to a scroll area on a touch screen, the
    /// text takes clicks alone: a drag scrolls, as a browser's does.
    fn select_text(
        &mut self,
        id: Id,
        (galley, anchor): (Arc<Galley>, Pos2),
        content: Rect,
        (text, selection): (&TextStyle, Selection),
        inherited: Inherited,
    ) -> Response {
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(content),
            inherited,
            Some(text),
            |ui| {
                ui.visuals_mut().selection.bg_fill = selection.fill;
                if let Some(color) = selection.text {
                    ui.visuals_mut().selection.stroke.color = color;
                }
                let response = ui.interact(content, id, inherited.drags.sense() - Sense::FOCUSABLE);
                response.widget_info(|| {
                    WidgetInfo::labeled(Role::Label, ui.is_enabled(), galley.text())
                });
                LabelSelectionState::label_text_selection(
                    ui,
                    &response,
                    anchor,
                    galley,
                    text.color,
                    Stroke::NONE,
                );
                response
            },
        )
    }

    /// A stable id for the node's selectable text: its key path, and the
    /// order among the text nodes that share one. The order counts every
    /// node of the frame, so culling some of them does not renumber the rest.
    fn selection_id(&self, index: usize, node: &PlacedNode) -> Id {
        self.ui
            .scope_id()
            .with(("select", node.path()))
            .with(self.ordinals.get(index).copied().unwrap_or_default())
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

    fn paint_dividers(&self, node: &PlacedNode, index: usize, inherited: Inherited) {
        let paint = node.paint();
        let painter = self.painter(inherited.clip, inherited.opacity);
        let in_flow: Vec<Rect> = self
            .nodes
            .children(index)
            .filter_map(|child| self.nodes.node(child))
            .filter(|node| node.flow == Flow::Normal)
            .map(|node| self.grid.rect(node.rect()))
            .collect();
        for (divide, axis) in [(paint.divide_x, Axis::X), (paint.divide_y, Axis::Y)] {
            let Some(divide) = divide else { continue };
            for (&before, &after) in in_flow.iter().zip(in_flow.iter().skip(1)) {
                if let Some(line) = divider(before, after, axis, divide.width, self.grid) {
                    painter.rect_filled(line, 0.0, divide.color);
                }
            }
        }
    }
}

impl<A> Walk<'_, A> {
    /// Notes how the overlay rooted at `first` is painted while it enters or
    /// leaves.
    fn stage(&mut self, first: usize, form: Form, staged: Staged) {
        let boxed = Stage::Box { staged, form };
        match form {
            Form::Modal(_) => {
                self.shared
                    .stages
                    .insert(first, Stage::Scrim(staged.scrim()));
                for child in self.nodes.children(first) {
                    self.shared.stages.insert(child, boxed);
                }
            }
            Form::Floating(_) => {
                self.shared.stages.insert(first, boxed);
            }
        }
    }

    /// Draws each pop-out window of `here` that is embedded in its viewport,
    /// in an `egui::Window`, and returns what became of them.
    fn embedded_windows(
        &mut self,
        here: Here,
        chrome: Option<window::Chrome>,
    ) -> Vec<window::Shown> {
        let ctx = self.ui.ctx().clone();
        let mut shown = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            let Some(window) = root.window() else {
                continue;
            };
            if root.home != here.home || window.presentation != window::Presentation::Embedded {
                continue;
            }
            let mut open = true;
            let drawn = window::embedded(window, chrome, &mut open).show(&ctx, |ui| {
                let inherited = Inherited::root(ui.clip_rect(), window.content.min);
                let content = ui.max_rect();
                self.in_layer(ui).visit(root.first, inherited);
                ui.allocate_rect(content, Sense::hover());
                content
            });
            if let Some(drawn) = drawn {
                shown.push(window::Shown {
                    root: index,
                    close_asked: !open,
                    content: drawn.inner.unwrap_or(window.content),
                    outer: Some(drawn.response.rect),
                });
            }
        }
        shown
    }

    /// Draws every attached overlay of `here`, in the order they were
    /// declared, each in a layer of its own, and returns the ones drawn in
    /// the order they were opened. Walks `self` from the page's `Ui`.
    fn overlays(&mut self, here: Here) -> Vec<Drawn> {
        let ctx = self.ui.ctx().clone();
        let screen = ctx.content_rect();
        let mut shown = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            if root.home != here.home {
                continue;
            }
            let Some(overlay) = root.overlay() else {
                continue;
            };
            if overlay.anchored == Anchored::Detached {
                continue;
            }
            let Some(root_rect) = self.nodes.node(root.first).map(PlacedNode::rect) else {
                continue;
            };
            let swallows = covers_screen(self.roots, self.nodes, index);
            let rect = if swallows { screen } else { root_rect };
            let layer = overlay.band.layer(overlay.id);
            // Seeded so that the layer is its box from the first frame, with
            // no sizing pass in which egui would hide it.
            ctx.memory_mut(|memory| {
                memory.areas_mut().set_state(
                    layer,
                    egui::AreaState {
                        pivot_pos: Some(rect.min),
                        pivot: egui::Align2::LEFT_TOP,
                        size: Some(rect.size()),
                        interactable: overlay.band.senses_input(),
                        last_became_visible_at: None,
                    },
                );
            });
            let staged = overlay.staged;
            let mut inherited = Inherited::root(screen, root_rect.min);
            if !staged.is_steady() {
                self.stage(root.first, overlay.form, staged);
            }
            if staged.is_exiting() {
                inherited.inertness = Inertness::Inert;
            }
            egui::Area::new(layer.id)
                .order(layer.order)
                .fixed_pos(rect.min)
                .constrain(false)
                .movable(false)
                .interactable(overlay.band.senses_input())
                .fade_in(false)
                .show(&ctx, |ui| {
                    ui.allocate_space(rect.size());
                    if swallows {
                        ui.interact(rect, overlay.id.with("swallow"), Sense::CLICK | Sense::DRAG);
                    }
                    self.in_layer(ui).visit(root.first, inherited);
                });
            shown.push((
                overlay.stack_index,
                Drawn {
                    id: overlay.id,
                    band: overlay.band,
                },
            ));
        }
        if let Some(top) = stacked_top_down(self.roots, here)
            .into_iter()
            .filter_map(|index| self.roots.get(index)?.overlay())
            .find(|overlay| {
                overlay.anchored == Anchored::Attached && matches!(overlay.form, Form::Modal(_))
            })
        {
            let layer = top.band.layer(top.id);
            ctx.memory_mut(|memory| memory.set_modal_layer(layer));
        }
        shown.sort_by_key(|(stack_index, _)| *stack_index);
        shown.into_iter().map(|(_, shown)| shown).collect()
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
    /// Each overlay is drawn after the page, in a layer of its own that is
    /// exactly its box (an [`egui::Area`] per overlay): its root takes the
    /// pointer from the layers below it, and an overlay whose anchor cannot be
    /// seen is not drawn at all. An overlay that swallows presses has that
    /// layer cover the screen, drawing nothing. Last, the stack of overlays is
    /// repaired (see [`overlay`](crate::overlay)), and the frame's input is
    /// judged against the open overlays: dismissal, focus entering, and the
    /// keys of [`Nav`](crate::overlay::Nav).
    ///
    /// Rects are rounded to physical pixels here; layout itself is exact.
    ///
    /// TODO(prose): answers the frame, to query after it is drawn; an app ignores it.
    ///
    /// ```
    /// use eggshell_ui::testing::PlacedFrame;
    /// use eggshell_ui::{Builder, LayoutState, tw};
    /// use egui_kittest::Harness;
    ///
    /// struct State {
    ///     layout: LayoutState,
    ///     drawn: Option<PlacedFrame>,
    /// }
    ///
    /// let mut harness = Harness::new_ui_state(
    ///     |ui, state: &mut State| {
    ///         let view = |b: &mut Builder<'_, '_, ()>| {
    ///             b.div_id("card", tw(), |b| { b.span("name", tw(), "Mono"); });
    ///         };
    ///         let placed = state.layout.build(ui.ctx(), view).layout(ui.max_rect());
    ///         state.drawn = Some(placed.draw(&mut (), ui));
    ///     },
    ///     State { layout: LayoutState::new(), drawn: None },
    /// );
    /// harness.step();
    /// let card = harness.state().drawn.as_ref().expect("the frame was drawn").at("card");
    /// assert_eq!(card.at("name").rect(), card.text("Mono").rect());
    /// ```
    pub fn draw(self, app: &mut A, ui: &mut Ui) -> PlacedFrame {
        let timer = Timer::start(ui.ctx(), Phase::Draw);
        let Self {
            frame:
                PlacedFrame {
                    ctx,
                    scope,
                    root,
                    kept,
                    nodes,
                    by_id,
                    prims,
                    roots,
                },
            draws,
            drag,
            dismiss,
            windows,
            pop_outs,
            exited,
            commands,
            size_in_flight,
        } = self;
        let overflow = (Debug::of(ui.ctx()) == Debug::Outlines)
            .then(|| debug::overflow(ui.ctx(), &nodes, &prims));
        let mut painting = Painting {
            ordinals: text_ordinals(&nodes, &prims),
            registrants: registrant_counts(&nodes),
            shared: Shared {
                prims,
                draws,
                recording: continuity::recording(ui.ctx()).then(Vec::new),
                culled: Vec::new(),
                box_ids: box_ids(scope, &nodes),
                stages: HashMap::new(),
            },
            kept,
            nodes,
            by_id,
            drag,
            dismiss,
            windows,
            pop_outs,
            roots,
            commands,
            size_in_flight,
            page: ui.ctx().viewport_id(),
        };
        painting.pass(app, ui, Here::PAGE);
        painting.native_windows(app, ui.ctx());
        for then in exited {
            then(app);
        }
        painting.record(ui.ctx());
        drop(timer);
        #[cfg(debug_assertions)]
        crate::inspect::dump_why(ui.ctx(), &painting.nodes);
        if let Some(overflow) = overflow {
            debug::paint(ui.ctx(), &painting.nodes, &overflow);
        }
        let Painting {
            kept,
            nodes,
            by_id,
            roots,
            shared: Shared { prims, .. },
            ..
        } = painting;
        PlacedFrame {
            ctx,
            scope,
            root,
            kept,
            nodes,
            by_id,
            prims,
            roots,
        }
    }
}

/// A frame being drawn: what the passes that draw it share. Each egui
/// viewport the frame is drawn in has a pass of its own, with that viewport's
/// input.
struct Painting<A> {
    kept: Handle,
    nodes: Nodes,
    by_id: HashMap<Id, usize>,
    drag: Decls<A>,
    dismiss: HashMap<Id, DismissFn<A>>,
    windows: HashMap<Id, window::Hooks<A>>,
    pop_outs: Vec<Id>,
    roots: Vec<PlacedRoot>,
    commands: Vec<Declared<A>>,
    size_in_flight: bool,
    /// The viewport the page is drawn in.
    page: egui::ViewportId,
    /// For each node, how many text nodes of the same path come before it.
    ordinals: Vec<u32>,
    /// `registrants[i]` counts the nodes before `i` that register widgets
    /// while culled.
    registrants: Vec<u32>,
    shared: Shared<A>,
}

impl<A> Painting<A> {
    /// Draws the roots of `here` into `ui`: its root, the embedded pop-out
    /// windows over it and its overlays, and judges the input of `ui`'s
    /// viewport against them.
    fn pass(&mut self, app: &mut A, ui: &mut Ui, here: Here) {
        let ctx = ui.ctx().clone();
        let Some((first, origin)) = root_and_node(&self.roots, &self.nodes, here.root)
            .map(|(root, node)| (root.first, node.rect().min))
        else {
            return;
        };
        let Self {
            kept,
            nodes,
            by_id,
            drag,
            dismiss,
            windows,
            pop_outs,
            roots,
            commands,
            size_in_flight,
            page,
            ordinals,
            registrants,
            shared,
        } = self;
        if here.home != Home::Page {
            window::enter_pass(&ctx, kept, roots, here);
        }
        window::carry_focus_in(&ctx, kept, (pop_outs, roots), here);
        let interruption = tooltip::interruption(&ctx, dnd::drag_state(&ctx, kept));
        tooltip::begin(&ctx, kept);
        focus::release_press_on_blur(&ctx);
        crate::clicks::widen_double_click(&ctx);
        let _anchor_focus =
            focus::AnchorFocusHeld::new(&ctx, overlay_input::pointer_on(&ctx, nodes, roots, here));
        let pressed = press_origin(&ctx)
            .map(|at| root_at(roots, nodes, at, here))
            .and_then(|root| Some(roots.get(root)?.overlay()?.id));
        let grid = PixelGrid {
            pixels_per_point: ctx.pixels_per_point(),
        };
        let inherited = Inherited::root(ui.clip_rect(), origin);
        let live = dnd::drag_state(&ctx, kept);
        let (dragging, finish) = dnd::begin(
            &ctx,
            kept,
            dnd::Tree {
                nodes,
                by_id,
                roots,
                here,
            },
            ui.clip_rect(),
            drag,
            app,
        );
        let mut animation =
            LayoutAnimation::begin(&ctx, kept, nodes).starts_slides(!*size_in_flight);
        if nodes.iter().any(|node| node.scroll().is_some()) {
            animation.anticipate(nodes);
        }
        let mut pass = Pass {
            subtrees: Vec::new(),
            animation,
            hot: hot_boxes(&ctx, nodes, &shared.box_ids),
            highlighted: nav::highlighted(kept),
            blur: Blur::Elsewhere,
            off_tab: std::collections::HashSet::new(),
            relations: Vec::new(),
        };
        nav::activate_highlighted(&ctx, kept);
        nav::apply_deferred(&ctx, kept);
        pass.off_tab = crate::roving::settle(&ctx, kept, nodes);
        pass.off_tab
            .extend(crate::hover::off_tab(nodes, roots, here));
        let keys = overlay_input::claim_keys(&ctx, kept, nodes, (roots, here), live);
        pass.blur = keys.blur;
        let shell = crate::Shell::new(&ctx, kept);
        let typing = kept.viewport(ctx.viewport_id(), |here| here.typing);
        let commanding = crate::commands::live_roots(
            overlay_input::topmost_root(roots, here),
            |root| roots.get(root).and_then(|root| root.parent),
            |root| roots.get(root).is_some_and(PlacedRoot::is_modal),
        );
        let tables = (commands.as_slice(), commanding.as_slice());
        crate::commands::visit_all(&ctx, &shell, typing, tables, app, &mut Visit::Fire);
        let mut claimed = Vec::new();
        let mut claim = Visit::Claim(&mut claimed);
        crate::commands::visit_all(&ctx, &shell, typing, tables, app, &mut claim);
        crate::web::ClaimedChords::of(&ctx).record(&ctx, claimed);
        scroll_keys::hold_arrows(&ctx, kept, nodes);
        let _drawing = crate::shell::Drawing::enter(&ctx, kept);
        let mut walk = Walk {
            app,
            ui,
            nodes,
            ordinals,
            registrants,
            shared,
            pass: &mut pass,
            grid,
            lift: dragging.lift,
            drag: &dragging,
            roots,
            shell: &shell,
        };
        walk.visit(first, inherited);
        let embedded = walk.embedded_windows(here, kept.with(|kept| kept.chrome));
        let shown = walk.overlays(here);
        stack::repair(&ctx, &shown, pressed);
        let popups = overlay_input::judge(
            overlay_input::Frame {
                ctx: &ctx,
                kept,
                nodes,
                roots,
                here,
                subtrees: &pass.subtrees,
                drag: live,
                keys,
            },
            dismiss,
            app,
        );
        pass.relations.extend(popups);
        pass.relations.extend(nav::relations(&ctx, kept));
        pass.relations.extend(crate::vlist::relations(
            kept,
            &shell,
            ctx.cumulative_pass_nr(),
        ));
        crate::relation::announce(&ctx, &pass.relations);
        finish.finish(&ctx, kept, nodes, drag, app);
        let quiet = crate::hover::end(crate::hover::Frame {
            ctx: &ctx,
            kept,
            nodes,
            by_id,
            roots,
            here,
            subtrees: &pass.subtrees,
        });
        tooltip::end(&ctx, kept, interruption, &quiet);
        crate::mount::settle(&ctx, kept, (nodes, by_id), (roots, here), typing);
        scroll_keys::note_press(&ctx, kept, nodes);
        scroll_keys::apply_keys(&ctx, kept, nodes);
        scroll_keys::follow_focus(&ctx, kept, &pass.subtrees);
        overlay_input::Typing::note(&ctx, kept, &pass.subtrees);
        focus::store(&ctx, kept, pass.subtrees);
        window::note_focus(&ctx, kept, (pop_outs, roots), here);
        pass.animation.finish();
        for shown in embedded {
            if let Some(window) = roots.get(shown.root).and_then(PlacedRoot::window) {
                window::report(&ctx, (kept, *page), window, shown, windows, app);
            }
        }
    }

    /// Shows each pop-out's native window, and draws its roots in a pass of
    /// its own, with the window's input.
    fn native_windows(&mut self, app: &mut A, ctx: &egui::Context) {
        let natives: Vec<(usize, window::PlacedWindow)> = self
            .roots
            .iter()
            .enumerate()
            .filter(|(index, root)| root.home == Home::Window(*index))
            .filter_map(|(index, root)| Some((index, root.window()?.clone())))
            .collect();
        for (index, window) in natives {
            let builder = window::builder(&self.kept, &window);
            ctx.show_viewport_immediate(window.viewport(), builder, |ui, _| {
                let _timer = Timer::window(
                    ui.ctx(),
                    Phase::Draw,
                    self.page,
                    (window.id, &window.window.title),
                );
                let here = Here::of(&self.roots, index);
                let content = egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        self.pass(app, ui, here);
                        ui.max_rect()
                    })
                    .inner;
                let shown = window::Shown::native(ui.ctx(), index, content);
                window::report(
                    ui.ctx(),
                    (&self.kept, self.page),
                    &window,
                    shown,
                    &mut self.windows,
                    app,
                );
            });
        }
    }

    /// Hands the continuity recorder what the frame drew, if it is recording.
    fn record(&mut self, ctx: &egui::Context) {
        let Some(frame) = self.shared.recording.take() else {
            return;
        };
        let laid_out = self
            .nodes
            .iter()
            .filter(|node| node.id().is_some())
            .map(|node| (node.path().to_owned(), node.rect()))
            .collect();
        continuity::store(
            ctx,
            RecordedPass {
                pass: ctx.cumulative_pass_nr(),
                nodes: frame,
                laid_out,
                culled: std::mem::take(&mut self.shared.culled),
                overlays: placed_overlays(&self.nodes, &self.roots),
            },
        );
    }
}

/// The overlays the frame draws, as the checks of placement judge them.
fn placed_overlays(nodes: &Nodes, roots: &[PlacedRoot]) -> Vec<OverlayFrame> {
    roots
        .iter()
        .filter_map(|root| {
            let overlay = root.overlay()?;
            let node = nodes.get(root.first)?;
            (overlay.anchored == Anchored::Attached).then(|| OverlayFrame {
                id: overlay.id,
                path: node.path().to_owned(),
                bounds: overlay.bounds,
                anchor: overlay.anchor,
                natural: overlay.natural,
                side: match overlay.form {
                    Form::Floating(side) => Some(side),
                    Form::Modal(_) => None,
                },
            })
        })
        .collect()
}

/// For each node, and one past the last, how many nodes before it register
/// widgets while culled.
fn registrant_counts(nodes: &Nodes) -> Vec<u32> {
    let mut counts = Vec::with_capacity(nodes.len() + 1);
    let mut count = 0;
    counts.push(count);
    for node in nodes.iter() {
        count += u32::from(node.registers());
        counts.push(count);
    }
    counts
}

/// For each node, how many text nodes with its path come before it: the
/// order that tells apart the text nodes that share a path.
fn text_ordinals(nodes: &Nodes, prims: &[Option<Prim>]) -> Vec<u32> {
    let mut seen: HashMap<&str, u32> = HashMap::new();
    nodes
        .iter()
        .zip(prims)
        .map(|(node, prim)| match prim {
            Some(prim) if prim.text().is_some() => {
                let count = seen.entry(node.path()).or_insert(0);
                *count += 1;
                *count
            }
            _ => 0,
        })
        .collect()
}

/// Salt of a draw closure's [`Ui`] ids. The `Ui` registers a widget under its
/// own id, which must not be the node's, whose widget is the border box.
const DRAW_SCOPE: &str = "eggshell-draw";

#[derive(Clone, Copy)]
struct NodeIdTag(egui::Id);

/// The border box of the pane a splitter sizes, as laid out.
#[derive(Clone, Copy)]
struct PaneTag(Option<Rect>);

/// The rect of the sibling on `side` of the node at `index`.
fn neighbour(nodes: &Nodes, index: usize, side: Side) -> Option<Rect> {
    let node = nodes.get(index)?;
    let depth = node.depth();
    let sibling = match side {
        Side::Before => nodes
            .before(index)
            .iter()
            .rev()
            .take_while(|before| before.depth() >= depth)
            .find(|before| before.depth() == depth)?,
        Side::After => nodes.get(node.end).filter(|next| next.depth() == depth)?,
    };
    Some(sibling.rect())
}

/// The laid-out border box of the pane the splitter whose draw closure is
/// running sizes.
pub(crate) fn pane_rect(ui: &Ui) -> Option<Rect> {
    let scope = ui.unique_id();
    ui.data(|data| data.get_temp::<PaneTag>(scope))
        .and_then(|tag| tag.0)
}

#[derive(Clone, Copy)]
struct BlurTag(Blur);

#[derive(Clone)]
struct TextTag(TextStyle);

#[derive(Clone)]
struct StyleTag(Arc<Style>);

/// The full text of the node's own text, which was drawn cut short.
#[derive(Clone)]
pub(crate) struct Elided(Arc<str>);

/// The full text of the node whose draw closure is running, if its own text
/// was drawn cut short this pass.
pub(crate) fn elided_text(ui: &Ui) -> Option<Arc<str>> {
    let scope = ui.unique_id();
    ui.data(|data| data.get_temp::<Elided>(scope))
        .map(|tag| tag.0)
}

/// The node's response with what its subtree's boxes won merged in, for
/// [`NodeUi::node_response`].
#[derive(Clone)]
struct BubbledTag(Response);

/// Whether a sensing node's response has anything of its subtree's boxes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Merged {
    Own,
    Bubbled,
}

/// A box the pointer is on or pressing, last pass, that senses nothing of its
/// own: egui gave the press to it, and it is the nearest sensing ancestor's.
///
/// A press goes to the nearest ancestor that senses clicks and no further, as
/// it did when boxes did not take presses. CSS bubbles to every ancestor
/// until a handler stops it; that would merge `PRESS` into every sensing
/// ancestor here, and a handler could tell the response to stop.
struct Hot {
    index: usize,
    flags: Flags,
    at: Option<Pos2>,
}

impl Hot {
    /// What every sensing ancestor sees: the pointer is over a descendant.
    const HOVER: Flags = Flags::HOVERED;
    /// What the nearest clicking ancestor sees.
    const PRESS: Flags = Flags::CLICKED
        .union(Flags::LONG_TOUCHED)
        .union(Flags::IS_POINTER_BUTTON_DOWN_ON);
}

/// The id of the widget each node's box registers, for the node's own id is
/// its response's. Identified nodes derive it from theirs; the others from
/// their key path and how many nodes with the same path come before them, so
/// that it holds while siblings elsewhere come and go, as a press is held.
fn box_ids(scope: Id, nodes: &Nodes) -> Vec<Id> {
    let mut seen: HashMap<&str, u32> = HashMap::new();
    nodes
        .iter()
        .map(|node| match node.id() {
            Some(id) => id.with("box"),
            None => {
                let ordinal = seen.entry(node.path()).or_default();
                *ordinal += 1;
                scope.with(("box", node.path(), *ordinal))
            }
        })
        .collect()
}

/// The boxes the pointer is on or pressing, from egui's last hit test.
fn hot_boxes(ctx: &egui::Context, nodes: &Nodes, box_ids: &[Id]) -> Vec<Hot> {
    if ctx.input(|input| input.pointer.latest_pos().is_none()) {
        return Vec::new();
    }
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.paints_box() && !node.senses() && node.kind() != NodeKind::Overlay)
        .filter_map(|(index, _)| {
            let response = ctx.read_response(*box_ids.get(index)?)?;
            response
                .flags
                .intersects(Hot::HOVER | Hot::PRESS)
                .then(|| Hot {
                    index,
                    flags: response.flags,
                    at: response.interact_pointer_pos(),
                })
        })
        .collect()
}

/// The nearest ancestor of the node at `index` that senses clicks.
fn nearest_clicker(nodes: &Nodes, index: usize) -> Option<usize> {
    nodes.ancestors(index).find(|&above| {
        nodes
            .get(above)
            .is_some_and(|above| above.interaction.sensing.clicks())
    })
}

/// Why a field in the draw closure's `Ui` that lost focus in this pass did,
/// as its layout's `claim_keys` decided.
/// The text style of the node whose draw closure is running, or the egui
/// style's for a `Ui` that is not a draw closure's.
pub(crate) fn node_text(ui: &Ui) -> TextStyle {
    let scope = ui.unique_id();
    ui.data(|data| data.get_temp::<TextTag>(scope))
        .map_or_else(|| TextStyle::from_egui(ui.ctx()), |tag| tag.0)
}

pub(crate) fn blur_cause(ui: &Ui) -> Blur {
    let scope = ui.unique_id();
    ui.data(|data| data.get_temp::<BlurTag>(scope))
        .map_or(Blur::Elsewhere, |tag| tag.0)
}

/// The node of the draw closure `ui` is given, or `None`, reported as
/// [`NodeUiOutsideDraw`](crate::Problem::NodeUiOutsideDraw), with the step
/// `what` skipped.
#[track_caller]
pub(crate) fn node_id_or_report(ui: &Ui, what: &'static str) -> Option<egui::Id> {
    let id = ui.node_id();
    if id.is_none() {
        crate::report::report(ui.ctx(), crate::Problem::NodeUiOutsideDraw, None, || {
            format!(
                "{what} needs the node of a draw closure, and this `Ui` is not the one a \
                 draw closure is given; skipped"
            )
        });
    }
    id
}

/// Reads the response the library gave the node whose draw closure is
/// running.
pub trait NodeUi {
    /// The node's response: clicks, drags, hover, pointer position.
    ///
    /// The library made the node's one hit test before painting it, with its
    /// border box, so hover styling and this response agree. A node that
    /// declares no sense gets a response that senses nothing. Call it on the
    /// [`Ui`] the draw closure is given, not on a nested one, and never
    /// interact with the node's own id yourself.
    ///
    /// TODO(prose): on any other `Ui`, reported as [`NodeUiOutsideDraw`](crate::Problem::NodeUiOutsideDraw), with the `Ui`'s own response.
    #[track_caller]
    fn node_response(&self) -> Response;

    /// The node's egui id: its key path, the one `Builder::id_of` reports.
    /// The [`scope_id`](Ui::scope_id) of the draw closure's `Ui` is derived
    /// from it, so ids made with `make_persistent_id` are stable per node.
    ///
    /// TODO(prose): `None` on any `Ui` other than the one a draw closure is given.
    ///
    /// ```
    /// use eggshell_ui::NodeUi as _;
    ///
    /// let ctx = egui::Context::default();
    /// ctx.run_ui(Default::default(), |ui| assert_eq!(ui.node_id(), None))
    ///     .drop_without_applying_deltas();
    /// ```
    fn node_id(&self) -> Option<egui::Id>;

    /// TODO(prose): the style the node is drawn with: its layout's, or that of
    /// the [`styled`](crate::Builder::styled) scope the node was built in. On
    /// any other `Ui`, one derived from egui's.
    ///
    /// ```
    /// use eggshell_ui::{NodeUi as _, Style};
    ///
    /// let ctx = egui::Context::default();
    /// ctx.run_ui(Default::default(), |ui| {
    ///     let derived = Style::from_egui(&ui.ctx().global_style());
    ///     assert_eq!(ui.node_style().text, derived.text);
    /// })
    /// .drop_without_applying_deltas();
    /// ```
    fn node_style(&self) -> Arc<Style>;

    /// Whether the node, or a widget inside it, has keyboard focus. See
    /// [`Shell::focus_within`](crate::Shell::focus_within).
    ///
    /// TODO(prose): `false` on any other `Ui`, reported as [`NodeUiOutsideDraw`](crate::Problem::NodeUiOutsideDraw).
    #[track_caller]
    fn has_focus_within(&self) -> bool;

    /// Declares the node the visible label `text` of the control with id
    /// `control`, another node of the same frame: assistive technology names
    /// the control by it. Returns whether the label was clicked, for the
    /// caller to forward to the control, as clicking a `<label>` does.
    ///
    /// The node must sense clicks, and is best left out of the Tab order
    /// (`focusable_none`), as the control is the one to focus.
    #[track_caller]
    fn label_for(&self, control: egui::Id, text: &str) -> bool;
}

impl NodeUi for Ui {
    fn node_id(&self) -> Option<egui::Id> {
        let scope = self.unique_id();
        self.data(|data| data.get_temp::<NodeIdTag>(scope))
            .map(|tag| tag.0)
    }

    fn node_style(&self) -> Arc<Style> {
        let scope = self.unique_id();
        self.data(|data| data.get_temp::<StyleTag>(scope))
            .map_or_else(|| Style::derived(self.ctx()), |tag| tag.0)
    }

    #[track_caller]
    fn has_focus_within(&self) -> bool {
        let Some(id) = node_id_or_report(self, "`has_focus_within`") else {
            return false;
        };
        crate::shell::shell_or_report(self, "`has_focus_within`")
            .is_some_and(|shell| shell.focus_within(id))
    }

    #[track_caller]
    fn label_for(&self, control: egui::Id, text: &str) -> bool {
        let response = self.node_response();
        response.widget_info(|| WidgetInfo::labeled(Role::Label, self.is_enabled(), text));
        self.ctx().accesskit_node_builder(control, |node| {
            node.clear_label();
            node.push_labelled_by(response.id.accesskit_id());
        });
        response.clicked()
    }

    #[track_caller]
    fn node_response(&self) -> Response {
        let Some(id) = node_id_or_report(self, "`node_response`") else {
            return self.response();
        };
        if let Some(BubbledTag(response)) = self.data(|data| data.get_temp(self.unique_id())) {
            return response;
        }
        self.ctx()
            .read_response(id)
            .unwrap_or_else(|| self.interact(self.max_rect(), id, Sense::hover()))
    }
}
