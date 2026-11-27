//! Overlays: a stock dropdown menu with a submenu, check and radio items, and
//! a tall list that flips to the side with room, clamps to it and scrolls.
//!
//! The dropdown owns whether it is open. Click it, or press Enter, Space or
//! Down on it; the arrows, Home and End move between the items, Enter
//! chooses, Right opens the submenu and Left closes it. Click away, press Esc
//! or Tab out to close. The list is a hand-built overlay of the same look.
//!
//! Copy and Paste are commands (`Commands`): the menu takes their title,
//! shortcut and enablement from the table, and Ctrl+C (Cmd+C on a Mac) runs
//! Copy with the menu closed. Paste is disabled, and so is its menu item.
//! Show grid is a checked command: Ctrl+Alt+G (Cmd+Option+G on a Mac)
//! toggles it, and its item's check follows. The chord is one no browser
//! keeps for itself; Ctrl+G is find next.
//!
//! Actions ends in a chevron, as a select does. Accent's trigger is a swatch
//! of its own and a chevron, and its menu is wider than the style's.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::common;

use eggshell_ui::overlay::{Anchor, Distance, MatchWidth, Nav, Placement, Wrap};
use eggshell_ui::owned::{Mark, button::Face};
use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, Chord, Command, Commands, tw};
use egui::{Color32, Key, Modifiers, Role};

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Top,
    Bottom,
    PopoverButton,
    Accent,
    Swatch,
    Menu,
    Item(Item),
    List,
    ListButton,
    Rows,
    Row(u32),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Item {
    Copy,
    Paste,
    Grid,
    Snap,
    Size(Size),
    Accent(usize),
    Arrange,
    Front,
    Back,
    Delete,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Default)]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];

    fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }
}

#[derive(Default)]
pub struct State {
    pub grid: bool,
    pub snap: bool,
    pub accent: usize,
    pub size: Size,
    pub list: bool,
    pub chosen: Option<String>,
}

/// Rows the list wants room for: more than most windows have.
pub const ROWS: u32 = 40;
pub const ROW_HEIGHT: f32 = 28.0;

/// ```html
/// <div class="flex h-full flex-col justify-between p-4">
///   <div> <button>Popover</button> </div>
///   <div> <button>List</button> </div>
/// </div>
/// ```
pub fn view<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.commands(&commands(), state);
    b.div_id(
        Part::Page,
        tw().flex().flex_col().justify_between().h_full().p(4.0),
        |s| {
            s.div_id(Part::Top, tw().flex().gap(2.0), |s| {
                actions(s, state);
                accent(s, state);
                if let Some(chosen) = &state.chosen {
                    s.text(tw().ml(4.0), format!("Chose {chosen}"));
                }
            });
            s.div_id(Part::Bottom, tw().flex(), |s| {
                s.button(Part::ListButton, "List")
                    .on_click(|state: &mut State| state.list = !state.list);
                if state.list {
                    list(s);
                }
            });
        },
    );
}

fn commands() -> Commands<State> {
    Commands::new()
        .add(
            Part::Item(Item::Copy),
            Command::new("Copy", |state: &mut State| {
                state.chosen = Some("Copy".into());
            })
            .shortcut(const { Chord::new(Modifiers::COMMAND, Key::C).expect("not reserved") }),
        )
        .add(
            Part::Item(Item::Paste),
            Command::new("Paste", |_| {})
                .shortcut(const { Chord::new(Modifiers::COMMAND, Key::V).expect("not reserved") })
                .enabled(|_| false),
        )
        .add(
            Part::Item(Item::Grid),
            Command::new("Show grid", |state: &mut State| state.grid = !state.grid)
                .checked(|state| Mark::from(state.grid))
                .shortcut(
                    const {
                        Chord::new(
                            Modifiers {
                                alt: true,
                                ..Modifiers::COMMAND
                            },
                            Key::G,
                        )
                        .expect("not reserved")
                    },
                ),
        )
}

fn actions(b: &mut Builder<'_, '_, State>, state: &State) {
    let (snap, size) = (state.snap, state.size);
    b.dropdown_menu(Part::PopoverButton, "Actions")
        .variant(eggshell_ui::theme::variant::OUTLINE)
        .chevron()
        .items(|m| {
            m.command(Part::Item(Item::Copy)).add();
            m.command(Part::Item(Item::Paste)).add();
            m.separator();
            m.label("View");
            m.command(Part::Item(Item::Grid)).add();
            m.checkbox_item(Part::Item(Item::Snap), "Snap to guides", Mark::from(snap))
                .on_change(|state: &mut State, mark| state.snap = mark == Mark::On);
            m.separator();
            m.label("Size");
            for each in Size::ALL {
                m.radio_item(
                    Part::Item(Item::Size(each)),
                    each.label(),
                    Mark::from(each == size),
                )
                .on_click(move |state: &mut State| state.size = each);
            }
            m.separator();
            m.submenu(Part::Item(Item::Arrange), "Arrange", |m| {
                m.item(Part::Item(Item::Front), "Bring to front")
                    .on_click(|state: &mut State| state.chosen = Some("Front".into()));
                m.item(Part::Item(Item::Back), "Send to back")
                    .on_click(|state: &mut State| state.chosen = Some("Back".into()));
            });
            m.separator();
            m.item(Part::Item(Item::Delete), "Delete")
                .shortcut("Del")
                .destructive()
                .on_click(|state: &mut State| state.chosen = Some("Delete".into()));
        });
}

/// The accents Accent offers: Tailwind's blue, green and rose 500.
const ACCENTS: [(&str, Color32); 3] = [
    ("Blue", Color32::from_rgb(0x3b, 0x82, 0xf6)),
    ("Green", Color32::from_rgb(0x22, 0xc5, 0x5e)),
    ("Rose", Color32::from_rgb(0xf4, 0x3f, 0x5e)),
];

fn accent(b: &mut Builder<'_, '_, State>, state: &State) {
    let chosen = state.accent;
    let color = ACCENTS
        .get(chosen)
        .map_or(Color32::GRAY, |(_, color)| *color);
    b.dropdown_menu(Part::Accent, Face::Empty("Accent".into()))
        .variant(eggshell_ui::theme::variant::OUTLINE)
        .children(move |s| {
            s.div_id(
                Part::Swatch,
                tw().size_px(14.0).rounded_full().bg(color),
                |_| {},
            );
        })
        .chevron()
        .menu_classes(tw().min_w_px(200.0))
        .items(|m| {
            for (at, (name, _)) in ACCENTS.iter().enumerate() {
                m.radio_item(
                    Part::Item(Item::Accent(at)),
                    *name,
                    Mark::from(at == chosen),
                )
                .on_click(move |state: &mut State| state.accent = at);
            }
        });
}

fn list(b: &mut Builder<'_, '_, State>) {
    let anchor = b.id_of(Part::ListButton);
    let list = common::popup_box(b);
    let row = common::popup_item(b);
    b.overlay(Part::List)
        .anchor(Anchor::Node(anchor))
        .placement(Placement::below().gap(Distance::new(4.0)))
        .match_width(MatchWidth::Anchor)
        .nav(Nav::Menu(Wrap::Around))
        .role(Role::Menu)
        .named("Options")
        .on_dismiss(|state: &mut State| state.list = false)
        .children(|s| {
            s.div_id(Part::Menu, list.min_w_px(160.0), |s| {
                s.scroll(Part::Rows, tw().h_px(ROWS as f32 * ROW_HEIGHT).min_h_0())
                    .axis(Axis::Y)
                    .children(|s| {
                        for index in 0..ROWS {
                            s.div_id(Part::Row(index), row.clone().h_px(ROW_HEIGHT), |s| {
                                s.text(tw(), format!("Option {index}"));
                            })
                            .role(Role::MenuItem)
                            .named(format!("Option {index}"))
                            .on_click(move |state: &mut State| {
                                state.chosen = Some(format!("option {index}"));
                                state.list = false;
                            });
                        }
                    });
            });
        });
}

fn main() -> eframe::Result {
    common::run_app("menu", State::default(), view)
}
