//! The Esc that unfocuses a text field belongs to the field: a custom node
//! that reads the keys sees no Escape for it, and still sees every other.

use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Key, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, focused_path};

const FIELD: &str = r#"/"Page"/"name""#;

#[derive(Default)]
struct App {
    name: String,
    escapes: u32,
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field("name", app.name.clone())
            .named("Name")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.name = text);
        s.div_id("canvas", tw().w_px(160.0).h_px(60.0), |_| {})
            .draw(|app: &mut App, ui| {
                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    app.escapes += 1;
                }
            });
    });
}

fn harness() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), view);
    harness.run_steps(4);
    harness
}

#[test]
fn the_escape_that_unfocuses_a_field_does_not_reach_a_custom_node() {
    let mut harness = harness();
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some("<widget>"),
        "precondition: the field has focus"
    );
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    assert_eq!(focused_path(&harness), None, "Esc unfocused the field");
    assert_eq!(harness.state().app.escapes, 0, "and went no further");
}

#[test]
fn with_nothing_focused_the_escape_reaches_the_custom_node() {
    let mut harness = harness();
    assert_eq!(
        focused_path(&harness),
        None,
        "precondition: nothing has focus"
    );
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    assert_eq!(harness.state().app.escapes, 1);
}

#[test]
fn a_second_escape_reaches_the_custom_node() {
    let mut harness = harness();
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    assert_eq!(
        harness.state().app.escapes,
        1,
        "the first was the field's, the second is the node's"
    );
}
