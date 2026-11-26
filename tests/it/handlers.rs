//! Handlers that capture owned data, and values that are only `PartialEq`:
//! what generated enums and row ids need of the owned controls. That this
//! file compiles is half of each test.

use eggshell_ui::prelude::*;
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Role, Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;
use eggshell_ui::testing::{Driver, Target};

/// Neither `Hash` nor `Debug`, as a generated FFI enum may be.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Off,
    Slow,
    Fast,
}

const MODES: [(Mode, &str); 3] = [
    (Mode::Off, "Off"),
    (Mode::Slow, "Slow"),
    (Mode::Fast, "Fast"),
];

struct App {
    row: String,
    segmented: Mode,
    radio: Mode,
    select: Mode,
    log: Vec<String>,
    pass_seen: u64,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let row = app.row.clone();
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        let tag = row.clone();
        b.segmented("seg", app.segmented, MODES)
            .on_change(move |app: &mut App, mode| {
                app.segmented = mode;
                app.log.push(format!("{tag}: segmented"));
            });
        let tag = row.clone();
        b.radio_group("radio", app.radio, MODES)
            .on_change(move |app: &mut App, mode| {
                app.radio = mode;
                app.log.push(format!("{tag}: radio"));
            });
        let tag = row.clone();
        b.select("select", app.select, MODES)
            .named("Mode")
            .on_change(move |app: &mut App, mode| {
                app.select = mode;
                app.log.push(format!("{tag}: select"));
            });
        let tag = row.clone();
        b.slider("slider", 1.0, 0.0..=10.0)
            .named("Level")
            .on_change(move |app: &mut App, _| app.log.push(format!("{tag}: slider")));
        let tag = row.clone();
        b.stepper("stepper", 1.0, 0.0..=10.0)
            .named("Count")
            .on_change(move |app: &mut App, _| app.log.push(format!("{tag}: stepper")));
        let tag = row.clone();
        b.checkbox("check", "Check", false)
            .on_change(move |app: &mut App, _| app.log.push(format!("{tag}: check")));
        let tag = row.clone();
        b.switch("switch", "Switch", false)
            .on_change(move |app: &mut App, _| app.log.push(format!("{tag}: switch")));
        b.button("copy", "Copy")
            .on_click_with(|app: &mut App, ctx| app.pass_seen = ctx.cumulative_pass_nr());
    });
}

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App {
            row: "row-7".to_owned(),
            segmented: Mode::Off,
            radio: Mode::Off,
            select: Mode::Off,
            log: Vec::new(),
            pass_seen: 0,
        },
        view,
    )
    .build();
    d.frames(8);
    d
}

#[test]
fn controls_take_handlers_that_own_data_and_values_without_hash() {
    let mut d = harness();
    d.click(Target::suffix("/\"seg\"/1"));
    d.click(Target::suffix("/\"radio\"/Circle(2)"));
    d.click(Target::suffix("/\"check\"/Box"));
    d.click(Target::suffix("/\"switch\"/Track"));
    d.click(Target::suffix("/\"select\"")).frames(4);
    d.harness()
        .get_by_role_and_label(Role::ListBoxOption, "Fast")
        .click();
    d.frames(4);
    let app = d.app();
    assert!(app.segmented == Mode::Slow);
    assert!(app.radio == Mode::Fast);
    assert!(app.select == Mode::Fast);
    for control in ["segmented", "radio", "check", "switch", "select"] {
        let line = format!("row-7: {control}");
        assert!(app.log.contains(&line), "{control}: {:?}", app.log);
    }
}

#[test]
fn a_slider_and_a_stepper_call_a_handler_that_owns_data() {
    let mut d = harness();
    d.harness().get_by_label("Level").focus();
    d.frames(2);
    d.press(egui::Key::ArrowRight).frames(2);
    d.harness().get_by_label("Count").focus();
    d.frames(2);
    d.press(egui::Key::ArrowUp).frames(2);
    let log = &d.app().log;
    assert!(log.contains(&"row-7: slider".to_owned()), "{log:?}");
    assert!(log.contains(&"row-7: stepper".to_owned()), "{log:?}");
}

#[test]
fn a_click_handler_gets_the_context() {
    let mut d = harness();
    assert_eq!(d.app().pass_seen, 0, "precondition");
    d.click(Target::suffix("/\"copy\"")).frames(3);
    assert!(d.app().pass_seen > 0);
}
