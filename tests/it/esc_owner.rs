//! Who Esc belongs to (#383, #402): the topmost layer that takes it, but
//! never one below the layer that holds the focus. Through the real path:
//! the driver presses Esc and the frame decides, with each test asserting
//! where the focus is and which layers are open before it does.

use eggshell_ui::overlay::{Anchor, DismissOn, Dismissed, Focus, Modal, Nav, Wrap};
use eggshell_ui::owned::notify::{self, Notice, NoticeId};
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::accesskit::Role;
use egui::{Color32, Key, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, focused_path};

const TYPING: &str = "<widget>";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Field {
    #[default]
    Page,
    Dialog,
    /// The stock autosuggest, on the page.
    Autosuggest,
}

#[derive(Default)]
struct App {
    dialog: bool,
    page_cancel: bool,
    popover: bool,
    field: Field,
    city: String,
    suggesting: bool,
    notices: Vec<Notice>,
    cancelled: u32,
    dialog_whys: Vec<Dismissed>,
    popover_whys: Vec<Dismissed>,
    suggest_whys: Vec<Dismissed>,
    noticed: Vec<(NoticeId, notify::Dismissed)>,
}

/// A text field whose suggestions are a listbox overlay anchored to it, open
/// from the first edit until dismissed.
fn city_field<'f>(s: &mut Builder<'_, 'f, App>, app: &'f App) {
    s.text_field("city", app.city.clone())
        .named("City")
        .classes(tw().w_px(160.0))
        .on_change(|app: &mut App, text| {
            app.city = text;
            app.suggesting = true;
        });
    if app.suggesting {
        let field = s.id_of("city");
        s.overlay("suggest")
            .anchor(Anchor::Node(field))
            .focus(Focus::Stay)
            .nav(Nav::Listbox(Wrap::Around))
            .role(Role::ListBox)
            .named("Suggestions")
            .on_dismiss_with(|app: &mut App, why| {
                app.suggest_whys.push(why);
                app.suggesting = false;
            })
            .children(|s| {
                s.div_id("Box", tw().flex().flex_col().bg(Color32::WHITE), |s| {
                    for city in ["Paris", "Porto"] {
                        s.div_id(city, tw().h_px(24.0).sense_click(), |s| s.text(tw(), city))
                            .role(Role::ListBoxOption)
                            .named(city);
                    }
                });
            });
    }
}

/// A popover anchored to the page's Anchor button, which keeps the focus
/// where it is and stays open while the keyboard works elsewhere.
fn popover(s: &mut Builder<'_, '_, App>) {
    let anchor = s.id_of("anchor");
    s.overlay("popover")
        .anchor(Anchor::Node(anchor))
        .focus(Focus::Stay)
        .dismiss_on(DismissOn::Outside)
        .on_dismiss_with(|app: &mut App, why| {
            app.popover_whys.push(why);
            app.popover = false;
        })
        .children(|s| s.div(tw().w_px(80.0).h_px(40.0).bg(Color32::WHITE), |_| {}));
}

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div_id("Page", tw().flex().flex_col().gap(2.0).p(4.0), |s| {
        s.button("page", "Page").add();
        s.button("anchor", "Anchor").add();
        if app.page_cancel {
            s.button("cancel", "Cancel")
                .cancel_action()
                .on_click(|app: &mut App| app.cancelled += 1);
        }
        match app.field {
            Field::Page => city_field(s, app),
            Field::Autosuggest => {
                s.autosuggest("city", &app.city, ["Paris", "Porto"].map(str::to_owned))
                    .named("City")
                    .classes(tw().w_px(160.0))
                    .on_change(|app: &mut App, text| app.city = text);
            }
            Field::Dialog => {}
        }
        if app.popover {
            popover(s);
        }
    });
    if app.dialog {
        b.overlay("dialog")
            .modal(Modal::Center)
            .on_dismiss_with(|app: &mut App, why| {
                app.dialog_whys.push(why);
                app.dialog = false;
            })
            .children(|s| {
                s.div(tw().flex().flex_col().p(4.0).w_px(240.0), |s| {
                    s.button("inside", "Inside").add();
                    if app.field == Field::Dialog {
                        city_field(s, app);
                    }
                });
            });
    }
    b.notifications("notices", &app.notices)
        .named("Alerts")
        .on_dismiss(|app: &mut App, id, why| {
            app.noticed.push((id, why));
            app.notices.retain(|notice| notice.id != id);
        })
        .add();
}

fn notice(id: u64) -> Notice {
    Notice::new(id, format!("Notice {id}"))
}

/// A driver over `app`, with the notices posted only once the rest has
/// settled, so that their layer is above any dialog `app` opens.
fn driver(mut app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let notices = std::mem::take(&mut app.notices);
    let mut d = support::driver(
        vec2(800.0, 600.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .build();
    d.frames(4);
    d.app_mut().notices = notices;
    d.frames(3);
    d
}

const NOTICE_1: &str = "Dismiss: Notice 1";

fn focus(d: &mut Driver<App>, role: Role, name: &str) {
    d.harness().get_by_role_and_label(role, name).focus();
    d.frames(2);
}

fn focus_on(d: &Driver<App>, role: Role, name: &str) -> bool {
    d.harness()
        .query_by_role_and_label(role, name)
        .is_some_and(|node| node.accesskit_node().is_focused())
}

fn open(d: &Driver<App>, role: Role, name: &str) -> bool {
    d.harness().query_by_role_and_label(role, name).is_some()
}

/// Types into the city field, which opens its suggestions with the field
/// focused.
fn suggesting(d: &mut Driver<App>) {
    d.click(Target::role(Role::TextInput, "City")).frames(4);
    d.type_text("Pa").frames(4);
    assert_eq!(
        focused_path(d).as_deref(),
        Some(TYPING),
        "precondition: the field has focus"
    );
    assert!(d.app().suggesting, "precondition: the suggestions are open");
    assert!(
        open(d, Role::ListBox, "Suggestions"),
        "precondition: the suggestions are drawn"
    );
}

#[test]
fn with_focus_in_a_notice_above_a_dialog_esc_dismisses_the_notice() {
    let mut d = driver(App {
        dialog: true,
        notices: vec![notice(1), notice(2)],
        ..App::default()
    });
    focus(&mut d, Role::Button, NOTICE_1);
    assert!(d.app().dialog, "precondition: the dialog is open");
    assert!(
        focus_on(&d, Role::Button, NOTICE_1),
        "precondition: focus is in the notice"
    );
    d.press(Key::Escape).frames(3);
    assert_eq!(
        d.app().noticed,
        [(NoticeId::new(1), notify::Dismissed::Escape)]
    );
    assert!(d.app().dialog, "the dialog stays open");
    assert!(d.app().dialog_whys.is_empty());
}

#[test]
fn with_focus_in_a_notice_the_pages_cancel_button_is_not_pressed() {
    let mut d = driver(App {
        page_cancel: true,
        notices: vec![notice(1)],
        ..App::default()
    });
    focus(&mut d, Role::Button, NOTICE_1);
    assert!(
        focus_on(&d, Role::Button, NOTICE_1),
        "precondition: focus is in the notice"
    );
    d.press(Key::Escape).frames(3);
    assert_eq!(
        d.app().noticed,
        [(NoticeId::new(1), notify::Dismissed::Escape)]
    );
    assert_eq!(d.app().cancelled, 0, "the page's cancel is not pressed");
}

#[test]
fn with_focus_in_a_dialog_under_a_notice_esc_closes_the_dialog() {
    let mut d = driver(App {
        dialog: true,
        notices: vec![notice(1)],
        ..App::default()
    });
    focus(&mut d, Role::Button, "Inside");
    assert!(
        focus_on(&d, Role::Button, "Inside"),
        "precondition: focus is in the dialog"
    );
    assert!(
        open(&d, Role::Button, NOTICE_1),
        "precondition: a notice is shown"
    );
    d.press(Key::Escape).frames(3);
    assert_eq!(d.app().dialog_whys, [Dismissed::Esc]);
    assert!(d.app().noticed.is_empty(), "the notice stays");
}

#[test]
fn with_focus_in_a_field_esc_closes_its_suggestions_and_keeps_the_field() {
    let mut d = driver(App::default());
    suggesting(&mut d);
    d.press(Key::Escape).frames(4);
    assert_eq!(d.app().suggest_whys, [Dismissed::Esc]);
    assert!(
        !open(&d, Role::ListBox, "Suggestions"),
        "the suggestions close"
    );
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(TYPING),
        "the field keeps the focus"
    );
    assert_eq!(d.app().city, "Pa", "and its text");
}

#[test]
fn a_second_esc_is_the_fields() {
    let mut d = driver(App::default());
    suggesting(&mut d);
    d.press(Key::Escape).frames(4);
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(TYPING),
        "precondition: the first Esc left the field focused"
    );
    d.press(Key::Escape).frames(4);
    assert_eq!(focused_path(&d), None, "the second unfocuses the field");
    assert_eq!(d.app().suggest_whys, [Dismissed::Esc]);
}

#[test]
fn with_focus_in_a_field_in_a_dialog_esc_closes_its_suggestions_only() {
    let mut d = driver(App {
        dialog: true,
        field: Field::Dialog,
        ..App::default()
    });
    suggesting(&mut d);
    d.press(Key::Escape).frames(4);
    assert_eq!(d.app().suggest_whys, [Dismissed::Esc]);
    assert!(d.app().dialog, "the dialog stays open");
    assert_eq!(focused_path(&d).as_deref(), Some(TYPING));
    d.press(Key::Escape).frames(4);
    assert_eq!(
        d.app().dialog_whys,
        [Dismissed::Esc],
        "the next Esc is the dialog's"
    );
}

#[test]
fn with_focus_on_the_page_esc_closes_the_topmost_layer() {
    let mut d = driver(App {
        popover: true,
        page_cancel: true,
        ..App::default()
    });
    focus(&mut d, Role::Button, "Page");
    assert!(
        focus_on(&d, Role::Button, "Page"),
        "precondition: focus is on the page, outside the popover and its anchor"
    );
    assert!(d.app().popover, "precondition: the popover is open");
    d.press(Key::Escape).frames(3);
    assert_eq!(d.app().popover_whys, [Dismissed::Esc]);
    assert_eq!(d.app().cancelled, 0, "the page's cancel is not pressed");
}

#[test]
fn with_no_layer_open_esc_presses_the_pages_cancel_button() {
    let mut d = driver(App {
        page_cancel: true,
        ..App::default()
    });
    focus(&mut d, Role::Button, "Page");
    assert!(focus_on(&d, Role::Button, "Page"), "precondition");
    d.press(Key::Escape).frames(3);
    assert_eq!(d.app().cancelled, 1);
}

#[test]
fn with_focus_in_a_field_its_suggestions_take_esc_before_the_pages_cancel() {
    let mut d = driver(App {
        page_cancel: true,
        ..App::default()
    });
    suggesting(&mut d);
    d.press(Key::Escape).frames(4);
    assert_eq!(d.app().suggest_whys, [Dismissed::Esc]);
    assert_eq!(d.app().cancelled, 0);
}

#[test]
fn esc_closes_the_stock_autosuggest_and_keeps_the_field() {
    let mut d = driver(App {
        field: Field::Autosuggest,
        ..App::default()
    });
    d.click(Target::role(Role::ComboBox, "City")).frames(4);
    d.type_text("Pa").frames(4);
    assert_eq!(focused_path(&d).as_deref(), Some(TYPING), "precondition");
    assert!(
        open(&d, Role::ListBoxOption, "Paris"),
        "precondition: suggesting"
    );
    d.press(Key::Escape).frames(4);
    assert!(
        !open(&d, Role::ListBoxOption, "Paris"),
        "the suggestions close"
    );
    assert_eq!(
        focused_path(&d).as_deref(),
        Some(TYPING),
        "the field keeps the focus"
    );
    assert_eq!(d.app().city, "Pa");
    d.frames(10);
    assert!(
        !open(&d, Role::ListBoxOption, "Paris"),
        "and they stay away until the next edit"
    );
}
