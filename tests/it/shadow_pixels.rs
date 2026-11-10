//! A shadow under a translucent rounded box, rasterised on the CPU from the
//! tessellated shapes, so no GPU is needed to look at actual pixels.

use bullseye::tw;
use egui::epaint::{ClippedPrimitive, Primitive, TessellationOptions, Tessellator, Vertex};
use egui::{Color32, Pos2, Rect, Shadow, pos2, vec2};

use super::draw::{SCREEN, shapes};

const GROUND: [f32; 4] = [230.0, 230.0, 230.0, 255.0];
const RADIUS: f32 = 14.0;

struct Canvas {
    pixels_per_point: f32,
    width: usize,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    fn at(&self, point: Pos2) -> [f32; 4] {
        let x = (point.x * self.pixels_per_point).floor() as usize;
        let y = (point.y * self.pixels_per_point).floor() as usize;
        self.pixels[y * self.width + x]
    }

    fn paint(&mut self, primitives: &[ClippedPrimitive]) {
        let ppp = self.pixels_per_point;
        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in primitives
        {
            let Primitive::Mesh(mesh) = primitive else {
                continue;
            };
            for tri in mesh.indices.as_chunks::<3>().0 {
                let mut v = tri.map(|index| mesh.vertices[index as usize]);
                // A GPU rasterises vertices snapped to a sub-pixel grid, which is what
                // makes two triangles sharing an edge agree on which pixels it touches.
                let snapped = |v: &Vertex| {
                    [v.pos.x, v.pos.y].map(|c| (f64::from(c * ppp) * 256.0).round() / 256.0)
                };
                let edge = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
                    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
                };
                if edge(snapped(&v[0]), snapped(&v[1]), snapped(&v[2])) < 0.0 {
                    v.swap(1, 2);
                }
                let at = v.each_ref().map(snapped);
                let area = edge(at[0], at[1], at[2]);
                if area == 0.0 {
                    continue;
                }
                // Pixels exactly on a shared edge belong to one triangle only, as on a GPU.
                let owns_edge =
                    |a: [f64; 2], b: [f64; 2]| b[1] < a[1] || (b[1] == a[1] && b[0] > a[0]);
                let bounds = Rect::from_points(&v.map(|v| v.pos)).intersect(*clip_rect);
                let (x0, x1) = (
                    (bounds.min.x * ppp).floor() as i64,
                    (bounds.max.x * ppp).ceil() as i64,
                );
                let (y0, y1) = (
                    (bounds.min.y * ppp).floor() as i64,
                    (bounds.max.y * ppp).ceil() as i64,
                );
                for y in y0.max(0)..y1 {
                    for x in x0.max(0)..x1 {
                        let centre = pos2((x as f32 + 0.5) / ppp, (y as f32 + 0.5) / ppp);
                        if !clip_rect.contains(centre) {
                            continue;
                        }
                        let p = [x as f64 + 0.5, y as f64 + 0.5];
                        let w = [
                            edge(at[1], at[2], p) / area,
                            edge(at[2], at[0], p) / area,
                            edge(at[0], at[1], p) / area,
                        ];
                        let covered = (0..3).all(|k| {
                            w[k] > 0.0
                                || (w[k] == 0.0 && owns_edge(at[(k + 1) % 3], at[(k + 2) % 3]))
                        });
                        if !covered {
                            continue;
                        }
                        let mut source = [0.0; 4];
                        for (vertex, weight) in v.iter().zip(w) {
                            let c = vertex.color.to_array();
                            for k in 0..4 {
                                source[k] += weight as f32 * f32::from(c[k]);
                            }
                        }
                        let dst = &mut self.pixels[y as usize * self.width + x as usize];
                        for k in 0..4 {
                            dst[k] = source[k] + dst[k] * (1.0 - source[3] / 255.0);
                        }
                    }
                }
            }
        }
    }
}

struct Scene {
    canvas: Canvas,
    border: Rect,
}

fn render(background: Color32, shadow: Option<Shadow>, pixels_per_point: f32) -> Scene {
    let shapes = shapes(&mut (), pixels_per_point, |b| {
        b.div(tw().p(10.0), |b| {
            let look = tw()
                .w_px(100.0)
                .h_px(50.0)
                .bg(background)
                .rounded_px(RADIUS);
            b.div(shadow.map_or(look.clone(), |s| look.shadow(s)), |_| {});
        });
    });
    let border = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.fill == background => Some(rect.rect),
            _ => None,
        })
        .expect("the box background is painted");
    let primitives = Tessellator::new(
        pixels_per_point,
        TessellationOptions::default(),
        [1, 1],
        Vec::new(),
    )
    .tessellate_shapes(shapes);
    let width = (SCREEN.width() * pixels_per_point) as usize;
    let height = (SCREEN.height() * pixels_per_point) as usize;
    let mut canvas = Canvas {
        pixels_per_point,
        width,
        pixels: vec![GROUND; width * height],
    };
    canvas.paint(&primitives);
    Scene { canvas, border }
}

/// Signed distance from `point` to the rounded border box; negative inside.
fn distance_to_shape(border: Rect, point: Pos2) -> f32 {
    let half = border.size() / 2.0;
    let q = (point - border.center()).abs() - half + vec2(RADIUS, RADIUS);
    q.max(vec2(0.0, 0.0)).length() + q.x.max(q.y).min(0.0) - RADIUS
}

fn shadow() -> Shadow {
    Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(120),
    }
}

fn translucent() -> Color32 {
    Color32::from_white_alpha(90)
}

fn opaque() -> Color32 {
    Color32::from_gray(230)
}

fn darker_than_ground(pixel: [f32; 4]) -> f32 {
    GROUND[0] - pixel[0]
}

fn assert_close(a: [f32; 4], b: [f32; 4], tolerance: f32, context: &str) {
    for k in 0..4 {
        assert!(
            (a[k] - b[k]).abs() <= tolerance,
            "{context}: {a:?} vs {b:?}"
        );
    }
}

/// ```html
/// <div class="p-10"><div class="w-[100px] h-[50px] rounded-[14px] bg-white/35 shadow-[0_8px_24px_#00000078]"></div></div>
/// ```
#[test]
fn the_shadow_fills_the_corner_cut_outs_of_a_translucent_box() {
    for ppp in [1.0, 2.0] {
        let glass = render(translucent(), Some(shadow()), ppp);
        let solid = render(opaque(), Some(shadow()), ppp);
        let border = glass.border;
        let inset = RADIUS * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
        let near = |left: bool, depth: f32| {
            let x = if left {
                border.min.x + depth
            } else {
                border.max.x - depth
            };
            pos2(x, border.max.y - depth)
        };
        for left in [true, false] {
            for depth in [inset - 1.0, 2.0, 1.0] {
                let point = near(left, depth);
                assert!(border.contains(point), "inside the rect");
                assert!(
                    distance_to_shape(border, point) > 0.5,
                    "outside the rounded shape at {point:?}"
                );
                let pixel = glass.canvas.at(point);
                assert!(
                    darker_than_ground(pixel) > 20.0,
                    "ppp {ppp}: shadow at {point:?}: {pixel:?}"
                );
                assert_close(
                    pixel,
                    solid.canvas.at(point),
                    2.0,
                    &format!("ppp {ppp} {point:?}"),
                );
            }
        }
    }
}

#[test]
fn the_shadow_stays_out_of_the_rounded_shape() {
    for ppp in [1.0, 2.0] {
        let glass = render(translucent(), Some(shadow()), ppp);
        let bare = render(translucent(), None, ppp);
        let border = glass.border;
        let point = pos2(border.min.x + RADIUS - 8.0, border.max.y - RADIUS + 8.0);
        assert!(
            distance_to_shape(border, point) < -2.0,
            "inside the rounded shape"
        );
        assert_close(glass.canvas.at(point), bare.canvas.at(point), 0.5, "inside");
    }
}

#[test]
fn no_seam_where_a_strip_meets_a_corner_region() {
    for ppp in [1.0, 2.0] {
        let glass = render(translucent(), Some(shadow()), ppp);
        let bare = render(translucent(), None, ppp);
        let solid = render(opaque(), Some(shadow()), ppp);
        let border = glass.border;
        let step = 1.0 / ppp;
        let mut outside = 0;
        let mut inside = 0;
        let mut y = border.min.y - 40.0;
        while y < border.max.y + 40.0 {
            let mut x = border.min.x - 40.0;
            while x < border.max.x + 40.0 {
                let p = pos2(x + step / 2.0, y + step / 2.0);
                let d = distance_to_shape(border, p);
                if d > 1.5 {
                    outside += 1;
                    assert_close(
                        glass.canvas.at(p),
                        solid.canvas.at(p),
                        2.0,
                        &format!("ppp {ppp} outside {p:?}"),
                    );
                } else if d < -1.5 {
                    inside += 1;
                    assert_close(
                        glass.canvas.at(p),
                        bare.canvas.at(p),
                        0.5,
                        &format!("ppp {ppp} inside {p:?}"),
                    );
                }
                x += step;
            }
            y += step;
        }
        assert!(
            outside > 1000 && inside > 1000,
            "precondition: both regions are sampled"
        );
    }
}
