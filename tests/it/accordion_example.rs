//! The `accordion` example: opening a section closes the open one, the list
//! below moves with both, and the whole is continuous.

use std::time::Duration;

use eggshell_ui::testing::{Animation, Driver, Target, Timeline, record};
use egui::accesskit::Role as AccessRole;
use egui::{Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/accordion.rs"]
mod example;

use example::State;

const ROW: &str = r#"/Page/List/Row(0)"#;

fn body(index: usize) -> String {
    format!("/Page/Accordion/{index}/Region")
}

fn harness() -> Driver<State> {
    let mut d = support::driver(
        vec2(500.0, 500.0),
        common::base_text(egui::Theme::Light),
        State::default(),
        example::view,
    )
    .step(Duration::from_millis(10))
    .settle_within(Duration::from_millis(40))
    .build();
    d.frames(4);
    d
}

fn height(d: &Driver<State>, index: usize) -> f32 {
    d.rect(Target::suffix(&body(index))).height()
}

#[test]
fn opening_a_section_closes_the_open_one_and_the_list_follows() {
    let mut d = harness();
    record(d.ctx());
    d.app_mut().toggle(0);
    d.frames(40);
    let first = height(&d, 0);
    assert!(
        first > 20.0,
        "precondition: the first section is open: {first}"
    );
    let list_with_first = d.rect(Target::suffix(ROW)).min.y;

    d.app_mut().toggle(1);
    d.frames(8);
    let (closing, opening) = (height(&d, 0), height(&d, 1));
    assert!(
        closing > 0.0 && closing < first && opening > 0.0,
        "precondition: both are moving: {closing} {opening}"
    );
    d.frames(40);
    assert_eq!(height(&d, 0), 0.0);
    let second = height(&d, 1);
    assert!(second > 20.0, "{second}");
    assert_eq!(
        d.rect(Target::suffix(ROW)).min.y,
        list_with_first - first + second
    );

    Timeline::recorded(d.ctx()).assert_continuous();
}

fn head(index: usize) -> String {
    format!("/Page/Accordion/{index}/Header")
}

/// The turn of the chevron painted in header `index`, in degrees from
/// pointing down, read off the icon's quad.
fn chevron_degrees(d: &Driver<State>, index: usize) -> f32 {
    let header = d.rect(Target::suffix(&head(index)));
    let icons: Vec<_> = support::textured(d)
        .into_iter()
        .filter(|mesh| header.contains(mesh.calc_bounds().center()))
        .collect();
    assert_eq!(icons.len(), 1, "the header draws one chevron");
    support::turn(icons[0])
}

#[test]
fn the_chevron_turns_with_the_section_by_the_same_progress() {
    let mut d = harness();
    d.app_mut().toggle(0);
    d.frames(40);
    let natural = height(&d, 0);
    assert_eq!(chevron_degrees(&d, 0).round(), 180.0, "open points up");
    assert_eq!(
        chevron_degrees(&d, 1).round() % 360.0,
        0.0,
        "closed points down"
    );
    d.app_mut().toggle(0);
    let mut between = 0;
    let mut last = 180.0;
    for _ in 0..40 {
        d.frames(1);
        let progress = height(&d, 0) / natural;
        let angle = chevron_degrees(&d, 0);
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
    let mut d = harness();
    d.app_mut().toggle(0);
    d.frames(8);
    let before = chevron_degrees(&d, 0);
    assert!(before > 30.0 && before < 170.0, "precondition: {before}");
    d.app_mut().toggle(0);
    d.frames(1);
    let after = chevron_degrees(&d, 0);
    assert!(
        (after - before).abs() < 25.0,
        "no jump: {before} then {after}"
    );
    d.frames(40);
    assert_eq!(chevron_degrees(&d, 0).round() % 360.0, 0.0);
}

#[test]
fn a_style_without_animation_time_snaps_the_chevron() {
    let mut d = harness();
    d.set_motion(Animation::Reduced);
    d.ctx().global_style_mut(|style| style.animation_time = 0.0);
    d.app_mut().toggle(0);
    d.frames(1);
    d.frames(1);
    assert_eq!(chevron_degrees(&d, 0).round(), 180.0);
    d.app_mut().toggle(0);
    d.frames(1);
    d.frames(1);
    assert_eq!(chevron_degrees(&d, 0).round() % 360.0, 0.0);
}

#[test]
fn a_header_is_a_button_that_says_whether_its_section_is_open() {
    let mut d = harness();
    let expanded = |d: &Driver<State>, title: &str| {
        d.harness()
            .get_by_role_and_label(AccessRole::Button, title)
            .accesskit_node()
            .data()
            .is_expanded()
    };
    assert_eq!(expanded(&d, "Is it animated?"), Some(false));
    d.app_mut().toggle(1);
    d.frames(4);
    assert_eq!(expanded(&d, "Is it animated?"), Some(true));
    assert_eq!(expanded(&d, "Is it accessible?"), Some(false));
}

#[test]
fn enter_and_space_on_a_focused_header_toggle_its_section() {
    let mut d = harness();
    for key in [Key::Enter, Key::Space] {
        d.press(Key::Tab).frames(1);
        d.harness()
            .get_by_role_and_label(AccessRole::Button, "Is it accessible?")
            .focus();
        d.frames(2);
        let before = d.app().open;
        d.press(key).frames(39);
        assert_ne!(d.app().open, before, "{key:?} toggles");
    }
}
