//! A text field draws with its node's text classes: color, font and
//! alignment (#130).

use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Rect, vec2};

use crate::support::{self, shapes};
use eggshell_ui::testing::{Driver, Target};

const FIELD: &str = r#"/"Page"/"field""#;
const RED: Color32 = Color32::from_rgb(200, 30, 30);

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        s.text_field("field", "42")
            .named("Field")
            .classes(tw().w_px(200.0).text(RED).text_right().role(TextSpec {
                size: 18.0,
                line_height: 24.0,
                family: FontFamily::Proportional,
                weight: Weight::Normal,
            }))
            .on_change(|_, _| {});
    });
}

fn harness() -> Driver<()> {
    let mut d = support::driver(vec2(300.0, 100.0), base_text(), (), view).build();
    d.frames(4);
    d
}

fn drawn(d: &Driver<()>) -> (Color32, f32, Rect) {
    shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == "42" => {
                let format = &text.galley.job.sections[0].format;
                let colour = text
                    .galley
                    .rows
                    .first()
                    .and_then(|row| row.glyphs.first())
                    .map_or(format.color, |_| {
                        text.override_text_color.unwrap_or(format.color)
                    });
                Some((
                    colour,
                    format.font_id.size,
                    text.galley.rect.translate(text.pos.to_vec2()),
                ))
            }
            _ => None,
        })
        .expect("the text was painted")
}

#[test]
fn the_field_draws_in_its_nodes_colour_and_size() {
    let d = harness();
    let (colour, size, _) = drawn(&d);
    assert_eq!(colour, RED);
    assert_eq!(size, 18.0);
}

#[test]
fn text_right_aligns_the_text_to_the_fields_right_edge() {
    let d = harness();
    let (_, _, text) = drawn(&d);
    let field = d.rect(Target::suffix(FIELD));
    assert!(
        field.right() - text.right() < 16.0,
        "the text ends at {} in a field ending at {}",
        text.right(),
        field.right()
    );
    assert!(
        text.left() > field.center().x,
        "and starts right of the middle"
    );
}

struct Digits {
    text: String,
    tabular: bool,
}

fn digits_view(b: &mut Builder<'_, '_, Digits>, app: &Digits) {
    b.div_id("Page", tw().flex().flex_col().p(2.0), |s| {
        let classes = tw().w_px(200.0);
        let classes = if app.tabular {
            classes.tabular_nums()
        } else {
            classes
        };
        s.text_field("field", app.text.clone())
            .named("Field")
            .classes(classes)
            .on_change(|app: &mut Digits, text| app.text = text);
    });
}

fn width_of(text: &'static str, tabular: bool) -> f32 {
    let mut d = support::driver(
        vec2(300.0, 100.0),
        base_text(),
        Digits {
            text: text.into(),
            tabular,
        },
        digits_view,
    )
    .build();
    d.frames(4);
    shapes(&d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => Some(shape.galley.rect.width()),
            _ => None,
        })
        .expect("the text was painted")
}

#[test]
fn tabular_nums_make_every_digit_as_wide_as_the_widest() {
    assert_ne!(
        width_of("1111", false),
        width_of("8888", false),
        "precondition: the face's digits differ in width"
    );
    assert_eq!(width_of("1111", true), width_of("8888", true));
}

#[test]
fn a_tabular_field_still_edits() {
    let mut d = support::driver(
        vec2(300.0, 100.0),
        base_text(),
        Digits {
            text: "1".into(),
            tabular: true,
        },
        digits_view,
    )
    .build();
    d.frames(4);
    d.click(Target::suffix(FIELD)).frames(6);
    d.event(egui::Event::Text("2".into())).frames(3);
    assert!(
        shapes(&d).into_iter().any(|shape| matches!(
            shape, Shape::Text(text) if text.galley.text() == "12" || text.galley.text() == "21")),
        "the typed digit is in the field"
    );
}
