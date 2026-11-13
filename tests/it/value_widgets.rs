//! The uniform `on_change` of the value widgets, and the hooks before it,
//! through kittest's real pointer and key events.

use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Key, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, click_node, hover_node};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Terms {
    #[default]
    Declined,
    Accepted,
}

#[derive(Default)]
struct App {
    terms: Terms,
    news: bool,
    mode: Terms,
    dark: bool,
    bold: Terms,
    italic: bool,
    name: String,
    other: String,
    changes: u32,
    submitted: u32,
    blurred: u32,
    hovered: bool,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(4.0).p(4.0), |b| {
        b.checkbox("terms", "Terms", app.terms)
            .values(Terms::Declined, Terms::Accepted)
            .on_response(|app: &mut App, response| app.hovered = response.hovered())
            .on_change(|app: &mut App, terms| app.terms = terms);
        b.checkbox("news", "News", app.news)
            .on_change(|app: &mut App, news| app.news = news);
        b.switch("mode", "Mode", app.mode)
            .values(Terms::Declined, Terms::Accepted)
            .on_change(|app: &mut App, mode| app.mode = mode);
        b.switch("dark", "Dark", app.dark)
            .on_change(|app: &mut App, dark| app.dark = dark);
        b.toggle("bold", "Bold", app.bold)
            .values(Terms::Declined, Terms::Accepted)
            .on_change(|app: &mut App, bold| app.bold = bold);
        b.toggle("italic", "Italic", app.italic)
            .on_change(|app: &mut App, italic| app.italic = italic);
        b.text_field("name", &app.name)
            .named("Name")
            .when_submitted(|app: &mut App| app.submitted += 1)
            .when_blurred(|app: &mut App, _| app.blurred += 1)
            .on_change(|app: &mut App, name| {
                app.changes += 1;
                app.name = name;
            });
        b.text_field("other", &app.other)
            .named("Other")
            .on_change(|app: &mut App, other| app.other = other);
    });
}

fn rig() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_stepped(
        vec2(400.0, 500.0),
        0.05,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    );
    harness.run_steps(12);
    harness
}

fn click(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    click_node(harness, path);
    harness.run_steps(12);
}

#[test]
fn a_two_valued_enum_and_a_bool_toggle_through_clicks() {
    let mut harness = rig();
    let app = &harness.state().app;
    assert_eq!(
        (
            app.terms, app.news, app.mode, app.dark, app.bold, app.italic
        ),
        (
            Terms::Declined,
            false,
            Terms::Declined,
            false,
            Terms::Declined,
            false
        ),
        "everything starts off"
    );
    for path in [
        "/\"terms\"/Box",
        "/\"news\"/Box",
        "/\"mode\"/Track",
        "/\"dark\"/Track",
        "/\"bold\"",
        "/\"italic\"",
    ] {
        click(&mut harness, path);
    }
    let app = &harness.state().app;
    assert_eq!(
        (
            app.terms, app.news, app.mode, app.dark, app.bold, app.italic
        ),
        (
            Terms::Accepted,
            true,
            Terms::Accepted,
            true,
            Terms::Accepted,
            true
        ),
        "one click turns each on"
    );
    click(&mut harness, "/\"terms\"/Label");
    click(&mut harness, "/\"bold\"");
    click(&mut harness, "/\"italic\"");
    let app = &harness.state().app;
    assert_eq!(app.terms, Terms::Declined, "the label toggles back");
    assert_eq!(app.bold, Terms::Declined);
    assert!(!app.italic);
}

#[test]
fn the_text_field_reports_real_edits_only() {
    let mut harness = rig();
    harness.run_steps(10);
    assert_eq!(harness.state().app.changes, 0, "idle frames report nothing");
    click_node(&mut harness, "/\"name\"");
    harness.run_steps(4);
    assert_eq!(harness.state().app.changes, 0, "focusing is not an edit");
    harness.event(egui::Event::Text("hi".into()));
    harness.run_steps(4);
    assert_eq!(harness.state().app.name, "hi");
    assert_eq!(harness.state().app.changes, 1, "typing reports once");
    harness.run_steps(10);
    assert_eq!(harness.state().app.changes, 1, "and not on later frames");
}

#[test]
fn enter_submits_and_tab_blurs() {
    let mut harness = rig();
    click_node(&mut harness, "/\"name\"");
    harness.run_steps(4);
    harness.event(egui::Event::Text("x".into()));
    harness.run_steps(4);
    let app = &harness.state().app;
    assert_eq!((app.submitted, app.blurred), (0, 0), "typing does neither");
    harness.key_press(Key::Enter);
    harness.run_steps(4);
    assert_eq!(harness.state().app.submitted, 1, "Enter submits");

    click_node(&mut harness, "/\"name\"");
    harness.run_steps(4);
    let before = harness.state().app.blurred;
    harness.key_press(Key::Tab);
    harness.run_steps(4);
    let app = &harness.state().app;
    assert_eq!(app.blurred, before + 1, "Tab away blurs");
    assert_eq!(app.submitted, 1, "Tab does not submit");
}

#[test]
fn on_response_sees_hover() {
    let mut harness = rig();
    assert!(!harness.state().app.hovered, "not hovered at the start");
    hover_node(&mut harness, "/\"terms\"/Box");
    harness.run_steps(4);
    assert!(harness.state().app.hovered);
}
