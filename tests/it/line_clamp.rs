//! `line_clamp(n)`: wrapped text limited to n lines, ending in an ellipsis.

use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, vec2};

use crate::support::{self, rect_of, shapes};

const LINE: f32 = 20.0;
const TEXT: &str =
    "The quick brown fox jumps over the lazy dog and keeps running far past the end of the field";

fn body() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: LINE,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

fn clamped(lines: Option<u32>) -> impl Fn(&mut Builder<'_, '_, ()>, &()) {
    move |b, _| {
        let classes = lines.map_or_else(tw, |lines| tw().line_clamp(lines));
        b.div(tw().w_px(120.0), |b| {
            b.span("text", classes, TEXT);
        });
    }
}

fn height(lines: Option<u32>) -> f32 {
    Headless::new(vec2(400.0, 400.0))
        .base_text(body())
        .layout(&(), clamped(lines))
        .at("text")
        .rect()
        .height()
}

#[test]
fn clamped_text_is_n_lines_tall() {
    let unclamped = height(None);
    assert!(
        unclamped > 3.0 * LINE,
        "precondition: the text wraps over more than three lines ({unclamped})"
    );
    for lines in 1..=3 {
        assert_eq!(height(Some(lines)), lines as f32 * LINE, "{lines} lines");
    }
}

#[test]
fn text_that_fits_the_clamp_is_not_cut() {
    assert_eq!(height(Some(50)), height(None));
}

#[test]
fn the_drawn_text_ends_in_an_ellipsis() {
    fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
        clamped(Some(2))(b, &());
    }
    let mut harness = support::harness(vec2(400.0, 400.0), body(), (), view);
    harness.run_steps(3);
    let galley = shapes(&harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text().starts_with("The quick") => {
                Some(text.galley.clone())
            }
            _ => None,
        })
        .expect("the text was painted");
    assert_eq!(galley.rows.len(), 2);
    let last = galley
        .rows
        .last()
        .and_then(|placed| placed.row.glyphs.last());
    assert_eq!(last.map(|glyph| glyph.chr), Some('…'));
    assert_eq!(rect_of(&harness, "/\"text\"").height(), 2.0 * LINE);
}
