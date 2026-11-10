//! Geometry for painting a shadow outside a rounded border box.
//!
//! A rect clip cannot express "outside a rounded rect". The shadow is painted
//! in four rect strips around the border box, and in each rounded corner the
//! region between the rect's corner and the arc is cut out of the tessellated
//! shadow mesh as a fan of triangles.

use egui::epaint::{Mesh, Vertex};
use egui::{CornerRadius, Pos2, Rect, pos2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Corner {
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

impl Corner {
    pub(crate) const ALL: [Self; 4] = [
        Self::NorthWest,
        Self::NorthEast,
        Self::SouthWest,
        Self::SouthEast,
    ];

    pub(crate) fn radius(self, radius: CornerRadius) -> u8 {
        match self {
            Self::NorthWest => radius.nw,
            Self::NorthEast => radius.ne,
            Self::SouthWest => radius.sw,
            Self::SouthEast => radius.se,
        }
    }

    /// The rect's corner, and the directions along its two edges into the rect.
    fn apex(self, rect: Rect) -> (Pos2, f32, f32) {
        match self {
            Self::NorthWest => (rect.min, 1.0, 1.0),
            Self::NorthEast => (pos2(rect.max.x, rect.min.y), -1.0, 1.0),
            Self::SouthWest => (pos2(rect.min.x, rect.max.y), 1.0, -1.0),
            Self::SouthEast => (rect.max, -1.0, -1.0),
        }
    }
}

/// How many fan segments make a corner arc whose chords stay well under a
/// physical pixel from the arc.
pub(crate) fn arc_segments(radius: f32, pixels_per_point: f32) -> usize {
    ((radius * pixels_per_point.max(1.0) * 0.5).ceil() as usize).clamp(4, 32)
}

/// Triangles covering the region inside `rect`, outside the quarter-circle
/// rounding of `corner`. Their outer edge is a chord polyline inscribed in the
/// arc, so they overreach the rounded shape by under a pixel at `segments`
/// from [`arc_segments`].
pub(crate) fn outside_corner_fan(
    rect: Rect,
    corner: Corner,
    radius: f32,
    segments: usize,
) -> Vec<[Pos2; 3]> {
    let (apex, sx, sy) = corner.apex(rect);
    let center = pos2(apex.x + sx * radius, apex.y + sy * radius);
    let on_arc = |i: usize| {
        let angle = std::f32::consts::FRAC_PI_2 * i as f32 / segments as f32;
        pos2(
            center.x - sx * radius * angle.cos(),
            center.y - sy * radius * angle.sin(),
        )
    };
    (0..segments)
        .map(|i| [apex, on_arc(i), on_arc(i + 1)])
        .collect()
}

fn lerp_vertex(a: &Vertex, b: &Vertex, t: f32) -> Vertex {
    Vertex {
        pos: a.pos.lerp(b.pos, t),
        uv: a.uv.lerp(b.uv, t),
        color: a.color.lerp_to_gamma(b.color, t),
    }
}

/// Keeps the part of the convex `polygon` on the non-negative side of `signed`.
fn clip_half_plane(polygon: &[Vertex], signed: impl Fn(Pos2) -> f32) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(polygon.len() + 1);
    for (i, current) in polygon.iter().enumerate() {
        let previous = &polygon[(i + polygon.len() - 1) % polygon.len()];
        let (d_prev, d_cur) = (signed(previous.pos), signed(current.pos));
        if (d_prev >= 0.0) != (d_cur >= 0.0) {
            out.push(lerp_vertex(previous, current, d_prev / (d_prev - d_cur)));
        }
        if d_cur >= 0.0 {
            out.push(*current);
        }
    }
    out
}

/// The part of `mesh` inside the triangle `tri`, with vertex colour and uv
/// interpolated at the cut edges (Sutherland-Hodgman, one edge at a time).
pub(crate) fn clip_mesh_to_triangle(mesh: &Mesh, tri: [Pos2; 3]) -> Mesh {
    let area =
        (tri[1] - tri[0]).x * (tri[2] - tri[0]).y - (tri[1] - tri[0]).y * (tri[2] - tri[0]).x;
    let winding = area.signum();
    let bounds = Rect::from_points(&tri);
    let mut out = Mesh::with_texture(mesh.texture_id);
    for corners in mesh.indices.as_chunks::<3>().0 {
        let corners = corners.map(|index| mesh.vertices[index as usize]);
        let triangle_bounds = Rect::from_points(&corners.map(|v| v.pos));
        if !bounds.intersects(triangle_bounds) {
            continue;
        }
        let mut polygon = corners.to_vec();
        for edge in 0..3 {
            let (from, to) = (tri[edge], tri[(edge + 1) % 3]);
            polygon = clip_half_plane(&polygon, |p| {
                winding * ((to.x - from.x) * (p.y - from.y) - (to.y - from.y) * (p.x - from.x))
            });
            if polygon.len() < 3 {
                break;
            }
        }
        if polygon.len() < 3 {
            continue;
        }
        let first = out.vertices.len() as u32;
        out.vertices.extend(polygon.iter());
        for i in 1..polygon.len() as u32 - 1 {
            out.indices.extend([first, first + i, first + i + 1]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    fn area(tri: &[Pos2; 3]) -> f32 {
        0.5 * ((tri[1].x - tri[0].x) * (tri[2].y - tri[0].y)
            - (tri[1].y - tri[0].y) * (tri[2].x - tri[0].x))
            .abs()
    }

    fn mesh_area(mesh: &Mesh) -> f32 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| area(&t.map(|index| mesh.vertices[index as usize].pos)))
            .sum()
    }

    fn quad(rect: Rect, color: Color32) -> Mesh {
        let mut mesh = Mesh::default();
        for p in [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ] {
            mesh.vertices.push(Vertex {
                pos: p,
                uv: p,
                color,
            });
        }
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        mesh
    }

    #[test]
    fn the_fan_covers_the_cut_out_and_not_the_rounded_shape() {
        let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 90.0));
        let radius = 14.0_f32;
        let cut_out = radius * radius * (1.0 - std::f32::consts::FRAC_PI_4);
        for corner in Corner::ALL {
            let fan = outside_corner_fan(rect, corner, radius, 32);
            let covered: f32 = fan.iter().map(area).sum();
            assert!(
                (covered - cut_out).abs() < 0.01 * cut_out,
                "{corner:?}: {covered} vs {cut_out}"
            );
            let (apex, sx, sy) = corner.apex(rect);
            let center = pos2(apex.x + sx * radius, apex.y + sy * radius);
            for tri in &fan {
                for vertex in &tri[1..] {
                    assert!(
                        (vertex.distance(center) - radius).abs() < 1e-3,
                        "fan edge lies on the arc"
                    );
                }
                let centroid = pos2(
                    (tri[0].x + tri[1].x + tri[2].x) / 3.0,
                    (tri[0].y + tri[1].y + tri[2].y) / 3.0,
                );
                assert!(centroid.distance(center) > radius * 0.9, "{corner:?}");
                assert!(rect.contains(centroid));
            }
        }
    }

    #[test]
    fn the_fan_overreaches_the_arc_by_under_a_pixel() {
        let radius = 20.0;
        for ppp in [1.0, 2.0] {
            let segments = arc_segments(radius, ppp);
            let sagitta =
                radius * (1.0 - (std::f32::consts::FRAC_PI_2 / segments as f32 / 2.0).cos());
            assert!(sagitta * ppp < 0.25, "ppp {ppp}: {sagitta}");
        }
    }

    #[test]
    fn clipping_keeps_exactly_the_part_inside_the_triangle() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let mesh = quad(rect, Color32::BLACK);
        // The big triangles cut the square along x + y = 15, dropping a corner of 12.5.
        for (tri, expected) in [
            ([pos2(-5.0, -5.0), pos2(20.0, -5.0), pos2(-5.0, 20.0)], 87.5),
            ([pos2(-5.0, -5.0), pos2(-5.0, 20.0), pos2(20.0, -5.0)], 87.5),
            ([pos2(2.0, 2.0), pos2(6.0, 2.0), pos2(2.0, 6.0)], 8.0),
        ] {
            let clipped = clip_mesh_to_triangle(&mesh, tri);
            assert!((mesh_area(&clipped) - expected).abs() < 1e-3, "{tri:?}");
        }
    }

    #[test]
    fn clipping_outside_the_mesh_is_empty() {
        let mesh = quad(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::BLACK,
        );
        let far = [pos2(5.0, 5.0), pos2(6.0, 5.0), pos2(5.0, 6.0)];
        assert!(clip_mesh_to_triangle(&mesh, far).indices.is_empty());
    }

    #[test]
    fn colour_is_interpolated_at_a_clipped_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
        let mut mesh = quad(rect, Color32::BLACK);
        mesh.vertices[1].color = Color32::WHITE;
        mesh.vertices[2].color = Color32::WHITE;
        let tri = [pos2(-1.0, -1.0), pos2(5.0, -1.0), pos2(5.0, 11.0)];
        let clipped = clip_mesh_to_triangle(&mesh, tri);
        let on_cut = clipped
            .vertices
            .iter()
            .filter(|v| (v.pos.x - 5.0).abs() < 1e-4 && v.pos.y > 0.0 && v.pos.y < 10.0);
        let mut seen = 0;
        for v in on_cut {
            seen += 1;
            let expected = Color32::BLACK.lerp_to_gamma(Color32::WHITE, 0.5);
            assert_eq!(v.color, expected, "{v:?}");
            assert!((v.uv.x - 5.0).abs() < 1e-4, "uv follows position: {v:?}");
        }
        assert!(seen >= 2, "the cut edge has vertices");
    }
}
