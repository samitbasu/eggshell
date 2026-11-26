//! The uniform `on_change` of the value widgets, and the hooks before it,
//! through kittest's real pointer and key events.

use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Key, Theme, vec2};

use crate::support;

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

fn rig() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(400.0, 500.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .build();
    d.frames(12);
    d
}

#[test]
fn a_two_valued_enum_and_a_bool_toggle_through_clicks() {
    let mut d = rig();
    let app = &d.app();
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
        d.click(Target::suffix(path)).frames(12);
    }
    let app = &d.app();
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
    d.click(Target::suffix("/\"terms\"/Label")).frames(12);
    d.click(Target::suffix("/\"bold\"")).frames(12);
    d.click(Target::suffix("/\"italic\"")).frames(12);
    let app = &d.app();
    assert_eq!(app.terms, Terms::Declined, "the label toggles back");
    assert_eq!(app.bold, Terms::Declined);
    assert!(!app.italic);
}

#[test]
fn the_text_field_reports_real_edits_only() {
    let mut d = rig();
    d.frames(10);
    assert_eq!(d.app().changes, 0, "idle frames report nothing");
    d.click(Target::suffix("/\"name\"")).frames(4);
    assert_eq!(d.app().changes, 0, "focusing is not an edit");
    d.type_text("hi").frames(3);
    assert_eq!(d.app().name, "hi");
    assert_eq!(d.app().changes, 1, "typing reports once");
    d.frames(10);
    assert_eq!(d.app().changes, 1, "and not on later frames");
}

#[test]
fn enter_submits_and_tab_blurs() {
    let mut d = rig();
    d.click(Target::suffix("/\"name\"")).frames(4);
    d.type_text("x").frames(3);
    let app = &d.app();
    assert_eq!((app.submitted, app.blurred), (0, 0), "typing does neither");
    d.press(Key::Enter).frames(3);
    assert_eq!(d.app().submitted, 1, "Enter submits");

    d.click(Target::suffix("/\"name\"")).frames(4);
    let before = d.app().blurred;
    d.press(Key::Tab).frames(3);
    let app = &d.app();
    assert_eq!(app.blurred, before + 1, "Tab away blurs");
    assert_eq!(app.submitted, 1, "Tab does not submit");
}

#[test]
fn on_response_sees_hover() {
    let mut d = rig();
    assert!(!d.app().hovered, "not hovered at the start");
    d.hover(Target::suffix("/\"terms\"/Box")).frames(4);
    assert!(d.app().hovered);
}
