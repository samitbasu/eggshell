//! Overlays: a stock dropdown menu with a submenu, check and radio items, and
//! a tall list that flips to the side with room, clamps to it and scrolls.
//!
//! The dropdown owns whether it is open. Click it, or press Enter, Space or
//! Down on it; the arrows, Home and End move between the items, Enter
//! chooses, Right opens the submenu and Left closes it. Click away, press Esc
//! or Tab out to close. The list is a hand-built overlay of the same look.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::overlay::{Anchor, Distance, MatchWidth, Nav, Placement};
use eggshell_ui::owned::{Enablement, Mark};
use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, tw};
use egui::Role;

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Top,
    Bottom,
    PopoverButton,
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
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    b.div_id(
        Part::Page,
        tw().flex().flex_col().justify_between().h_full().p(4.0),
        |s| {
            s.div_id(Part::Top, tw().flex().gap(2.0), |s| {
                actions(s, state);
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

fn actions(b: &mut Builder<'_, '_, State>, state: &State) {
    let (grid, snap, size) = (state.grid, state.snap, state.size);
    b.dropdown_menu(Part::PopoverButton, "Actions")
        .variant(eggshell_ui_shadcn::variant::OUTLINE)
        .items(|m| {
            m.item(Part::Item(Item::Copy), "Copy")
                .shortcut("Ctrl+C")
                .on_click(|state: &mut State| state.chosen = Some("Copy".into()));
            m.item(Part::Item(Item::Paste), "Paste")
                .shortcut("Ctrl+V")
                .enablement(Enablement::Disabled)
                .on_click(|_| {});
            m.separator();
            m.label("View");
            m.checkbox_item(Part::Item(Item::Grid), "Show grid", Mark::from(grid))
                .on_change(|state: &mut State, mark| state.grid = mark == Mark::On);
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

fn list(b: &mut Builder<'_, '_, State>) {
    let anchor = b.id_of(Part::ListButton);
    let list = common::popup_box(b);
    let row = common::popup_item(b);
    b.overlay(Part::List)
        .anchor(Anchor::Node(anchor))
        .placement(Placement::below().gap(Distance::new(4.0)))
        .match_width(MatchWidth::Anchor)
        .nav(Nav::Menu)
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
    common::run_app("menu", State::default(), layout)
}
