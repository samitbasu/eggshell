//! Overlays: a popover under its button, a submenu nested in it, and a tall
//! list that flips to the side with room, clamps to it and scrolls.
//!
//! An overlay is declared in the scope of its button, so that it finds the
//! button's id with `id_of`. The app owns the open state and the library calls
//! `on_dismiss` for a press outside, Esc, or focus leaving: click away, press
//! Esc, or Tab out to close. The menus take the arrow keys, Home and End; Enter
//! chooses; Left closes the submenu. The boxes reuse the select's popup look.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::overlay::{Anchor, Distance, MatchWidth, Nav, Placement};
use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, Scope, Style, Tw, tw};
use egui::Role;

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Top,
    Bottom,
    Popover,
    PopoverButton,
    Menu,
    Item(Item),
    Submenu,
    List,
    ListButton,
    Rows,
    Row(u32),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Item {
    Copy,
    Arrange,
    Delete,
    Front,
    Back,
}

#[derive(Default)]
pub struct State {
    pub popover: bool,
    pub submenu: bool,
    pub list: bool,
    pub chosen: Option<String>,
}

/// Rows the list wants room for: more than most windows have.
pub const ROWS: u32 = 40;
pub const ROW_HEIGHT: f32 = 28.0;

/// The box of a menu or list: the select's popup look.
fn popup_box(b: &Builder<'_, '_, State>) -> Tw {
    let select = &Style::of(b.ctx()).select;
    let colors = select.popup.paint.states.rest;
    let classes = tw()
        .flex()
        .flex_col()
        .min_h_0()
        .bg(colors.fill)
        .border(select.popup.border_width)
        .border_color(colors.border)
        .rounded_px(f32::from(select.popup_radius))
        .p(select.popup_padding / 4.0)
        .text(colors.text);
    match select.popup.paint.shadow {
        Some(shadow) => classes.shadow(shadow),
        None => classes,
    }
}

/// An item of a menu or list: the select's item look.
fn item_classes(b: &Builder<'_, '_, State>) -> Tw {
    let select = &Style::of(b.ctx()).select;
    let hover = select.item.paint.states.hover;
    tw().px(select.item_padding.x / 4.0)
        .py(select.item_padding.y / 4.0)
        .rounded_px(f32::from(select.item_radius))
        .sense_click()
        .hover(move |p| p.bg(hover.fill).text(hover.text))
}

fn menu_item(s: &mut Scope<'_, '_, State>, item: Item, label: &str, act: fn(&mut State)) {
    let classes = item_classes(s);
    s.div_id(Part::Item(item), classes, |s| s.text(tw(), label))
        .role(Role::MenuItem)
        .named(label)
        .on_click(act);
}

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
                s.button(Part::PopoverButton, "Popover")
                    .on_click(|state: &mut State| {
                        state.popover = !state.popover;
                        state.submenu = false;
                    });
                if let Some(chosen) = &state.chosen {
                    s.text(tw().ml(4.0), format!("Chose {chosen}"));
                }
                if state.popover {
                    popover(s, state);
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

fn popover(b: &mut Builder<'_, '_, State>, state: &State) {
    let anchor = b.id_of(Part::PopoverButton);
    let menu = popup_box(b);
    b.overlay(Part::Popover)
        .anchor(Anchor::Node(anchor))
        .placement(Placement::below().gap(Distance::new(4.0)))
        .nav(Nav::Menu)
        .role(Role::Menu)
        .named("Actions")
        .on_dismiss(|state: &mut State| {
            state.popover = false;
            state.submenu = false;
        })
        .children(|s| {
            s.div_id(Part::Menu, menu.min_w_px(128.0), |s| {
                menu_item(s, Item::Copy, "Copy", |state| {
                    state.chosen = Some("Copy".into());
                    state.popover = false;
                });
                menu_item(s, Item::Arrange, "Arrange  >", |state| {
                    state.submenu = !state.submenu;
                });
                menu_item(s, Item::Delete, "Delete", |state| {
                    state.chosen = Some("Delete".into());
                    state.popover = false;
                });
                if state.submenu {
                    let parent = s.id_of(Part::Item(Item::Arrange));
                    let nested = popup_box(s);
                    s.overlay(Part::Submenu)
                        .anchor(Anchor::Node(parent))
                        .placement(Placement::submenu())
                        .nav(Nav::Submenu)
                        .role(Role::Menu)
                        .named("Arrange")
                        .on_dismiss(|state: &mut State| state.submenu = false)
                        .children(|s| {
                            s.div_id(Part::Menu, nested.min_w_px(128.0), |s| {
                                menu_item(s, Item::Front, "Bring to front", |state| {
                                    state.chosen = Some("Front".into());
                                    state.popover = false;
                                });
                                menu_item(s, Item::Back, "Send to back", |state| {
                                    state.chosen = Some("Back".into());
                                    state.popover = false;
                                });
                            });
                        });
                }
            });
        });
}

fn list(b: &mut Builder<'_, '_, State>) {
    let anchor = b.id_of(Part::ListButton);
    let list = popup_box(b);
    let row = item_classes(b);
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
