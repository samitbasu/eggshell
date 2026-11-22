//! Notifications (#347) through the real path: a toast slot and a notice
//! stack, built, laid out and drawn by the rig, on a clock the driver steps.

use core::time::Duration;

use eggshell_ui::overlay::Modal;
use eggshell_ui::testing::{Driver, Target, WindowFocus};
use eggshell_ui::{Builder, Distance, TextStyle, tw};
use eggshell_ui_extras::notify::{Dismissed, Lifetime, Notice, NoticeId, NotifyExt, ScreenAnchor};
use eggshell_ui_shadcn::Tokens;
use egui::accesskit::{Live, Role};
use egui::{Color32, Key, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

const STEP: f32 = 0.05;
const DWELL: Duration = Duration::from_millis(1000);
const SIZE: egui::Vec2 = vec2(800.0, 600.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    CopyDetails,
}

struct App {
    toast: Option<Notice>,
    notices: Vec<Notice<Act>>,
    closable: bool,
    inset: f32,
    dialog: bool,
    dismissed: Vec<(NoticeId, Dismissed)>,
    acted: Vec<(NoticeId, Act)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            toast: None,
            notices: Vec::new(),
            closable: false,
            inset: 28.0,
            dialog: false,
            dismissed: Vec::new(),
            acted: Vec::new(),
        }
    }
}

fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div(tw().p(4.0), |b| {
        b.button("page", "Page").add();
    });
    if app.dialog {
        b.overlay("dialog")
            .modal(Modal::Center)
            .on_dismiss(|app: &mut App| app.dialog = false)
            .children(|s| {
                s.div(tw().p(4.0).w_px(200.0), |s| {
                    s.button("inside", "Inside").add();
                });
            });
    }
    let toast = b
        .notifications("toast", app.toast.as_slice())
        .at(ScreenAnchor::BottomCenter, Distance::new(app.inset))
        .lifetime(Lifetime::Dwell(DWELL))
        .on_dismiss(|app: &mut App, id, why| {
            app.dismissed.push((id, why));
            if app.toast.as_ref().is_some_and(|toast| toast.id == id) {
                app.toast = None;
            }
        });
    if app.closable {
        toast.closable().add();
    } else {
        toast.add();
    }
    b.notifications("notices", &app.notices)
        .at(ScreenAnchor::TopRight, Distance::new(16.0))
        .named("Alerts")
        .on_action(|app: &mut App, id, act| app.acted.push((id, act)))
        .on_dismiss(|app: &mut App, id, why| {
            app.dismissed.push((id, why));
            app.notices.retain(|notice| notice.id != id);
        })
        .add();
}

/// A driver that settles with no notices, then posts `app`'s and draws one
/// frame, so that a dwell starts at a known time.
fn harness(mut app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let (toast, notices) = (app.toast.take(), std::mem::take(&mut app.notices));
    let mut d = support::driver(
        SIZE,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .build();
    d.frames(2);
    let app = d.app_mut();
    (app.toast, app.notices) = (toast, notices);
    d.frames(1);
    d
}

const TOAST: &str = "/\"toast\"/\"stack\"";
const NOTICES: &str = "/\"notices\"/\"stack\"";

fn card(stack: &str, id: u64) -> String {
    format!("{stack}/NoticeId({id})")
}

fn shown(d: &Driver<App>, stack: &str) -> Vec<String> {
    let prefix = format!("{stack}/NoticeId(");
    support::nodes(d)
        .iter()
        .filter(|(path, _)| path.starts_with(&prefix) && !path[prefix.len()..].contains('/'))
        .map(|(path, _)| path.clone())
        .collect()
}

fn toast(id: u64) -> Option<Notice> {
    Some(Notice::new(id, format!("Toast {id}")))
}

fn notice(id: u64) -> Notice<Act> {
    Notice::new(id, format!("Notice {id}")).action("Copy details", Act::CopyDetails)
}

#[test]
fn a_toast_expires_after_its_dwell_on_the_stepped_clock() {
    let mut d = harness(App {
        toast: toast(1),
        ..App::default()
    });
    assert_eq!(shown(&d, TOAST), [card(TOAST, 1)]);
    d.advance(DWELL - Duration::from_millis(200));
    assert_eq!(shown(&d, TOAST), [card(TOAST, 1)], "not yet");
    assert!(d.app().dismissed.is_empty());
    d.advance(Duration::from_millis(400));
    assert_eq!(d.app().dismissed, [(NoticeId::new(1), Dismissed::Expired)]);
    assert!(shown(&d, TOAST).is_empty());
}

#[test]
fn a_running_dwell_books_a_repaint_for_its_expiry() {
    let delay = |d: &Driver<App>| {
        d.harness().output().viewport_output[&egui::ViewportId::ROOT].repaint_delay
    };
    let mut d = harness(App::default());
    d.frames(10);
    assert_eq!(delay(&d), Duration::MAX, "precondition: the page is idle");
    d.app_mut().toast = toast(1);
    d.frames(10);
    let left = DWELL - Duration::from_secs_f32(9.0 * STEP);
    assert!(
        delay(&d).abs_diff(left) <= Duration::from_secs_f32(STEP),
        "a repaint is booked for the expiry, {left:?} away: {:?}",
        delay(&d)
    );
}

#[test]
fn the_dwell_holds_while_the_pointer_is_over_the_stack() {
    let mut d = harness(App {
        toast: toast(1),
        ..App::default()
    });
    let over = d.rect(Target::suffix(&card(TOAST, 1))).center();
    d.hover(over);
    d.advance(DWELL * 3);
    assert_eq!(shown(&d, TOAST), [card(TOAST, 1)], "held");
    assert!(d.app().dismissed.is_empty());
    d.hover(egui::pos2(5.0, 300.0));
    d.advance(DWELL);
    assert_eq!(
        d.app().dismissed,
        [(NoticeId::new(1), Dismissed::Expired)],
        "resumes once the pointer leaves"
    );
}

#[test]
fn the_dwell_holds_while_focus_is_inside_and_while_the_window_is_in_the_background() {
    let mut d = harness(App {
        toast: toast(1),
        closable: true,
        ..App::default()
    });
    d.harness()
        .get_by_role_and_label(Role::Button, "Dismiss: Toast 1")
        .focus();
    d.advance(DWELL * 2);
    assert!(d.app().dismissed.is_empty(), "held by focus");

    d.harness()
        .get_by_role_and_label(Role::Button, "Page")
        .focus();
    d.frames(2);
    d.window_focus(WindowFocus::Blurred);
    d.advance(DWELL * 2);
    assert!(
        d.app().dismissed.is_empty(),
        "held while the window is unfocused"
    );
    d.window_focus(WindowFocus::Focused);
    d.advance(DWELL);
    assert_eq!(d.app().dismissed, [(NoticeId::new(1), Dismissed::Expired)]);
}

#[test]
fn a_replacement_keeps_one_toast_and_starts_its_own_dwell() {
    let mut d = harness(App {
        toast: toast(1),
        ..App::default()
    });
    d.advance(DWELL / 2);
    d.app_mut().toast = toast(2);
    d.frames(2);
    assert_eq!(shown(&d, TOAST), [card(TOAST, 2)]);
    d.advance(DWELL * 3 / 4);
    assert_eq!(
        shown(&d, TOAST),
        [card(TOAST, 2)],
        "past the first toast's expiry, the second is still up"
    );
    d.advance(DWELL / 2);
    assert_eq!(
        d.app().dismissed,
        [(NoticeId::new(2), Dismissed::Expired)],
        "only the toast on screen expires"
    );
}

#[test]
fn a_persistent_notice_never_expires_and_newest_sits_nearest_the_top_anchor() {
    let mut d = harness(App {
        notices: vec![notice(1), notice(2)],
        ..App::default()
    });
    d.advance(DWELL * 5);
    assert!(d.app().dismissed.is_empty());
    let (older, newer) = (
        d.rect(Target::suffix(&card(NOTICES, 1))),
        d.rect(Target::suffix(&card(NOTICES, 2))),
    );
    assert!(newer.max.y <= older.min.y, "{newer:?} above {older:?}");
}

#[test]
fn esc_dismisses_a_notice_only_while_focus_is_inside_it() {
    let mut d = harness(App {
        notices: vec![notice(1), notice(2)],
        ..App::default()
    });
    d.press(Key::Escape).frames(1);
    assert!(d.app().dismissed.is_empty(), "focus is elsewhere");

    d.harness()
        .get_by_role_and_label(Role::Button, "Dismiss: Notice 1")
        .focus();
    d.frames(2);
    d.press(Key::Escape).frames(1);
    assert_eq!(d.app().dismissed, [(NoticeId::new(1), Dismissed::Escape)]);
    assert_eq!(shown(&d, NOTICES), [card(NOTICES, 2)]);
}

#[test]
fn the_close_button_dismisses_its_notice() {
    let mut d = harness(App {
        notices: vec![notice(1), notice(2)],
        ..App::default()
    });
    d.click(Target::suffix(&format!("{}/\"close\"", card(NOTICES, 2))));
    d.frames(2);
    assert_eq!(d.app().dismissed, [(NoticeId::new(2), Dismissed::Closed)]);
    assert_eq!(shown(&d, NOTICES), [card(NOTICES, 1)]);
}

#[test]
fn a_toast_has_no_close_button_unless_asked() {
    let mut d = harness(App {
        toast: toast(1),
        ..App::default()
    });
    let close = |d: &Driver<App>| {
        d.harness()
            .query_by_role_and_label(Role::Button, "Dismiss: Toast 1")
            .is_some()
    };
    assert!(!close(&d));
    d.app_mut().closable = true;
    d.frames(2);
    assert!(close(&d));
}

#[test]
fn an_action_button_runs_on_action_and_leaves_the_notice_up() {
    let mut d = harness(App {
        notices: vec![notice(1)],
        ..App::default()
    });
    d.harness()
        .get_by_role_and_label(Role::Button, "Copy details")
        .click();
    d.frames(2);
    assert_eq!(d.app().acted, [(NoticeId::new(1), Act::CopyDetails)]);
    assert_eq!(shown(&d, NOTICES), [card(NOTICES, 1)]);
}

#[test]
fn a_stack_is_a_polite_status_region_by_its_name() {
    let d = harness(App {
        toast: toast(1),
        notices: vec![notice(1)],
        ..App::default()
    });
    for name in ["Notifications", "Alerts"] {
        let region = d.harness().get_by_role_and_label(Role::Status, name);
        assert_eq!(region.accesskit_node().data().live(), Some(Live::Polite));
    }
}

#[test]
fn the_stack_follows_its_inset_and_its_rect_is_queryable() {
    let mut d = harness(App {
        toast: toast(1),
        ..App::default()
    });
    let at = |d: &mut Driver<App>, inset: f32| {
        d.app_mut().inset = inset;
        d.frames(2);
        d.frame()
            .overlay("toast")
            .map_or(egui::Rect::NOTHING, |overlay| overlay.rect())
    };
    let flush = at(&mut d, 0.0);
    let (at_28, at_41) = (at(&mut d, 28.0), at(&mut d, 41.0));
    assert!(flush.is_positive(), "precondition: the stack is placed");
    assert_eq!(at_28.max.y, flush.max.y - 28.0);
    assert_eq!(at_41.max.y, flush.max.y - 41.0);
    assert_eq!(at_41.size(), flush.size());
    assert_eq!(at_41.center().x, flush.center().x, "centred");
    assert!(
        (flush.center().x - SIZE.x / 2.0).abs() < 10.0,
        "centred: {flush:?}"
    );
}

#[test]
fn a_notice_floats_above_an_open_dialog_and_takes_the_pointer() {
    let mut d = harness(App {
        dialog: true,
        ..App::default()
    });
    d.app_mut().notices.push(notice(1));
    d.frames(2);
    assert!(d.app().dialog, "precondition: the dialog is open");
    d.click(Target::suffix(&format!("{}/\"close\"", card(NOTICES, 1))));
    d.frames(2);
    assert_eq!(d.app().dismissed, [(NoticeId::new(1), Dismissed::Closed)]);
    assert!(d.app().dialog, "the press was the notice's");
}

#[test]
fn the_preset_gives_sonners_notification_look() {
    use eggshell_ui::Weight;
    use eggshell_ui_shadcn::{Shadcn, ShadcnStyle};
    let vega = Shadcn::light();
    let notify = vega.notify(&vega.style());
    assert_eq!(notify.fill, vega.tokens.popover, "--normal-bg");
    assert_eq!(notify.border, vega.tokens.border, "--normal-border");
    assert_eq!(notify.text, vega.tokens.popover_foreground, "--normal-text");
    assert_eq!(notify.border_width, 1.0);
    assert_eq!(notify.width, 356.0, "TOAST_WIDTH");
    assert_eq!(notify.padding, 16.0);
    assert_eq!(notify.gap, 6.0);
    assert_eq!(notify.stack_gap, 14.0, "GAP");
    assert_eq!(notify.visible, 3, "VISIBLE_TOASTS_AMOUNT");
    assert_eq!(notify.inset, Distance::new(24.0), "VIEWPORT_OFFSET");
    assert_eq!(notify.radius, 18, "rounded-2xl, 1.8 radii");
    let shadow = notify.shadow.expect("a shadow");
    assert_eq!(
        (shadow.offset, shadow.blur, shadow.spread, shadow.color.a()),
        ([0, 4], 12, 0, 26),
        "0 4px 12px rgba(0, 0, 0, 0.1)"
    );
    assert_eq!((notify.role.size, notify.role.line_height), (13.0, 19.5));
    assert_eq!(notify.role.weight, Weight::Medium, "data-title: 500");
    assert_eq!(
        notify.action.fill, vega.tokens.popover_foreground,
        "--normal-text"
    );
    assert_eq!(notify.action.text, vega.tokens.popover, "--normal-bg");
    assert_eq!(
        (
            notify.action.height,
            notify.action.padding_x,
            notify.action.radius
        ),
        (24.0, 8.0, 4)
    );
    assert_eq!(
        (notify.action.role.size, notify.action.role.weight),
        (12.0, Weight::Medium)
    );
    assert_eq!(
        (notify.close.size, notify.close.outset, notify.close.glyph),
        (20.0, 7.0, 12.0),
        "20 pt, moved out 35 %"
    );
    assert_eq!(notify.close.border, Color32::from_gray(237), "gray4");
    assert_eq!(notify.close.hover_fill, Color32::from_gray(248), "gray2");
    let dark = Shadcn::dark();
    let notify_dark = dark.notify(&dark.style());
    assert_eq!(
        notify_dark.close.border, dark.tokens.border,
        "--normal-border"
    );
    assert_eq!(
        notify_dark.close.hover_fill,
        Color32::from_gray(31),
        "hsl(0 0% 12%)"
    );
    let lyra = Shadcn::light().with_style(ShadcnStyle::Lyra);
    assert_eq!(lyra.notify(&lyra.style()).radius, 0, "rounded-none");
}

#[test]
fn a_card_is_the_styles_width_and_a_stack_shows_only_its_newest() {
    let mut d = harness(App {
        notices: (1..=5).map(notice).collect(),
        ..App::default()
    });
    d.advance(DWELL);
    let shown = shown(&d, NOTICES);
    assert_eq!(shown.len(), 3, "3 visible: {shown:?}");
    for newest in [5, 4, 3] {
        let width = d.rect(Target::suffix(&card(NOTICES, newest))).width();
        assert!((width - 356.0).abs() < 0.5, "356 wide: {width}");
    }
    assert!(
        shown
            .iter()
            .all(|path| !path.contains("NoticeId(1)") && !path.contains("NoticeId(2)")),
        "the oldest two are not shown"
    );
}

/// A notice's card draws from the style's sheet: shadcn's rounding is a
/// rule over its look (vega `rounded-2xl`), and an App rule restyles it.
#[test]
fn a_notice_card_draws_from_the_sheet() {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        SIZE,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(support::shadcn_with(|style| {
        style
            .sheet
            .as_app()
            .rule(eggshell_ui_extras::notify::NotifyPart::Toast, |t, _| {
                t.bg(egui::Color32::RED)
            });
    }))
    .build();
    d.frames(2);
    d.app_mut().notices = vec![notice(1)];
    d.frames(2);
    let card = d.rect(Target::suffix(&card(NOTICES, 1)));
    let drawn = support::box_at(&d, card);
    assert_eq!(drawn.fill, egui::Color32::RED, "the App's rule");
    assert_eq!(
        drawn.corner_radius,
        egui::CornerRadius::same(18),
        "rounded-2xl"
    );
}
