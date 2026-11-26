//! A menu bar (the `menubar` feature): File, Edit, View and a disabled
//! Help, mostly command items.
//!
//! Click a menu, or Tab to the bar and press Enter, Space or Down. While a
//! menu is open, move the pointer along the bar or press Left and Right to
//! switch menus; Esc closes the menu and leaves the focus on its trigger,
//! where Left and Right walk the closed bar. Right on Open Recent opens its
//! submenu.
//!
//! The items are commands (`Commands`): their chords work with every menu
//! closed. Show Grid is a checked command (Ctrl+Alt+G, Cmd+Option+G on a
//! Mac) and the densities are chosen ones, so their marks follow the chords
//! too. Redo is disabled until something is undone.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, Chord, Command, Commands};
use egui::{Key, Modifiers};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Part {
    Page,
    Bar,
    File,
    Edit,
    View,
    Help,
    Recent,
    Recents(usize),
    About,
    Log,
    Cmd(Cmd),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Cmd {
    New,
    Open,
    Save,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Grid,
    Compact,
    Comfortable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Default)]
pub struct State {
    pub grid: bool,
    pub density: Density,
    pub undone: usize,
    pub last: Option<&'static str>,
}

const RECENT: [&str; 3] = ["notes.md", "budget.csv", "draft.txt"];

const COMMAND_ALT: Modifiers = Modifiers {
    alt: true,
    ..Modifiers::COMMAND
};

const COMMAND_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::COMMAND
};

const NEW: Chord = Chord::new(Modifiers::COMMAND, Key::N).expect("not reserved");
const OPEN: Chord = Chord::new(Modifiers::COMMAND, Key::O).expect("not reserved");
const SAVE: Chord = Chord::new(Modifiers::COMMAND, Key::S).expect("not reserved");
const UNDO: Chord = Chord::new(Modifiers::COMMAND, Key::Z).expect("not reserved");
const REDO: Chord = Chord::new(COMMAND_SHIFT, Key::Z).expect("not reserved");
const CUT: Chord = Chord::new(Modifiers::COMMAND, Key::X).expect("not reserved");
const COPY: Chord = Chord::new(Modifiers::COMMAND, Key::C).expect("not reserved");
const PASTE: Chord = Chord::new(Modifiers::COMMAND, Key::V).expect("not reserved");
const GRID: Chord = Chord::new(COMMAND_ALT, Key::G).expect("not reserved");
const COMPACT: Chord = Chord::new(COMMAND_ALT, Key::Num1).expect("not reserved");
const COMFORTABLE: Chord = Chord::new(COMMAND_ALT, Key::Num2).expect("not reserved");

fn commands() -> Commands<State> {
    let said = |title: &'static str, chord| Command::new(title, |_: &mut State| {}).shortcut(chord);
    Commands::new()
        .add(Part::Cmd(Cmd::New), said("New", NEW))
        .add(Part::Cmd(Cmd::Open), said("Open…", OPEN))
        .add(Part::Cmd(Cmd::Save), said("Save", SAVE))
        .add(
            Part::Cmd(Cmd::Undo),
            Command::new("Undo", |state: &mut State| state.undone += 1).shortcut(UNDO),
        )
        .add(
            Part::Cmd(Cmd::Redo),
            Command::new("Redo", |state: &mut State| {
                state.undone = state.undone.saturating_sub(1);
            })
            .shortcut(REDO)
            .enabled(|state| state.undone > 0),
        )
        .add(Part::Cmd(Cmd::Cut), said("Cut", CUT))
        .add(Part::Cmd(Cmd::Copy), said("Copy", COPY))
        .add(Part::Cmd(Cmd::Paste), said("Paste", PASTE))
        .add(
            Part::Cmd(Cmd::Grid),
            Command::new("Show Grid", |state: &mut State| state.grid = !state.grid)
                .checked(|state| Mark::from(state.grid))
                .shortcut(GRID),
        )
        .add(
            Part::Cmd(Cmd::Compact),
            Command::new("Compact", |state: &mut State| {
                state.density = Density::Compact
            })
            .chosen(|state| Mark::from(state.density == Density::Compact))
            .shortcut(COMPACT),
        )
        .add(
            Part::Cmd(Cmd::Comfortable),
            Command::new("Comfortable", |state: &mut State| {
                state.density = Density::Comfortable;
            })
            .chosen(|state| Mark::from(state.density == Density::Comfortable))
            .shortcut(COMFORTABLE),
        )
}

/// ```html
/// <div class="flex flex-col items-start gap-4 p-4">
///   <Menubar>…File, Edit, View, Help…</Menubar>
///   <p>Last: …</p>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.commands(&commands(), state);
    b.div_id(
        Part::Page,
        tw().flex().flex_col().items_start().gap(4.0).p(4.0),
        |s| {
            s.menubar(Part::Bar).menus(|bar| {
                bar.menu(Part::File, "File").items(|m| {
                    m.command(Part::Cmd(Cmd::New)).add();
                    m.command(Part::Cmd(Cmd::Open)).add();
                    m.submenu(Part::Recent, "Open Recent", |m| {
                        for (at, name) in RECENT.into_iter().enumerate() {
                            m.item(Part::Recents(at), name)
                                .on_click(move |state: &mut State| state.last = Some(name));
                        }
                    });
                    m.separator();
                    m.command(Part::Cmd(Cmd::Save)).add();
                });
                bar.menu(Part::Edit, "Edit").items(|m| {
                    m.command(Part::Cmd(Cmd::Undo)).add();
                    m.command(Part::Cmd(Cmd::Redo)).add();
                    m.separator();
                    m.command(Part::Cmd(Cmd::Cut)).add();
                    m.command(Part::Cmd(Cmd::Copy)).add();
                    m.command(Part::Cmd(Cmd::Paste)).add();
                });
                bar.menu(Part::View, "View").items(|m| {
                    m.command(Part::Cmd(Cmd::Grid)).add();
                    m.separator();
                    m.label("Density");
                    m.command(Part::Cmd(Cmd::Compact)).add();
                    m.command(Part::Cmd(Cmd::Comfortable)).add();
                });
                bar.menu(Part::Help, "Help")
                    .enablement(Enablement::Disabled)
                    .items(|m| {
                        m.item(Part::About, "About").on_click(|_: &mut State| {});
                    });
            });
            let last = state.last.unwrap_or("nothing yet");
            s.div_id(Part::Log, tw(), |s| {
                s.text(tw(), format!("Last: {last}; undone {}", state.undone));
            });
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("menubar", State::default(), layout)
}
