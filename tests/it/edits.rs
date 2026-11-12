//! `on_edit` of the slider and the stepper: one Began and one Ended around
//! the values a gesture sets, through kittest's real pointer and key events.

use eggshell_ui::owned::Edit;
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Event, Key, Theme, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, click_node, drag, press_and_release, rect_of};

#[derive(Debug, PartialEq)]
enum Logged {
    Edit(Edit),
    Value(f64),
}

#[derive(Default)]
struct App {
    log: Vec<Logged>,
}

fn view(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(
        tw().flex()
            .flex_col()
            .items_start()
            .gap(8.0)
            .p(8.0)
            .w_px(300.0),
        |b| {
            b.slider("slider", 50.0, 0.0..=100.0)
                .named("Level")
                .on_edit(|app: &mut App, edit| app.log.push(Logged::Edit(edit)))
                .on_change(|app: &mut App, value| app.log.push(Logged::Value(value)));
            b.stepper("stepper", 5.0, 0.0..=10.0)
                .named("Count")
                .on_edit(|app: &mut App, edit| app.log.push(Logged::Edit(edit)))
                .on_change(|app: &mut App, value| app.log.push(Logged::Value(value)));
        },
    );
}

fn harness() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.run_steps(8);
    harness
}

fn log<'h>(harness: &'h Harness<'_, Rig<App>>) -> &'h [Logged] {
    &harness.state().app.log
}

fn edits(harness: &Harness<'_, Rig<App>>) -> Vec<Edit> {
    log(harness)
        .iter()
        .filter_map(|logged| match logged {
            Logged::Edit(edit) => Some(*edit),
            Logged::Value(_) => None,
        })
        .collect()
}

fn assert_bracketed(harness: &Harness<'_, Rig<App>>) {
    let log = log(harness);
    assert!(
        log.iter().any(|logged| matches!(logged, Logged::Value(_))),
        "precondition: the gesture set a value: {log:?}"
    );
    assert_eq!(edits(harness), [Edit::Began, Edit::Ended], "{log:?}");
    assert_eq!(log.first(), Some(&Logged::Edit(Edit::Began)), "{log:?}");
    assert_eq!(log.last(), Some(&Logged::Edit(Edit::Ended)), "{log:?}");
}

#[test]
fn dragging_the_thumb_is_one_edit() {
    let mut harness = harness();
    let thumb = rect_of(&harness, "/\"slider\"/Track/Thumb").center();
    let track = rect_of(&harness, "/\"slider\"/Track");
    drag(&mut harness, thumb, pos2(track.left() + 60.0, thumb.y));
    assert_bracketed(&harness);
}

#[test]
fn a_press_that_starts_on_the_track_and_drags_is_one_edit() {
    let mut harness = harness();
    let thumb = rect_of(&harness, "/\"slider\"/Track/Thumb");
    let track = rect_of(&harness, "/\"slider\"/Track");
    let start = pos2(track.right() - 20.0, track.center().y);
    assert!(
        !thumb.contains(start),
        "precondition: the press misses the thumb"
    );
    drag(&mut harness, start, pos2(track.left() + 80.0, start.y));
    assert_bracketed(&harness);
}

#[test]
fn a_click_on_the_track_is_one_edit() {
    let mut harness = harness();
    let track = rect_of(&harness, "/\"slider\"/Track");
    press_and_release(&mut harness, pos2(track.left() + 30.0, track.center().y));
    harness.run_steps(3);
    assert_bracketed(&harness);
}

#[test]
fn each_key_press_is_one_edit() {
    let mut harness = harness();
    harness.get_by_label("Level").focus();
    harness.run_steps(3);
    assert!(edits(&harness).is_empty(), "focusing is not an edit");
    for _ in 0..2 {
        harness.key_press(Key::ArrowRight);
        harness.run_steps(3);
    }
    assert_eq!(
        edits(&harness),
        [Edit::Began, Edit::Ended, Edit::Began, Edit::Ended]
    );
}

#[test]
fn stepper_buttons_are_one_edit_each() {
    let mut harness = harness();
    click_node(&mut harness, "/\"stepper\"/Box/Buttons/Up");
    harness.run_steps(3);
    assert_bracketed(&harness);
}

#[test]
fn typing_in_the_stepper_is_one_edit_until_the_field_loses_focus() {
    let mut harness = harness();
    click_node(&mut harness, "/\"stepper\"/Box/Field");
    harness.run_steps(3);
    assert!(edits(&harness).is_empty(), "focusing is not an edit");
    for text in ["7", "8"] {
        harness.event(Event::Text(text.to_owned()));
        harness.run_steps(2);
    }
    assert_eq!(edits(&harness), [Edit::Began], "{:?}", log(&harness));
    harness.key_press(Key::Tab);
    harness.run_steps(3);
    assert_eq!(edits(&harness), [Edit::Began, Edit::Ended]);
}
