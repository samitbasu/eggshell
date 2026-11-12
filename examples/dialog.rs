//! Modals: a dialog, a confirmation opened from inside it, a role picker
//! nested in it, and a sheet docked to the right.
//!
//! A modal covers the screen with a scrim that takes every press and the
//! wheel (the page behind does not scroll) and traps focus. A press on the
//! scrim or Esc dismisses the top dialog; the confirmation must be answered
//! and takes only Esc. Return presses the default button (Save, Continue) and
//! Esc the cancel button (Cancel), wherever the focus is inside the dialog.
//!
//! The boxes are ordinary classes copying shadcn's `DialogContent` and
//! `SheetContent`; the scrim is the style's.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::overlay::{DismissOn, Modal, Nav, Side};
use eggshell_ui::prim::text_field::Field;
use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, Scope, Tw, tw};
use eggshell_ui_shadcn::{Tokens, variant};
use egui::{Color32, Role, Shadow};

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Intro,
    Actions,
    OpenDialog,
    OpenSheet,
    Row(u32),
    Dialog,
    Box,
    Group(&'static str),
    Name,
    Role,
    RoleButton,
    RoleMenu,
    RoleItem(usize),
    Footer,
    Delete,
    Cancel,
    Save,
    Confirm,
    Continue,
    Sheet,
    Close,
}

pub const ROLES: [&str; 3] = ["Viewer", "Editor", "Owner"];

/// Rows of filler under the buttons, so that the page can scroll.
const ROWS: u32 = 40;

#[derive(Default)]
pub struct State {
    pub dialog: bool,
    pub confirm: bool,
    pub picker: bool,
    pub sheet: bool,
    pub name: String,
    pub role: usize,
    pub saved: Option<String>,
    pub deleted: u32,
}

/// `shadow-lg`, without its negative spread.
fn shadow_lg() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 15,
        spread: 0,
        color: Color32::from_black_alpha(26),
    }
}

/// ```html
/// <div class="grid w-full gap-4 border bg-background p-6 shadow-lg sm:rounded-lg"> … </div>
/// ```
fn content_box(t: &Tokens) -> Tw {
    tw().flex()
        .flex_col()
        .gap(4.0)
        .p(6.0)
        .bg(t.background)
        .border(1.0)
        .border_color(t.border)
        .shadow(shadow_lg())
}

/// ```html
/// <div class="fixed … w-full max-w-lg rounded-lg border bg-background p-6 shadow-lg"> … </div>
/// ```
fn dialog_box(t: &Tokens) -> Tw {
    content_box(t)
        .w_full()
        .max_w_px(512.0)
        .rounded_px(f32::from(t.radius))
}

/// ```html
/// <div class="h-full w-3/4 border-l bg-background p-6 shadow-lg sm:max-w-sm"> … </div>
/// ```
fn sheet_box(t: &Tokens) -> Tw {
    content_box(t)
        .h_full()
        .w_frac(3.0, 4.0)
        .max_w_px(384.0)
        .border(0.0)
        .border_l(1.0)
}

/// ```html
/// <div class="flex flex-col gap-2">
///   <h2 class="text-lg font-semibold">title</h2>
///   <p class="text-sm text-muted-foreground">description</p>
/// </div>
/// ```
fn header(s: &mut Scope<'_, '_, State>, t: &Tokens, title: &str, description: &str) {
    s.div(tw().flex().flex_col().gap(2.0), |s| {
        s.text(tw().role(common::sized(18.0)), title);
        s.text(
            tw().role(common::sized(14.0)).text(t.muted_foreground),
            description,
        );
    });
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
        s.text(tw().role(common::sized(14.0)), label);
        control(s);
    });
}

/// ```html
/// <div class="flex flex-col gap-6 p-6">
///   <div class="flex gap-2">  <button>Edit profile</button> <button>Open sheet</button> </div>
///   <p>Row 0</p> …
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Page, tw().w_full().h_full())
        .axis(Axis::Y)
        .children(|s| {
            s.div_id(Part::Intro, tw().flex().flex_col().gap(6.0).p(6.0), |s| {
                s.div_id(Part::Actions, tw().flex().items_center().gap(2.0), |s| {
                    s.button(Part::OpenDialog, "Edit profile")
                        .on_click(|state: &mut State| state.dialog = true);
                    s.button(Part::OpenSheet, "Open sheet")
                        .variant(variant::OUTLINE)
                        .on_click(|state: &mut State| state.sheet = true);
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
    if state.dialog {
        dialog(b, state, &t);
    }
    if state.sheet {
        sheet(b, &t);
    }
}

fn dialog(b: &mut Builder<'_, '_, State>, state: &State, t: &Tokens) {
    let dialog = dialog_box(t);
    b.overlay(Part::Dialog)
        .modal(Modal::Center)
        .role(Role::Dialog)
        .named("Edit profile")
        .on_dismiss(|state: &mut State| {
            state.dialog = false;
            state.picker = false;
        })
        .children(|s| {
            s.div_id(Part::Box, dialog, |s| {
                header(
                    s,
                    t,
                    "Edit profile",
                    "Make changes to your profile here. Click save when you're done.",
                );
                field(s, "Name", |s| {
                    common::text_box(
                        s,
                        Part::Name,
                        ("Name", Role::TextInput),
                        Field::singleline(&state.name).hint("Your name"),
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
                s.div_id(Part::Footer, tw().flex().justify_between(), |s| {
                    s.button(Part::Delete, "Delete")
                        .variant(variant::DESTRUCTIVE)
                        .on_click(|state: &mut State| state.confirm = true);
                    s.div(tw().flex().gap(2.0), |s| {
                        s.button(Part::Cancel, "Cancel")
                            .variant(variant::OUTLINE)
                            .cancel_action()
                            .on_click(|state: &mut State| {
                                state.dialog = false;
                                state.picker = false;
                            });
                        s.button(Part::Save, "Save changes")
                            .variant(variant::PRIMARY)
                            .default_action()
                            .on_click(|state: &mut State| {
                                state.saved = Some(state.name.clone());
                                state.dialog = false;
                                state.picker = false;
                            });
                    });
                });
                if state.confirm {
                    confirm(s, t);
                }
            });
        });
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
        .nav(Nav::Menu)
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

fn confirm(s: &mut Scope<'_, '_, State>, t: &Tokens) {
    let dialog = dialog_box(t);
    s.overlay(Part::Confirm)
        .modal(Modal::Center)
        .role(Role::AlertDialog)
        .named("Are you absolutely sure?")
        .dismiss_on(DismissOn::EscOnly)
        .on_dismiss(|state: &mut State| state.confirm = false)
        .children(|s| {
            s.div_id(Part::Box, dialog.max_w_px(448.0), |s| {
                header(
                    s,
                    t,
                    "Are you absolutely sure?",
                    "This action cannot be undone. This will permanently delete your profile.",
                );
                s.div(tw().flex().justify_end().gap(2.0), |s| {
                    s.button(Part::Cancel, "Cancel")
                        .variant(variant::OUTLINE)
                        .cancel_action()
                        .on_click(|state: &mut State| state.confirm = false);
                    s.button(Part::Continue, "Continue")
                        .variant(variant::DESTRUCTIVE)
                        .default_action()
                        .on_click(|state: &mut State| {
                            state.deleted += 1;
                            state.confirm = false;
                            state.dialog = false;
                            state.picker = false;
                        });
                });
            });
        });
}

fn sheet(b: &mut Builder<'_, '_, State>, t: &Tokens) {
    let sheet = sheet_box(t);
    b.overlay(Part::Sheet)
        .modal(Modal::Edge(Side::Right))
        .role(Role::Dialog)
        .named("Settings")
        .on_dismiss(|state: &mut State| state.sheet = false)
        .children(|s| {
            s.div_id(Part::Box, sheet, |s| {
                header(
                    s,
                    t,
                    "Settings",
                    "Adjust how the app behaves. Esc or a click outside closes the sheet.",
                );
                s.div(tw().flex().justify_end().mt(4.0), |s| {
                    s.button(Part::Close, "Close")
                        .variant(variant::OUTLINE)
                        .cancel_action()
                        .on_click(|state: &mut State| state.sheet = false);
                });
            });
        });
}

fn main() -> eframe::Result {
    common::run_app_styled("dialog", State::default(), common::use_accent, layout)
}
