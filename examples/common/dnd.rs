//! What the drag-and-drop spike examples share (#71): the drag image and the
//! 2D insertion rule.

use egui::emath::TSTransform;
use egui::epaint::Shape;
use egui::{Color32, Context, Id, LayerId, Order, Pos2, Rect, Vec2};

/// The shapes a node painted, relative to the top-left of its rect.
pub struct Captured {
    pub shapes: Vec<Shape>,
    pub size: Vec2,
}

/// Copies the shapes painted inside `rect` on `layer`: the stand-in for the
/// per-node shape record that #35 would keep.
pub fn capture(ctx: &Context, layer: LayerId, rect: Rect) -> Captured {
    let shapes = ctx.graphics(|g| {
        g.get(layer)
            .map(|list| {
                list.all_entries()
                    .filter(|clipped| rect.contains_rect(clipped.shape.visual_bounding_rect()))
                    .map(|clipped| {
                        let mut shape = clipped.shape.clone();
                        shape.transform(TSTransform::from_translation(-rect.min.to_vec2()));
                        shape
                    })
                    .collect()
            })
            .unwrap_or_default()
    });
    Captured {
        shapes,
        size: rect.size(),
    }
}

/// The layer above every other, where the drag image goes.
pub fn image_layer() -> LayerId {
    LayerId::new(Order::Tooltip, Id::NULL.with("dnd-image"))
}

/// Paints captured shapes centred on `pos`.
pub fn paint_ghost(ctx: &Context, captured: &Captured, pos: Pos2) {
    let painter = ctx.layer_painter(image_layer());
    let delta = (pos - captured.size / 2.0).to_vec2();
    for shape in &captured.shapes {
        let mut shape = shape.clone();
        shape.transform(TSTransform::from_translation(delta));
        painter.add(shape);
    }
}

/// Paints a small text chip centred on `pos`.
pub fn paint_chip(ctx: &Context, text: &str, pos: Pos2) {
    let painter = ctx.layer_painter(image_layer());
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );
    let rect = Rect::from_center_size(pos, galley.size() + egui::vec2(16.0, 8.0));
    painter.rect_filled(rect, 6.0, Color32::from_black_alpha(200));
    painter.galley(rect.min + egui::vec2(8.0, 4.0), galley, Color32::WHITE);
}

/// Where a payload dropped at `pos` goes among items laid out in reading
/// order (rows of items wrapping): the index of the first item the pointer is
/// before. An item is "before" the pointer's row if it starts below `pos`, or
/// it is in the pointer's row (its vertical span holds `pos.y`, or `pos.y`
/// lies in the gap above it) and `pos` is left of its centre.
///
/// `rects` are the items' final layout rects, the dragged item excluded.
pub fn insertion_index(rects: &[Rect], pos: Pos2) -> usize {
    rects
        .iter()
        .position(|r| pos.y < r.min.y || (pos.y <= r.max.y && pos.x < r.center().x))
        .unwrap_or(rects.len())
}
