//! `when_blurred` is told why the field lost focus, the frame it reports it.

use eggshell_ui::{Blur, Builder, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Key, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, focused_path};

const FIELD: &str = r#"/"Page"/"name""#;
const CANVAS: &str = r#"/"Page"/"canvas""#;

#[derive(Default)]
struct App {
    draft: String,
    saved: String,
    other: String,
    causes: Vec<Blur>,
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
        s.div_id("canvas", tw().w_px(160.0).h_px(60.0), |_| {})
            .draw(|app: &mut App, ui| {
                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    app.escapes += 1;
                }
            });
    });
}

fn typing() -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 300.0), base_text(), App::default(), view);
    harness.run_steps(4);
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    assert_eq!(
        focused_path(&harness).as_deref(),
        Some("<widget>"),
        "precondition: the field has focus"
    );
    harness.event(egui::Event::Text("abc".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.draft, "abc", "precondition: typed");
    harness
}

#[test]
fn escape_blurs_as_escaped_and_the_node_beneath_sees_no_escape() {
    let mut harness = typing();
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!(app.causes, [Blur::Escaped]);
    assert_eq!(app.escapes, 0, "the Esc stayed the field's");
    assert_eq!(app.submitted, 0);
}

#[test]
fn clicking_elsewhere_blurs_as_elsewhere() {
    let mut harness = typing();
    click_node(&mut harness, CANVAS);
    harness.run_steps(6);
    assert_eq!(harness.state().app.causes, [Blur::Elsewhere]);
}

#[test]
fn tab_blurs_as_elsewhere() {
    let mut harness = typing();
    harness.key_press(Key::Tab);
    harness.run_steps(6);
    assert_eq!(harness.state().app.causes, [Blur::Elsewhere]);
}

#[test]
fn enter_submits_then_blurs_as_submitted() {
    let mut harness = typing();
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!(app.submitted, 1, "when_submitted fired");
    assert_eq!(app.causes, [Blur::Submitted]);
}

#[test]
fn a_later_blur_is_not_remembered_as_the_earlier_escape() {
    let mut harness = typing();
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    click_node(&mut harness, CANVAS);
    harness.run_steps(6);
    assert_eq!(harness.state().app.causes, [Blur::Escaped, Blur::Elsewhere]);
}

#[test]
fn escape_reverts_and_leaving_writes() {
    let mut harness = typing();
    harness.state_mut().app.revert_on_escape = true;
    harness.key_press(Key::Escape);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!(
        (app.draft.as_str(), app.saved.as_str()),
        ("", ""),
        "reverted"
    );

    click_node(&mut harness, FIELD);
    harness.run_steps(6);
    harness.event(egui::Event::Text("xyz".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.saved, "", "not written while editing");
    click_node(&mut harness, CANVAS);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!((app.draft.as_str(), app.saved.as_str()), ("xyz", "xyz"));
}
