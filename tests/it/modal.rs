//! Modals and default/cancel actions through the real path: events dispatched
//! by `egui_kittest::Harness`, build, layout, draw and the judgement of the
//! frame's input.

use eggshell_ui::overlay::{DismissOn, Modal, Shown, Side};
use eggshell_ui::owned::Enablement;
use eggshell_ui::owned::text_field::Field;
use eggshell_ui::testing::{Driver, PlacedFrame, Target};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Key, Pos2, Role, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_node, id_of};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dialog.rs"]
mod dialog;

const SCREEN: egui::Vec2 = vec2(800.0, 600.0);

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

/// What Return and Esc do, and what dismissal does, in a modal of our own
/// making so that every combination can be set up.
mod actions {
    use super::*;

    #[derive(Clone, Copy, Debug, Hash)]
    enum P {
        Page,
        Open,
        PageOk,
        PageCancel,
        Dlg,
        Box,
        Name,
        Notes,
        Check,
        Radio,
        Switch,
        Ok,
        Cancel,
        Other,
    }

    #[derive(Default)]
    struct App {
        open: bool,
        log: Vec<String>,
        cancel: bool,
        default: bool,
        page_actions: bool,
        dismiss_on: DismissOn,
        default_disabled: bool,
        cancel_disabled: bool,
        name: String,
        notes: String,
        check: bool,
        radio: u8,
        switch: bool,
    }

    fn log(entry: &'static str) -> impl FnOnce(&mut App) + 'static {
        move |app| app.log.push(entry.into())
    }

    fn enablement(disabled: bool) -> Enablement {
        if disabled {
            Enablement::Disabled
        } else {
            Enablement::Enabled
        }
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(
            P::Page,
            tw().flex().flex_col().gap(2.0).w_full().h_full().p(2.0),
            |s| {
                s.button(P::Open, "Open")
                    .on_click(|app: &mut App| app.open = true);
                if app.page_actions {
                    s.button(P::PageOk, "Page ok")
                        .default_action()
                        .on_click(log("page default"));
                    s.button(P::PageCancel, "Page cancel")
                        .cancel_action()
                        .on_click(log("page cancel"));
                }
            },
        );
        if app.open {
            b.overlay(P::Dlg)
                .modal(Modal::Center)
                .dismiss_on(app.dismiss_on)
                .on_dismiss(|app: &mut App| {
                    app.open = false;
                    app.log.push("dismissed".into());
                })
                .children(|s| {
                    s.div_id(
                        P::Box,
                        tw().flex()
                            .flex_col()
                            .gap(2.0)
                            .w_px(300.0)
                            .p(2.0)
                            .bg(Color32::WHITE),
                        |s| {
                            s.text_field_with(P::Name, Field::singleline(&app.name).named("Name"))
                                .on_change(|app: &mut App, text| app.name = text);
                            s.text_field_with(
                                P::Notes,
                                Field::multiline(&app.notes, 3).named("Notes"),
                            )
                            .on_change(|app: &mut App, text| app.notes = text);
                            s.div(tw().flex().items_center().gap(2.0), |s| {
                                s.checkbox(P::Check, "Check", app.check)
                                    .on_change(|app: &mut App, check| app.check = check);
                                s.radio_group(P::Radio, app.radio, [(0, "Zero"), (1, "One")])
                                    .on_change(|app: &mut App, radio| app.radio = radio);
                                s.switch(P::Switch, "Switch", app.switch)
                                    .on_change(|app: &mut App, switch| app.switch = switch);
                            });
                            s.button(P::Other, "Other").on_click(log("other"));
                            let mut ok = s
                                .button(P::Ok, "Ok")
                                .enablement(enablement(app.default_disabled));
                            if app.default {
                                ok = ok.default_action();
                            }
                            ok.on_click(log("default"));
                            let mut cancel = s
                                .button(P::Cancel, "Cancel")
                                .enablement(enablement(app.cancel_disabled));
                            if app.cancel {
                                cancel = cancel.cancel_action();
                            }
                            cancel.on_click(|app: &mut App| {
                                app.log.push("cancel".into());
                                app.open = false;
                            });
                        },
                    );
                });
        }
    }

    fn harness(app: App) -> Driver<App> {
        let mut d = support::driver(vec2(400.0, 400.0), base_text(), app, view).build();
        d.frames(4);
        d
    }

    fn opened(app: App) -> Driver<App> {
        let mut d = harness(App { open: true, ..app });
        d.frames(12);
        d
    }

    fn log_of(d: &Driver<App>) -> Vec<String> {
        d.app().log.clone()
    }

    fn focus(d: &mut Driver<App>, path: &str) {
        d.click(Target::suffix(path)).frames(12);
        assert!(
            focused_node(d).is_some_and(|node| node.starts_with(path)),
            "precondition: {path} has focus, not {:?}",
            focused_node(d)
        );
    }

    fn tab_to(d: &mut Driver<App>, path: &str) {
        for _ in 0..12 {
            if support::focused_path(d).as_deref() == Some(path) {
                return;
            }
            d.press(Key::Tab).frames(11);
        }
        panic!("Tab never reached {path}: {:?}", support::focused_path(d));
    }

    #[test]
    fn a_press_on_the_scrim_dismisses_and_the_press_inside_the_box_does_not() {
        let mut d = opened(App::default());
        let padding = d.rect(Target::suffix("/Dlg/Box")).min + vec2(4.0, 4.0);
        d.click(padding).frames(12);
        assert!(d.app().open, "the box is not the scrim");
        let scrim = pos2(5.0, 5.0);
        assert!(
            !d.rect(Target::suffix("/Dlg/Box")).contains(scrim),
            "precondition"
        );
        d.click(scrim).frames(12);
        assert_eq!(log_of(&d), ["dismissed"]);
        assert!(!d.app().open);
    }

    #[test]
    fn esc_dismisses_a_modal_with_no_cancel_button() {
        let mut d = opened(App::default());
        d.press(Key::Escape).frames(11);
        assert_eq!(log_of(&d), ["dismissed"]);
    }

    #[test]
    fn a_modal_that_must_be_answered_takes_esc_but_not_the_scrim() {
        let mut d = opened(App {
            dismiss_on: DismissOn::EscOnly,
            ..App::default()
        });
        d.click(pos2(5.0, 5.0)).frames(12);
        assert!(log_of(&d).is_empty(), "the scrim does nothing");
        d.press(Key::Escape).frames(11);
        assert_eq!(log_of(&d), ["dismissed"]);
    }

    #[test]
    fn never_ignores_both() {
        let mut d = opened(App {
            dismiss_on: DismissOn::Never,
            ..App::default()
        });
        d.click(pos2(5.0, 5.0)).frames(12);
        d.press(Key::Escape).frames(11);
        assert!(log_of(&d).is_empty());
        assert!(d.app().open);
    }

    #[test]
    fn a_press_on_the_scrim_is_swallowed_and_does_not_reach_the_page() {
        let mut d = opened(App::default());
        let page_button = d.rect(Target::suffix("/Page/Open")).center();
        assert!(
            !d.rect(Target::suffix("/Dlg/Box")).contains(page_button),
            "precondition: the page's button is not under the box"
        );
        d.app_mut().open = true;
        d.click(page_button).frames(12);
        assert_eq!(log_of(&d), ["dismissed"]);
        assert!(!d.app().open, "the click that closed it did not reopen it");
    }

    #[test]
    fn esc_presses_the_cancel_button_instead_of_dismissing() {
        let mut d = opened(App {
            cancel: true,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert_eq!(
            log_of(&d),
            ["cancel"],
            "the button ran, and on_dismiss did not"
        );
        assert!(!d.app().open);
    }

    #[test]
    fn the_scrim_still_dismisses_when_there_is_a_cancel_button() {
        let mut d = opened(App {
            cancel: true,
            ..App::default()
        });
        d.click(pos2(5.0, 5.0)).frames(12);
        assert_eq!(log_of(&d), ["dismissed"]);
    }

    #[test]
    fn a_disabled_cancel_button_holds_the_modal_open_and_takes_the_key() {
        let mut d = opened(App {
            cancel: true,
            cancel_disabled: true,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert!(log_of(&d).is_empty());
        assert!(d.app().open);
    }

    #[test]
    fn a_cancel_button_takes_esc_whatever_dismiss_on_says() {
        let mut d = opened(App {
            cancel: true,
            dismiss_on: DismissOn::Never,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert_eq!(log_of(&d), ["cancel"]);
    }

    #[test]
    fn esc_goes_to_the_page_cancel_button_when_no_overlay_is_open() {
        let mut d = harness(App {
            page_actions: true,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert_eq!(log_of(&d), ["page cancel"]);
    }

    #[test]
    fn esc_goes_to_the_modal_and_not_to_the_page_cancel_button() {
        let mut d = opened(App {
            page_actions: true,
            cancel: true,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert_eq!(log_of(&d), ["cancel"]);
    }

    #[test]
    fn return_in_a_single_line_field_presses_the_default_button() {
        let mut d = opened(App {
            default: true,
            ..App::default()
        });
        focus(&mut d, "/Dlg/Box/Name");
        d.press(Key::Enter).frames(11);
        assert_eq!(log_of(&d), ["default"]);
    }

    #[test]
    fn return_with_nothing_focused_in_the_modal_presses_its_default() {
        let mut d = opened(App {
            default: true,
            page_actions: true,
            ..App::default()
        });
        d.ctx().memory_mut(|memory| memory.stop_text_input());
        d.ctx().memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        d.frames(12);
        d.press(Key::Enter).frames(11);
        assert_eq!(log_of(&d), ["default"], "the modal's, not the page's");
    }

    #[test]
    fn return_in_a_multi_line_field_is_the_fields() {
        let mut d = opened(App {
            default: true,
            ..App::default()
        });
        focus(&mut d, "/Dlg/Box/Notes");
        d.press(Key::Enter).frames(11);
        assert!(log_of(&d).is_empty());
        assert_eq!(d.app().notes, "\n", "the field took the newline");
    }

    #[test]
    fn return_on_a_focused_button_is_that_buttons() {
        let mut d = opened(App {
            default: true,
            ..App::default()
        });
        tab_to(&mut d, "/Dlg/Box/Other");
        d.press(Key::Enter).frames(11);
        assert_eq!(log_of(&d), ["other"], "not also the default");
    }

    /// Tabs until a node at or under `path` has focus.
    fn tab_into(d: &mut Driver<App>, path: &str) {
        for _ in 0..12 {
            if focused_node(d).is_some_and(|node| node.starts_with(path)) {
                return;
            }
            d.press(Key::Tab).frames(11);
        }
        panic!("Tab never reached {path}: {:?}", focused_node(d));
    }

    #[test]
    fn return_on_a_focused_checkbox_radio_or_switch_presses_the_default_button() {
        for control in ["/Dlg/Box/Check", "/Dlg/Box/Radio", "/Dlg/Box/Switch"] {
            let mut d = opened(App {
                default: true,
                ..App::default()
            });
            tab_into(&mut d, control);
            d.press(Key::Enter).frames(11);
            assert_eq!(log_of(&d), ["default"], "{control}");
            let app = &d.app();
            assert_eq!(
                (app.check, app.radio, app.switch),
                (false, 0, false),
                "Return did not also change {control}"
            );
        }
    }

    #[test]
    fn a_disabled_default_button_does_not_fire() {
        let mut d = opened(App {
            default: true,
            default_disabled: true,
            ..App::default()
        });
        focus(&mut d, "/Dlg/Box/Name");
        d.press(Key::Enter).frames(11);
        assert!(log_of(&d).is_empty());
    }

    #[test]
    fn return_belongs_to_the_modal_and_not_to_the_page_default() {
        let mut d = opened(App {
            default: true,
            page_actions: true,
            ..App::default()
        });
        focus(&mut d, "/Dlg/Box/Name");
        d.press(Key::Enter).frames(11);
        assert_eq!(log_of(&d), ["default"]);
    }

    #[test]
    fn return_without_an_overlay_presses_the_page_default() {
        let mut d = harness(App {
            page_actions: true,
            ..App::default()
        });
        d.press(Key::Enter).frames(11);
        assert_eq!(log_of(&d), ["page default"]);
    }

    #[test]
    fn return_with_no_default_button_does_nothing() {
        let mut d = opened(App::default());
        focus(&mut d, "/Dlg/Box/Name");
        d.press(Key::Enter).frames(11);
        assert!(log_of(&d).is_empty());
    }
}

/// The dialog example: a dialog, a confirmation opened from it, a role menu
/// nested in it, and a sheet, over a page that scrolls.
mod example {
    use super::dialog::{Part, State, layout};
    use super::*;

    fn harness(state: State) -> Driver<State> {
        let mut d = support::prepared_driver(
            SCREEN,
            common::base_text(egui::Theme::Light),
            state,
            layout,
            common::use_accent,
        )
        .build();
        support::set_page(&d, support::Page::Themed);
        d.frames(4);
        d
    }

    fn dialog() -> Driver<State> {
        let mut d = harness(State::default());
        d.click(Target::suffix("/Page/Intro/Actions/OpenDialog"))
            .frames(12);
        assert!(d.app().dialog.is_open(), "precondition: open");
        d
    }

    fn confirm() -> Driver<State> {
        let mut d = dialog();
        d.click(Target::suffix("/Dialog/Box/Footer/Leading"))
            .frames(12);
        assert!(d.app().confirm.is_open(), "precondition: confirm open");
        d
    }

    fn page_offset(d: &Driver<State>) -> f32 {
        let scroll = id_of(d, "/Page");
        d.shell()
            .viewport(scroll)
            .expect("the page was laid out")
            .offset()
            .y
    }

    fn focus(d: &Driver<State>) -> String {
        focused_node(d).unwrap_or_default()
    }

    fn tab(d: &mut Driver<State>) {
        d.press(Key::Tab).frames(11);
    }

    fn lap(d: &mut Driver<State>) -> Vec<String> {
        (0..16)
            .map(|_| {
                tab(d);
                focus(d)
            })
            .collect()
    }

    #[test]
    fn a_dialog_covers_the_screen_with_the_scrim_and_centres_its_box() {
        let d = dialog();
        let screen = egui::Rect::from_min_size(Pos2::ZERO, SCREEN);
        assert_eq!(d.rect(Target::suffix("/Dialog")), screen);
        let boxed = d.rect(Target::suffix("/Dialog/Box"));
        assert!(
            (boxed.center() - screen.center()).length() < 1.0,
            "{boxed:?} is centred"
        );
        assert!(
            boxed.width() <= 512.0 + 0.5 && boxed.width() > 400.0,
            "{boxed:?}"
        );
        let scrim = d.shell().style().scrim;
        assert_eq!(
            support::fill_of(&d, "/Dialog"),
            Some(scrim),
            "the root is painted with the style's scrim"
        );
    }

    #[test]
    fn a_sheet_docks_to_its_side_and_stretches_along_it() {
        let mut d = harness(State::default());
        d.click(Target::suffix("/Page/Intro/Actions/OpenSheet"))
            .frames(12);
        let boxed = d.rect(Target::suffix("/Sheet/Box"));
        assert!(d.app().sheet.is_open(), "precondition: open");
        assert_eq!(boxed.max.x, SCREEN.x);
        assert_eq!((boxed.min.y, boxed.max.y), (0.0, SCREEN.y));
        assert!(boxed.width() <= 384.0 + 0.5, "{boxed:?}");
    }

    #[test]
    fn a_modal_reports_how_it_presents() {
        use eggshell_ui::testing::Headless;
        let placed = Headless::new(SCREEN)
            .style(&eggshell_ui::SharedStyle::new(crate::support::shadcn()))
            .layout(&State::default(), |b, _| {
                let open = State {
                    dialog: Shown::Open,
                    sheet: Shown::Open,
                    ..State::default()
                };
                layout(b, &open);
            });
        assert_eq!(
            placed.expect_overlay(Part::Dialog).modal(),
            Some(Modal::Center)
        );
        assert_eq!(placed.expect_overlay(Part::Dialog).side(), None);
        assert_eq!(
            placed.expect_overlay(Part::Sheet).modal(),
            Some(Modal::Edge(Side::Right))
        );
        assert!(
            placed.expect_overlay(Part::Sheet).stack_index()
                > placed.expect_overlay(Part::Dialog).stack_index(),
            "opening order"
        );
    }

    #[test]
    fn the_modal_has_the_dialog_role_and_the_modal_flag() {
        let d = dialog();
        let node = d
            .harness()
            .get_by_role_and_label(Role::Dialog, "Edit profile");
        assert!(node.accesskit_node().data().is_modal());
        let closed = self::harness(State::default());
        assert!(closed.harness().query_by_role(Role::Dialog).is_none());
    }

    #[test]
    fn the_wheel_scrolls_the_page_until_a_modal_covers_it() {
        let mut open = harness(State::default());
        open.wheel(pos2(100.0, 300.0), vec2(0.0, -120.0)).frames(5);
        assert!(page_offset(&open) > 20.0, "precondition: the page scrolls");

        let mut modal = dialog();
        let before = page_offset(&modal);
        let outside_box = pos2(20.0, 300.0);
        assert!(
            !modal
                .rect(Target::suffix("/Dialog/Box"))
                .contains(outside_box),
            "precondition: the pointer is on the scrim"
        );
        modal.wheel(outside_box, vec2(0.0, -120.0)).frames(5);
        assert_eq!(page_offset(&modal), before, "scroll lock");
        modal.app_mut().dialog = Shown::Closed;
        modal.frames(12);
        modal.wheel(outside_box, vec2(0.0, -120.0)).frames(5);
        assert!(
            page_offset(&modal) > before + 20.0,
            "and it scrolls again once closed"
        );
    }

    #[test]
    fn the_page_takes_no_pointer_input_under_the_scrim() {
        let mut d = dialog();
        let sheet_button = d
            .rect(Target::suffix("/Page/Intro/Actions/OpenSheet"))
            .center();
        assert!(
            !d.rect(Target::suffix("/Dialog/Box")).contains(sheet_button),
            "precondition: the page's button is outside the box"
        );
        d.click(sheet_button).frames(12);
        assert!(!d.app().sheet.is_open(), "it never got the press");
        assert!(!d.app().dialog.is_open(), "the scrim press dismissed");
    }

    #[test]
    fn focus_enters_the_dialog_and_tab_stays_in_it() {
        let mut d = dialog();
        assert!(focus(&d).starts_with("/Dialog/"), "entered: {}", focus(&d));
        let lap = lap(&mut d);
        assert!(
            lap.iter().all(|node| node.starts_with("/Dialog/")),
            "trapped: {lap:?}"
        );
        assert!(
            lap.iter().any(|node| node.ends_with("/Default")),
            "and reaches the end of it: {lap:?}"
        );
    }

    #[test]
    fn focus_returns_to_the_opener_when_the_dialog_closes() {
        let mut d = harness(State::default());
        let opener = "/Page/Intro/Actions/OpenDialog";
        for _ in 0..8 {
            if support::focused_path(&d).as_deref() == Some(opener) {
                break;
            }
            tab(&mut d);
        }
        assert_eq!(support::focused_path(&d).as_deref(), Some(opener));
        d.press(Key::Enter).frames(11);
        assert!(d.app().dialog.is_open(), "precondition: open");
        assert!(focus(&d).starts_with("/Dialog/"));
        d.press(Key::Escape).frames(11);
        assert!(!d.app().dialog.is_open(), "precondition: closed");
        assert_eq!(support::focused_path(&d).as_deref(), Some(opener));
    }

    #[test]
    fn esc_presses_cancel_and_return_presses_save() {
        let mut d = dialog();
        d.click(Target::suffix("/Dialog/Box/Body/Group(\"Name\")/Name"));
        d.type_text("Ada").frames(11);
        assert_eq!(d.app().name, "Ada", "precondition: typed");
        d.press(Key::Enter).frames(11);
        assert_eq!(d.app().saved.as_deref(), Some("Ada"));
        assert!(!d.app().dialog.is_open());

        let mut d = self::dialog();
        d.press(Key::Escape).frames(11);
        assert!(!d.app().dialog.is_open());
        assert_eq!(d.app().saved, None, "cancel saves nothing");
    }

    #[test]
    fn a_confirmation_opened_from_the_dialog_is_above_it_and_the_dialog_is_inert() {
        let mut d = confirm();
        let save = d.rect(Target::suffix("/Dialog/Box/Footer/Default"));
        let confirm_box = d.rect(Target::suffix("/Dialog/Box/Body/Confirm/Box"));
        assert!(
            !confirm_box.contains(save.center()),
            "precondition: Save is outside the confirmation"
        );
        d.click(save.center()).frames(12);
        assert!(d.app().saved.is_none(), "Save is under the scrim");
        assert!(d.app().dialog.is_open() && d.app().confirm.is_open());
        let lap = lap(&mut d);
        assert!(
            lap.iter()
                .all(|node| node.starts_with("/Dialog/Box/Body/Confirm/")),
            "focus is trapped in the confirmation: {lap:?}"
        );
    }

    #[test]
    fn the_confirmation_must_be_answered_and_esc_cancels_only_it() {
        let mut d = confirm();
        d.click(pos2(5.0, 5.0)).frames(12);
        assert!(
            d.app().confirm.is_open() && d.app().dialog.is_open(),
            "the scrim does nothing to an EscOnly confirmation, nor to the dialog below"
        );
        d.press(Key::Escape).frames(11);
        assert!(!d.app().confirm.is_open(), "its cancel button ran");
        assert!(d.app().dialog.is_open(), "the dialog is still open");
    }

    #[test]
    fn return_in_the_confirmation_is_the_focused_buttons_and_never_the_dialogs() {
        let mut d = confirm();
        assert!(
            focus(&d).ends_with("/Confirm/Box/Footer/Cancel"),
            "precondition: focus entered at Cancel, as an alert dialog does: {}",
            focus(&d)
        );
        d.press(Key::Enter).frames(11);
        assert!(!d.app().confirm.is_open(), "Cancel ran");
        assert_eq!(d.app().deleted, 0);
        assert!(d.app().saved.is_none(), "not the dialog's Save");
        assert!(d.app().dialog.is_open());
    }

    #[test]
    fn the_confirmations_default_button_answers_return_when_focus_is_not_a_button() {
        let mut d = confirm();
        d.ctx().memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        d.frames(12);
        d.press(Key::Enter).frames(11);
        assert_eq!(d.app().deleted, 1);
        assert!(d.app().saved.is_none(), "not Save");
    }

    #[test]
    fn a_menu_nested_in_the_dialog_is_above_the_trap_and_usable() {
        let mut d = dialog();
        d.click(Target::suffix(
            "/Dialog/Box/Body/Group(\"Role\")/RoleButton",
        ))
        .frames(12);
        assert!(d.app().picker, "precondition: the menu opened");
        let item = d.rect(Target::suffix(
            "/Dialog/Box/Body/Group(\"Role\")/RoleMenu/Role/RoleItem(1)",
        ));
        d.click(item.center()).frames(12);
        assert_eq!(d.app().role, 1);
        assert!(!d.app().picker);
        assert!(d.app().dialog.is_open(), "the dialog stays open");
    }

    #[test]
    fn a_press_on_the_scrim_closes_the_open_menu_and_the_dialog() {
        let mut d = dialog();
        d.click(Target::suffix(
            "/Dialog/Box/Body/Group(\"Role\")/RoleButton",
        ))
        .frames(12);
        assert!(d.app().picker, "precondition: the menu opened");
        d.click(pos2(5.0, 5.0)).frames(12);
        assert!(!d.app().picker && !d.app().dialog.is_open());
    }

    const ROLES_LAST: usize = 2;

    const ROLE_BUTTON: &str = "/Dialog/Box/Body/Group(\"Role\")/RoleButton";

    fn item(index: usize) -> String {
        format!("/Dialog/Box/Body/Group(\"Role\")/RoleMenu/Role/RoleItem({index})")
    }

    fn open_menu_with_the_pointer() -> Driver<State> {
        let mut d = dialog();
        d.click(Target::suffix(ROLE_BUTTON)).frames(12);
        assert!(d.app().picker, "precondition: open");
        d
    }

    fn highlight(d: &Driver<State>) -> egui::Color32 {
        d.shell().style().tokens.accent
    }

    #[test]
    fn a_menu_opened_with_the_pointer_takes_no_focus_and_down_goes_to_the_first_item() {
        let mut d = open_menu_with_the_pointer();
        assert!(
            !focus(&d).contains("RoleItem"),
            "nothing is focused yet: {}",
            focus(&d)
        );
        d.press(Key::ArrowDown).frames(11);
        assert_eq!(focus(&d), item(0));
        d.press(Key::ArrowDown).frames(11);
        assert_eq!(focus(&d), item(1));
        d.press(Key::ArrowUp).frames(11);
        assert_eq!(focus(&d), item(0));
    }

    #[test]
    fn up_from_a_menu_opened_with_the_pointer_goes_to_the_last_item() {
        let mut d = open_menu_with_the_pointer();
        d.press(Key::ArrowUp).frames(11);
        assert_eq!(focus(&d), item(ROLES_LAST));
    }

    #[test]
    fn a_menu_opened_from_the_keyboard_focuses_its_first_item() {
        let mut d = dialog();
        d.click(Target::suffix("/Dialog/Box/Body/Group(\"Name\")/Name"));
        for _ in 0..4 {
            if support::focused_path(&d).as_deref() == Some(ROLE_BUTTON) {
                break;
            }
            tab(&mut d);
        }
        assert_eq!(support::focused_path(&d).as_deref(), Some(ROLE_BUTTON));
        d.press(Key::Enter).frames(11);
        assert!(d.app().picker, "precondition: open");
        assert_eq!(focus(&d), item(0));
    }

    #[test]
    fn the_item_with_keyboard_focus_has_the_highlighted_look() {
        let mut d = open_menu_with_the_pointer();
        let outside = support::fill_of(&d, &item(1));
        assert_ne!(outside, Some(highlight(&d)), "precondition");
        d.press(Key::ArrowDown).frames(11);
        d.press(Key::ArrowDown).frames(11);
        assert_eq!(focus(&d), item(1), "precondition");
        assert_eq!(support::fill_of(&d, &item(1)), Some(highlight(&d)));
        assert_ne!(
            support::fill_of(&d, &item(0)),
            Some(highlight(&d)),
            "only the focused item"
        );
    }

    #[test]
    fn esc_in_the_menu_closes_only_the_menu() {
        let mut d = dialog();
        d.click(Target::suffix(
            "/Dialog/Box/Body/Group(\"Role\")/RoleButton",
        ))
        .frames(12);
        d.press(Key::Escape).frames(11);
        assert!(!d.app().picker);
        assert!(d.app().dialog.is_open());
    }
}

/// A context that lays out two frames each frame, as the examples' shell does
/// (a theme bar, then the page): the second frame declares no overlay, and
/// must not close the first one's.
mod two_frames {
    use eggshell_ui::LayoutState;
    use egui::Ui;

    use super::dialog::State;
    use super::*;

    struct Shell {
        header: LayoutState,
        state: State,
    }

    fn frame(ui: &mut Ui, shell: &mut Shell, page: &mut LayoutState) -> PlacedFrame {
        let area = ui.max_rect();
        let ctx = ui.ctx().clone();
        let top = egui::Rect::from_min_size(area.min, vec2(area.width(), 10.0));
        shell
            .header
            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                b.div_id("header", tw().w_full().h_px(10.0), |_| {});
            })
            .layout(top)
            .draw(&mut (), ui);
        page.set_base_text(Some(common::base_text(egui::Theme::Light)));
        page.build(&ctx, |b| dialog::layout(b, &shell.state))
            .layout(area)
            .draw(&mut shell.state, ui)
    }

    #[test]
    fn tab_leaves_a_text_field_and_the_dialog_stays_open() {
        let mut d = support::recorded(support::frame_driver(
            SCREEN,
            common::base_text(egui::Theme::Light),
            Shell {
                header: LayoutState::new(&eggshell_ui::SharedStyle::new(support::shadcn())),
                state: State::default(),
            },
            frame,
        ));
        d.frames(4);
        d.app_mut().state.dialog = Shown::Open;
        d.frames(8);
        let name = d.ctx().memory(|memory| memory.focused());
        assert!(name.is_some(), "precondition: focus entered the dialog");
        d.press(Key::Tab).frames(7);
        let moved = d.ctx().memory(|memory| memory.focused());
        assert_ne!(moved, name, "Tab moved the focus");
        assert!(moved.is_some(), "and into the dialog's next control");
        assert!(d.app().state.dialog.is_open());
    }
}

/// A key that dismisses an overlay or presses a button belongs to it: a draw
/// closure under it never sees the event (#146).
mod keys_are_taken {
    use eggshell_ui::overlay::{Anchor, Focus, Nav, Wrap};

    use super::*;

    #[derive(Clone, Copy, Debug, Hash)]
    enum P {
        Canvas,
        Menu,
        Item,
        Ok,
    }

    #[derive(Default)]
    struct App {
        menu: bool,
        default: bool,
        escapes: u32,
        enters: u32,
        log: Vec<&'static str>,
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(P::Canvas, tw().w_full().h_full(), |_| {})
            .draw(|app: &mut App, ui| {
                let (escapes, enters) = ui.ctx().input(|input| {
                    let count = |wanted: Key| {
                        input
                            .events
                            .iter()
                            .filter(|event| {
                                matches!(event, Event::Key { key, pressed: true, .. } if *key == wanted)
                            })
                            .count() as u32
                    };
                    (count(Key::Escape), count(Key::Enter))
                });
                app.escapes += escapes;
                app.enters += enters;
            });
        if app.default {
            b.button(P::Ok, "Ok")
                .default_action()
                .on_click(|app: &mut App| app.log.push("default"));
        }
        if app.menu {
            b.overlay(P::Menu)
                .anchor(Anchor::Point(pos2(100.0, 100.0)))
                .nav(Nav::Menu(Wrap::Around))
                .focus(Focus::Enter)
                .on_dismiss(|app: &mut App| {
                    app.menu = false;
                    app.log.push("dismissed");
                })
                .children(|s| {
                    s.button(P::Item, "Item").add();
                });
        }
    }

    fn harness(app: App) -> Driver<App> {
        let mut d = support::driver(vec2(400.0, 300.0), base_text(), app, view).build();
        d.frames(12);
        d
    }

    #[test]
    fn esc_that_dismisses_a_menu_is_not_seen_by_the_node_under_it() {
        let mut d = harness(App {
            menu: true,
            ..App::default()
        });
        d.press(Key::Escape).frames(11);
        assert_eq!(d.app().log, ["dismissed"], "precondition");
        assert_eq!(d.app().escapes, 0);
    }

    #[test]
    fn esc_with_no_overlay_reaches_the_node() {
        let mut d = harness(App::default());
        d.press(Key::Escape).frames(11);
        assert_eq!(d.app().escapes, 1);
    }

    #[test]
    fn return_that_presses_a_default_button_is_not_seen_by_the_node() {
        let mut d = harness(App {
            default: true,
            ..App::default()
        });
        d.press(Key::Enter).frames(11);
        assert_eq!(d.app().log, ["default"], "precondition");
        assert_eq!(d.app().enters, 0);
    }

    #[test]
    fn return_with_no_default_button_reaches_the_node() {
        let mut d = harness(App::default());
        d.press(Key::Enter).frames(11);
        assert_eq!(d.app().enters, 1);
    }
}

/// Buttons that dismiss their overlay (`button::Chain::dismisses`), and the
/// accessible description of an overlay and of a node.
mod closers {
    use super::*;

    #[derive(Clone, Copy, Debug, Hash)]
    enum P {
        Page,
        Plain,
        Dlg,
        Box,
        Close,
        Cancel,
        Note,
        Pop,
        PopClose,
    }

    #[derive(Default)]
    struct App {
        open: bool,
        popover: bool,
        dismiss_on: DismissOn,
        log: Vec<String>,
    }

    fn view(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(P::Page, tw().w_full().h_full().p(2.0), |s| {
            s.button(P::Plain, "Plain")
                .dismisses()
                .on_click(|app: &mut App| app.log.push("plain".into()));
        });
        if app.open {
            b.overlay(P::Dlg)
                .modal(Modal::Center)
                .dismiss_on(app.dismiss_on)
                .role(Role::Dialog)
                .named("Edit")
                .description("Make your changes.")
                .on_dismiss(|app: &mut App| {
                    app.open = false;
                    app.log.push("dismissed".into());
                })
                .children(|s| {
                    s.div_id(
                        P::Box,
                        tw().flex()
                            .flex_col()
                            .gap(2.0)
                            .w_px(300.0)
                            .p(2.0)
                            .bg(Color32::WHITE),
                        |s| {
                            s.div_id(P::Note, tw().sense_hover(), |_| {})
                                .role(Role::Note)
                                .named("Note")
                                .description("Read me after the name.");
                            s.button(P::Cancel, "Cancel")
                                .cancel_action()
                                .dismisses()
                                .add();
                            s.button(P::Close, "Close")
                                .dismisses()
                                .on_click(|app: &mut App| app.log.push("close".into()));
                            if app.popover {
                                let anchor = s.id_of(P::Close);
                                s.overlay(P::Pop)
                                    .anchor(eggshell_ui::overlay::Anchor::Node(anchor))
                                    .on_dismiss(|app: &mut App| {
                                        app.popover = false;
                                        app.log.push("popover dismissed".into());
                                    })
                                    .children(|s| {
                                        s.button(P::PopClose, "Done").dismisses().add();
                                    });
                            }
                        },
                    );
                });
        }
    }

    fn opened(app: App) -> Driver<App> {
        let mut d = support::driver(
            vec2(400.0, 400.0),
            base_text(),
            App { open: true, ..app },
            view,
        )
        .build();
        d.frames(12);
        d
    }

    #[test]
    fn a_closer_runs_its_handler_then_on_dismiss_once() {
        let mut d = opened(App::default());
        d.click(Target::suffix("/Dlg/Box/Close")).frames(12);
        assert_eq!(d.app().log, ["close", "dismissed"]);
        assert!(!d.app().open);
    }

    #[test]
    fn esc_presses_a_handlerless_dismissing_cancel_which_runs_on_dismiss_once() {
        let mut d = opened(App::default());
        d.press(Key::Escape).frames(11);
        assert_eq!(d.app().log, ["dismissed"]);
        assert!(!d.app().open);
    }

    #[test]
    fn a_closer_closes_a_modal_that_must_be_answered() {
        let mut d = opened(App {
            dismiss_on: DismissOn::Never,
            ..App::default()
        });
        d.click(pos2(5.0, 5.0)).frames(12);
        assert!(d.app().log.is_empty(), "precondition: held");
        d.click(Target::suffix("/Dlg/Box/Close")).frames(12);
        assert_eq!(d.app().log, ["close", "dismissed"]);
    }

    #[test]
    fn a_closer_in_a_nested_overlay_closes_only_that_overlay() {
        let mut d = opened(App {
            popover: true,
            ..App::default()
        });
        d.click(Target::suffix("/Dlg/Box/Pop/PopClose")).frames(12);
        assert_eq!(d.app().log, ["popover dismissed"]);
        assert!(d.app().open, "the dialog stays");
    }

    #[test]
    fn a_closer_outside_any_overlay_only_runs_its_handler() {
        let mut d = support::driver(vec2(400.0, 400.0), base_text(), App::default(), view).build();
        d.frames(12);
        d.click(Target::suffix("/Page/Plain")).frames(12);
        assert_eq!(d.app().log, ["plain"]);
    }

    #[test]
    fn an_overlay_and_a_node_carry_their_descriptions() {
        let d = opened(App::default());
        let dialog = d.harness().get_by_role_and_label(Role::Dialog, "Edit");
        assert_eq!(
            dialog.accesskit_node().data().description(),
            Some("Make your changes.")
        );
        let note = d.harness().get_by_role_and_label(Role::Note, "Note");
        assert_eq!(
            note.accesskit_node().data().description(),
            Some("Read me after the name.")
        );
    }
}
