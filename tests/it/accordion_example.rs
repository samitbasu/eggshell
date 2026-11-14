//! The `accordion` example: opening a section closes the open one, the list
//! below moves with both, and the whole is continuous.

use eggshell_ui::testing::{Timeline, record};
use egui::accesskit::Role as AccessRole;
use egui::epaint::Shape;
use egui::{Key, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/accordion.rs"]
mod example;

use example::State;

const ROW: &str = r#"/Page/List/Row(0)"#;

fn body(index: usize) -> String {
    format!("/Page/Accordion/Item({index})/Body({index})")
}

fn harness() -> egui_kittest::Harness<'static, support::Rig<State>> {
    let mut harness = support::harness_stepped(
        vec2(500.0, 500.0),
        0.01,
        common::base_text(egui::Theme::Light),
        State::default(),
        example::layout,
    );
    harness.run_steps(4);
    harness
}

fn height(harness: &egui_kittest::Harness<'_, support::Rig<State>>, index: usize) -> f32 {
    rect_of(harness, &body(index)).height()
}

#[test]
fn opening_a_section_closes_the_open_one_and_the_list_follows() {
    let mut harness = harness();
    record(&harness.ctx);
    harness.state_mut().app.toggle(0);
    harness.run_steps(40);
    let first = height(&harness, 0);
    assert!(
        first > 20.0,
        "precondition: the first section is open: {first}"
    );
    let list_with_first = rect_of(&harness, ROW).min.y;

    harness.state_mut().app.toggle(1);
    harness.run_steps(8);
    let (closing, opening) = (height(&harness, 0), height(&harness, 1));
    assert!(
        closing > 0.0 && closing < first && opening > 0.0,
        "precondition: both are moving: {closing} {opening}"
    );
    harness.run_steps(40);
    assert_eq!(height(&harness, 0), 0.0);
    let second = height(&harness, 1);
    assert!(second > 20.0, "{second}");
    assert_eq!(
        rect_of(&harness, ROW).min.y,
        list_with_first - first + second
    );

    Timeline::recorded(&harness.ctx).assert_continuous();
}

fn head(index: usize) -> String {
    format!("/Page/Accordion/Item({index})/Head({index})")
}

/// The turn of the chevron painted in header `index`, in degrees from
/// pointing down, read off the two segments it is drawn with.
fn chevron_degrees(harness: &egui_kittest::Harness<'_, support::Rig<State>>, index: usize) -> f32 {
    let header = rect_of(harness, &head(index));
    let segments: Vec<[Pos2; 2]> = support::shapes(harness)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, .. } if header.contains(points[0]) => Some(*points),
            _ => None,
        })
        .collect();
    assert_eq!(
        segments.len(),
        2,
        "the chevron is two segments: {segments:?}"
    );
    let vertex = segments[0][1];
    let ends = segments[0][0].lerp(segments[1][1], 0.5);
    let v = vertex - ends;
    (-v.x).atan2(v.y).to_degrees().rem_euclid(360.0)
}

#[test]
fn the_chevron_turns_with_the_section_by_the_same_progress() {
    let mut harness = harness();
    harness.state_mut().app.toggle(0);
    harness.run_steps(40);
    let natural = height(&harness, 0);
    assert_eq!(
        chevron_degrees(&harness, 0).round(),
        180.0,
        "open points up"
    );
    assert_eq!(
        chevron_degrees(&harness, 1).round() % 360.0,
        0.0,
        "closed points down"
    );
    harness.state_mut().app.toggle(0);
    let mut between = 0;
    let mut last = 180.0;
    for _ in 0..40 {
        harness.step();
        let progress = height(&harness, 0) / natural;
        let angle = chevron_degrees(&harness, 0);
        assert!(
            (angle - 180.0 * progress).abs() < 3.0,
            "{angle} degrees at progress {progress}"
        );
        assert!(
            angle <= last + 0.01,
            "it only turns back: {last} then {angle}"
        );
        last = angle;
        between += usize::from(angle > 20.0 && angle < 160.0);
    }
    assert!(between > 5, "precondition: it was in between frames");
}

#[test]
fn a_toggle_in_the_middle_turns_the_chevron_back_from_where_it_is() {
    let mut harness = harness();
    harness.state_mut().app.toggle(0);
    harness.run_steps(8);
    let before = chevron_degrees(&harness, 0);
    assert!(before > 30.0 && before < 170.0, "precondition: {before}");
    harness.state_mut().app.toggle(0);
    harness.step();
    let after = chevron_degrees(&harness, 0);
    assert!(
        (after - before).abs() < 25.0,
        "no jump: {before} then {after}"
    );
    harness.run_steps(40);
    assert_eq!(chevron_degrees(&harness, 0).round() % 360.0, 0.0);
}

#[test]
fn a_style_without_animation_time_snaps_the_chevron() {
    let mut harness = harness();
    harness.state_mut().reduced_motion = support::ReducedMotion::Yes;
    harness
        .ctx
        .global_style_mut(|style| style.animation_time = 0.0);
    harness.state_mut().app.toggle(0);
    harness.step();
    harness.step();
    assert_eq!(chevron_degrees(&harness, 0).round(), 180.0);
    harness.state_mut().app.toggle(0);
    harness.step();
    harness.step();
    assert_eq!(chevron_degrees(&harness, 0).round() % 360.0, 0.0);
}

#[test]
fn a_header_is_a_button_that_says_whether_its_section_is_open() {
    let mut harness = harness();
    let expanded = |harness: &egui_kittest::Harness<'_, support::Rig<State>>, title: &str| {
        harness
            .get_by_role_and_label(AccessRole::Button, title)
            .accesskit_node()
            .data()
            .is_expanded()
    };
    assert_eq!(expanded(&harness, "Is it animated?"), Some(false));
    harness.state_mut().app.toggle(1);
    harness.run_steps(4);
    assert_eq!(expanded(&harness, "Is it animated?"), Some(true));
    assert_eq!(expanded(&harness, "Is it accessible?"), Some(false));
}

#[test]
fn enter_and_space_on_a_focused_header_toggle_its_section() {
    let mut harness = harness();
    for key in [Key::Enter, Key::Space] {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        harness
            .get_by_role_and_label(AccessRole::Button, "Is it accessible?")
            .focus();
        harness.run_steps(2);
        let before = harness.state().app.open;
        harness.key_press(key);
        harness.run_steps(40);
        assert_ne!(harness.state().app.open, before, "{key:?} toggles");
    }
}
