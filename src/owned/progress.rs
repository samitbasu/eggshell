//! The owned progress bar: a determinate fraction, drawn as a track with a
//! fill that ends in a straight edge at the value.

use egui::{Painter, Pos2, Rect, Role, Shape, Stroke, WidgetInfo, pos2};

use super::slider::track_classes;
use super::{Tip, tipped};
use crate::builder::Slot;
use crate::{Builder, NodeUi as _, Progress, Style, Tw, tw};

/// Segments of each quarter-circle of a corner.
const ARC: usize = 8;

/// The outline of `rect` with corners of `radius`, clockwise from the top
/// left corner's end of the left edge.
fn rounded_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    let corners = [
        (pos2(rect.left() + r, rect.top() + r), 180.0_f32),
        (pos2(rect.right() - r, rect.top() + r), 270.0),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), 90.0),
    ];
    let mut points = Vec::new();
    for (centre, start) in corners {
        for step in 0..=ARC {
            let angle = (start + 90.0 * step as f32 / ARC as f32).to_radians();
            points.push(pos2(centre.x + r * angle.cos(), centre.y + r * angle.sin()));
        }
    }
    points
}

/// `points`, a convex polygon, cut to the half-plane `x <= edge`.
fn cut_right_of(points: &[Pos2], edge: f32) -> Vec<Pos2> {
    let mut cut = Vec::new();
    for (&a, &b) in points.iter().zip(points.iter().cycle().skip(1)) {
        let (a_in, b_in) = (a.x <= edge, b.x <= edge);
        if a_in {
            cut.push(a);
        }
        if a_in != b_in {
            let t = (edge - a.x) / (b.x - a.x);
            cut.push(pos2(edge, a.y + t * (b.y - a.y)));
        }
    }
    cut
}

/// The fill of a bar whose track is `track` with corners of `radius`, up to
/// `value`: the part of the rounded track left of the value, so its left end
/// follows the track's cap and its right edge is straight. Empty at zero.
fn fill_outline(track: Rect, radius: f32, value: Progress) -> Vec<Pos2> {
    let value = f32::from(value);
    if value <= 0.0 {
        return Vec::new();
    }
    let edge = track.left() + value * track.width();
    cut_right_of(&rounded_outline(track, radius), edge)
}

fn paint_fill(painter: &Painter, track: Rect, radius: f32, value: Progress, color: egui::Color32) {
    let outline = fill_outline(track, radius, value);
    if outline.len() >= 3 {
        painter.add(Shape::convex_polygon(outline, color, Stroke::NONE));
    }
}

/// A bar filled to `progress`, drawn from the [`Style`]'s progress entry, and
/// announced to assistive technology as a progress indicator whose value is
/// the percentage. The fill ends in a straight edge at the value and follows
/// the track's rounded end on the left. Start one with
/// [`Builder::progress`] and finish it with [`add`](Self::add).
///
/// ```html
/// <div role="progressbar" aria-valuenow="40" class="w-full h-2 rounded-full bg-primary/20 overflow-hidden">
///   <div class="h-full w-full bg-primary -translate-x-[60%]"></div>
/// </div>
/// ```
#[must_use = "a progress bar is not added until you call `.add()`"]
pub struct ProgressChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    progress: Progress,
    name: Option<String>,
    tooltip: Tip,
    classes: Tw,
}

impl<'a, 'f, A: 'static> ProgressChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, progress: Progress) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            progress,
            name: None,
            tooltip: None,
            classes: tw(),
        }
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the bar, such as its width,
    /// which is the space it is given; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shown while the pointer hovers the bar, as egui's `on_hover_text`
    /// shows it. It adds to the accessible name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Adds the bar.
    pub fn add(self) {
        let Self {
            mut b,
            slot,
            progress,
            name,
            tooltip,
            classes,
        } = self;
        let style = Style::of(b.ctx());
        let bar = &style.progress;
        let track = track_classes((bar.height, bar.radius), bar.track).merge(classes);
        let (radius, fill) = (f32::from(bar.radius), bar.range.fill);
        let percent = f64::from(f32::from(progress) * 100.0);
        let name = name.unwrap_or_default();
        b.div_id_with(slot.id, &slot.label, track, |b| {
            b.paint(tw().w_full().h_full(), move |painter, rect, _| {
                paint_fill(painter, rect, radius, progress, fill)
            });
        })
        .draw(move |_, ui| {
            let response = tipped(ui.node_response(), &tooltip);
            response.widget_info(|| WidgetInfo {
                value: Some(percent),
                ..WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), &name)
            });
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(100.0);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: fn() -> Rect = || Rect::from_min_max(pos2(10.0, 20.0), pos2(210.0, 28.0));

    fn bounds(points: &[Pos2]) -> Rect {
        Rect::from_points(points)
    }

    fn area(points: &[Pos2]) -> f32 {
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f32>()
            .abs()
            / 2.0
    }

    fn fill(value: f32) -> Vec<Pos2> {
        fill_outline(TRACK(), 255.0, Progress::new(value))
    }

    #[test]
    fn zero_draws_nothing() {
        assert!(fill(0.0).is_empty());
    }

    #[test]
    fn one_is_the_whole_rounded_track() {
        let all = fill(1.0);
        assert_eq!(bounds(&all), TRACK());
        let whole = rounded_outline(TRACK(), 255.0);
        assert!((area(&all) - area(&whole)).abs() < 1e-3);
        let pill = 192.0 * 8.0 + std::f32::consts::PI * 16.0;
        assert!((area(&all) - pill).abs() < 0.5, "{}", area(&all));
    }

    #[test]
    fn the_right_edge_is_straight_at_the_value() {
        for value in [0.01, 0.5, 0.9] {
            let outline = fill(value);
            let edge = 10.0 + value * 200.0;
            let on_edge: Vec<&Pos2> = outline.iter().filter(|p| p.x == edge).collect();
            assert_eq!(on_edge.len(), 2, "{value}: {outline:?}");
            assert!(outline.iter().all(|p| p.x <= edge));
            assert_eq!(bounds(&outline).right(), edge);
        }
    }

    #[test]
    fn a_value_inside_the_cap_is_the_caps_sliver() {
        let outline = fill(0.01);
        let rect = bounds(&outline);
        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.right(), 12.0);
        assert!(rect.height() < 8.0, "the cut is narrower than the track");
        assert!(rect.height() > 0.0);
        assert!(area(&outline) < 8.0 * 2.0, "inside the 2 pt by 8 pt box");
    }

    #[test]
    fn the_middle_is_a_capped_bar_with_a_square_end() {
        let outline = fill(0.5);
        let rect = bounds(&outline);
        assert_eq!((rect.left(), rect.right()), (10.0, 110.0));
        assert_eq!((rect.top(), rect.bottom()), (20.0, 28.0));
        let expected = 96.0 * 8.0 + std::f32::consts::PI * 16.0 / 2.0;
        assert!(
            (area(&outline) - expected).abs() < 0.5,
            "{}",
            area(&outline)
        );
    }
}
