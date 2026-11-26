//! `slider::Chain::typed_range` (#328): the editable readout reaches a range
//! wider than the track's; the thumb pins at its end, the arrow keys step
//! from where the value is, and the step's lattice stays the track's.

use eggshell_ui::owned::number::{Step, ValueRange};
use eggshell_ui::prelude::*;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Event, Key, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, rect_of};

struct App {
    gain: f64,
    drag: ValueRange,
    typed: Option<ValueRange>,
    step: Step,
}

impl Default for App {
    fn default() -> Self {
        Self {
            gain: -20.0,
            drag: ValueRange::from(-60.0..=0.0),
            typed: Some(ValueRange::from(-380.0..=380.0)),
            step: Step::ONE,
        }
    }
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().p(4.0).w_px(300.0), |b| {
        let slider = b
            .slider("gain", app.gain, app.drag)
            .named("Gain")
            .step(app.step)
            .readout(|value| format!("{value:.0}"))
            .editable(|text| text.trim().parse().ok());
        let slider = match app.typed {
            Some(typed) => slider.typed_range(typed),
            None => slider,
        };
        slider.on_change(|app: &mut App, value| app.gain = value);
    });
}

const READOUT: &str = r#"/"gain"/Box/Header/Readout"#;
const THUMB: &str = r#"/"gain"/Box/Track/Thumb"#;
const TRACK: &str = r#"/"gain"/Box/Track"#;

fn harness_with(app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.run_steps(6);
    harness
}

/// Opens the readout, replaces its text with `text` and presses Enter.
fn type_in(harness: &mut Harness<'_, Rig<App>>, text: &str) {
    click_node(harness, READOUT);
    harness.run_steps(4);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(3);
    harness.key_press(Key::Enter);
    harness.run_steps(6);
}

#[test]
fn typing_reaches_the_typed_range_and_is_clamped_to_it() {
    let mut harness = harness_with(App::default());
    type_in(&mut harness, "300");
    assert_eq!(harness.state().app.gain, 300.0);
    type_in(&mut harness, "500");
    assert_eq!(harness.state().app.gain, 380.0);
}

#[test]
fn without_a_typed_range_typing_is_clamped_to_the_track() {
    let mut harness = harness_with(App {
        typed: None,
        ..App::default()
    });
    type_in(&mut harness, "300");
    assert_eq!(harness.state().app.gain, 0.0);
}

#[test]
fn the_thumb_pins_at_the_end_of_the_track() {
    let harness = harness_with(App {
        gain: 300.0,
        ..App::default()
    });
    let (thumb, track) = (rect_of(&harness, THUMB), rect_of(&harness, TRACK));
    assert!(
        (thumb.right() - track.right()).abs() < 0.5,
        "{thumb:?} in {track:?}"
    );
}

#[test]
fn the_step_lattice_is_anchored_at_the_tracks_start() {
    let mut harness = harness_with(App {
        gain: 20.0,
        drag: ValueRange::from(1.0..=50.0),
        typed: Some(ValueRange::from(-100.0..=100.0)),
        step: Step::new(7.0),
    });
    type_in(&mut harness, "0");
    assert_eq!(
        harness.state().app.gain,
        1.0,
        "1 + 7k nearest 0 is 1; a lattice from -100 would give -2"
    );
}

#[test]
fn the_arrow_keys_step_from_where_the_value_is_and_home_end_go_to_the_track() {
    let mut harness = harness_with(App {
        gain: 300.0,
        ..App::default()
    });
    harness.get_by_label("Gain").focus();
    harness.run_steps(3);
    harness.key_press(Key::ArrowLeft);
    harness.run_steps(3);
    assert_eq!(harness.state().app.gain, 299.0, "one step, not a jump to 0");
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(3);
    assert_eq!(harness.state().app.gain, 301.0);
    harness.key_press(Key::End);
    harness.run_steps(3);
    assert_eq!(harness.state().app.gain, 0.0, "End is the track's end");
}

#[test]
fn the_accessible_range_widens_to_hold_the_value() {
    let harness = harness_with(App {
        gain: 300.0,
        ..App::default()
    });
    let node = harness.get_by_label("Gain");
    assert_eq!(node.accesskit_node().numeric_value(), Some(300.0));
    assert_eq!(node.accesskit_node().min_numeric_value(), Some(-60.0));
    assert_eq!(node.accesskit_node().max_numeric_value(), Some(300.0));
}

#[test]
fn a_typed_range_that_misses_part_of_the_track_is_reported_and_widened() {
    let mut harness = harness_with(App {
        typed: Some(ValueRange::from(-10.0..=10.0)),
        ..App::default()
    });
    let report =
        eggshell_ui::testing::assert_reported(&harness.ctx, eggshell_ui::Problem::BadRange);
    assert!(report.message.contains("typed range"), "{}", report.message);
    type_in(&mut harness, "-50");
    assert_eq!(
        harness.state().app.gain,
        -50.0,
        "the track's start still reaches"
    );
    type_in(&mut harness, "50");
    assert_eq!(harness.state().app.gain, 10.0, "the typed end holds");
}
