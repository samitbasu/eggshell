//! The form through the full path: Tab order, typing, inert leaves, the
//! selects, toggle buttons and swatches, with accessible names checked.

use eggshell_ui::testing::{Driver, Target};
use egui::{Key, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/form.rs"]
mod form;

use crate::support;

use form::{Footer, Mark, Notifications, Preferences, Profile, Section, State, Theme, layout};
use std::fmt::Debug;
use support::focused_node;

const WINDOW: Vec2 = vec2(760.0, 900.0);

fn harness_with(state: State) -> Driver<State> {
    let mut d =
        support::driver(WINDOW, common::base_text(egui::Theme::Light), state, layout).build();
    d.frames(3);
    d
}

fn harness() -> Driver<State> {
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

/// Clicks the field `part` and types `text` into it.
fn fill(d: &mut Driver<State>, part: Profile, text: &str) {
    d.click(Target::suffix(&path(Section::Profile, part)))
        .frames(2);
    d.type_text(text).frames(2);
}

/// Every Tab stop, in order, until the first comes round again.
fn tab_stops(d: &mut Driver<State>) -> Vec<String> {
    let mut order = Vec::new();
    support::drive_past_bar(d);
    for _ in 0..40 {
        d.press(Key::Tab).frames(1);
        let Some(stop) = focused_node(d) else {
            break;
        };
        if order.first() == Some(&stop) {
            break;
        }
        order.push(stop);
    }
    order
}

#[test]
fn tab_visits_the_controls_top_to_bottom_and_skips_what_is_inert() {
    let mut d = harness();
    let order = tab_stops(&mut d);

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
        .map(|stop| d.rect(Target::suffix(stop)).min.y)
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
            d.rect(Target::suffix(&skipped)).width() > 0.0,
            "precondition: {skipped:?} is on screen"
        );
        assert!(!order.contains(&skipped), "{skipped:?} is inert");
    }
}

#[test]
fn save_becomes_a_tab_stop_once_the_form_can_be_saved() {
    let mut d = harness();
    fill(&mut d, Profile::Name, "Ada");
    fill(&mut d, Profile::Email, "ada@example.com");
    assert!(d.app().can_save());
    d.frames(2);

    let order = tab_stops(&mut d);
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
    let mut d = harness();
    d.click(Target::suffix(&path(Section::Footer, Footer::Save)))
        .frames(2);
    assert_eq!(d.app().saves, 0, "Save is inert while empty");

    fill(&mut d, Profile::Name, "Ada Lovelace");
    assert_eq!(d.app().name, "Ada Lovelace");
    d.click(Target::suffix(&path(Section::Footer, Footer::Save)))
        .frames(2);
    assert_eq!(d.app().saves, 0, "an email is needed too");

    fill(&mut d, Profile::Email, "ada@example.com");
    d.click(Target::suffix(&path(Section::Footer, Footer::Save)))
        .frames(3);
    assert_eq!(d.app().saves, 1);
}

#[test]
fn a_switch_toggles_and_the_inert_one_does_not() {
    let mut d = harness();
    assert!(d.app().email_alerts, "precondition");
    d.click(Target::suffix(&track_path(Notifications::EmailAlerts)))
        .frames(2);
    assert!(!d.app().email_alerts);

    d.click(Target::suffix(&track_path(Notifications::PushAlerts)))
        .frames(2);
    assert!(d.app().push_alerts);

    d.click(Target::suffix(&track_path(Notifications::Beta)))
        .frames(2);
    assert!(!d.app().beta, "inert");
}

#[test]
fn a_select_chooses_and_the_inert_one_does_not_open() {
    let mut d = harness();
    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Language,
    )))
    .frames(3);
    d.harness().get_by_label("Deutsch").click();
    d.frames(3);
    assert_eq!(d.app().language, 1);

    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Region,
    )))
    .frames(3);
    assert!(
        d.harness().query_by_label("Asia Pacific").is_none(),
        "the inert select's popup did not open"
    );
    assert_eq!(d.app().region, 0);
}

#[test]
fn the_selects_and_fields_keep_their_widths_whatever_they_show() {
    let mut d = harness();
    let widths = |d: &Driver<State>| {
        [
            path(Section::Profile, Profile::Name),
            path(Section::Preferences, Preferences::Language),
            path(Section::Preferences, Preferences::Region),
        ]
        .map(|stop| d.rect(Target::suffix(&stop)).width())
    };
    let before = widths(&d);
    d.app_mut().language = 3;
    d.app_mut().name = "x".repeat(200);
    d.frames(3);
    assert_eq!(widths(&d), before);
}

#[test]
fn the_bio_grows_with_its_lines_up_to_six_rows() {
    let mut d = harness();
    let height = |d: &Driver<State>| {
        d.rect(Target::suffix(&path(Section::Profile, Profile::Bio)))
            .height()
    };
    let mut heights = Vec::new();
    for lines in [1, 3, 4, 6, 8, 12] {
        d.app_mut().bio = vec!["line"; lines].join("\n");
        d.frames(3);
        heights.push(height(&d));
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
    let mut d = harness();
    assert_eq!(d.app().theme, Theme::System);
    d.click(Target::suffix("/Preferences/\"theme\"/1"))
        .frames(2);
    assert_eq!(d.app().theme, Theme::Dark);
}

#[test]
fn toggle_buttons_are_independent_and_the_keyboard_presses_them() {
    let mut d = harness();
    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Mark(Mark::Bold),
    )));
    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Mark(Mark::Italic),
    )))
    .frames(2);
    assert_eq!(d.app().marks, [Mark::Bold, Mark::Italic]);

    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Mark(Mark::Bold),
    )))
    .frames(2);
    assert_eq!(d.app().marks, [Mark::Italic]);

    let target = path(Section::Preferences, Preferences::Mark(Mark::Underline));
    for _ in 0..40 {
        if focused_node(&d).as_ref() == Some(&target) {
            break;
        }
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(focused_node(&d), Some(target), "precondition");
    d.press(Key::Space).frames(1);
    assert_eq!(d.app().marks, [Mark::Italic, Mark::Underline]);
}

#[test]
fn exactly_one_swatch_is_the_accent() {
    let mut d = harness();
    let plum = form::SWATCHES[7];
    assert_ne!(d.app().accent, plum, "precondition");
    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Accent(plum.name),
    )))
    .frames(2);
    assert_eq!(d.app().accent, plum);
    d.click(Target::suffix(&path(
        Section::Preferences,
        Preferences::Accent("Sage"),
    )))
    .frames(2);
    assert_eq!(d.app().accent.name, "Sage");
}

/// Whether a rect stroked in the focus colour that surrounds `node` was
/// painted last frame.
fn rings(d: &Driver<State>, node: &str) -> bool {
    let node = d.rect(Target::suffix(node));
    let ring = common::ring(&common::tokens(d.ctx()));
    d.harness().output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::epaint::Shape::Rect(rect)
            if rect.stroke.color == ring && rect.rect.expand(1.0).contains_rect(node))
    })
}

#[test]
fn a_focused_field_rings_its_box_and_a_focused_toggle_rings_itself() {
    let mut d = harness();
    let (email, bold) = (
        path(Section::Profile, Profile::Email),
        path(Section::Preferences, Preferences::Mark(Mark::Bold)),
    );
    assert!(!rings(&d, &email), "precondition: nothing is focused");

    d.click(Target::suffix(&email)).frames(3);
    assert_eq!(focused_node(&d), Some(field_path(Profile::Email)));
    assert!(rings(&d, &email), "the box rings while its field has focus");
    assert!(!rings(&d, &bold));

    for _ in 0..40 {
        if focused_node(&d).as_ref() == Some(&bold) {
            break;
        }
        d.press(Key::Tab).frames(1);
    }
    assert_eq!(focused_node(&d), Some(bold.clone()), "precondition");
    assert!(rings(&d, &bold));
    assert!(!rings(&d, &email), "the field's box let go");
}
