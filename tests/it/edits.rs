//! `on_edit` of the slider and the stepper: one Began and one Ended around
//! the values a gesture sets, through kittest's real pointer and key events.

use eggshell_ui::owned::Edit;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Event, Key, Theme, pos2, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

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

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 300.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.frames(8);
    d
}

fn log(d: &Driver<App>) -> &[Logged] {
    &d.app().log
}

fn edits(d: &Driver<App>) -> Vec<Edit> {
    log(d)
        .iter()
        .filter_map(|logged| match logged {
            Logged::Edit(edit) => Some(*edit),
            Logged::Value(_) => None,
        })
        .collect()
}

fn assert_bracketed(d: &Driver<App>) {
    let log = log(d);
    assert!(
        log.iter().any(|logged| matches!(logged, Logged::Value(_))),
        "precondition: the gesture set a value: {log:?}"
    );
    assert_eq!(edits(d), [Edit::Began, Edit::Ended], "{log:?}");
    assert_eq!(log.first(), Some(&Logged::Edit(Edit::Began)), "{log:?}");
    assert_eq!(log.last(), Some(&Logged::Edit(Edit::Ended)), "{log:?}");
}

#[test]
fn dragging_the_thumb_is_one_edit() {
    let mut d = harness();
    let thumb = d.rect(Target::suffix("/\"slider\"/Track/Thumb")).center();
    let track = d.rect(Target::suffix("/\"slider\"/Track"));
    d.drag(thumb, pos2(track.left() + 60.0, thumb.y));
    assert_bracketed(&d);
}

#[test]
fn a_press_that_starts_on_the_track_and_drags_is_one_edit() {
    let mut d = harness();
    let thumb = d.rect(Target::suffix("/\"slider\"/Track/Thumb"));
    let track = d.rect(Target::suffix("/\"slider\"/Track"));
    let start = pos2(track.right() - 20.0, track.center().y);
    assert!(
        !thumb.contains(start),
        "precondition: the press misses the thumb"
    );
    d.drag(start, pos2(track.left() + 80.0, start.y));
    assert_bracketed(&d);
}

#[test]
fn a_click_on_the_track_is_one_edit() {
    let mut d = harness();
    let track = d.rect(Target::suffix("/\"slider\"/Track"));
    press_and_release(&mut d, pos2(track.left() + 30.0, track.center().y));
    d.frames(3);
    assert_bracketed(&d);
}

#[test]
fn each_key_press_is_one_edit() {
    let mut d = harness();
    d.harness().get_by_label("Level").focus();
    d.frames(3);
    assert!(edits(&d).is_empty(), "focusing is not an edit");
    for _ in 0..2 {
        d.press(Key::ArrowRight).frames(2);
    }
    assert_eq!(
        edits(&d),
        [Edit::Began, Edit::Ended, Edit::Began, Edit::Ended]
    );
}

#[test]
fn stepper_buttons_are_one_edit_each() {
    let mut d = harness();
    d.click(Target::suffix("/\"stepper\"/Box/Buttons/Up"))
        .frames(3);
    assert_bracketed(&d);
}

#[test]
fn typing_in_the_stepper_is_one_edit_until_the_field_loses_focus() {
    let mut d = harness();
    d.click(Target::suffix("/\"stepper\"/Box/Field")).frames(3);
    assert!(edits(&d).is_empty(), "focusing is not an edit");
    for text in ["7", "8"] {
        d.event(Event::Text(text.to_owned())).frames(1);
    }
    assert_eq!(edits(&d), [Edit::Began], "{:?}", log(&d));
    d.press(Key::Tab).frames(2);
    assert_eq!(edits(&d), [Edit::Began, Edit::Ended]);
}

/// A primary press and release at `at`, a frame each, with no hover first.
fn press_and_release(d: &mut Driver<App>, at: egui::Pos2) {
    for pressed in [true, false] {
        d.event(Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}
