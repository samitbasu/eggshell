//! An underline sits under its text's glyphs, not at the bottom of a tall
//! line (#356): egui draws it at the foot of the line box, which typeset's
//! leading pushes halfway to the next line.

use eggshell_ui::testing::Driver;
use eggshell_ui::{Builder, Span, Spans, TextSpec, TextStyle, Weight, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Pos2, Rect, vec2};

use crate::support;

const SIZE: f32 = 15.0;
const TALL: f32 = 40.0;
const LINKED: &str = "see the guide here";
const PLAIN: &str = "underlined words";

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().items_start().p(4.0), |b| {
        let spans: Spans = [
            Span::new("see "),
            Span::new("the guide").link("guide"),
            Span::new(" here"),
        ]
        .into_iter()
        .collect();
        b.spans("linked", tw(), spans);
        b.text(tw().underline(), PLAIN);
    });
}

fn harness() -> Driver<()> {
    let text = TextSpec {
        size: SIZE,
        line_height: TALL,
        family: FontFamily::Proportional,
        weight: Weight::Normal,
    };
    let mut d = support::driver(
        vec2(400.0, 200.0),
        TextStyle::new(text, Color32::BLACK),
        (),
        view,
    )
    .build();
    d.frames(3);
    d
}

/// The screen y of the first row's baseline in the galley painted with `text`.
fn baseline(d: &Driver<()>, text: &str) -> f32 {
    let painted = d.painted().expect(text);
    let galley = painted.galley();
    let at = painted.rect().min - galley.rect.min.to_vec2();
    let row = &galley.rows[0];
    at.y + row.pos.y + row.row.glyphs[0].pos.y
}

/// The horizontal lines painted across the middle of `part`, within its row.
fn lines_under(d: &Driver<()>, part: Rect) -> Vec<[Pos2; 2]> {
    support::shapes(d)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, .. }
                if (points[0].y - points[1].y).abs() < 0.01
                    && part.y_range().contains(points[0].y)
                    && points[0].x.min(points[1].x) <= part.center().x
                    && points[0].x.max(points[1].x) >= part.center().x =>
            {
                Some(*points)
            }
            _ => None,
        })
        .collect()
}

#[track_caller]
fn assert_under_the_glyphs(d: &Driver<()>, text: &str, part: &str) {
    let base = baseline(d, text);
    let lines = lines_under(d, part_rect(d, text, part));
    assert_eq!(lines.len(), 1, "one underline under {part:?}: {lines:?}");
    let below = lines[0][0].y - base;
    assert!(
        below > 0.0 && below <= 0.35 * SIZE,
        "{part:?}'s underline is {below} below its baseline, in a {TALL} line"
    );
}

#[test]
fn a_links_underline_sits_under_its_glyphs_in_a_tall_line() {
    let d = harness();
    assert_under_the_glyphs(&d, LINKED, "the guide");
    let outside = lines_under(&d, part_rect(&d, LINKED, "here"));
    assert!(
        outside.is_empty(),
        "the rest is not underlined: {outside:?}"
    );
}

#[test]
fn underlined_text_sits_its_line_under_its_glyphs_too() {
    let d = harness();
    assert_under_the_glyphs(&d, PLAIN, "words");
}

/// Where `part` of the text painted as `text` is, on one row.
fn part_rect(d: &Driver<()>, text: &str, part: &str) -> Rect {
    d.painted()
        .expect(text)
        .part_rect(part)
        .expect("precondition: the part is on one row")
}
