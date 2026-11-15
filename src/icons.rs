//! Vector icons drawn in the text colour, and a standard set.

use std::f32::consts::PI;
use std::sync::OnceLock;

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

use crate::{Angle, Mirror};

/// The side of the grid icons are drawn on, as lucide's are.
const GRID: f32 = 24.0;

/// One subpath of an icon: its points on the grid, the corners to round, and
/// whether it closes.
#[derive(Clone, Debug, PartialEq)]
struct Subpath {
    points: Vec<Pos2>,
    corners: Vec<Pos2>,
    closed: bool,
}

/// An icon's strokes: lucide's `d` attributes on a 24 point grid.
#[derive(Debug)]
pub struct Strokes {
    paths: &'static [&'static str],
    parsed: OnceLock<Vec<Subpath>>,
}

impl Strokes {
    /// TODO(prose): strokes from SVG path data (`M m L l H h V v C c A a Z z`).
    pub const fn new(paths: &'static [&'static str]) -> Self {
        Self {
            paths,
            parsed: OnceLock::new(),
        }
    }

    fn subpaths(&self) -> &[Subpath] {
        self.parsed
            .get_or_init(|| self.paths.iter().flat_map(|d| parse(d)).collect())
    }

    /// TODO(prose): the icon stroked in `color` in `rect`.
    pub fn paint(&self, painter: &Painter, rect: Rect, color: Color32) {
        self.paint_with(painter, rect, color, (Angle::ZERO, Mirror::None));
    }

    /// TODO(prose): [`paint`](Self::paint) turned and mirrored.
    pub fn paint_with(
        &self,
        painter: &Painter,
        rect: Rect,
        color: Color32,
        (rotate, mirror): (Angle, Mirror),
    ) {
        let side = rect.width().min(rect.height());
        let scale = side / GRID;
        let origin = rect.center() - egui::vec2(side, side) / 2.0;
        let turn = f32::from(rotate);
        let (sin, cos) = turn.sin_cos();
        let place = |p: Pos2| {
            let (mut x, mut y) = (p.x, p.y);
            if matches!(mirror, Mirror::Horizontal | Mirror::Both) {
                x = GRID - x;
            }
            if matches!(mirror, Mirror::Vertical | Mirror::Both) {
                y = GRID - y;
            }
            let (dx, dy) = (x - GRID / 2.0, y - GRID / 2.0);
            let (x, y) = (
                GRID / 2.0 + dx * cos - dy * sin,
                GRID / 2.0 + dx * sin + dy * cos,
            );
            origin + egui::vec2(x, y) * scale
        };
        let width = 2.0 * scale;
        let stroke = Stroke::new(width, color);
        for path in self.subpaths() {
            let points: Vec<Pos2> = path.points.iter().copied().map(place).collect();
            painter.add(if path.closed {
                Shape::closed_line(points, stroke)
            } else {
                Shape::line(points, stroke)
            });
            for corner in &path.corners {
                painter.circle_filled(place(*corner), width / 2.0, color);
            }
        }
    }
}

/// The subpaths of the SVG path data `d`.
fn parse(d: &str) -> Vec<Subpath> {
    let mut reader = Reader { rest: d };
    let mut paths: Vec<Subpath> = Vec::new();
    let (mut at, mut start) = (pos2(0.0, 0.0), pos2(0.0, 0.0));
    let mut command = None;
    while let Some(next) = reader.command(command) {
        command = Some(next);
        let relative = next.is_ascii_lowercase();
        let from = |reader: &mut Reader, at: Pos2| {
            let (x, y) = (reader.number(), reader.number());
            if relative {
                at + egui::vec2(x, y)
            } else {
                pos2(x, y)
            }
        };
        match next.to_ascii_uppercase() {
            'M' => {
                at = from(&mut reader, at);
                start = at;
                paths.push(Subpath {
                    points: vec![at],
                    corners: vec![at],
                    closed: false,
                });
                command = Some(if relative { 'l' } else { 'L' });
            }
            'Z' => {
                if let Some(path) = paths.last_mut() {
                    path.closed = true;
                }
                at = start;
            }
            letter => {
                let to = match letter {
                    'L' => from(&mut reader, at),
                    'H' => {
                        let x = reader.number();
                        pos2(if relative { at.x + x } else { x }, at.y)
                    }
                    'V' => {
                        let y = reader.number();
                        pos2(at.x, if relative { at.y + y } else { y })
                    }
                    'C' => {
                        let (c1, c2, to) = (
                            from(&mut reader, at),
                            from(&mut reader, at),
                            from(&mut reader, at),
                        );
                        if let Some(path) = paths.last_mut() {
                            path.points.extend(cubic(at, c1, c2, to));
                        }
                        to
                    }
                    'A' => {
                        let (rx, ry, turn) = (reader.number(), reader.number(), reader.number());
                        let (large, sweep) = (reader.flag(), reader.flag());
                        let to = from(&mut reader, at);
                        if let Some(path) = paths.last_mut() {
                            path.points
                                .extend(arc(at, (rx, ry, turn), (large, sweep), to));
                        }
                        to
                    }
                    other => unreachable!("lucide uses no path command {other}"),
                };
                if let Some(path) = paths.last_mut() {
                    if !matches!(letter, 'C' | 'A') {
                        path.points.push(to);
                    }
                    path.corners.push(to);
                }
                at = to;
            }
        }
    }
    paths
}

/// A cursor over path data.
struct Reader<'a> {
    rest: &'a str,
}

impl Reader<'_> {
    fn skip(&mut self) {
        self.rest = self
            .rest
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',');
    }

    /// The next command letter, or `previous` repeated when numbers follow.
    fn command(&mut self, previous: Option<char>) -> Option<char> {
        self.skip();
        let first = self.rest.chars().next()?;
        if first.is_ascii_alphabetic() {
            self.rest = &self.rest[1..];
            Some(first)
        } else {
            previous
        }
    }

    fn number(&mut self) -> f32 {
        self.skip();
        let bytes = self.rest.as_bytes();
        let mut end = 0;
        if matches!(bytes.first(), Some(b'-' | b'+')) {
            end += 1;
        }
        let mut dot = false;
        while let Some(&byte) = bytes.get(end) {
            match byte {
                b'0'..=b'9' => end += 1,
                b'.' if !dot => {
                    dot = true;
                    end += 1;
                }
                _ => break,
            }
        }
        let (number, rest) = self.rest.split_at(end);
        self.rest = rest;
        number
            .parse()
            .unwrap_or_else(|_| panic!("a number in path data, at {number:?}"))
    }

    /// An arc flag, which may be run together with what follows.
    fn flag(&mut self) -> bool {
        self.skip();
        let flag = self.rest.starts_with('1');
        self.rest = &self.rest[1..];
        flag
    }
}

/// Points along the cubic from `from` to `to`, without `from`.
fn cubic(from: Pos2, c1: Pos2, c2: Pos2, to: Pos2) -> Vec<Pos2> {
    const STEPS: usize = 12;
    (1..=STEPS)
        .map(|step| {
            let t = step as f32 / STEPS as f32;
            let u = 1.0 - t;
            (from.to_vec2() * (u * u * u)
                + c1.to_vec2() * (3.0 * u * u * t)
                + c2.to_vec2() * (3.0 * u * t * t)
                + to.to_vec2() * (t * t * t))
                .to_pos2()
        })
        .collect()
}

/// Points along the SVG elliptical arc from `from` to `to`, without `from`
/// (the endpoint to centre conversion of the SVG specification).
fn arc(
    from: Pos2,
    (rx, ry, turn): (f32, f32, f32),
    (large, sweep): (bool, bool),
    to: Pos2,
) -> Vec<Pos2> {
    if from == to {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        return vec![to];
    }
    let (sin, cos) = turn.to_radians().sin_cos();
    let half = (from - to) / 2.0;
    let (x1, y1) = (cos * half.x + sin * half.y, -sin * half.x + cos * half.y);
    let excess = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if excess > 1.0 {
        rx *= excess.sqrt();
        ry *= excess.sqrt();
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let k = sign * (numerator / denominator).max(0.0).sqrt();
    let (cx1, cy1) = (k * rx * y1 / ry, -k * ry * x1 / rx);
    let middle = from.lerp(to, 0.5);
    let centre = pos2(
        cos * cx1 - sin * cy1 + middle.x,
        sin * cx1 + cos * cy1 + middle.y,
    );
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let start = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut sweep_angle = angle(
        (x1 - cx1) / rx,
        (y1 - cy1) / ry,
        (-x1 - cx1) / rx,
        (-y1 - cy1) / ry,
    );
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= 2.0 * PI;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += 2.0 * PI;
    }
    let steps = (sweep_angle.abs() / (PI / 16.0)).ceil().max(1.0) as usize;
    (1..=steps)
        .map(|step| {
            if step == steps {
                return to;
            }
            let theta = start + sweep_angle * step as f32 / steps as f32;
            let (x, y) = (rx * theta.cos(), ry * theta.sin());
            pos2(cos * x - sin * y + centre.x, sin * x + cos * y + centre.y)
        })
        .collect()
}

macro_rules! icons {
    ($($(#[$doc:meta])* $name:ident: [$($d:expr),+ $(,)?];)+) => {
        $(
            $(#[$doc])*
            pub static $name: Strokes = Strokes::new(&[$($d),+]);
        )+
    };
}

icons! {
    /// TODO(prose): lucide `x`.
    CLOSE: ["M18 6 6 18", "m6 6 12 12"];
    /// TODO(prose): lucide `check`.
    CHECK: ["M20 6 9 17l-5-5"];
    /// TODO(prose): lucide `chevron-down`.
    CHEVRON_DOWN: ["m6 9 6 6 6-6"];
    /// TODO(prose): lucide `chevron-up`.
    CHEVRON_UP: ["m18 15-6-6-6 6"];
    /// TODO(prose): lucide `chevron-left`.
    CHEVRON_LEFT: ["m15 18-6-6 6-6"];
    /// TODO(prose): lucide `chevron-right`.
    CHEVRON_RIGHT: ["m9 18 6-6-6-6"];
    /// TODO(prose): lucide `plus`.
    PLUS: ["M5 12h14", "M12 5v14"];
    /// TODO(prose): lucide `minus`.
    MINUS: ["M5 12h14"];
    /// TODO(prose): lucide `maximize-2`.
    MAXIMIZE: ["M15 3h6v6", "m21 3-7 7", "m3 21 7-7", "M9 21H3v-6"];
    /// TODO(prose): lucide `external-link`, the pop-out.
    POP_OUT: [
        "M15 3h6v6",
        "M10 14 21 3",
        "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6",
    ];
    /// TODO(prose): lucide `pause`.
    PAUSE: [
        "M14 3h4a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
        "M6 3h4a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
    ];
    /// TODO(prose): lucide `play`.
    PLAY: ["M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z"];
    /// TODO(prose): lucide `triangle-alert`.
    WARNING: [
        "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
        "M12 9v4",
        "M12 17h.01",
    ];
    /// TODO(prose): lucide `search`.
    SEARCH: ["m21 21-4.34-4.34", "M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0z"];
    /// TODO(prose): lucide `circle-help`.
    HELP: [
        "M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0z",
        "M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3",
        "M12 17h.01",
    ];
    /// TODO(prose): lucide `settings`, the gear.
    GEAR: [
        "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
        "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(d: &str) -> Vec<Vec<(f32, f32)>> {
        parse(d)
            .iter()
            .map(|path| path.points.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    #[test]
    fn relative_and_absolute_lines_make_the_same_polyline() {
        let expected = vec![vec![(20.0, 6.0), (9.0, 17.0), (4.0, 12.0)]];
        assert_eq!(points("M20 6 9 17l-5-5"), expected);
        assert_eq!(points("M20 6L9 17L4 12"), expected);
    }

    #[test]
    fn h_and_v_are_lines_and_each_move_starts_a_subpath() {
        assert_eq!(
            points("M5 12h14M12 5v14"),
            vec![
                vec![(5.0, 12.0), (19.0, 12.0)],
                vec![(12.0, 5.0), (12.0, 19.0)]
            ]
        );
    }

    #[test]
    fn numbers_may_run_together() {
        assert_eq!(points("m1.5.5-.25-1")[0], vec![(1.5, 0.5), (1.25, -0.5)]);
    }

    #[test]
    fn z_closes_a_subpath() {
        let paths = parse("M0 0h4v4z");
        assert!(paths[0].closed);
        assert_eq!(paths[0].corners.len(), 3);
    }

    #[test]
    fn an_arc_ends_where_it_says_and_bulges_the_right_way() {
        let path = &parse("M0 0a5 5 0 0 1 10 0")[0];
        let (first, last) = (path.points[0], *path.points.last().unwrap());
        assert!((first - pos2(0.0, 0.0)).length() < 1e-4);
        assert!((last - pos2(10.0, 0.0)).length() < 1e-3);
        assert!(
            path.points.iter().all(|p| p.y <= 1e-3) && path.points.iter().any(|p| p.y < -4.9),
            "sweep 1 from left to right goes over the top (y up is negative)"
        );
    }
}
