//! The form through the full path: Tab order, typing, inert leaves, the
//! selects, toggle buttons and swatches, with accessible names checked.

use egui::{Key, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/form.rs"]
mod form;

use crate::support;

use form::{Footer, Mark, Notifications, Preferences, Profile, Section, State, Theme, layout};
use std::fmt::Debug;
use support::{Rig, click_node, focused_node, rect_of};

const WINDOW: Vec2 = vec2(760.0, 900.0);

fn harness_with(state: State) -> Harness<'static, Rig<State>> {
    let mut harness =
        support::harness(WINDOW, common::base_text(egui::Theme::Light), state, layout);
    harness.run_steps(3);
    harness
}

fn harness() -> Harness<'static, Rig<State>> {
    harness_with(State::default())
}

fn path(section: Section, part: impl Debug) -> String {
    format!("/{section:?}/{part:?}")
}

/// The track of the switch named `part`, which is what is focused and clicked.
fn track_path(part: Notifications) -> String {
    format!("/Notifications/{part:?}/Track")
}

/// The leaf of a text field, which sits inside the box named `part`.
fn field_path(part: Profile) -> String {
    format!("/Profile/{part:?}/\"field\"")
}

fn type_text(harness: &mut Harness<'_, Rig<State>>, text: &str) {
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run_steps(3);
}

/// Clicks the field `part` and types `text` into it.
fn fill(harness: &mut Harness<'_, Rig<State>>, part: Profile, text: &str) {
    click_node(harness, &path(Section::Profile, part));
    harness.run_steps(2);
    type_text(harness, text);
}

/// Every Tab stop, in order, until the first comes round again.
fn tab_stops(harness: &mut Harness<'_, Rig<State>>) -> Vec<String> {
    let mut order = Vec::new();
    for _ in 0..40 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        let stop = focused_node(harness).expect("Tab always lands on a stop");
        if order.first() == Some(&stop) {
            break;
        }
        order.push(stop);
    }
    order
}

#[test]
fn tab_visits_the_controls_top_to_bottom_and_skips_what_is_inert() {
    let mut harness = harness();
    let order = tab_stops(&mut harness);

    let expected: Vec<String> = [Profile::Name, Profile::Email, Profile::Bio]
        .into_iter()
        .map(field_path)
        .chain([Notifications::EmailAlerts, Notifications::PushAlerts].map(track_path))
        .chain([path(Section::Preferences, Preferences::Language)])
        .chain((0..3).map(|cell| format!("/Preferences/\"theme\"/{cell}")))
        .chain(Mark::ALL.map(|mark| path(Section::Preferences, Preferences::Mark(mark))))
        .chain(
            form::SWATCHES
                .map(|swatch| path(Section::Preferences, Preferences::Accent(swatch.name))),
        )
        .collect();
    assert_eq!(order, expected);

    let tops: Vec<f32> = order
        .iter()
        .map(|stop| rect_of(&harness, stop).min.y)
        .collect();
    assert!(
        tops.is_sorted(),
        "the order is the visual order, top to bottom: {tops:?}"
    );
    for skipped in [
        track_path(Notifications::Beta),
        path(Section::Preferences, Preferences::Region),
        path(Section::Footer, Footer::Save),
    ] {
        assert!(
            rect_of(&harness, &skipped).width() > 0.0,
            "precondition: {skipped:?} is on screen"
        );
        assert!(!order.contains(&skipped), "{skipped:?} is inert");
    }
}

#[test]
fn save_becomes_a_tab_stop_once_the_form_can_be_saved() {
    let mut harness = harness();
    fill(&mut harness, Profile::Name, "Ada");
    fill(&mut harness, Profile::Email, "ada@example.com");
    assert!(harness.state().app.can_save());
    harness.run_steps(2);

    let order = tab_stops(&mut harness);
    let at = |stop: String| order.iter().position(|found| *found == stop);
    let last_swatch = at(path(Section::Preferences, Preferences::Accent("Plum")))
        .expect("the swatches are stops");
    assert_eq!(
        at(path(Section::Footer, Footer::Save)),
        Some(last_swatch + 1),
        "{order:?}"
    );
}

#[test]
fn typing_fills_the_fields_and_save_counts_once_it_is_allowed() {
    let mut harness = harness();
    click_node(&mut harness, &path(Section::Footer, Footer::Save));
    harness.run_steps(2);
    assert_eq!(harness.state().app.saves, 0, "Save is inert while empty");

    fill(&mut harness, Profile::Name, "Ada Lovelace");
    assert_eq!(harness.state().app.name, "Ada Lovelace");
    click_node(&mut harness, &path(Section::Footer, Footer::Save));
    harness.run_steps(2);
    assert_eq!(harness.state().app.saves, 0, "an email is needed too");

    fill(&mut harness, Profile::Email, "ada@example.com");
    click_node(&mut harness, &path(Section::Footer, Footer::Save));
    harness.run_steps(3);
    assert_eq!(harness.state().app.saves, 1);
}

#[test]
fn a_switch_toggles_and_the_inert_one_does_not() {
    let mut harness = harness();
    assert!(harness.state().app.email_alerts, "precondition");
    click_node(&mut harness, &track_path(Notifications::EmailAlerts));
    harness.run_steps(2);
    assert!(!harness.state().app.email_alerts);

    click_node(&mut harness, &track_path(Notifications::PushAlerts));
    harness.run_steps(2);
    assert!(harness.state().app.push_alerts);

    click_node(&mut harness, &track_path(Notifications::Beta));
    harness.run_steps(2);
    assert!(!harness.state().app.beta, "inert");
}

#[test]
fn a_select_chooses_and_the_inert_one_does_not_open() {
    let mut harness = harness();
    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Language),
    );
    harness.run_steps(3);
    harness.get_by_label("Deutsch").click();
    harness.run_steps(3);
    assert_eq!(harness.state().app.language, 1);

    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Region),
    );
    harness.run_steps(3);
    assert!(
        harness.query_by_label("Asia Pacific").is_none(),
        "the inert select's popup did not open"
    );
    assert_eq!(harness.state().app.region, 0);
}

#[test]
fn the_selects_and_fields_keep_their_widths_whatever_they_show() {
    let mut harness = harness();
    let widths = |harness: &Harness<'_, Rig<State>>| {
        [
            path(Section::Profile, Profile::Name),
            path(Section::Preferences, Preferences::Language),
            path(Section::Preferences, Preferences::Region),
        ]
        .map(|stop| rect_of(harness, &stop).width())
    };
    let before = widths(&harness);
    harness.state_mut().app.language = 3;
    harness.state_mut().app.name = "x".repeat(200);
    harness.run_steps(3);
    assert_eq!(widths(&harness), before);
}

#[test]
fn the_bio_grows_with_its_lines_up_to_six_rows() {
    let mut harness = harness();
    let height = |harness: &Harness<'_, Rig<State>>| {
        rect_of(harness, &path(Section::Profile, Profile::Bio)).height()
    };
    let mut heights = Vec::new();
    for lines in [1, 3, 4, 6, 8, 12] {
        harness.state_mut().app.bio = vec!["line"; lines].join("\n");
        harness.run_steps(3);
        heights.push(height(&harness));
    }
    assert_eq!(heights[0], heights[1], "three rows are the minimum");
    assert!(heights[2] > heights[1], "{heights:?}");
    assert!(heights[3] > heights[2], "{heights:?}");
    assert_eq!(heights[4], heights[5], "capped: {heights:?}");
    assert!(
        heights[3] <= heights[4],
        "six rows fit the cap: {heights:?}"
    );
}

#[test]
fn a_segmented_control_chooses_the_theme() {
    let mut harness = harness();
    assert_eq!(harness.state().app.theme, Theme::System);
    click_node(&mut harness, "/Preferences/\"theme\"/1");
    harness.run_steps(2);
    assert_eq!(harness.state().app.theme, Theme::Dark);
}

#[test]
fn toggle_buttons_are_independent_and_the_keyboard_presses_them() {
    let mut harness = harness();
    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Mark(Mark::Bold)),
    );
    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Mark(Mark::Italic)),
    );
    harness.run_steps(2);
    assert_eq!(harness.state().app.marks, [Mark::Bold, Mark::Italic]);

    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Mark(Mark::Bold)),
    );
    harness.run_steps(2);
    assert_eq!(harness.state().app.marks, [Mark::Italic]);

    let target = path(Section::Preferences, Preferences::Mark(Mark::Underline));
    for _ in 0..40 {
        if focused_node(&harness).as_ref() == Some(&target) {
            break;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert_eq!(focused_node(&harness), Some(target), "precondition");
    harness.key_press(Key::Space);
    harness.run_steps(2);
    assert_eq!(harness.state().app.marks, [Mark::Italic, Mark::Underline]);
}

#[test]
fn exactly_one_swatch_is_the_accent() {
    let mut harness = harness();
    let plum = form::SWATCHES[7];
    assert_ne!(harness.state().app.accent, plum, "precondition");
    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Accent(plum.name)),
    );
    harness.run_steps(2);
    assert_eq!(harness.state().app.accent, plum);
    click_node(
        &mut harness,
        &path(Section::Preferences, Preferences::Accent("Sage")),
    );
    harness.run_steps(2);
    assert_eq!(harness.state().app.accent.name, "Sage");
}

/// Whether a rect stroked in the focus colour that surrounds `node` was
/// painted last frame.
fn rings(harness: &Harness<'_, Rig<State>>, node: &str) -> bool {
    let node = rect_of(harness, node);
    let ring = common::ring(&common::tokens(&harness.ctx));
    harness.output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::epaint::Shape::Rect(rect)
            if rect.stroke.color == ring && rect.rect.expand(1.0).contains_rect(node))
    })
}

#[test]
fn a_focused_field_rings_its_box_and_a_focused_toggle_rings_itself() {
    let mut harness = harness();
    let (email, bold) = (
        path(Section::Profile, Profile::Email),
        path(Section::Preferences, Preferences::Mark(Mark::Bold)),
    );
    assert!(!rings(&harness, &email), "precondition: nothing is focused");

    click_node(&mut harness, &email);
    harness.run_steps(3);
    assert_eq!(focused_node(&harness), Some(field_path(Profile::Email)));
    assert!(
        rings(&harness, &email),
        "the box rings while its field has focus"
    );
    assert!(!rings(&harness, &bold));

    for _ in 0..40 {
        if focused_node(&harness).as_ref() == Some(&bold) {
            break;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    assert_eq!(focused_node(&harness), Some(bold.clone()), "precondition");
    assert!(rings(&harness, &bold));
    assert!(!rings(&harness, &email), "the field's box let go");
}
