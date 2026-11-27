//! Tabs (`eggshell_ui::owned::tabs`): shadcn/ui's account and password demo,
//! a code switcher, tabs with icons and a disabled one, and tabs that select
//! on Enter rather than as the arrow keys move.
//!
//! The list is one Tab stop. The arrow keys move along it and wrap, and Home
//! and End go to the ends. Only the selected tab's panel is built.

use eggshell_examples::common;

use common::glyphs::{self, Glyph};
use eggshell_ui::owned::{Enablement, choice::Choice};
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settings {
    Account,
    Password,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Html,
    Css,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Report {
    Overview,
    Analytics,
    Exports,
}

pub struct State {
    pub settings: Settings,
    pub name: String,
    pub username: String,
    pub current: String,
    pub new: String,
    pub lang: Lang,
    pub appearance: Appearance,
    pub report: Report,
}

impl Default for State {
    fn default() -> Self {
        Self {
            settings: Settings::Account,
            name: "Pedro Duarte".to_owned(),
            username: "@peduarte".to_owned(),
            current: String::new(),
            new: String::new(),
            lang: Lang::Rust,
            appearance: Appearance::Light,
            report: Report::Overview,
        }
    }
}

/// A card around a form: a title, a description, the fields, and a button.
///
/// ```html
/// <div class="flex flex-col gap-6 rounded-xl border p-6">
///   <div><h3>Title</h3><p class="text-muted-foreground">…</p></div>
///   <div class="flex flex-col gap-3">…</div>
///   <button>…</button>
/// </div>
/// ```
fn card(b: &mut B<'_, '_>, title: &str, description: &str, body: impl FnOnce(&mut B<'_, '_>)) {
    let tokens = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .gap(6.0)
            .p(6.0)
            .rounded_xl()
            .border(1.0)
            .border_color(tokens.border),
        |b| {
            b.div(tw().flex().flex_col().gap(1.5), |b| {
                b.text(tw().role(common::sized(16.0)), title);
                b.text(tw().text(tokens.muted_foreground), description);
            });
            body(b);
        },
    );
}

/// A labelled text field bound to one of the state's strings.
fn field(b: &mut B<'_, '_>, label: &'static str, value: &str, set: fn(&mut State, String)) {
    b.div(tw().flex().flex_col().gap(2.0), |b| {
        b.text(tw(), label);
        b.text_field(label, value.to_owned())
            .named(label)
            .classes(tw().w_full())
            .on_change(move |state: &mut State, text| set(state, text));
    });
}

/// shadcn/ui's tabs demo: an account form and a password form.
fn settings(b: &mut B<'_, '_>, state: &State) {
    let (name, username) = (state.name.clone(), state.username.clone());
    let (current, new) = (state.current.clone(), state.new.clone());
    b.div(tw().w_px(400.0), |b| {
        b.tabs(
            "settings",
            state.settings,
            [
                (Settings::Account, "Account"),
                (Settings::Password, "Password"),
            ],
        )
        .named("Settings")
        .on_change(|state: &mut State, settings| state.settings = settings)
        .panel(move |b, settings| match settings {
            Settings::Account => card(
                b,
                "Account",
                "Make changes to your account here. Click save when you're done.",
                |b| {
                    b.div(tw().flex().flex_col().gap(3.0), |b| {
                        field(b, "Name", &name, |state, text| state.name = text);
                        field(b, "Username", &username, |state, text| {
                            state.username = text
                        });
                    });
                    b.button("save-account", "Save changes").add();
                },
            ),
            Settings::Password => card(
                b,
                "Password",
                "Change your password here. After saving, you'll be logged out.",
                |b| {
                    b.div(tw().flex().flex_col().gap(3.0), |b| {
                        field(b, "Current password", &current, |state, text| {
                            state.current = text;
                        });
                        field(b, "New password", &new, |state, text| state.new = text);
                    });
                    b.button("save-password", "Save password").add();
                },
            ),
        });
    });
}

/// A code switcher, as on a docs page.
fn code(b: &mut B<'_, '_>, state: &State) {
    let muted = common::tokens(b.ctx()).muted;
    b.tabs(
        "code",
        state.lang,
        [
            (Lang::Rust, "Rust"),
            (Lang::Html, "HTML"),
            (Lang::Css, "CSS"),
        ],
    )
    .named("Language")
    .on_change(|state: &mut State, lang| state.lang = lang)
    .panel(move |b, lang| {
        let source = match lang {
            Lang::Rust => "fn main() {\n    println!(\"Hello\");\n}",
            Lang::Html => "<main>\n  <p>Hello</p>\n</main>",
            Lang::Css => "main {\n  padding: 1rem;\n}",
        };
        b.div(tw().w_px(400.0).p(4.0).rounded_lg().bg(muted), |b| {
            b.text(tw().role(common::mono(13.0)), source);
        });
    });
}

/// Tabs with icons, the last one disabled with its reason.
fn appearance(b: &mut B<'_, '_>, state: &State) {
    let icon = |glyph| glyphs::icon(b.ctx(), glyph);
    let options = [
        Choice::new(Appearance::Light, "Light").icon(icon(Glyph::Sun)),
        Choice::new(Appearance::Dark, "Dark").icon(icon(Glyph::Moon)),
        Choice::new(Appearance::System, "System")
            .icon(icon(Glyph::Monitor))
            .refused("Not on this platform"),
    ];
    b.tabs("appearance", state.appearance, options)
        .named("Appearance")
        .on_change(|state: &mut State, appearance| state.appearance = appearance)
        .panel(|b, appearance| {
            b.text(tw(), format!("{appearance:?} is chosen."));
        });
}

/// Tabs that select on Enter or Space: the arrows only move the focus, for
/// panels that are slow to build.
fn reports(b: &mut B<'_, '_>, state: &State) {
    b.tabs(
        "reports",
        state.report,
        [
            Choice::new(Report::Overview, "Overview"),
            Choice::new(Report::Analytics, "Analytics"),
            Choice::new(Report::Exports, "Exports").enablement(Enablement::Disabled),
        ],
    )
    .named("Reports")
    .activate_on_enter()
    .on_change(|state: &mut State, report| state.report = report)
    .panel(|b, report| {
        b.text(tw(), format!("The {report:?} report."));
    });
}

pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(6.0).p(6.0), |b| {
        b.text(tw().text(muted), "Account and password");
        settings(b, state);
        b.text(tw().text(muted), "Code");
        code(b, state);
        b.text(tw().text(muted), "Icons, and a disabled tab");
        appearance(b, state);
        b.text(tw().text(muted), "Enter or Space selects");
        reports(b, state);
    });
}

fn main() -> eframe::Result {
    common::run_app("tabs", State::default(), layout)
}
