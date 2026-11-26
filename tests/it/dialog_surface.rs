//! The extras dialog surface through the real path: the footer's order and
//! actions, first focus, the close button, the sheet's sides, the popover box
//! and what assistive technology reads.

use eggshell_ui::overlay::{Anchor, Modal, Shown, Side};
use eggshell_ui::owned::dialog::{self, Dialog};
use eggshell_ui::owned::text_field::Field;
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, tw};
use egui::{Key, Pos2, Role, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, click_node, focused_path, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

const SCREEN: egui::Vec2 = vec2(800.0, 600.0);

#[derive(Clone, Copy, Debug, Hash)]
enum P {
    Page,
    Trigger,
    Dlg,
    Name,
    Pop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Dialog,
    Alert,
    Sheet(Side),
    Popover,
}

#[derive(Default)]
struct App {
    kind: Kind,
    open: Shown,
    field: bool,
    name: String,
    log: Vec<&'static str>,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id(P::Page, tw().w_full().h_full().p(4.0), |s| {
        s.button(P::Trigger, "Open")
            .on_click(|app: &mut App| app.open = Shown::Open);
        if app.kind == Kind::Popover && app.open.is_open() {
            let trigger = s.id_of(P::Trigger);
            s.overlay(P::Pop)
                .anchor(Anchor::Node(trigger))
                .on_dismiss(|app: &mut App| app.open = Shown::Closed)
                .popover(|s| {
                    s.text(tw().font_medium(), "Dimensions");
                    s.text(tw(), "Set the dimensions for the layer.");
                    // shadcn's demo: `grid grid-cols-3 items-center gap-4`,
                    // a label and an input `col-span-2`.
                    for key in ["width", "height"] {
                        s.div(tw().flex().items_center().gap(4.0), |row| {
                            row.text(tw().w_px(80.0).shrink_0(), key);
                            row.text_field_with(key, Field::singleline(&app.name).named(key))
                                .classes(tw().flex_1())
                                .on_change(|_: &mut App, _| {});
                        });
                    }
                });
        }
    });
    let close = |app: &mut App| {
        app.open = Shown::Closed;
        app.log.push("dismissed");
    };
    let dialog = || Dialog::new("Edit profile").description("Make changes here.");
    match app.kind {
        Kind::Dialog => b
            .overlay(P::Dlg)
            .modal(Modal::Center)
            .shown(app.open)
            .on_dismiss(close)
            .dialog(dialog(), content(app)),
        Kind::Alert => b
            .overlay(P::Dlg)
            .modal(Modal::Center)
            .shown(app.open)
            .on_dismiss(close)
            .alert_dialog(dialog(), content(app)),
        Kind::Sheet(side) => b
            .overlay(P::Dlg)
            .modal(Modal::Edge(side))
            .shown(app.open)
            .on_dismiss(close)
            .dialog(dialog(), content(app)),
        Kind::Popover => {}
    }
}

fn content<'c, 'f>(app: &'c App) -> impl FnOnce(&mut dialog::Content<'c, 'f, App>) + 'c {
    move |d| {
        if app.field {
            d.body(move |s| {
                s.text_field_with(P::Name, Field::singleline(&app.name).named("Name"))
                    .on_change(|app: &mut App, text| app.name = text);
            });
        }
        d.footer(|f| {
            f.default("Save", |button| {
                button.on_click(|app: &mut App| {
                    app.open = Shown::Closed;
                    app.log.push("save");
                });
            })
            .cancel("Cancel", |button| button.add())
            .leading("Delete", |button| {
                button.on_click(|app: &mut App| app.log.push("delete"));
            })
        });
    }
}

fn prepare(_: &egui::Context, _: &App) {}

fn harness(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_prepared(
        SCREEN,
        common::base_text(egui::Theme::Light),
        app,
        view,
        prepare,
    );
    harness.run_steps(4);
    harness
}

/// Opens the overlay from the keyboard, so that focus enters it.
fn opened(app: App) -> Harness<'static, Rig<App>> {
    let mut harness = harness(app);
    open_from_the_keyboard(&mut harness);
    harness
}

fn open_from_the_keyboard(harness: &mut Harness<'static, Rig<App>>) {
    for _ in 0..4 {
        if focused_path(harness).as_deref() == Some("/Page/Trigger") {
            break;
        }
        press_key(harness, Key::Tab);
    }
    assert_eq!(
        focused_path(harness).as_deref(),
        Some("/Page/Trigger"),
        "precondition"
    );
    press_key(harness, Key::Enter);
    assert!(harness.state().app.open.is_open(), "precondition: open");
}

fn settle(harness: &mut Harness<'_, Rig<App>>) {
    harness.run_steps(16);
}

fn press_key(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    settle(harness);
}

fn log(harness: &Harness<'_, Rig<App>>) -> Vec<&'static str> {
    harness.state().app.log.clone()
}

const LEADING: &str = "/Dlg/Box/Footer/Leading";
const CANCEL: &str = "/Dlg/Box/Footer/Cancel";
const DEFAULT: &str = "/Dlg/Box/Footer/Default";
const CLOSE: &str = "/Dlg/Box/Close";

#[test]
fn the_footer_is_leading_cancel_default_on_screen_and_in_tab_order() {
    let mut harness = opened(App::default());
    let x = |path| rect_of(&harness, path).center().x;
    assert!(
        x(LEADING) < x(CANCEL) && x(CANCEL) < x(DEFAULT),
        "the default is last, whatever order the slots were set in"
    );
    assert!(
        rect_of(&harness, CANCEL).max.x < rect_of(&harness, DEFAULT).min.x,
        "precondition: the buttons do not overlap"
    );
    let mut lap = Vec::new();
    for _ in 0..5 {
        lap.push(focused_path(&harness).unwrap_or_default());
        press_key(&mut harness, Key::Tab);
    }
    assert_eq!(lap, [LEADING, CANCEL, DEFAULT, CLOSE, LEADING]);
}

#[test]
fn first_focus_is_the_first_field_and_never_the_close_button() {
    let harness = opened(App {
        field: true,
        ..App::default()
    });
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some("/Dlg/Box/Body/Name")
    );
}

#[test]
fn return_in_a_field_presses_the_default_button() {
    let mut harness = opened(App {
        field: true,
        ..App::default()
    });
    press_key(&mut harness, Key::Enter);
    assert_eq!(log(&harness), ["save"]);
}

#[test]
fn esc_presses_the_handlerless_cancel_which_runs_on_dismiss_once() {
    let mut harness = opened(App::default());
    press_key(&mut harness, Key::Escape);
    assert_eq!(log(&harness), ["dismissed"]);
    assert!(!harness.state().app.open.is_open());
}

#[test]
fn the_close_button_runs_on_dismiss_once_and_sits_top_right() {
    let mut harness = opened(App::default());
    let (close, boxed) = (rect_of(&harness, CLOSE), rect_of(&harness, "/Dlg/Box"));
    assert!(boxed.contains_rect(close), "{close:?} in {boxed:?}");
    // The decided look (#411, 2026-11-20): shadcn's `top-4 right-4`.
    assert_eq!(close.min.y - boxed.min.y, 16.0, "top-4: {close:?}");
    assert_eq!(boxed.max.x - close.max.x, 16.0, "right-4: {close:?}");
    click_node(&mut harness, CLOSE);
    settle(&mut harness);
    assert_eq!(log(&harness), ["dismissed"]);
}

#[test]
fn the_dialog_is_named_and_described_by_its_header() {
    let harness = opened(App::default());
    let node = harness.get_by_role_and_label(Role::Dialog, "Edit profile");
    assert_eq!(
        node.accesskit_node().data().description(),
        Some("Make changes here.")
    );
    assert!(node.accesskit_node().data().is_modal());
}

#[test]
fn an_alert_dialog_has_no_close_button_and_ignores_the_scrim() {
    let mut harness = opened(App {
        kind: Kind::Alert,
        ..App::default()
    });
    assert!(
        harness
            .query_by_role_and_label(Role::AlertDialog, "Edit profile")
            .is_some()
    );
    assert!(
        !harness.state().nodes.iter().any(|(path, _)| path == CLOSE),
        "no ×"
    );
    let scrim = Pos2::new(5.0, 5.0);
    assert!(
        !rect_of(&harness, "/Dlg/Box").contains(scrim),
        "precondition"
    );
    support::hover_over(&mut harness, scrim);
    support::press_and_release(&mut harness, scrim);
    settle(&mut harness);
    assert!(log(&harness).is_empty(), "the scrim does nothing");
    press_key(&mut harness, Key::Escape);
    assert_eq!(log(&harness), ["dismissed"]);
}

#[test]
fn an_alert_dialog_without_a_leading_button_enters_at_cancel() {
    fn alert(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(P::Page, tw().w_full().h_full().p(4.0), |s| {
            s.button(P::Trigger, "Open")
                .on_click(|app: &mut App| app.open = Shown::Open);
        });
        b.overlay(P::Dlg)
            .modal(Modal::Center)
            .shown(app.open)
            .on_dismiss(|app: &mut App| app.open = Shown::Closed)
            .alert_dialog(Dialog::new("Sure?"), |d| {
                d.footer(|f| {
                    f.cancel("Cancel", |button| button.add())
                        .default("Continue", |button| button.add())
                });
            });
    }
    let mut harness = support::harness_prepared(
        SCREEN,
        common::base_text(egui::Theme::Light),
        App::default(),
        alert,
        prepare,
    );
    harness.run_steps(4);
    open_from_the_keyboard(&mut harness);
    assert_eq!(focused_path(&harness).as_deref(), Some(CANCEL));
}

#[test]
fn an_alert_dialog_with_a_leading_button_and_a_field_enters_at_cancel() {
    let harness = opened(App {
        kind: Kind::Alert,
        field: true,
        ..App::default()
    });
    assert_eq!(focused_path(&harness).as_deref(), Some(CANCEL));
}

#[test]
fn an_alert_dialog_enters_at_a_field_marked_focus_on_open() {
    fn alert(b: &mut Builder<'_, '_, App>, app: &App) {
        b.div_id(P::Page, tw().w_full().h_full().p(4.0), |s| {
            s.button(P::Trigger, "Open")
                .on_click(|app: &mut App| app.open = Shown::Open);
        });
        b.overlay(P::Dlg)
            .modal(Modal::Center)
            .shown(app.open)
            .on_dismiss(|app: &mut App| app.open = Shown::Closed)
            .alert_dialog(Dialog::new("Type the name to delete"), |d| {
                d.body(|s| {
                    s.text_field_with(P::Name, Field::singleline(&app.name).named("Name"))
                        .focus_on_open()
                        .on_change(|app: &mut App, text| app.name = text);
                });
                d.footer(|f| {
                    f.cancel("Cancel", |button| button.add())
                        .default("Delete", |button| button.add())
                });
            });
    }
    let mut harness = support::harness_prepared(
        SCREEN,
        common::base_text(egui::Theme::Light),
        App::default(),
        alert,
        prepare,
    );
    harness.run_steps(4);
    open_from_the_keyboard(&mut harness);
    assert_eq!(
        support::focused_node(&harness).as_deref(),
        Some("/Dlg/Box/Body/Name"),
        "the marked field, not the cancel button"
    );
    eggshell_ui::testing::assert_no_reports(&harness.ctx);
}

#[test]
fn a_sheet_docks_to_each_side() {
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        let harness = opened(App {
            kind: Kind::Sheet(side),
            ..App::default()
        });
        let boxed = rect_of(&harness, "/Dlg/Box");
        let docked = match side {
            Side::Left => boxed.min.x == 0.0 && boxed.height() == SCREEN.y,
            Side::Right => boxed.max.x == SCREEN.x && boxed.height() == SCREEN.y,
            Side::Top => boxed.min.y == 0.0 && boxed.width() == SCREEN.x,
            Side::Bottom => boxed.max.y == SCREEN.y && boxed.width() == SCREEN.x,
        };
        assert!(docked, "{side:?}: {boxed:?}");
        assert!(
            boxed.width() <= 384.5 || matches!(side, Side::Top | Side::Bottom),
            "{side:?}: {boxed:?}"
        );
    }
}

#[test]
fn the_popover_box_is_the_styles() {
    let mut harness = harness(App {
        kind: Kind::Popover,
        ..App::default()
    });
    click_node(&mut harness, "/Page/Trigger");
    settle(&mut harness);
    assert!(
        harness.state().app.open.is_open(),
        "precondition: {:?}",
        log(&harness)
    );
    assert_eq!(rect_of(&harness, "/Page/Pop/Box").width(), 288.0);
}

/// What a popover holds stays inside its box: a field's natural width
/// (egui's 280 px) is more than the room its row leaves it, and it shrinks
/// to fit, shadcn's Input's `min-w-0` (#411's render round showed the rows'
/// fields spilling out).
#[test]
fn a_popovers_field_stays_inside_its_box() {
    let mut harness = harness(App {
        kind: Kind::Popover,
        ..App::default()
    });
    click_node(&mut harness, "/Page/Trigger");
    settle(&mut harness);
    let popover = rect_of(&harness, "/Page/Pop/Box");
    let inside: Vec<(String, egui::Rect)> = support::nodes(&harness)
        .into_iter()
        .filter(|(path, _)| path.starts_with("/Page/Pop/Box/"))
        .collect();
    assert!(
        inside.iter().any(|(path, _)| path.contains("width")),
        "precondition: the field is in the popover: {inside:?}"
    );
    for (path, rect) in inside {
        assert!(
            popover.shrink(16.0 - 0.5).contains_rect(rect),
            "{path} {rect:?} is inside the box's padding, {popover:?}"
        );
    }
}

/// The decided looks (#411, 2026-11-20: current shadcn everywhere), as the
/// preset's generated sheets draw them in Vega: the dialog `sm:max-w-md`
/// with a ring, not a border, `p-6`, its title `leading-none`; the alert
/// dialog `sm:max-w-lg`; a side's sheet `w-3/4 sm:max-w-sm`.
#[test]
fn the_surfaces_are_current_shadcns() {
    let dialog = opened(App::default());
    let boxed = rect_of(&dialog, "/Dlg/Box");
    assert_eq!(boxed.width(), 448.0, "sm:max-w-md");
    assert_eq!(support::box_at(&dialog, boxed).stroke.width, 0.0, "a ring");
    let header = rect_of(&dialog, "/Dlg/Box/Header");
    assert_eq!(header.min.x - boxed.min.x, 24.0, "p-6");
    let title = support::nodes(&dialog)
        .into_iter()
        .find(|(path, _)| path.starts_with("/Dlg/Box/Header/"))
        .map(|(_, rect)| rect)
        .expect("the title");
    assert_eq!(title.height(), 14.0, "text-sm leading-none");
    let alert = opened(App {
        kind: Kind::Alert,
        ..App::default()
    });
    assert_eq!(rect_of(&alert, "/Dlg/Box").width(), 512.0, "sm:max-w-lg");
    let sheet = opened(App {
        kind: Kind::Sheet(Side::Right),
        ..App::default()
    });
    assert_eq!(rect_of(&sheet, "/Dlg/Box").width(), 384.0, "sm:max-w-sm");
}
