//! The draw phase: [`Placed::draw`](crate::Placed::draw) walks the laid-out
//! tree in pre-order and paints box, primitive and draw closure per node.

use egui::emath::TSTransform;
use egui::epaint::{Mesh, RectShape, Shape, StrokeKind, Tessellator};
use egui::layers::ShapeIdx;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use egui::text_selection::LabelSelectionState;
use egui::{
    Align, Color32, Frame, Galley, Id, Image, Layout, Painter, Pos2, Rect, Response, Role, Sense,
    Stroke, Ui, UiBuilder, UiKind, UiStackInfo, Vec2, WidgetInfo, pos2, vec2,
};
use taffy::Overflow;

use crate::continuity::{self, Motion, NodeFrame, RecordedPass};
use crate::contract::{Selection, Style};
use crate::debug::{self, Debug, Phase, Timer};
use crate::dnd::{self, Stakes};
use crate::focus;
use crate::measure::TextLeaf;
use crate::nav;
use crate::overlay::{Anchored, Band, Nav};
use crate::overlay_input::{self, press_origin};
use crate::pipeline::{ColorSource, DrawFn, Flow, NodeKind, Placed, PlacedNode};
use crate::prim::{Icon, Prim};
use crate::roots::{Form, PlacedRoot, covers_screen, root_of, stacked_top_down};
use crate::scroll::{self, Moved};
use crate::scroll_keys;
use crate::shadow_corners::{Corner, arc_segments, clip_mesh_to_triangle, outside_corner_fan};
use crate::stack::{self, Shown};
use crate::text::{Interactive, TextAlign, TextStyle};
use crate::transition::{self, LayoutAnimation};
use crate::tw::{
    BoxPaint, Flip, Grouping, Look, Opacity, PointerEvents, Presence, Progress, RingPlacement,
    Sides, State,
};

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

/// Whether nodes outside the clip are left out of the draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Culling {
    Off,
    On,
}

/// What a node passes down to its subtree.
#[derive(Clone, Copy)]
struct Inherited {
    culling: Culling,
    clip: Rect,
    opacity: Opacity,
    inertness: Inertness,
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
            culling: Culling::Off,
            clip,
            opacity: Opacity::OPAQUE,
            inertness: Inertness::Live,
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

/// The indices of the children of the node at `index`, in layout order.
pub(crate) fn children_of(nodes: &[PlacedNode], index: usize) -> Vec<usize> {
    let end = nodes[index].end;
    let mut children = Vec::new();
    let mut next = index + 1;
    while next < end {
        children.push(next);
        next = nodes[next].end;
    }
    children
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

/// What the walk of a frame accumulates, whichever `Ui` it is in.
struct Shared<A> {
    prims: Vec<Option<Prim>>,
    draws: Vec<Option<DrawFn<A>>>,
    subtrees: Vec<(Id, Box<[Id]>)>,
    animation: LayoutAnimation,
    recording: Option<Vec<NodeFrame>>,
    /// While recording, the identified nodes left out as culled.
    culled: Vec<Id>,
    /// The nodes of the layer being walked that take the pointer, in the
    /// order they were registered: what a later blocker may cover.
    beneath: Vec<Beneath>,
    /// The items listboxes highlight, which look hovered.
    highlighted: Vec<Id>,
}

/// How far two boxes may overlap and still only touch: layout leaves
/// neighbours overlapping by a fraction of a point.
const TOUCH: f32 = 0.5;

/// A node that takes the pointer, registered before the node being walked.
struct Beneath {
    index: usize,
    /// One past the last descendant's index.
    end: usize,
    /// Where the pointer can reach it.
    reach: Rect,
}

struct Walk<'a, A> {
    app: &'a mut A,
    ui: &'a mut Ui,
    nodes: &'a [PlacedNode],
    /// For each node, how many text nodes of the same path come before it.
    ordinals: &'a [u32],
    /// `registrants[i]` counts the nodes before `i` that register widgets
    /// while culled.
    registrants: &'a [u32],
    shared: &'a mut Shared<A>,
    grid: PixelGrid,
    selection: Selection,
    lift: Option<Lift>,
    drag: &'a dnd::Draw,
    roots: &'a [PlacedRoot],
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
            grid: self.grid,
            selection: self.selection,
            lift: self.lift,
            drag: self.drag,
            roots: self.roots,
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
    fn interact(
        &mut self,
        node: &PlacedNode,
        sense: Sense,
        boxes: &Boxes,
        inherited: Inherited,
    ) -> Response {
        let id = node.id().expect("a sensing node is identified");
        let mut response = None;
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(boxes.border),
            inherited,
            None,
            |ui| {
                let own = ui.interact(boxes.border, id, sense);
                if let Some(plan) = node.scroll.as_ref().filter(|_| !node.senses()) {
                    let role = plan.description.role.unwrap_or(Role::ScrollView);
                    let name = plan.description.name.as_deref().unwrap_or_default();
                    crate::owned::describe(&own, ui.is_enabled(), role, None, name);
                } else if let Some(description) = node.description() {
                    let role = description.role.unwrap_or(Role::GenericContainer);
                    let name = description.name.as_deref().unwrap_or_default();
                    crate::owned::describe(&own, ui.is_enabled(), role, None, name);
                }
                response = Some(own);
            },
        );
        response.expect("node_ui runs its contents")
    }

    /// Registers what the node does to the pointer that reaches nodes beneath
    /// it. A node that blocks it (see [`PlacedNode::blocks_pointer`]) takes
    /// it from the nodes registered before it that it covers, and a node that
    /// takes the pointer itself is one a later blocker may cover.
    ///
    /// Only what can be covered gets a blocker: a node's ancestors are not
    /// beneath it, and a painted card in a scroll container covers nothing, so
    /// dragging the card still scrolls. The root of an overlay always blocks,
    /// since what lies beneath it is another layer.
    fn cover(&mut self, index: usize, boxes: &Boxes, inherited: Inherited) {
        let node = &self.nodes[index];
        let reach = node.rect().intersect(inherited.clip);
        let covers = || {
            reach.is_positive()
                && self.shared.beneath.iter().any(|other| {
                    let ancestor = other.index < index && index < other.end;
                    let overlap = other.reach.intersect(reach);
                    !ancestor && overlap.width() > TOUCH && overlap.height() > TOUCH
                })
        };
        let blocks = inherited.pointer_events == PointerEvents::Auto && node.blocks_pointer();
        if blocks && (node.kind() == NodeKind::Overlay || covers()) {
            self.block(index, boxes, inherited);
        }
        if node.senses() || node.draws_widgets() {
            self.shared.beneath.push(Beneath {
                index,
                end: node.end,
                reach,
            });
        }
    }

    /// The node's blocker: a widget over its border box that takes clicks and
    /// drags, registered before the node's descendants so that they win.
    fn block(&mut self, index: usize, boxes: &Boxes, inherited: Inherited) {
        let node = &self.nodes[index];
        let id = crate::key::root_id().with("blocker").with(index);
        let overlay = self
            .roots
            .iter()
            .find(|root| root.first == index)
            .and_then(PlacedRoot::overlay);
        let modal = overlay.is_some_and(|overlay| matches!(overlay.form, Form::Modal(_)));
        let described = overlay
            .map(|overlay| &overlay.description)
            .filter(|description| description.is_set());
        node_ui(
            self.ui,
            UiBuilder::new().max_rect(boxes.border),
            inherited,
            None,
            |ui| {
                let response = ui.interact(boxes.border, id, Sense::CLICK | Sense::DRAG);
                if let Some(description) = described {
                    let role = description.role.unwrap_or(Role::GenericContainer);
                    let name = description.name.as_deref().unwrap_or_default();
                    crate::owned::describe(&response, ui.is_enabled(), role, None, name);
                }
                if modal {
                    ui.ctx()
                        .accesskit_node_builder(response.id, |node| node.set_modal());
                }
            },
        );
        debug_assert!(node.blocks_pointer());
    }

    /// What the node's one hit test senses, if it gets one: what its classes
    /// declare, or, for a scroll container with nothing focusable inside, a
    /// place in the Tab order.
    fn sense_of(&self, index: usize) -> Option<Sense> {
        let node = &self.nodes[index];
        if node.senses() {
            let interaction = &node.interaction;
            return Some(interaction.focusability.sense(interaction.sensing));
        }
        if self.scroller_takes_focus(index) {
            return Some(Sense::focusable_noninteractive());
        }
        let plan = node.scroll.as_ref()?;
        plan.description.is_set().then(Sense::hover)
    }

    /// Whether the scroll container at `index` is a Tab stop in this layout.
    fn scroller_takes_focus(&self, index: usize) -> bool {
        let node = &self.nodes[index];
        let Some(viewport) = node.scroll_viewport() else {
            return false;
        };
        node.scroll
            .as_ref()
            .is_some_and(|plan| plan.is_tab_stop(viewport.size()))
            && !node.is_inert()
    }

    /// Hovering the node or any sensing descendant hovers the node, as
    /// `:hover` does in CSS. An item of a listbox is hovered only by its
    /// highlight, which the pointer moves (see [`nav`]): the pointer and the
    /// keys never leave two items looking hovered.
    fn hovering(&self, index: usize, own: &Response) -> Hovering {
        let ctx = self.ui.ctx();
        let node = &self.nodes[index];
        let descendants = &self.nodes[index + 1..node.end];
        let highlighted = node.id().is_some_and(|id| {
            self.shared.highlighted.contains(&id) || (node.is_item() && focus::holds_focus(ctx, id))
        });
        let pointed = own.hovered()
            || descendants
                .iter()
                .filter(|descendant| descendant.senses())
                .filter_map(PlacedNode::id)
                .any(|id| ctx.read_response(id).is_some_and(|r| r.hovered()));
        let in_listbox = node.is_item()
            && self.roots[root_of(self.roots, index)]
                .overlay()
                .is_some_and(|overlay| overlay.behavior.nav == Nav::Listbox);
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
                State::DragAccepts => stakes.accepts,
                State::DragOver => stakes.over,
                State::DragLifted => stakes.lifted,
            };
            let target = if on { Progress::END } else { Progress::START };
            match (variants.transition, node.id()) {
                (Some(duration), Some(id)) => transition::animate(
                    self.ui.ctx(),
                    id.with(("eggshell-state", state as u8)),
                    target,
                    duration,
                    variants.easing,
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
        if parent.culling == Culling::On && self.culled(index, parent.clip) {
            if self.shared.recording.is_some() {
                let left_out = self.nodes[index..self.nodes[index].end]
                    .iter()
                    .filter_map(PlacedNode::id);
                self.shared.culled.extend(left_out);
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
        let before = self.nodes[index]
            .id()
            .map(|id| (id, focus::registered(self.ui.ctx(), layer)));
        visit(self);
        if let Some((id, from)) = before {
            let widgets = focus::focusable_since(self.ui.ctx(), layer, from);
            self.shared.subtrees.push((id, widgets));
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
        let node = &self.nodes[index];
        if node.flow == Flow::Hidden || self.registrants[node.end] == self.registrants[index] {
            return;
        }
        let inertness = parent.inertness.or(node.is_inert().into());
        let hidden = Inherited {
            inertness,
            opacity: Opacity::TRANSPARENT,
            ..parent
        };
        self.noting(index, |walk| {
            let boxes = Boxes::of(node, walk.grid);
            if let Some(sense) = walk.sense_of(index) {
                walk.interact(node, sense, &boxes, hidden);
            }
            if node.draws_widgets() {
                walk.run_draw(index, &boxes, hidden, node.text());
            }
            for child in walk.in_paint_order(index) {
                walk.register_culled(child, hidden);
            }
        });
    }

    /// The children of the node at `index`, bottom first.
    fn in_paint_order(&self, index: usize) -> Vec<usize> {
        let mut children = children_of(self.nodes, index);
        children.sort_by_key(|&child| self.nodes[child].z());
        children
    }

    /// Whether the node and its subtree can be left out: nothing of it
    /// reaches `clip`, no slide in flight can bring it there, and it holds
    /// neither the keyboard focus nor the drag.
    fn culled(&self, index: usize, clip: Rect) -> bool {
        let node = &self.nodes[index];
        let within = index..node.end;
        let ctx = self.ui.ctx();
        node.flow != Flow::Hidden
            && !node
                .reach
                .intersects(clip.expand(self.shared.animation.slide_reach()))
            && !self.drag.involves(&within)
            && !(ctx.memory(|memory| memory.focused()).is_some()
                && self.nodes[within]
                    .iter()
                    .filter_map(PlacedNode::id)
                    .any(|id| focus::focus_within(ctx, id)))
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

        let pointer_events = parent.pointer_events.merge(node.pointer_events());
        self.cover(
            index,
            &boxes,
            Inherited {
                inertness,
                pointer_events,
                ..parent
            },
        );

        let response = self.sense_of(index).map(|sense| {
            self.interact(
                node,
                sense,
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

        let opacity = parent.opacity * look.paint.opacity;
        let lifted = self.lift.filter(|lift| lift.index == index);
        let slide = match (interaction.variants.animate_layout, node.id(), lifted) {
            (Some(duration), Some(id), None) => self.shared.animation.offset(
                id,
                node.rect().min - parent.origin,
                duration,
                interaction.variants.easing,
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
                let translate = look.translate + slide;
                (translate, parent.shift + translate)
            }
        };
        if let (Some(frame), Some(id)) = (&mut self.shared.recording, node.id()) {
            frame.push(NodeFrame {
                id,
                path: node.path().to_owned(),
                layout_rect: node.rect(),
                painted_rect: node.rect().translate(shift),
                ancestors: if lifted.is_some() {
                    Vec2::ZERO
                } else {
                    parent.shift
                },
                scrolled: parent.scrolled,
                opacity,
                motion: match (interaction.variants.animate_layout, lifted) {
                    (Some(_), None) => Motion::Slides,
                    _ => Motion::Snaps,
                },
                slide: self
                    .shared
                    .animation
                    .slide(id, interaction.variants.easing, translate),
            });
        }
        let moved_from = (translate != Vec2::ZERO || lifted.is_some()).then(|| self.next_shape());
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
            // A lifted node is painted where the pointer carries it, not where
            // layout put it.
            culling: match lifted {
                Some(_) => Culling::Off,
                None => parent.culling,
            },
            backdrop: parent.backdrop.blend(painted_background),
            clip: content_clip(parent.clip, boxes.padding, node.overflow),
            opacity,
            inertness,
            pointer_events,
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
        if let Some(prim) = self.shared.prims[index].take() {
            self.paint_prim(prim, index, node, &text, &boxes, inherited);
        }
        self.run_draw(index, &boxes, inherited, &text);

        if node.scroll.is_some() {
            self.scroll_content(index, &boxes, inherited);
        } else {
            let in_layout_order = children_of(self.nodes, index);
            for child in self.in_paint_order(index) {
                self.visit(child, inherited);
            }
            self.paint_dividers(index, &in_layout_order, inherited);
        }
        if self.scroller_takes_focus(index)
            && !interaction.variants.has_focus()
            && response.as_ref().is_some_and(Response::has_focus)
        {
            let ring = Style::of(self.ui.ctx()).focus;
            self.painter(parent.clip, opacity).add(RectShape::stroke(
                boxes.border,
                look.paint.radius,
                Stroke::new(ring.width, ring.color),
                StrokeKind::Inside,
            ));
        }
        if let Some(first) = moved_from {
            self.translate_shapes_since(first, translate);
            if lifted.is_some() {
                self.lift_shapes_since(first);
            }
        }
    }

    /// Runs the node's draw closure in its own `Ui`, if it has one.
    fn run_draw(&mut self, index: usize, boxes: &Boxes, inherited: Inherited, text: &TextStyle) {
        let node = &self.nodes[index];
        if let Some(draw) = node.draw.and_then(|slot| self.shared.draws[slot].take()) {
            let id = node.id().expect("a node with a draw closure is identified");
            let builder = UiBuilder::new()
                .max_rect(boxes.content)
                .scope_id(id.with(DRAW_SCOPE));
            let app = &mut *self.app;
            node_ui(self.ui, builder, inherited, Some(text), |ui| {
                let scope = ui.unique_id();
                ui.data_mut(|data| data.insert_temp(scope, NodeIdTag(id)));
                draw(app, ui)
            });
        }
    }

    /// Shows the content of the scroll container at `index` in an egui
    /// `ScrollArea` over its viewport: egui owns the offset, the wheel and
    /// the bars, and the content, laid out at that offset, is walked inside
    /// it with the nodes outside the viewport left out.
    fn scroll_content(&mut self, index: usize, boxes: &Boxes, inherited: Inherited) {
        let nodes = self.nodes;
        let node = &nodes[index];
        let plan = node.scroll.as_ref().expect("a scroll container has a plan");
        let id = node.id().expect("a scroll container is identified");
        let area = egui::ScrollArea::new(plan.overflows(boxes.padding.size() - plan.bars.gutter))
            .id_salt(scroll::AREA_SALT)
            .auto_shrink(false)
            // egui inflates a viewport smaller than 64 points to 64 and clamps
            // the offset to that, so a short container stopped short of its end
            // and then drifted back.
            .min_scrolled_width(0.0)
            .min_scrolled_height(0.0)
            .content_margin(egui::Margin::ZERO)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
        let area = match plan.moved {
            Moved::Yes => area.scroll_offset(plan.offset),
            Moved::No => area,
        };
        let Self {
            app,
            ui,
            ordinals,
            registrants,
            shared,
            grid,
            selection,
            lift,
            drag,
            roots,
            ..
        } = self;
        // The viewport clips the content, whatever egui reserves for its bars.
        let viewport = node
            .scroll_viewport()
            .expect("a scroll container has a plan");
        let content = Inherited {
            clip: inherited.clip.intersect(grid.rect(viewport)),
            culling: Culling::On,
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
                ui.spacing_mut().scroll = plan.bars.egui(&outer.spacing.scroll);
                plan.bars.paint_in(ui.visuals_mut());
                area.show_viewport(ui, |ui, _| {
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
                        grid: *grid,
                        selection: *selection,
                        lift: *lift,
                        drag,
                        roots,
                    }
                    .visit(index + 1, content);
                });
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
    fn translate_shapes_since(&self, first: ShapeIdx, by: Vec2) {
        let layer = self.ui.layer_id();
        self.ui.ctx().graphics_mut(|graphics| {
            let list = graphics.entry(layer);
            let end = list.next_idx();
            list.transform_range(first, end, TSTransform::from_translation(by));
        });
    }

    fn paint_prim(
        &mut self,
        prim: Prim,
        index: usize,
        node: &PlacedNode,
        text: &TextStyle,
        boxes: &Boxes,
        inherited: Inherited,
    ) {
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
                let top = node
                    .text_valign()
                    .offset(content.height() - galley.size().y);
                let anchor: Pos2 = match text.align {
                    TextAlign::Left => content.left_top(),
                    TextAlign::Center => content.center_top(),
                    TextAlign::Right => content.right_top(),
                } + vec2(0.0, top);
                if text.select.selects(inherited.interactive) {
                    let id = self.selection_id(index, node);
                    self.select_text(id, galley, anchor, content, text, inherited);
                } else {
                    self.painter(inherited.clip, inherited.opacity)
                        .galley(anchor, galley, text.color);
                }
            }
            Prim::Icon(icon) => self.paint_image(image_for(icon, text.color), content, inherited),
            Prim::Image(src) => self.paint_image(Image::new(src), content, inherited),
            Prim::Paint(paint) => paint(
                &self.painter(inherited.clip, inherited.opacity),
                content,
                text.color,
            ),
        }
    }

    /// Paints `galley` through egui's label selection, which draws the
    /// highlight, follows drags across labels and copies on Ctrl+C. The
    /// galley and its position are the ones plain painting uses, so layout
    /// and pixels are the same without a selection. The whole content box
    /// takes the pointer, so a drag may end in the blank beside the text.
    fn select_text(
        &mut self,
        id: Id,
        galley: Arc<Galley>,
        anchor: Pos2,
        content: Rect,
        text: &TextStyle,
        inherited: Inherited,
    ) {
        let selection = self.selection;
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
                let response = ui.interact(content, id, Sense::click_and_drag() - Sense::FOCUSABLE);
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
            },
        );
    }

    /// A stable id for the node's selectable text: its key path, and the
    /// order among the text nodes that share one. The order counts every
    /// node of the frame, so culling some of them does not renumber the rest.
    fn selection_id(&self, index: usize, node: &PlacedNode) -> Id {
        self.ui
            .scope_id()
            .with(("select", node.path()))
            .with(self.ordinals[index])
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

impl<A> Walk<'_, A> {
    /// Draws every attached overlay, in the order they were declared, each in
    /// a layer of its own, and returns the ones drawn in the order they were
    /// opened. Walks `self` from the page's `Ui`.
    fn overlays(&mut self) -> Vec<Shown> {
        let ctx = self.ui.ctx().clone();
        let screen = ctx.content_rect();
        let mut shown = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            let Some(overlay) = root.overlay() else {
                continue;
            };
            if overlay.anchored == Anchored::Detached {
                continue;
            }
            let root_rect = self.nodes[root.first].rect();
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
                        interactable: true,
                        last_became_visible_at: None,
                    },
                );
            });
            self.shared.beneath.clear();
            egui::Area::new(layer.id)
                .order(layer.order)
                .fixed_pos(rect.min)
                .constrain(false)
                .movable(false)
                .interactable(overlay.band != Band::Tooltip)
                .fade_in(false)
                .show(&ctx, |ui| {
                    ui.allocate_space(rect.size());
                    if swallows {
                        ui.interact(rect, overlay.id.with("swallow"), Sense::CLICK | Sense::DRAG);
                    }
                    self.in_layer(ui)
                        .visit(root.first, Inherited::root(screen, root_rect.min));
                });
            shown.push((
                overlay.stack_index,
                Shown {
                    id: overlay.id,
                    band: overlay.band,
                },
            ));
        }
        if let Some(top) = stacked_top_down(self.roots)
            .into_iter()
            .filter_map(|index| self.roots[index].overlay())
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
    pub fn draw(self, app: &mut A, ui: &mut Ui) {
        let timer = Timer::start(ui.ctx(), Phase::Draw);
        let _anchor_focus = focus::AnchorFocusHeld::new(
            ui.ctx(),
            overlay_input::pointer_on(ui.ctx(), &self.nodes, &self.roots),
        );
        let pressed = press_origin(ui.ctx())
            .map(|at| self.root_at(at))
            .and_then(|root| Some(self.roots[root].overlay()?.id));
        let Self {
            ctx: _,
            stack: scope,
            nodes,
            prims,
            draws,
            by_id,
            mut drag,
            mut dismiss,
            roots,
        } = self;
        let overflow = (Debug::of(ui.ctx()) == Debug::Outlines)
            .then(|| debug::overflow(ui.ctx(), &nodes, &prims));
        let grid = PixelGrid {
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let root = Inherited::root(ui.clip_rect(), nodes[0].rect().min);
        let live = dnd::drag_state(ui.ctx());
        let (dragging, finish) = dnd::begin(
            ui.ctx(),
            ui.scope_id(),
            dnd::Tree {
                nodes: &nodes,
                by_id: &by_id,
                roots: &roots,
            },
            ui.clip_rect(),
            &mut drag,
            app,
        );
        let selection = Style::of(ui.ctx()).selection;
        let mut animation = LayoutAnimation::begin(ui.ctx(), ui.scope_id(), nodes[0].rect());
        if nodes.iter().any(|node| node.scroll.is_some()) {
            animation.anticipate(&nodes);
        }
        let recording = continuity::recording(ui.ctx()).then(Vec::new);
        let ordinals = text_ordinals(&nodes, &prims);
        let registrants = registrant_counts(&nodes);
        let mut shared = Shared {
            prims,
            draws,
            subtrees: Vec::new(),
            animation,
            recording,
            culled: Vec::new(),
            beneath: Vec::new(),
            highlighted: nav::highlighted(ui.ctx()),
        };
        nav::activate_highlighted(ui.ctx());
        nav::apply_deferred(ui.ctx());
        let keys = overlay_input::claim_keys(ui.ctx(), &nodes, &roots, live);
        scroll_keys::hold_arrows(ui.ctx(), &nodes);
        let mut walk = Walk {
            app,
            ui,
            nodes: &nodes,
            ordinals: &ordinals,
            registrants: &registrants,
            shared: &mut shared,
            grid,
            selection,
            lift: dragging.lift,
            drag: &dragging,
            roots: &roots,
        };
        walk.visit(0, root);
        let ctx = walk.ui.ctx().clone();
        let shown = walk.overlays();
        stack::repair(&ctx, &shown, pressed);
        overlay_input::judge(
            overlay_input::Frame {
                ctx: &ctx,
                scope,
                nodes: &nodes,
                roots: &roots,
                subtrees: &shared.subtrees,
                drag: live,
                keys,
            },
            &mut dismiss,
            app,
        );
        finish.finish(&ctx, &nodes, &mut drag, app);
        scroll_keys::note_press(&ctx, &nodes);
        scroll_keys::apply_keys(&ctx, &nodes);
        scroll_keys::follow_focus(&ctx, &shared.subtrees);
        if let Some(frame) = shared.recording.take() {
            let laid_out = nodes
                .iter()
                .filter(|node| node.id().is_some())
                .map(|node| (node.path().to_owned(), node.rect()))
                .collect();
            continuity::store(
                &ctx,
                RecordedPass {
                    pass: ctx.cumulative_pass_nr(),
                    nodes: frame,
                    laid_out,
                    culled: std::mem::take(&mut shared.culled),
                },
            );
        }
        focus::store(&ctx, shared.subtrees);
        shared.animation.finish();
        drop(timer);
        if let Some(overflow) = overflow {
            debug::paint(&ctx, &nodes, &overflow);
        }
    }
}

/// For each node, and one past the last, how many nodes before it register
/// widgets while culled.
fn registrant_counts(nodes: &[PlacedNode]) -> Vec<u32> {
    let mut counts = Vec::with_capacity(nodes.len() + 1);
    let mut count = 0;
    counts.push(count);
    for node in nodes {
        count += u32::from(node.registers());
        counts.push(count);
    }
    counts
}

/// For each node, how many text nodes with its path come before it: the
/// order that tells apart the text nodes that share a path.
fn text_ordinals(nodes: &[PlacedNode], prims: &[Option<Prim>]) -> Vec<u32> {
    let mut seen: HashMap<&str, u32> = HashMap::new();
    nodes
        .iter()
        .zip(prims)
        .map(|(node, prim)| match prim {
            Some(Prim::Text(_)) => {
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

    /// Declares the node the visible label `text` of the control with id
    /// `control`, another node of the same frame: assistive technology names
    /// the control by it. Returns whether the label was clicked, for the
    /// caller to forward to the control, as clicking a `<label>` does.
    ///
    /// The node must sense clicks, and is best left out of the Tab order
    /// (`focusable_none`), as the control is the one to focus.
    fn label_for(&self, control: egui::Id, text: &str) -> bool;
}

impl NodeUi for Ui {
    fn node_id(&self) -> egui::Id {
        let scope = self.unique_id();
        self.data(|data| data.get_temp::<NodeIdTag>(scope))
            .map(|tag| tag.0)
            .expect("node_response and node_id are called on the Ui a draw closure is given")
    }

    fn has_focus_within(&self) -> bool {
        focus::focus_within(self.ctx(), self.node_id())
    }

    fn label_for(&self, control: egui::Id, text: &str) -> bool {
        let response = self.node_response();
        response.widget_info(|| WidgetInfo::labeled(Role::Label, self.is_enabled(), text));
        self.ctx().accesskit_node_builder(control, |node| {
            node.clear_label();
            node.push_labelled_by(response.id.accesskit_id());
        });
        response.clicked()
    }

    fn node_response(&self) -> Response {
        let id = self.node_id();
        self.ctx()
            .read_response(id)
            .unwrap_or_else(|| self.interact(self.max_rect(), id, Sense::hover()))
    }
}
