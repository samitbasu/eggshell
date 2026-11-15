//! What Enter does in a one-line field, and focus that the application asks
//! for with a token (#150).

use eggshell_ui::prim::text_field::Enter;
use eggshell_ui::{Blur, Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Key, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, focused_node};

const NAME: &str = r#"/"Page"/"name""#;
const OTHER: &str = r#"/"Page"/"other""#;

#[derive(Default)]
struct App {
    draft: String,
    other: String,
    submitted: u32,
    blurs: Vec<Blur>,
    enter: Enter,
    token: u32,
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
            .on_enter(app.enter)
            .focus_on_change(app.token)
            .when_submitted(|app: &mut App| app.submitted += 1)
            .when_blurred(|app: &mut App, cause| app.blurs.push(cause))
            .on_change(|app: &mut App, text| app.draft = text);
        s.text_field("other", app.other.clone())
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|app: &mut App, text| app.other = text);
    });
}

fn harness(enter: Enter) -> Harness<'static, Rig<App>> {
    let app = App {
        enter,
        ..App::default()
    };
    let mut harness = support::harness(vec2(400.0, 200.0), base_text(), app, view);
    harness.run_steps(4);
    harness
}

fn typing(enter: Enter) -> Harness<'static, Rig<App>> {
    let mut harness = harness(enter);
    click_node(&mut harness, NAME);
    harness.run_steps(6);
    harness.event(Event::Text("abc".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.draft, "abc", "precondition: typed");
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(NAME),
        "precondition: the field has focus"
    );
    harness
}

#[test]
fn enter_with_keep_focus_submits_and_the_field_stays_focused() {
    let mut harness = typing(Enter::KeepFocus);
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!(app.submitted, 1);
    assert!(app.blurs.is_empty(), "no blur: {:?}", app.blurs);
    assert_eq!(focused_node(&harness).as_deref(), Some(NAME));
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    assert_eq!(harness.state().app.submitted, 2, "and again");
}

#[test]
fn enter_by_default_submits_and_blurs() {
    let mut harness = typing(Enter::Submit);
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    let app = &harness.state().app;
    assert_eq!(app.submitted, 1);
    assert_eq!(app.blurs, [Blur::Submitted]);
    assert_eq!(focused_node(&harness), None);
}

#[test]
fn an_enter_that_confirms_an_ime_composition_does_not_submit() {
    let mut harness = typing(Enter::KeepFocus);
    harness.event(Event::Ime(egui::ImeEvent::Commit("x".into())));
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    assert_eq!(
        harness.state().app.submitted,
        0,
        "the Enter belonged to the IME"
    );
    harness.key_press(Key::Enter);
    harness.run_steps(6);
    assert_eq!(harness.state().app.submitted, 1, "the next one submits");
}

#[test]
fn a_changed_token_focuses_the_field_once() {
    let mut harness = harness(Enter::Submit);
    assert_eq!(
        focused_node(&harness),
        None,
        "the first sight of a token asks for nothing"
    );
    harness.state_mut().app.token = 1;
    harness.run_steps(4);
    assert_eq!(focused_node(&harness).as_deref(), Some(NAME));
    click_node(&mut harness, OTHER);
    harness.run_steps(6);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(OTHER),
        "an unchanged token leaves the user's focus alone"
    );
    harness.state_mut().app.token = 2;
    harness.run_steps(4);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some(NAME),
        "a new token asks again"
    );
}
