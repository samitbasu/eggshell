//! `b.number` (#328): the slider's editable readout without the track,
//! through the same implementation.

use eggshell_ui::owned::{
    Enablement,
    number::{Edit, Step},
};
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Color32, Event, Key, Role, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_node, rect_of, text_color};

const INK: Color32 = Color32::from_rgb(20, 40, 60);

#[derive(Default)]
struct App {
    gain: f64,
    edits: Vec<Edit>,
    disabled: bool,
    refusal: Option<&'static str>,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().gap(4.0).p(4.0).text(INK), |b| {
        b.number("gain", app.gain, -380.0..=380.0)
            .format(|value| format!("{value:.1} dB"))
            .editable(|text| text.trim_end_matches("dB").trim().parse().ok())
            .step(Step::new(0.5))
            .named("Gain")
            .enablement(Enablement::from(!app.disabled))
            .when_some(app.refusal, |number, why| number.refused(why))
            .on_edit(|app: &mut App, edit| app.edits.push(edit))
            .on_change(|app: &mut App, value| app.gain = value);
        b.number("shown", 7.0, 0.0..=10.0)
            .named("Shown")
            .editable(|text| text.parse().ok())
            .add();
    });
}

const GAIN: &str = r#"/"gain"/Readout"#;
const SHOWN: &str = r#"/"shown"/Readout"#;

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(
        vec2(300.0, 200.0),
        TextStyle::new(
            eggshell_ui::TextSpec {
                size: 14.0,
                line_height: 20.0,
                family: egui::FontFamily::Proportional,
                weight: eggshell_ui::Weight::Normal,
            },
            Color32::BLACK,
        ),
        app,
        view,
    );
    harness.run_steps(4);
    harness
}

fn type_and(harness: &mut Harness<'_, Rig<App>>, text: &str, key: Key) {
    click_node(harness, GAIN);
    harness.run_steps(4);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(3);
    harness.key_press(key);
    harness.run_steps(4);
}

#[test]
fn it_shows_the_value_as_formatted_in_the_text_colour_around_it() {
    let harness = harness_with(App::default());
    assert_eq!(text_color(&harness, rect_of(&harness, GAIN), "0.0 dB"), INK);
    assert_eq!(text_color(&harness, rect_of(&harness, SHOWN), "7"), INK);
}

#[test]
fn enter_commits_a_typed_value_clamped_and_snapped_inside_one_edit() {
    let mut harness = harness_with(App::default());
    type_and(&mut harness, "12.3", Key::Enter);
    assert_eq!(harness.state().app.gain, 12.5, "snapped to 0.5");
    assert_eq!(harness.state().app.edits, [Edit::Began, Edit::Ended]);
    type_and(&mut harness, "900", Key::Enter);
    assert_eq!(harness.state().app.gain, 380.0, "clamped");
}

#[test]
fn escape_reverts() {
    let mut harness = harness_with(App::default());
    type_and(&mut harness, "12", Key::Escape);
    assert_eq!(harness.state().app.gain, 0.0);
    assert!(focused_node(&harness).is_none(), "the field closed");
}

#[test]
fn it_is_a_button_that_opens_a_field() {
    let mut harness = harness_with(App::default());
    harness.get_by_role_and_label(Role::Button, "Edit Gain");
    click_node(&mut harness, GAIN);
    harness.run_steps(4);
    assert_eq!(focused_node(&harness), Some(format!("{GAIN}/Field")));
}

#[test]
fn one_added_without_a_parse_only_shows_its_value() {
    let mut harness = harness_with(App::default());
    assert!(harness.query_by_label("Edit Shown").is_none());
    click_node(&mut harness, SHOWN);
    harness.run_steps(4);
    assert!(focused_node(&harness).is_none(), "nothing opens");
}

#[test]
fn a_disabled_one_does_not_open() {
    let mut harness = harness_with(App {
        disabled: true,
        ..App::default()
    });
    click_node(&mut harness, GAIN);
    harness.run_steps(4);
    assert!(focused_node(&harness).is_none());
    assert!(harness.state().app.edits.is_empty());
}

fn painted(harness: &Harness<'_, Rig<App>>, text: &str) -> bool {
    support::shapes(harness).iter().any(|shape| match shape {
        Shape::Text(shape) => shape.galley.text() == text,
        _ => false,
    })
}

#[test]
fn a_refused_one_is_disabled_and_a_press_shows_its_reason_without_opening() {
    let mut harness = harness_with(App {
        refusal: Some("Locked by the mixer"),
        ..App::default()
    });
    let button = harness.get_by_role_and_label(Role::Button, "Edit Gain");
    assert!(button.accesskit_node().is_disabled());
    assert_eq!(
        button
            .accesskit_node()
            .description()
            .map(|text| text.to_string()),
        Some("Locked by the mixer".to_owned())
    );
    assert!(!painted(&harness, "Locked by the mixer"));
    click_node(&mut harness, GAIN);
    harness.run_steps(4);
    assert!(focused_node(&harness).is_none(), "nothing opens");
    assert!(harness.state().app.edits.is_empty());
    assert!(
        painted(&harness, "Locked by the mixer"),
        "the press shows it"
    );
}

#[test]
fn without_a_reason_it_is_not_refused() {
    let mut harness = harness_with(App::default());
    let button = harness.get_by_role_and_label(Role::Button, "Edit Gain");
    assert!(!button.accesskit_node().is_disabled());
    assert_eq!(button.accesskit_node().description(), None);
    click_node(&mut harness, GAIN);
    harness.run_steps(4);
    assert_eq!(focused_node(&harness), Some(format!("{GAIN}/Field")));
}
