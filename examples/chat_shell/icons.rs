//! Small line icons, drawn with the painter so the example needs no assets.

use egui::{Color32, Painter, Rect, Stroke, pos2, vec2};

#[derive(Clone, Copy, Debug)]
pub enum Glyph {
    Plus,
    Search,
    Folder,
    Cube,
    Sliders,
    Dots,
    Chevron,
    ArrowUp,
}

fn stroke(color: Color32) -> Stroke {
    Stroke::new(1.5, color)
}

/// A paint function for [`eggshell_ui::Builder::paint`] that draws `glyph` in `color`.
pub fn glyph(glyph: Glyph, color: Color32) -> impl Fn(&Painter, Rect, Color32) + 'static {
    move |painter, rect, _| {
        let c = rect.center();
        let r = rect.width().min(rect.height()) * 0.5 * 0.7;
        let s = stroke(color);
        match glyph {
            Glyph::Plus => {
                painter.line_segment([c - vec2(r, 0.0), c + vec2(r, 0.0)], s);
                painter.line_segment([c - vec2(0.0, r), c + vec2(0.0, r)], s);
            }
            Glyph::Search => {
                painter.circle_stroke(c - vec2(r * 0.15, r * 0.15), r * 0.7, s);
                painter.line_segment([c + vec2(r * 0.35, r * 0.35), c + vec2(r, r)], s);
            }
            Glyph::Folder => {
                let body = Rect::from_center_size(c + vec2(0.0, r * 0.1), vec2(2.0 * r, 1.5 * r));
                painter.rect_stroke(body, 2.0, s, egui::StrokeKind::Middle);
                painter.line_segment(
                    [
                        body.left_top() + vec2(0.0, -r * 0.3),
                        body.left_top() + vec2(r * 0.8, -r * 0.3),
                    ],
                    s,
                );
            }
            Glyph::Cube => {
                painter.rect_stroke(
                    Rect::from_center_size(c, vec2(1.6 * r, 1.6 * r)),
                    2.0,
                    s,
                    egui::StrokeKind::Middle,
                );
            }
            Glyph::Sliders => {
                for (dy, knob) in [(-0.6, -0.4), (0.0, 0.4), (0.6, -0.1)] {
                    let y = c.y + dy * r;
                    painter.line_segment([pos2(c.x - r, y), pos2(c.x + r, y)], s);
                    painter.circle_filled(pos2(c.x + knob * r, y), 2.0, color);
                }
            }
            Glyph::Dots => {
                for dx in [-0.8, 0.0, 0.8] {
                    painter.circle_filled(c + vec2(dx * r, 0.0), 1.5, color);
                }
            }
            Glyph::Chevron => {
                let h = r * 0.5;
                painter.line_segment([c + vec2(-h, -h * 0.5), c + vec2(0.0, h * 0.5)], s);
                painter.line_segment([c + vec2(0.0, h * 0.5), c + vec2(h, -h * 0.5)], s);
            }
            Glyph::ArrowUp => {
                painter.line_segment([c + vec2(0.0, r), c - vec2(0.0, r)], s);
                painter.line_segment([c - vec2(0.0, r), c + vec2(-r * 0.7, -r * 0.2)], s);
                painter.line_segment([c - vec2(0.0, r), c + vec2(r * 0.7, -r * 0.2)], s);
            }
        }
    }
}
