//! The text edit as a leaf, through the full path: the builder sizes it with
//! the library's computation, the draw phase places egui's widget in the
//! leaf's rect, and pointer and keyboard events reach it.

use eggshell_ui::widget::{Stock, TextEdit};
use eggshell_ui::{Builder, TextSpec, TextStyle, Width, tw};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;

use crate::support;

use support::{Rig, click_node, rect_of};

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::WHITE,
    )
}

#[derive(Default)]
struct Notes {
    text: String,
}

/// ```html
/// <div class="p-2"><input type="text" class="w-[200px]" value="Notes"></div>
/// ```
fn notes(b: &mut Builder<'_, '_, Notes>, app: &Notes) {
    b.div(tw().p(2.0), |b| {
        b.widget(
            "notes",
            tw().w_px(200.0),
            TextEdit::singleline(&app.text).named("Notes"),
            |app, ui, field| {
                field.show(ui, &mut app.text);
            },
        );
    });
}

#[test]
fn a_leaf_is_as_large_as_the_size() {
    let mut harness = support::harness(vec2(400.0, 100.0), base_text(), Notes::default(), notes);
    harness.run();
    let size = TextEdit::singleline("").size(&harness.ctx, &base_text(), Some(Width::new(200.0)));
    assert_eq!(rect_of(&harness, "/\"notes\"").size(), size);
}

#[test]
fn a_text_edit_takes_focus_and_edits() {
    let mut harness = support::harness(vec2(400.0, 100.0), base_text(), Notes::default(), notes);
    harness.run();
    click_node(&mut harness, "/\"notes\"");
    harness.run_steps(2);
    harness.event(egui::Event::Text("hello".to_owned()));
    harness.run_steps(3);
    assert_eq!(harness.state().app.text, "hello");
}

#[derive(Default)]
struct Secret {
    password: String,
}

fn secret(b: &mut Builder<'_, '_, Secret>, app: &Secret) {
    b.widget(
        "password",
        tw().w_px(200.0),
        TextEdit::singleline(&app.password)
            .masked()
            .named("Password"),
        |app, ui, field| {
            field.show(ui, &mut app.password);
        },
    );
}

/// The text of every galley painted in the last frame.
fn painted_text<A>(harness: &Harness<'_, Rig<A>>) -> Vec<String> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_masked_field_edits_the_text_and_paints_dots() {
    let mut harness = support::harness(vec2(300.0, 100.0), base_text(), Secret::default(), secret);
    harness.run_steps(2);
    let unmasked = rect_of(&harness, "/\"password\"");
    click_node(&mut harness, "/\"password\"");
    harness.run_steps(2);
    harness.event(egui::Event::Text("hunter2".to_owned()));
    harness.run_steps(3);

    assert_eq!(harness.state().app.password, "hunter2");
    let painted = painted_text(&harness).concat();
    assert!(!painted.contains("hunter2"), "painted {painted:?}");
    assert!(painted.contains('•'), "painted {painted:?}");
    assert_eq!(rect_of(&harness, "/\"password\""), unmasked);
}
