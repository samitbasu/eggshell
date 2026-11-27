//! The command palette, a combobox and an autosuggest (#345): Ctrl+K (Cmd+K
//! on macOS) opens a palette over the app's command table and a page list;
//! the combobox (an input group: type in it, or press its chevron) picks a framework, the autosuggest a city as typed.

use eggshell_examples::common;

use eggshell_ui::owned::Mark;
use eggshell_ui::owned::command::Open;
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, Chord, Command, Commands, tw};
use egui::{Key, Modifiers};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Part {
    Page,
    Palette,
    Opener,
    Framework,
    City,
    Said,
    Action(Action),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum Action {
    Search,
    NewFile,
    Save,
    Grid,
    Delete,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Framework {
    Next,
    SvelteKit,
    Nuxt,
    Remix,
    Astro,
}

pub const FRAMEWORKS: [(Framework, &str); 5] = [
    (Framework::Next, "Next.js"),
    (Framework::SvelteKit, "SvelteKit"),
    (Framework::Nuxt, "Nuxt.js"),
    (Framework::Remix, "Remix"),
    (Framework::Astro, "Astro"),
];

const CITIES: [&str; 8] = [
    "Amsterdam",
    "Athens",
    "Berlin",
    "Bern",
    "Lisbon",
    "London",
    "Paris",
    "Zürich",
];

const PAGES: [(&str, &[&str]); 6] = [
    ("Getting started", &["install", "intro"]),
    ("Layout", &["flex", "grid"]),
    ("Overlays", &["popover", "menu", "dialog"]),
    ("Commands", &["shortcut", "keyboard"]),
    ("Virtual lists", &["scroll", "long"]),
    ("Theming", &["shadcn", "colours"]),
];

#[derive(Default)]
pub struct State {
    pub palette: bool,
    pub grid: bool,
    pub framework: Option<Framework>,
    pub city: String,
    pub said: String,
}

fn search() -> Chord {
    const { Chord::new(Modifiers::COMMAND, Key::K).expect("not reserved") }
}

fn commands() -> Commands<State> {
    Commands::new()
        .add(
            Part::Action(Action::Search),
            Command::new("Search…", |state: &mut State| state.palette = true).shortcut(search()),
        )
        .add(
            Part::Action(Action::NewFile),
            Command::new("New file", |state: &mut State| {
                state.said = "New file".into()
            }),
        )
        .add(
            Part::Action(Action::Save),
            Command::new("Save", |state: &mut State| state.said = "Saved".into())
                .shortcut(const { Chord::new(Modifiers::COMMAND, Key::S).expect("not reserved") }),
        )
        .add(
            Part::Action(Action::Grid),
            Command::new("Show grid", |state: &mut State| state.grid = !state.grid)
                .checked(|state| Mark::from(state.grid)),
        )
        .add(
            Part::Action(Action::Delete),
            Command::new("Delete", |_| {}).enabled(|_| false),
        )
}

pub fn view<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    let table = commands();
    b.commands(&table, state);
    b.div_id(
        Part::Page,
        tw().flex().flex_col().gap(4.0).p(6.0).w_px(360.0),
        |s| {
            s.button(Part::Opener, "Search… (Ctrl+K)")
                .on_click(|state: &mut State| state.palette = true);
            s.combobox(
                Part::Framework,
                state.framework,
                FRAMEWORKS.map(|(value, label)| (value, label.to_owned())),
            )
            .placeholder("Select framework…")
            .named("Framework")
            .classes(tw().w_px(200.0))
            .on_clear(|state: &mut State| state.framework = None)
            .on_change(|state: &mut State, value| state.framework = Some(value));
            s.autosuggest(Part::City, &state.city, CITIES.map(str::to_owned))
                .placeholder("City")
                .named("City")
                .classes(tw().w_px(200.0))
                .on_change(|state: &mut State, city| state.city = city);
            s.div_id(Part::Said, tw(), |s| {
                s.text(tw(), format!("Last: {}", state.said));
            });
        },
    );
    b.command_dialog(
        Part::Palette,
        Open::from(state.palette),
        |state: &mut State| {
            state.palette = false;
        },
    )
    .placeholder("Type a command or search…")
    .named("Command palette")
    .items(|items| {
        items.group("Pages", |items| {
            for (title, tags) in PAGES {
                items
                    .item(title, title)
                    .keywords(tags.iter().copied())
                    .on_pick(move |state: &mut State| state.said = format!("Opened {title}"));
            }
        });
        items.commands("Actions", &table, state);
    });
}

#[allow(dead_code, reason = "the tests include this file for `view`")]
fn main() -> eframe::Result {
    common::run_app("command_palette", State::default(), view)
}
