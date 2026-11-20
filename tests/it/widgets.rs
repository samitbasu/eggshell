//! The text edit as a leaf, through the full path: the builder sizes it with
//! the library's computation, the draw phase places egui's widget in the
//! leaf's rect, and pointer and keyboard events reach it.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::widget::{Stock, TextEdit};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, Width, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 16.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
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
    let mut d = support::driver(vec2(400.0, 100.0), base_text(), Notes::default(), notes).build();
    d.harness_mut().run();
    let size = TextEdit::singleline("").size(
        d.ctx(),
        &base_text(),
        &d.shell().style(),
        Some(Width::new(200.0)),
    );
    assert_eq!(d.rect(Target::suffix("/\"notes\"")).size(), size);
}

#[test]
fn a_text_edit_takes_focus_and_edits() {
    let mut d = support::driver(vec2(400.0, 100.0), base_text(), Notes::default(), notes).build();
    d.harness_mut().run();
    d.click(Target::suffix("/\"notes\"")).frames(2);
    d.event(egui::Event::Text("hello".to_owned())).frames(2);
    assert_eq!(d.app().text, "hello");
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
fn painted_text<A>(d: &Driver<A>) -> Vec<String> {
    d.harness()
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
    let mut d = support::driver(vec2(300.0, 100.0), base_text(), Secret::default(), secret).build();
    d.frames(2);
    let unmasked = d.rect(Target::suffix("/\"password\""));
    d.click(Target::suffix("/\"password\"")).frames(2);
    d.event(egui::Event::Text("hunter2".to_owned())).frames(2);

    assert_eq!(d.app().password, "hunter2");
    let painted = painted_text(&d).concat();
    assert!(!painted.contains("hunter2"), "painted {painted:?}");
    assert!(painted.contains('•'), "painted {painted:?}");
    assert_eq!(d.rect(Target::suffix("/\"password\"")), unmasked);
}
