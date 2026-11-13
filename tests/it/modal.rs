//! Modals and default/cancel actions through the real path: events dispatched
//! by `egui_kittest::Harness`, build, layout, draw and the judgement of the
//! frame's input.

use eggshell_ui::overlay::{DismissOn, Modal, Shown, Side};
use eggshell_ui::owned::Enablement;
use eggshell_ui::prim::text_field::Field;
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::{Color32, Event, FontFamily, Key, MouseWheelUnit, Pos2, Role, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_node, id_of, rect_of};

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
        },
        Color32::BLACK,
    )
}

fn settle<A>(harness: &mut Harness<'_, Rig<A>>) {
    harness.run_steps(12);
}

fn click_at<A>(harness: &mut Harness<'_, Rig<A>>, at: Pos2) {
    support::hover_over(harness, at);
    support::press_and_release(harness, at);
    settle(harness);
}

fn press_key<A>(harness: &mut Harness<'_, Rig<A>>, key: Key) {
    harness.key_press(key);
    settle(harness);
}

fn wheel<A>(harness: &mut Harness<'_, Rig<A>>, at: Pos2) {
    support::hover_over(harness, at);
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -120.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    harness.run_steps(6);
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

    fn harness(app: App) -> Harness<'static, Rig<App>> {
        let mut harness = support::harness(vec2(400.0, 400.0), base_text(), app, view);
        harness.run_steps(4);
        harness
    }

    fn opened(app: App) -> Harness<'static, Rig<App>> {
        let mut harness = harness(App { open: true, ..app });
        settle(&mut harness);
        harness
    }

    fn log_of(harness: &Harness<'_, Rig<App>>) -> Vec<String> {
        harness.state().app.log.clone()
    }

    fn focus(harness: &mut Harness<'_, Rig<App>>, path: &str) {
        click_node(harness, path);
        settle(harness);
        assert!(
            focused_node(harness).is_some_and(|node| node.starts_with(path)),
            "precondition: {path} has focus, not {:?}",
            focused_node(harness)
        );
    }

    fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
        for _ in 0..12 {
            if support::focused_path(harness).as_deref() == Some(path) {
                return;
            }
            press_key(harness, Key::Tab);
        }
        panic!(
            "Tab never reached {path}: {:?}",
            support::focused_path(harness)
        );
    }

    #[test]
    fn a_press_on_the_scrim_dismisses_and_the_press_inside_the_box_does_not() {
        let mut harness = opened(App::default());
        let padding = rect_of(&harness, "/Dlg/Box").min + vec2(4.0, 4.0);
        click_at(&mut harness, padding);
        assert!(harness.state().app.open, "the box is not the scrim");
        let scrim = pos2(5.0, 5.0);
        assert!(
            !rect_of(&harness, "/Dlg/Box").contains(scrim),
            "precondition"
        );
        click_at(&mut harness, scrim);
        assert_eq!(log_of(&harness), ["dismissed"]);
        assert!(!harness.state().app.open);
    }

    #[test]
    fn esc_dismisses_a_modal_with_no_cancel_button() {
        let mut harness = opened(App::default());
        press_key(&mut harness, Key::Escape);
        assert_eq!(log_of(&harness), ["dismissed"]);
    }

    #[test]
    fn a_modal_that_must_be_answered_takes_esc_but_not_the_scrim() {
        let mut harness = opened(App {
            dismiss_on: DismissOn::EscOnly,
            ..App::default()
        });
        click_at(&mut harness, pos2(5.0, 5.0));
        assert!(log_of(&harness).is_empty(), "the scrim does nothing");
        press_key(&mut harness, Key::Escape);
        assert_eq!(log_of(&harness), ["dismissed"]);
    }

    #[test]
    fn never_ignores_both() {
        let mut harness = opened(App {
            dismiss_on: DismissOn::Never,
            ..App::default()
        });
        click_at(&mut harness, pos2(5.0, 5.0));
        press_key(&mut harness, Key::Escape);
        assert!(log_of(&harness).is_empty());
        assert!(harness.state().app.open);
    }

    #[test]
    fn a_press_on_the_scrim_is_swallowed_and_does_not_reach_the_page() {
        let mut harness = opened(App::default());
        let page_button = rect_of(&harness, "/Page/Open").center();
        assert!(
            !rect_of(&harness, "/Dlg/Box").contains(page_button),
            "precondition: the page's button is not under the box"
        );
        harness.state_mut().app.open = true;
        click_at(&mut harness, page_button);
        assert_eq!(log_of(&harness), ["dismissed"]);
        assert!(
            !harness.state().app.open,
            "the click that closed it did not reopen it"
        );
    }

    #[test]
    fn esc_presses_the_cancel_button_instead_of_dismissing() {
        let mut harness = opened(App {
            cancel: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert_eq!(
            log_of(&harness),
            ["cancel"],
            "the button ran, and on_dismiss did not"
        );
        assert!(!harness.state().app.open);
    }

    #[test]
    fn the_scrim_still_dismisses_when_there_is_a_cancel_button() {
        let mut harness = opened(App {
            cancel: true,
            ..App::default()
        });
        click_at(&mut harness, pos2(5.0, 5.0));
        assert_eq!(log_of(&harness), ["dismissed"]);
    }

    #[test]
    fn a_disabled_cancel_button_holds_the_modal_open_and_takes_the_key() {
        let mut harness = opened(App {
            cancel: true,
            cancel_disabled: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert!(log_of(&harness).is_empty());
        assert!(harness.state().app.open);
    }

    #[test]
    fn a_cancel_button_takes_esc_whatever_dismiss_on_says() {
        let mut harness = opened(App {
            cancel: true,
            dismiss_on: DismissOn::Never,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert_eq!(log_of(&harness), ["cancel"]);
    }

    #[test]
    fn esc_goes_to_the_page_cancel_button_when_no_overlay_is_open() {
        let mut harness = harness(App {
            page_actions: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert_eq!(log_of(&harness), ["page cancel"]);
    }

    #[test]
    fn esc_goes_to_the_modal_and_not_to_the_page_cancel_button() {
        let mut harness = opened(App {
            page_actions: true,
            cancel: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert_eq!(log_of(&harness), ["cancel"]);
    }

    #[test]
    fn return_in_a_single_line_field_presses_the_default_button() {
        let mut harness = opened(App {
            default: true,
            ..App::default()
        });
        focus(&mut harness, "/Dlg/Box/Name");
        press_key(&mut harness, Key::Enter);
        assert_eq!(log_of(&harness), ["default"]);
    }

    #[test]
    fn return_with_nothing_focused_in_the_modal_presses_its_default() {
        let mut harness = opened(App {
            default: true,
            page_actions: true,
            ..App::default()
        });
        harness.ctx.memory_mut(|memory| memory.stop_text_input());
        harness.ctx.memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        settle(&mut harness);
        press_key(&mut harness, Key::Enter);
        assert_eq!(log_of(&harness), ["default"], "the modal's, not the page's");
    }

    #[test]
    fn return_in_a_multi_line_field_is_the_fields() {
        let mut harness = opened(App {
            default: true,
            ..App::default()
        });
        focus(&mut harness, "/Dlg/Box/Notes");
        press_key(&mut harness, Key::Enter);
        assert!(log_of(&harness).is_empty());
        assert_eq!(
            harness.state().app.notes,
            "\n",
            "the field took the newline"
        );
    }

    #[test]
    fn return_on_a_focused_button_is_that_buttons() {
        let mut harness = opened(App {
            default: true,
            ..App::default()
        });
        tab_to(&mut harness, "/Dlg/Box/Other");
        press_key(&mut harness, Key::Enter);
        assert_eq!(log_of(&harness), ["other"], "not also the default");
    }

    #[test]
    fn a_disabled_default_button_does_not_fire() {
        let mut harness = opened(App {
            default: true,
            default_disabled: true,
            ..App::default()
        });
        focus(&mut harness, "/Dlg/Box/Name");
        press_key(&mut harness, Key::Enter);
        assert!(log_of(&harness).is_empty());
    }

    #[test]
    fn return_belongs_to_the_modal_and_not_to_the_page_default() {
        let mut harness = opened(App {
            default: true,
            page_actions: true,
            ..App::default()
        });
        focus(&mut harness, "/Dlg/Box/Name");
        press_key(&mut harness, Key::Enter);
        assert_eq!(log_of(&harness), ["default"]);
    }

    #[test]
    fn return_without_an_overlay_presses_the_page_default() {
        let mut harness = harness(App {
            page_actions: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Enter);
        assert_eq!(log_of(&harness), ["page default"]);
    }

    #[test]
    fn return_with_no_default_button_does_nothing() {
        let mut harness = opened(App::default());
        focus(&mut harness, "/Dlg/Box/Name");
        press_key(&mut harness, Key::Enter);
        assert!(log_of(&harness).is_empty());
    }
}

/// The dialog example: a dialog, a confirmation opened from it, a role menu
/// nested in it, and a sheet, over a page that scrolls.
mod example {
    use super::dialog::{Part, State, layout};
    use super::*;

    fn harness(state: State) -> Harness<'static, Rig<State>> {
        let mut harness = support::harness_prepared(
            SCREEN,
            common::base_text(egui::Theme::Light),
            state,
            layout,
            common::use_accent,
        );
        harness.state_mut().page = support::Page::Themed;
        harness.run_steps(4);
        harness
    }

    fn dialog() -> Harness<'static, Rig<State>> {
        let mut harness = harness(State::default());
        click_node(&mut harness, "/Page/Intro/Actions/OpenDialog");
        settle(&mut harness);
        assert!(harness.state().app.dialog.is_open(), "precondition: open");
        harness
    }

    fn confirm() -> Harness<'static, Rig<State>> {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Footer/Delete");
        settle(&mut harness);
        assert!(
            harness.state().app.confirm.is_open(),
            "precondition: confirm open"
        );
        harness
    }

    fn page_offset(harness: &Harness<'_, Rig<State>>) -> f32 {
        let scroll = id_of(harness, "/Page");
        eggshell_ui::scroll::viewport(&harness.ctx, scroll)
            .expect("the page was laid out")
            .offset()
            .y
    }

    fn focus(harness: &Harness<'_, Rig<State>>) -> String {
        focused_node(harness).unwrap_or_default()
    }

    fn tab(harness: &mut Harness<'_, Rig<State>>) {
        press_key(harness, Key::Tab);
    }

    fn lap(harness: &mut Harness<'_, Rig<State>>) -> Vec<String> {
        (0..16)
            .map(|_| {
                tab(harness);
                focus(harness)
            })
            .collect()
    }

    #[test]
    fn a_dialog_covers_the_screen_with_the_scrim_and_centres_its_box() {
        let harness = dialog();
        let screen = egui::Rect::from_min_size(Pos2::ZERO, SCREEN);
        assert_eq!(rect_of(&harness, "/Dialog"), screen);
        let boxed = rect_of(&harness, "/Dialog/Box");
        assert!(
            (boxed.center() - screen.center()).length() < 1.0,
            "{boxed:?} is centred"
        );
        assert!(
            boxed.width() <= 512.0 + 0.5 && boxed.width() > 400.0,
            "{boxed:?}"
        );
        let scrim = eggshell_ui::Style::of(&harness.ctx).scrim;
        assert_eq!(
            support::fill_of(&harness, "/Dialog"),
            Some(scrim),
            "the root is painted with the style's scrim"
        );
    }

    #[test]
    fn a_sheet_docks_to_its_side_and_stretches_along_it() {
        let mut harness = harness(State::default());
        click_node(&mut harness, "/Page/Intro/Actions/OpenSheet");
        settle(&mut harness);
        let boxed = rect_of(&harness, "/Sheet/Box");
        assert!(harness.state().app.sheet.is_open(), "precondition: open");
        assert_eq!(boxed.max.x, SCREEN.x);
        assert_eq!((boxed.min.y, boxed.max.y), (0.0, SCREEN.y));
        assert!(boxed.width() <= 384.0 + 0.5, "{boxed:?}");
    }

    #[test]
    fn a_modal_reports_how_it_presents() {
        use eggshell_ui::testing::Headless;
        let placed = Headless::new(SCREEN)
            .prepare(eggshell_ui_shadcn::Shadcn::apply_both)
            .layout(&State::default(), |b, _| {
                let open = State {
                    dialog: Shown::Open,
                    sheet: Shown::Open,
                    ..State::default()
                };
                layout(b, &open);
            });
        assert_eq!(placed.overlay(Part::Dialog).modal(), Some(Modal::Center));
        assert_eq!(placed.overlay(Part::Dialog).side(), None);
        assert_eq!(
            placed.overlay(Part::Sheet).modal(),
            Some(Modal::Edge(Side::Right))
        );
        assert!(
            placed.overlay(Part::Sheet).stack_index() > placed.overlay(Part::Dialog).stack_index(),
            "opening order"
        );
    }

    #[test]
    fn the_modal_has_the_dialog_role_and_the_modal_flag() {
        let harness = dialog();
        let node = harness.get_by_role_and_label(Role::Dialog, "Edit profile");
        assert!(node.accesskit_node().data().is_modal());
        let closed = self::harness(State::default());
        assert!(closed.query_by_role(Role::Dialog).is_none());
    }

    #[test]
    fn the_wheel_scrolls_the_page_until_a_modal_covers_it() {
        let mut open = harness(State::default());
        wheel(&mut open, pos2(100.0, 300.0));
        assert!(page_offset(&open) > 20.0, "precondition: the page scrolls");

        let mut modal = dialog();
        let before = page_offset(&modal);
        let outside_box = pos2(20.0, 300.0);
        assert!(
            !rect_of(&modal, "/Dialog/Box").contains(outside_box),
            "precondition: the pointer is on the scrim"
        );
        wheel(&mut modal, outside_box);
        assert_eq!(page_offset(&modal), before, "scroll lock");
        modal.state_mut().app.dialog = Shown::Closed;
        settle(&mut modal);
        wheel(&mut modal, outside_box);
        assert!(
            page_offset(&modal) > before + 20.0,
            "and it scrolls again once closed"
        );
    }

    #[test]
    fn the_page_takes_no_pointer_input_under_the_scrim() {
        let mut harness = dialog();
        let sheet_button = rect_of(&harness, "/Page/Intro/Actions/OpenSheet").center();
        assert!(
            !rect_of(&harness, "/Dialog/Box").contains(sheet_button),
            "precondition: the page's button is outside the box"
        );
        click_at(&mut harness, sheet_button);
        assert!(
            !harness.state().app.sheet.is_open(),
            "it never got the press"
        );
        assert!(
            !harness.state().app.dialog.is_open(),
            "the scrim press dismissed"
        );
    }

    #[test]
    fn focus_enters_the_dialog_and_tab_stays_in_it() {
        let mut harness = dialog();
        assert!(
            focus(&harness).starts_with("/Dialog/"),
            "entered: {}",
            focus(&harness)
        );
        let lap = lap(&mut harness);
        assert!(
            lap.iter().all(|node| node.starts_with("/Dialog/")),
            "trapped: {lap:?}"
        );
        assert!(
            lap.iter().any(|node| node.ends_with("/Save")),
            "and reaches the end of it: {lap:?}"
        );
    }

    #[test]
    fn focus_returns_to_the_opener_when_the_dialog_closes() {
        let mut harness = harness(State::default());
        let opener = "/Page/Intro/Actions/OpenDialog";
        for _ in 0..8 {
            if support::focused_path(&harness).as_deref() == Some(opener) {
                break;
            }
            tab(&mut harness);
        }
        assert_eq!(support::focused_path(&harness).as_deref(), Some(opener));
        press_key(&mut harness, Key::Enter);
        assert!(harness.state().app.dialog.is_open(), "precondition: open");
        assert!(focus(&harness).starts_with("/Dialog/"));
        press_key(&mut harness, Key::Escape);
        assert!(
            !harness.state().app.dialog.is_open(),
            "precondition: closed"
        );
        assert_eq!(support::focused_path(&harness).as_deref(), Some(opener));
    }

    #[test]
    fn esc_presses_cancel_and_return_presses_save() {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Group(\"Name\")/Name");
        harness.event(Event::Text("Ada".into()));
        settle(&mut harness);
        assert_eq!(harness.state().app.name, "Ada", "precondition: typed");
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.saved.as_deref(), Some("Ada"));
        assert!(!harness.state().app.dialog.is_open());

        let mut harness = self::dialog();
        press_key(&mut harness, Key::Escape);
        assert!(!harness.state().app.dialog.is_open());
        assert_eq!(harness.state().app.saved, None, "cancel saves nothing");
    }

    #[test]
    fn a_confirmation_opened_from_the_dialog_is_above_it_and_the_dialog_is_inert() {
        let mut harness = confirm();
        let save = rect_of(&harness, "/Dialog/Box/Footer/Save");
        let confirm_box = rect_of(&harness, "/Dialog/Box/Confirm/Box");
        assert!(
            !confirm_box.contains(save.center()),
            "precondition: Save is outside the confirmation"
        );
        click_at(&mut harness, save.center());
        assert!(
            harness.state().app.saved.is_none(),
            "Save is under the scrim"
        );
        assert!(harness.state().app.dialog.is_open() && harness.state().app.confirm.is_open());
        let lap = lap(&mut harness);
        assert!(
            lap.iter()
                .all(|node| node.starts_with("/Dialog/Box/Confirm/")),
            "focus is trapped in the confirmation: {lap:?}"
        );
    }

    #[test]
    fn the_confirmation_must_be_answered_and_esc_cancels_only_it() {
        let mut harness = confirm();
        click_at(&mut harness, pos2(5.0, 5.0));
        assert!(
            harness.state().app.confirm.is_open() && harness.state().app.dialog.is_open(),
            "the scrim does nothing to an EscOnly confirmation, nor to the dialog below"
        );
        press_key(&mut harness, Key::Escape);
        assert!(
            !harness.state().app.confirm.is_open(),
            "its cancel button ran"
        );
        assert!(
            harness.state().app.dialog.is_open(),
            "the dialog is still open"
        );
    }

    #[test]
    fn return_in_the_confirmation_is_the_focused_buttons_and_never_the_dialogs() {
        let mut harness = confirm();
        assert!(
            focus(&harness).ends_with("/Confirm/Box/Cancel"),
            "precondition: focus entered at Cancel, as an alert dialog does: {}",
            focus(&harness)
        );
        press_key(&mut harness, Key::Enter);
        assert!(!harness.state().app.confirm.is_open(), "Cancel ran");
        assert_eq!(harness.state().app.deleted, 0);
        assert!(harness.state().app.saved.is_none(), "not the dialog's Save");
        assert!(harness.state().app.dialog.is_open());
    }

    #[test]
    fn the_confirmations_default_button_answers_return_when_focus_is_not_a_button() {
        let mut harness = confirm();
        harness.ctx.memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        settle(&mut harness);
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.deleted, 1);
        assert!(harness.state().app.saved.is_none(), "not Save");
    }

    #[test]
    fn a_menu_nested_in_the_dialog_is_above_the_trap_and_usable() {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Group(\"Role\")/RoleButton");
        settle(&mut harness);
        assert!(harness.state().app.picker, "precondition: the menu opened");
        let item = rect_of(
            &harness,
            "/Dialog/Box/Group(\"Role\")/RoleMenu/Role/RoleItem(1)",
        );
        click_at(&mut harness, item.center());
        assert_eq!(harness.state().app.role, 1);
        assert!(!harness.state().app.picker);
        assert!(
            harness.state().app.dialog.is_open(),
            "the dialog stays open"
        );
    }

    #[test]
    fn a_press_on_the_scrim_closes_the_open_menu_and_the_dialog() {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Group(\"Role\")/RoleButton");
        settle(&mut harness);
        assert!(harness.state().app.picker, "precondition: the menu opened");
        click_at(&mut harness, pos2(5.0, 5.0));
        assert!(!harness.state().app.picker && !harness.state().app.dialog.is_open());
    }

    const ROLES_LAST: usize = 2;

    const ROLE_BUTTON: &str = "/Dialog/Box/Group(\"Role\")/RoleButton";

    fn item(index: usize) -> String {
        format!("/Dialog/Box/Group(\"Role\")/RoleMenu/Role/RoleItem({index})")
    }

    fn open_menu_with_the_pointer() -> Harness<'static, Rig<State>> {
        let mut harness = dialog();
        click_node(&mut harness, ROLE_BUTTON);
        settle(&mut harness);
        assert!(harness.state().app.picker, "precondition: open");
        harness
    }

    fn highlight(harness: &Harness<'_, Rig<State>>) -> egui::Color32 {
        eggshell_ui::Style::of(&harness.ctx)
            .menu
            .item
            .paint
            .states
            .hover
            .fill
    }

    #[test]
    fn a_menu_opened_with_the_pointer_takes_no_focus_and_down_goes_to_the_first_item() {
        let mut harness = open_menu_with_the_pointer();
        assert!(
            !focus(&harness).contains("RoleItem"),
            "nothing is focused yet: {}",
            focus(&harness)
        );
        press_key(&mut harness, Key::ArrowDown);
        assert_eq!(focus(&harness), item(0));
        press_key(&mut harness, Key::ArrowDown);
        assert_eq!(focus(&harness), item(1));
        press_key(&mut harness, Key::ArrowUp);
        assert_eq!(focus(&harness), item(0));
    }

    #[test]
    fn up_from_a_menu_opened_with_the_pointer_goes_to_the_last_item() {
        let mut harness = open_menu_with_the_pointer();
        press_key(&mut harness, Key::ArrowUp);
        assert_eq!(focus(&harness), item(ROLES_LAST));
    }

    #[test]
    fn a_menu_opened_from_the_keyboard_focuses_its_first_item() {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Group(\"Name\")/Name");
        for _ in 0..4 {
            if support::focused_path(&harness).as_deref() == Some(ROLE_BUTTON) {
                break;
            }
            tab(&mut harness);
        }
        assert_eq!(
            support::focused_path(&harness).as_deref(),
            Some(ROLE_BUTTON)
        );
        press_key(&mut harness, Key::Enter);
        assert!(harness.state().app.picker, "precondition: open");
        assert_eq!(focus(&harness), item(0));
    }

    #[test]
    fn the_item_with_keyboard_focus_has_the_highlighted_look() {
        let mut harness = open_menu_with_the_pointer();
        let outside = support::fill_of(&harness, &item(1));
        assert_ne!(outside, Some(highlight(&harness)), "precondition");
        press_key(&mut harness, Key::ArrowDown);
        press_key(&mut harness, Key::ArrowDown);
        assert_eq!(focus(&harness), item(1), "precondition");
        assert_eq!(
            support::fill_of(&harness, &item(1)),
            Some(highlight(&harness))
        );
        assert_ne!(
            support::fill_of(&harness, &item(0)),
            Some(highlight(&harness)),
            "only the focused item"
        );
    }

    #[test]
    fn esc_in_the_menu_closes_only_the_menu() {
        let mut harness = dialog();
        click_node(&mut harness, "/Dialog/Box/Group(\"Role\")/RoleButton");
        settle(&mut harness);
        press_key(&mut harness, Key::Escape);
        assert!(!harness.state().app.picker);
        assert!(harness.state().app.dialog.is_open());
    }
}

mod style {
    use super::*;

    #[test]
    fn the_neutral_scrim_is_egui_modals_and_the_presets_is_black_at_half() {
        assert_eq!(
            eggshell_ui::Style::default().scrim,
            Color32::from_black_alpha(100)
        );
        for preset in [
            eggshell_ui_shadcn::Shadcn::light(),
            eggshell_ui_shadcn::Shadcn::dark(),
        ] {
            assert_eq!(preset.style().scrim, Color32::from_black_alpha(204));
        }
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

    fn frame(ui: &mut Ui, shell: &mut Shell, page: &mut LayoutState) {
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
            .draw(&mut shell.state, ui);
    }

    #[test]
    fn tab_leaves_a_text_field_and_the_dialog_stays_open() {
        let mut harness = support::frame_harness(
            SCREEN,
            common::base_text(egui::Theme::Light),
            Shell {
                header: LayoutState::new(),
                state: State::default(),
            },
            frame,
        );
        harness.run_steps(4);
        harness.state_mut().app.state.dialog = Shown::Open;
        harness.run_steps(8);
        let name = harness.ctx.memory(|memory| memory.focused());
        assert!(name.is_some(), "precondition: focus entered the dialog");
        harness.key_press(Key::Tab);
        harness.run_steps(8);
        let moved = harness.ctx.memory(|memory| memory.focused());
        assert_ne!(moved, name, "Tab moved the focus");
        assert!(moved.is_some(), "and into the dialog's next control");
        assert!(harness.state().app.state.dialog.is_open());
    }
}

/// A key that dismisses an overlay or presses a button belongs to it: a draw
/// closure under it never sees the event (#146).
mod keys_are_taken {
    use eggshell_ui::overlay::{Anchor, Focus, Nav};

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
                .nav(Nav::Menu)
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

    fn harness(app: App) -> Harness<'static, Rig<App>> {
        let mut harness = support::harness(vec2(400.0, 300.0), base_text(), app, view);
        settle(&mut harness);
        harness
    }

    #[test]
    fn esc_that_dismisses_a_menu_is_not_seen_by_the_node_under_it() {
        let mut harness = harness(App {
            menu: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Escape);
        assert_eq!(harness.state().app.log, ["dismissed"], "precondition");
        assert_eq!(harness.state().app.escapes, 0);
    }

    #[test]
    fn esc_with_no_overlay_reaches_the_node() {
        let mut harness = harness(App::default());
        press_key(&mut harness, Key::Escape);
        assert_eq!(harness.state().app.escapes, 1);
    }

    #[test]
    fn return_that_presses_a_default_button_is_not_seen_by_the_node() {
        let mut harness = harness(App {
            default: true,
            ..App::default()
        });
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.log, ["default"], "precondition");
        assert_eq!(harness.state().app.enters, 0);
    }

    #[test]
    fn return_with_no_default_button_reaches_the_node() {
        let mut harness = harness(App::default());
        press_key(&mut harness, Key::Enter);
        assert_eq!(harness.state().app.enters, 1);
    }
}
