//! `when_blurred` is told why the field lost focus, the frame it reports it.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Blur, Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, vec2};
use egui_kittest::Harness;

use crate::support::{self, focused_path};

const FIELD: &str = r#"/"Page"/"name""#;
const CANVAS: &str = r#"/"Page"/"canvas""#;
const STEPPER: &str = r#"/"Page"/"count"/Box/Field"#;
const READOUT: &str = r#"/"Page"/"levels"/"level"/Box/Header/Readout"#;

#[derive(Default)]
struct App {
    draft: String,
    saved: String,
    other: String,
    causes: Vec<Blur>,
    count: f64,
    count_causes: Vec<Blur>,
    level: f64,
    levels: Vec<f64>,
    submitted: u32,
    escapes: u32,
    revert_on_escape: bool,
}

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(2.0), |s| {
        s.text_field("name", app.draft.clone())
            .named("Name")
            .classes(tw().w_px(160.0))
            .when_submitted(|app: &mut App| app.submitted += 1)
            .when_blurred(|app: &mut App, cause| {
                app.causes.push(cause);
                if app.revert_on_escape {
                    match cause {
                        Blur::Escaped => app.draft = app.saved.clone(),
                        Blur::Submitted | Blur::Elsewhere => app.saved = app.draft.clone(),
                    }
                }
            })
            .on_change(|app: &mut App, text| app.draft = text);
        s.text_field("other", app.other.clone())
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.other = text);
        s.stepper("count", app.count, 0.0..=100.0)
            .named("Count")
            .when_blurred(|app: &mut App, cause| app.count_causes.push(cause))
            .on_change(|app: &mut App, count| app.count = count);
        s.div_id("levels", tw().w_px(240.0), |s| {
            s.slider("level", app.level, 0.0..=100.0)
                .label("Level")
                .readout(|level| format!("{level:.0}"))
                .editable(|text| text.trim().parse().ok())
                .on_change(|app: &mut App, level| {
                    app.level = level;
                    app.levels.push(level);
                });
        });
        s.div_id("canvas", tw().w_px(160.0).h_px(60.0), |_| {})
            .draw(|app: &mut App, ui| {
                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    app.escapes += 1;
                }
            });
    });
}

fn driver() -> Driver<App> {
    support::driver(vec2(400.0, 300.0), base_text(), App::default(), view).build()
}

fn typing() -> Driver<App> {
    let mut d = driver();
    d.frames(4);
    d.click(Target::suffix(FIELD)).frames(6);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some("<widget>"),
        "precondition: the field has focus"
    );
    d.type_text("abc").frames(3);
    assert_eq!(d.app().draft, "abc", "precondition: typed");
    d
}

/// Sends only the event, leaving `input.focused` set, unlike `Driver::window_focus`.
fn blur_the_window(d: &mut Driver<App>) {
    d.event(egui::Event::WindowFocused(false)).frames(5);
}

#[test]
fn the_window_losing_focus_blurs_a_text_field_as_elsewhere() {
    let mut d = typing();
    blur_the_window(&mut d);
    assert_eq!(focused_path(&d), None, "the field gave up its focus");
    let app = d.app();
    assert_eq!(app.causes, [Blur::Elsewhere]);
    assert_eq!(app.draft, "abc", "what was typed stays");
}

#[test]
fn the_window_losing_focus_blurs_a_steppers_field_as_elsewhere() {
    let mut d = driver();
    d.frames(4);
    d.click(Target::suffix(STEPPER)).frames(6);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some("<widget>"),
        "precondition: the stepper's field has focus"
    );
    d.type_text("7").frames(3);
    blur_the_window(&mut d);
    assert_eq!(focused_path(&d), None, "the field gave up its focus");
    let app = d.app();
    assert_eq!(app.count_causes, [Blur::Elsewhere]);
}

#[test]
fn the_window_losing_focus_closes_a_sliders_readout_and_writes_nothing() {
    let mut d = driver();
    d.frames(4);
    d.click(Target::suffix(READOUT)).frames(6);
    let field = format!("{READOUT}/Field");
    assert_eq!(
        support::focused_node(&d).as_deref(),
        Some(field.as_str()),
        "precondition: the readout opened into a field with focus"
    );
    d.type_text("9").frames(3);
    blur_the_window(&mut d);
    assert_eq!(focused_path(&d), None, "the field gave up its focus");
    assert!(
        !support::nodes(&d).iter().any(|(path, _)| *path == field),
        "the readout closed"
    );
    assert!(d.app().levels.is_empty());
}

#[test]
fn escape_blurs_as_escaped_and_the_node_beneath_sees_no_escape() {
    let mut d = typing();
    d.press(Key::Escape).frames(5);
    let app = d.app();
    assert_eq!(app.causes, [Blur::Escaped]);
    assert_eq!(app.escapes, 0, "the Esc stayed the field's");
    assert_eq!(app.submitted, 0);
}

#[test]
fn clicking_elsewhere_blurs_as_elsewhere() {
    let mut d = typing();
    d.click(Target::suffix(CANVAS)).frames(6);
    assert_eq!(d.app().causes, [Blur::Elsewhere]);
}

#[test]
fn tab_blurs_as_elsewhere() {
    let mut d = typing();
    d.press(Key::Tab).frames(5);
    assert_eq!(d.app().causes, [Blur::Elsewhere]);
}

#[test]
fn enter_submits_then_blurs_as_submitted() {
    let mut d = typing();
    d.press(Key::Enter).frames(5);
    let app = d.app();
    assert_eq!(app.submitted, 1, "when_submitted fired");
    assert_eq!(app.causes, [Blur::Submitted]);
}

#[test]
fn a_later_blur_is_not_remembered_as_the_earlier_escape() {
    let mut d = typing();
    d.press(Key::Escape).frames(5);
    d.click(Target::suffix(FIELD)).frames(6);
    d.click(Target::suffix(CANVAS)).frames(6);
    assert_eq!(d.app().causes, [Blur::Escaped, Blur::Elsewhere]);
}

#[test]
fn escape_reverts_and_leaving_writes() {
    let mut d = typing();
    d.app_mut().revert_on_escape = true;
    d.press(Key::Escape).frames(5);
    let app = d.app();
    assert_eq!(
        (app.draft.as_str(), app.saved.as_str()),
        ("", ""),
        "reverted"
    );

    d.click(Target::suffix(FIELD)).frames(6);
    d.type_text("xyz").frames(3);
    assert_eq!(d.app().saved, "", "not written while editing");
    d.click(Target::suffix(CANVAS)).frames(6);
    let app = d.app();
    assert_eq!((app.draft.as_str(), app.saved.as_str()), ("xyz", "xyz"));
}

#[derive(Default)]
struct Log {
    top: Vec<Blur>,
    bottom: Vec<Blur>,
}

struct Two {
    top: eggshell_ui::LayoutState,
    bottom: eggshell_ui::LayoutState,
    log: Log,
}

#[test]
fn the_cause_does_not_leak_between_two_layouts() {
    use egui_kittest::kittest::Queryable as _;
    let state = Two {
        top: eggshell_ui::LayoutState::new().with_base_text(base_text()),
        bottom: eggshell_ui::LayoutState::new().with_base_text(base_text()),
        log: Log::default(),
    };
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .build_ui_state(
            |ui, two: &mut Two| {
                let Two { top, bottom, log } = two;
                let window = ui.max_rect();
                let (upper, lower) = (
                    egui::Rect::from_min_size(window.min, vec2(400.0, 100.0)),
                    egui::Rect::from_min_size(window.min + vec2(0.0, 150.0), vec2(400.0, 100.0)),
                );
                top.build(ui.ctx(), |b: &mut Builder<'_, '_, Log>| {
                    b.text_field("top", "")
                        .named("Top")
                        .classes(tw().w_px(160.0))
                        .when_blurred(|log: &mut Log, cause| log.top.push(cause))
                        .on_change(|_: &mut Log, _| {});
                })
                .layout(upper)
                .draw(log, ui);
                bottom
                    .build(ui.ctx(), |b: &mut Builder<'_, '_, Log>| {
                        b.text_field("bottom", "")
                            .named("Bottom")
                            .classes(tw().w_px(160.0))
                            .when_blurred(|log: &mut Log, cause| log.bottom.push(cause))
                            .on_change(|_: &mut Log, _| {});
                    })
                    .layout(lower)
                    .draw(log, ui);
            },
            state,
        );
    harness.run_steps(4);

    harness.get_by_label("Top").click();
    harness.run_steps(6);
    harness.get_by_label("Bottom").click();
    harness.run_steps(6);
    assert_eq!(harness.state().log.top, [Blur::Elsewhere], "precondition");

    harness.key_press(Key::Escape);
    harness.run_steps(6);
    let log = &harness.state().log;
    assert_eq!(log.bottom, [Blur::Escaped]);
    assert_eq!(log.top, [Blur::Elsewhere], "the other layout heard nothing");

    harness.get_by_label("Top").click();
    harness.run_steps(6);
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    let log = &harness.state().log;
    assert_eq!(log.top, [Blur::Elsewhere, Blur::Escaped]);
    assert_eq!(log.bottom, [Blur::Escaped], "nor did this one");
}
