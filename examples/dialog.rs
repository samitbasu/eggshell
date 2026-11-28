//! Modals: a dialog, a confirmation opened from inside it, a role picker
//! nested in it, and a sheet docked to the right.
//!
//! A modal covers the screen with a scrim that takes every press and the
//! wheel (the page behind does not scroll) and traps focus. A press on the
//! scrim or Esc dismisses the top dialog; the confirmation must be answered
//! and takes only Esc. Return presses the default button (Save, Continue) and
//! Esc the cancel button (Cancel), wherever the focus is inside the dialog.
//!
//! The boxes, headers, footers and close buttons are eggshell-ui-extras'
//! dialog surface, in the style's look; the scrim is the core style's.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::common;

use eggshell_ui::overlay::{Modal, Nav, Shown, Side, Wrap};
use eggshell_ui::owned::dialog::Dialog;
use eggshell_ui::owned::text_field::Field;
use eggshell_ui::prelude::*;
use eggshell_ui::scroll::Axis;
use eggshell_ui::theme::variant;
use eggshell_ui::{Builder, Scope, tw};
use egui::Role;

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Intro,
    Actions,
    OpenDialog,
    OpenSheet,
    Row(u32),
    Dialog,
    Group(&'static str),
    Name,
    Role,
    RoleButton,
    RoleMenu,
    RoleItem(usize),
    Confirm,
    Sheet,
}

pub const ROLES: [&str; 3] = ["Viewer", "Editor", "Owner"];

/// Rows of filler under the buttons, so that the page can scroll.
const ROWS: u32 = 40;

#[derive(Default)]
pub struct State {
    pub dialog: Shown,
    pub confirm: Shown,
    pub picker: bool,
    pub sheet: Shown,
    pub name: String,
    pub role: usize,
    pub saved: Option<String>,
    pub deleted: u32,
}

/// ```html
/// <div class="flex flex-col gap-2"> <label class="text-sm font-medium">label</label> … </div>
/// ```
fn field<'f>(
    s: &mut Scope<'_, 'f, State>,
    label: &'static str,
    control: impl FnOnce(&mut Scope<'_, 'f, State>),
) {
    s.div_id(Part::Group(label), tw().flex().flex_col().gap(2.0), |s| {
        s.text(tw().role(common::sized(14.0)).font_medium(), label);
        control(s);
    });
}

/// ```html
/// <div class="flex flex-col gap-6 p-6">
///   <div class="flex gap-2">  <button>Edit profile</button> <button>Open sheet</button> </div>
///   <p>Row 0</p> …
/// </div>
/// ```
pub fn view(b: &mut Builder<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Page, tw().w_full().h_full())
        .axis(Axis::Y)
        .children(|s| {
            s.div_id(Part::Intro, tw().flex().flex_col().gap(6.0).p(6.0), |s| {
                s.div_id(Part::Actions, tw().flex().items_center().gap(2.0), |s| {
                    s.button(Part::OpenDialog, "Edit profile")
                        .on_click(|state: &mut State| state.dialog = Shown::Open);
                    s.button(Part::OpenSheet, "Open sheet")
                        .variant(variant::OUTLINE)
                        .on_click(|state: &mut State| state.sheet = Shown::Open);
                    if let Some(saved) = &state.saved {
                        s.text(
                            tw().text(t.muted_foreground),
                            format!("Saved {saved}, {}", ROLES[state.role]),
                        );
                    }
                    if state.deleted > 0 {
                        s.text(
                            tw().text(t.muted_foreground),
                            format!("Deleted {} time(s)", state.deleted),
                        );
                    }
                });
                for row in 0..ROWS {
                    s.text(
                        tw().text(t.muted_foreground),
                        format!("Row {row}: the wheel scrolls this page, until a modal is open."),
                    );
                }
            });
        });
    dialog(b, state);
    sheet(b, state);
}

fn dialog(b: &mut Builder<'_, '_, State>, state: &State) {
    b.overlay(Part::Dialog)
        .modal(Modal::Center)
        .shown(state.dialog)
        .on_dismiss(|state: &mut State| {
            state.dialog = Shown::Closed;
            state.picker = false;
        })
        .dialog(
            Dialog::new("Edit profile")
                .description("Make changes to your profile here. Click save when you're done."),
            |d| {
                d.body(|s| {
                    field(s, "Name", |s| {
                        common::text_box(
                            s,
                            Part::Name,
                            ("Name", Role::TextInput),
                            (Field::singleline(&state.name).hint("Your name"), None),
                            |state: &mut State| &mut state.name,
                        );
                    });
                    field(s, "Role", |s| {
                        s.button(Part::RoleButton, ROLES[state.role])
                            .variant(variant::OUTLINE)
                            .opens_popup()
                            .classes(tw().justify_between().w_full())
                            .named("Role")
                            .on_click(|state: &mut State| state.picker = !state.picker);
                        if state.picker {
                            role_menu(s, state);
                        }
                    });
                    confirm(s, state);
                });
                d.footer(|f| {
                    f.leading("Delete", |button| {
                        button
                            .variant(variant::DESTRUCTIVE)
                            .on_click(|state: &mut State| state.confirm = Shown::Open);
                    })
                    .cancel("Cancel", |button| button.add())
                    .default("Save changes", |button| {
                        button.on_click(|state: &mut State| {
                            state.saved = Some(state.name.clone());
                            state.dialog = Shown::Closed;
                            state.picker = false;
                        });
                    })
                });
            },
        );
}

fn role_menu(s: &mut Scope<'_, '_, State>, state: &State) {
    let anchor = s.id_of(Part::RoleButton);
    let menu = common::popup_box(s);
    let item = common::popup_item(s);
    s.overlay(Part::RoleMenu)
        .anchor(eggshell_ui::overlay::Anchor::Node(anchor))
        .placement(
            eggshell_ui::overlay::Placement::below().gap(eggshell_ui::overlay::Distance::new(4.0)),
        )
        .match_width(eggshell_ui::overlay::MatchWidth::Anchor)
        .nav(Nav::Menu(Wrap::Around))
        .role(Role::Menu)
        .named("Role")
        .on_dismiss(|state: &mut State| state.picker = false)
        .children(|s| {
            s.div_id(Part::Role, menu, |s| {
                for (index, role) in ROLES.iter().enumerate() {
                    let selected = index == state.role;
                    s.div_id(Part::RoleItem(index), item.clone(), |s| {
                        s.text(
                            tw(),
                            if selected {
                                format!("{role}  (current)")
                            } else {
                                (*role).to_owned()
                            },
                        );
                    })
                    .role(Role::MenuItem)
                    .named(*role)
                    .on_click(move |state: &mut State| {
                        state.role = index;
                        state.picker = false;
                    });
                }
            });
        });
}

fn confirm(s: &mut Scope<'_, '_, State>, state: &State) {
    s.overlay(Part::Confirm)
        .modal(Modal::Center)
        .shown(state.confirm)
        .on_dismiss(|state: &mut State| state.confirm = Shown::Closed)
        .alert_dialog(
            Dialog::new("Are you absolutely sure?")
                .description(
                    "This action cannot be undone. This will permanently delete your profile.",
                )
                .classes(tw().max_w_px(448.0)),
            |d| {
                d.footer(|f| {
                    f.cancel("Cancel", |button| button.add())
                        .default("Continue", |button| {
                            button
                                .variant(variant::DESTRUCTIVE)
                                .on_click(|state: &mut State| {
                                    state.deleted += 1;
                                    state.confirm = Shown::Closed;
                                    state.dialog = Shown::Closed;
                                    state.picker = false;
                                });
                        })
                });
            },
        );
}

fn sheet(b: &mut Builder<'_, '_, State>, state: &State) {
    b.overlay(Part::Sheet)
        .modal(Modal::Edge(Side::Right))
        .shown(state.sheet)
        .on_dismiss(|state: &mut State| state.sheet = Shown::Closed)
        .dialog(
            Dialog::new("Settings").description(
                "Adjust how the app behaves. Esc or a click outside closes the sheet.",
            ),
            |d| {
                d.footer(|f| f.cancel("Close", |button| button.add()));
            },
        );
}

fn main() -> eframe::Result {
    common::run_app_styled("dialog", State::default(), common::use_accent, view)
}
