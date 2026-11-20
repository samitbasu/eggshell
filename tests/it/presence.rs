//! Overlay presence through the real path: events dispatched by
//! `egui_kittest::Harness`, build, layout, draw and the judgement of the
//! frame's input, with a fixed time step so that a transition lasts a known
//! number of frames.

use core::time::Duration;
use std::cell::Cell;

use eggshell_ui::overlay::{Entrance, Modal, Shown, Side};
use eggshell_ui::testing::{Driver, Motion, Target, Timeline, record};
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, Key, Pos2, pos2, vec2};

use crate::support::{self, focused_node, id_of, press_and_release};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dialog.rs"]
mod dialog;

const SCREEN: egui::Vec2 = vec2(800.0, 600.0);
/// Frames are this far apart.
const STEP: Duration = Duration::from_millis(20);
const BOX_WIDTH: f32 = 200.0;
const BOX_FILL: Color32 = Color32::from_rgb(10, 20, 30);

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Page,
    Opener,
    Sheet,
    Box,
    Inner,
}

#[derive(Default)]
struct App {
    shown: Shown,
    built: Cell<u32>,
    exited: u32,
    dismissed: u32,
    page_clicks: u32,
    inner_clicks: u32,
}

#[derive(Clone, Copy)]
struct Enter {
    entrance: Entrance,
    millis: u64,
}

const SLIDE: Enter = Enter {
    entrance: Entrance::Slide(Side::Right),
    millis: 200,
};

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

fn page(b: &mut Builder<'_, '_, App>) {
    b.div_id(Part::Page, tw().w_full().h_full(), |s| {
        s.button(Part::Opener, "Open").on_click(|app: &mut App| {
            app.page_clicks += 1;
            app.shown = Shown::Open;
        });
    });
}

fn sheet(b: &mut Builder<'_, '_, App>, app: &App, motion: Enter) {
    b.overlay(Part::Sheet)
        .modal(Modal::Edge(Side::Right))
        .shown(app.shown)
        .transition(Duration::from_millis(motion.millis))
        .ease_linear()
        .entrance(motion.entrance)
        .on_dismiss(|app: &mut App| {
            app.dismissed += 1;
            app.shown = Shown::Closed;
        })
        .on_exited(|app: &mut App| app.exited += 1)
        .children(|s| {
            app.built.set(app.built.get() + 1);
            s.div_id(
                Part::Box,
                tw().w_px(BOX_WIDTH).h_full().bg(BOX_FILL).p(4.0),
                |s| {
                    s.button(Part::Inner, "Inner")
                        .on_click(|app: &mut App| app.inner_clicks += 1);
                },
            );
        });
}

fn slide_view(b: &mut Builder<'_, '_, App>, app: &App) {
    page(b);
    sheet(b, app, SLIDE);
}

fn fade_view(b: &mut Builder<'_, '_, App>, app: &App) {
    page(b);
    sheet(
        b,
        app,
        Enter {
            entrance: Entrance::Fade,
            millis: 200,
        },
    );
}

fn zoom_view(b: &mut Builder<'_, '_, App>, app: &App) {
    page(b);
    sheet(
        b,
        app,
        Enter {
            entrance: Entrance::Zoom,
            millis: 200,
        },
    );
}

fn slow_view(b: &mut Builder<'_, '_, App>, app: &App) {
    page(b);
    sheet(
        b,
        app,
        Enter {
            entrance: Entrance::Slide(Side::Right),
            millis: 1000,
        },
    );
}

fn harness(view: support::View<App>) -> Driver<App> {
    let mut d = support::driver(SCREEN, base_text(), App::default(), view)
        .step(STEP)
        .settle_within(STEP * 4)
        .build();
    d.frames(2);
    d
}

fn open(d: &mut Driver<App>) {
    d.app_mut().shown = Shown::Open;
    d.frames(1);
}

fn close(d: &mut Driver<App>) {
    d.app_mut().shown = Shown::Closed;
    d.frames(1);
}

fn painted_box(d: &Driver<App>) -> Option<egui::epaint::RectShape> {
    support::shapes(d)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.fill.to_srgba_unmultiplied()[..3]
                    == BOX_FILL.to_srgba_unmultiplied()[..3] =>
            {
                Some(rect.clone())
            }
            _ => None,
        })
}

/// How far right of its layout the box is painted.
fn painted_offset(d: &Driver<App>) -> f32 {
    let painted = painted_box(d).expect("the box is painted");
    painted.rect.min.x - d.rect(Target::suffix("/Sheet/Box")).min.x
}

fn built(d: &Driver<App>) -> u32 {
    d.app().built.get()
}

fn page_button_at() -> Pos2 {
    pos2(30.0, 20.0)
}

#[test]
fn children_are_not_built_while_the_overlay_is_closed() {
    let mut d = harness(slide_view);
    d.frames(5);
    assert_eq!(built(&d), 0);
    assert!(
        support::nodes(&d)
            .iter()
            .all(|(path, _)| !path.starts_with("/Sheet")),
        "{:?}",
        support::nodes(&d)
    );
}

#[test]
fn the_box_slides_in_by_its_own_size_and_arrives() {
    let mut d = harness(slide_view);
    open(&mut d);
    let first = painted_offset(&d);
    assert!(
        (first - BOX_WIDTH).abs() < 1.0,
        "starts a box width off: {first}"
    );
    let mut seen = vec![first];
    for _ in 0..12 {
        d.frames(1);
        seen.push(painted_offset(&d));
    }
    assert!(
        seen.windows(2).all(|pair| pair[1] <= pair[0]),
        "never moves back: {seen:?}"
    );
    assert!(seen[5] > 0.0 && seen[5] < BOX_WIDTH, "part-way: {seen:?}");
    assert_eq!(*seen.last().expect("steps ran"), 0.0, "{seen:?}");
}

#[test]
fn layout_is_final_on_the_first_frame() {
    let mut d = harness(slide_view);
    open(&mut d);
    let first = d.rect(Target::suffix("/Sheet/Box"));
    d.frames(12);
    assert_eq!(d.rect(Target::suffix("/Sheet/Box")), first);
    assert_eq!(first.max.x, SCREEN.x);
}

#[test]
fn the_phase_runs_entering_open_exiting_closed() {
    let mut d = harness(slide_view);
    open(&mut d);
    assert!(painted_offset(&d) > 0.0, "entering");
    d.frames(12);
    assert_eq!(painted_offset(&d), 0.0, "open");
    let open_builds = built(&d);
    close(&mut d);
    d.frames(1);
    let leaving = painted_offset(&d);
    assert!(leaving > 0.0 && leaving < BOX_WIDTH, "exiting: {leaving}");
    assert!(built(&d) > open_builds, "still built while it exits");
    d.frames(12);
    let after = built(&d);
    d.frames(3);
    assert_eq!(built(&d), after, "not built once closed");
    assert!(painted_box(&d).is_none(), "closed");
    assert!(
        support::nodes(&d)
            .iter()
            .all(|(path, _)| !path.starts_with("/Sheet")),
        "gone from the frame"
    );
}

#[test]
fn on_exited_fires_once_when_the_exit_has_run() {
    let mut d = harness(slide_view);
    open(&mut d);
    d.frames(12);
    close(&mut d);
    assert_eq!(d.app().exited, 0, "not yet: it is leaving");
    d.frames(4);
    assert_eq!(d.app().exited, 0, "still leaving");
    d.frames(12);
    assert_eq!(d.app().exited, 1);
    d.frames(12);
    assert_eq!(d.app().exited, 1, "once");
}

#[test]
fn reopening_mid_exit_reverses_from_where_the_box_is() {
    let mut d = harness(slide_view);
    open(&mut d);
    d.frames(12);
    close(&mut d);
    d.frames(4);
    let before = painted_offset(&d);
    assert!(before > 0.0 && before < BOX_WIDTH, "precondition: {before}");
    open(&mut d);
    let after = painted_offset(&d);
    assert!(
        (after - before).abs() < BOX_WIDTH / 4.0,
        "continues from {before}, not a jump to {after}"
    );
    d.frames(12);
    assert_eq!(painted_offset(&d), 0.0, "open again");
    assert_eq!(d.app().exited, 0, "it never left");
}

#[test]
fn a_leaving_overlay_takes_no_input_and_still_blocks_the_page() {
    let mut d = harness(slow_view);
    open(&mut d);
    d.frames(60);
    let inner = d.rect(Target::suffix("/Sheet/Box/Inner")).center();
    close(&mut d);
    d.frames(2);
    assert!(painted_offset(&d) > 0.0, "precondition: leaving");
    let exited = d.app().exited;
    assert_eq!(exited, 0, "precondition: not gone");

    press_and_release(&mut d, page_button_at());
    assert_eq!(d.app().page_clicks, 0, "the page is blocked");
    assert_eq!(d.app().dismissed, 0);

    press_and_release(&mut d, inner);
    assert_eq!(d.app().inner_clicks, 0, "the box is inert");
    assert_eq!(d.app().dismissed, 0, "not judged again");
    assert_eq!(d.app().shown, Shown::Closed);
}

#[test]
fn the_page_takes_the_pointer_again_once_the_overlay_is_gone() {
    let mut d = harness(slide_view);
    open(&mut d);
    d.frames(12);
    close(&mut d);
    d.frames(14);
    press_and_release(&mut d, page_button_at());
    assert_eq!(d.app().page_clicks, 1);
}

#[test]
fn esc_while_entering_dismisses_and_the_box_turns_round() {
    let mut d = harness(slide_view);
    open(&mut d);
    d.frames(3);
    let entering = painted_offset(&d);
    assert!(
        entering > 0.0 && entering < BOX_WIDTH,
        "precondition: {entering}"
    );
    d.press(Key::Escape);
    assert_eq!(d.app().dismissed, 1);
    assert_eq!(d.app().shown, Shown::Closed);
    d.frames(2);
    let leaving = painted_offset(&d);
    d.frames(2);
    assert!(
        painted_offset(&d) > leaving,
        "moving out: {leaving} then {}",
        painted_offset(&d)
    );
    d.frames(14);
    assert!(painted_box(&d).is_none());
    assert_eq!(d.app().exited, 1);
}

#[test]
fn esc_does_not_dismiss_a_leaving_overlay_again() {
    let mut d = harness(slow_view);
    open(&mut d);
    d.frames(60);
    close(&mut d);
    d.press(Key::Escape);
    assert_eq!(d.app().dismissed, 0);
}

#[test]
fn focus_returns_to_the_opener_after_an_animated_close() {
    let mut d = harness(slide_view);
    let opener = id_of(&d, "/Page/Opener");
    d.ctx().memory_mut(|memory| memory.request_focus(opener));
    d.frames(2);
    assert_eq!(focused_node(&d).as_deref(), Some("/Page/Opener"));
    open(&mut d);
    d.frames(12);
    assert_eq!(
        focused_node(&d).as_deref(),
        Some("/Sheet/Box/Inner"),
        "focus entered the sheet"
    );
    d.press(Key::Escape).frames(19);
    assert!(painted_box(&d).is_none(), "precondition: closed");
    assert_eq!(focused_node(&d).as_deref(), Some("/Page/Opener"));
}

#[test]
fn nothing_requests_a_repaint_once_the_motion_has_settled() {
    let mut d = harness(slide_view);
    assert!(!d.ctx().has_requested_repaint(), "closed and idle");
    open(&mut d);
    assert!(d.ctx().has_requested_repaint(), "entering");
    d.frames(12);
    d.frames(1);
    assert!(!d.ctx().has_requested_repaint(), "open and idle");
    close(&mut d);
    assert!(d.ctx().has_requested_repaint(), "exiting");
    d.frames(14);
    assert!(!d.ctx().has_requested_repaint(), "closed and idle");
}

#[test]
fn a_fade_leaves_the_box_opaque_and_fades_only_the_scrim() {
    let mut d = harness(fade_view);
    open(&mut d);
    d.frames(4);
    assert_eq!(
        painted_box(&d).expect("painted").fill,
        BOX_FILL,
        "the box is not faded"
    );
    assert_eq!(painted_offset(&d), 0.0, "a fade does not move the box");
    let scrim = support::shapes(&d)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.rect.width() >= SCREEN.x - 1.0 => Some(rect.fill),
            _ => None,
        })
        .expect("the scrim is painted");
    let full = eggshell_ui::Style::of(d.ctx()).scrim.a();
    assert!(
        scrim.a() > 0 && scrim.a() < full,
        "the scrim is part-way: {scrim:?} of {full}"
    );
    d.frames(12);
    assert_eq!(painted_box(&d).expect("painted").fill, BOX_FILL);
}

#[test]
fn a_zoom_scales_the_box_about_its_centre() {
    let mut d = harness(zoom_view);
    open(&mut d);
    d.frames(3);
    let layout = d.rect(Target::suffix("/Sheet/Box"));
    let part_way = painted_box(&d).expect("painted").rect;
    assert!(
        part_way.width() > layout.width() * 0.95 && part_way.width() < layout.width() * 0.99,
        "between 95% and full: {part_way:?} of {layout:?}"
    );
    assert!(
        (part_way.center() - layout.center()).length() < 1.0,
        "about the centre"
    );
    d.frames(12);
    let settled = painted_box(&d).expect("painted").rect;
    assert!((settled.width() - layout.width()).abs() < 0.5);
}

#[test]
fn an_overlay_without_motion_in_the_style_appears_and_vanishes_at_once() {
    fn instant_view(b: &mut Builder<'_, '_, App>, app: &App) {
        page(b);
        b.overlay(Part::Sheet)
            .modal(Modal::Edge(Side::Right))
            .shown(app.shown)
            .on_exited(|app: &mut App| app.exited += 1)
            .children(|s| {
                s.div_id(
                    Part::Box,
                    tw().w_px(BOX_WIDTH).h_full().bg(BOX_FILL),
                    |_| {},
                );
            });
    }
    let mut d = harness(instant_view);
    eggshell_ui::Style::default().install_for(d.ctx(), d.ctx().theme());
    open(&mut d);
    assert_eq!(painted_offset(&d), 0.0);
    close(&mut d);
    assert!(painted_box(&d).is_none());
    assert_eq!(d.app().exited, 1);
}

#[test]
fn the_continuity_checker_accepts_a_sheet_opening_and_closing() {
    let mut d = harness(slide_view);
    record(d.ctx());
    open(&mut d);
    d.frames(14);
    close(&mut d);
    d.frames(16);
    let timeline = Timeline::recorded(d.ctx());
    let presenting = (0..timeline.len())
        .filter(|&frame| {
            timeline
                .frame(frame)
                .expect("a recorded frame")
                .iter()
                .any(|node| node.motion == Motion::Presents)
        })
        .count();
    assert!(presenting >= 20, "precondition: {presenting} frames moved");
    timeline.assert_continuous();
}

/// The dialog example's sheet and dialog, with the shadcn preset's motion.
mod example {
    use super::dialog::State;
    use super::*;

    fn harness() -> Driver<State> {
        let mut d = support::prepared_driver(
            SCREEN,
            common::base_text(egui::Theme::Light),
            State::default(),
            dialog::layout,
            common::use_accent,
        )
        .build();
        support::set_page(&d, support::Page::Themed);
        d.frames(4);
        d
    }

    #[test]
    fn the_sheet_slides_in_over_the_shadcn_duration_and_slides_out() {
        let mut d = harness();
        d.app_mut().sheet = Shown::Open;
        d.frames(1);
        let layout = d.rect(Target::suffix("/Sheet/Box"));
        let painted = |d: &Driver<State>| {
            support::shapes(d)
                .into_iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect)
                        if rect.rect.height() >= layout.height() - 1.0
                            && rect.rect.width() < layout.width() + 2.0 =>
                    {
                        Some(rect.rect.min.x)
                    }
                    _ => None,
                })
                .fold(f32::MAX, f32::min)
        };
        assert!(
            painted(&d) > layout.min.x + layout.width() / 2.0,
            "starts off to the right"
        );
        d.frames(4);
        let part_way = painted(&d);
        assert!(part_way > layout.min.x, "still on its way after 0.2 s");
        d.frames(14);
        assert!((painted(&d) - layout.min.x).abs() < 1.0, "arrived");
        d.app_mut().sheet = Shown::Closed;
        d.frames(2);
        assert!(
            support::nodes(&d).iter().any(|(p, _)| p == "/Sheet/Box"),
            "still there while it leaves"
        );
        d.frames(14);
        assert!(
            support::nodes(&d)
                .iter()
                .all(|(p, _)| !p.starts_with("/Sheet")),
            "gone"
        );
    }

    #[test]
    fn the_dialog_fades_and_zooms_in() {
        let mut d = harness();
        d.app_mut().dialog = Shown::Open;
        d.frames(1);
        let layout = d.rect(Target::suffix("/Dialog/Box"));
        let width = |d: &Driver<State>| {
            support::shapes(d)
                .into_iter()
                .filter_map(|shape| match shape {
                    egui::Shape::Rect(rect)
                        if (rect.rect.center() - layout.center()).length() < 2.0
                            && rect.rect.width() > layout.width() * 0.9
                            && rect.rect.width() < layout.width() * 1.1
                            && rect.rect.height() > layout.height() * 0.9 =>
                    {
                        Some(rect.rect.width())
                    }
                    _ => None,
                })
                .fold(0.0_f32, f32::max)
        };
        let first = width(&d);
        assert!(first < layout.width() * 0.97, "zoomed out: {first}");
        d.frames(14);
        assert!(width(&d) >= layout.width() - 1.0, "settled");
    }

    #[test]
    fn the_dialog_box_stays_opaque_while_the_scrim_fades() {
        let mut d = harness();
        d.app_mut().dialog = Shown::Open;
        d.frames(2);
        let layout = d.rect(Target::suffix("/Dialog/Box"));
        let fills: Vec<_> = support::shapes(&d)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.rect.width() > layout.width() * 0.9
                        && rect.rect.width() < layout.width() * 1.1
                        && rect.rect.height() > layout.height() * 0.9 =>
                {
                    Some(rect.fill)
                }
                _ => None,
            })
            .collect();
        assert!(
            fills.iter().any(|fill| fill.a() == 255),
            "precondition: the box is painted, mid-motion: {fills:?}"
        );
        assert!(
            fills
                .iter()
                .filter(|fill| fill.r() > 0)
                .all(|fill| fill.a() == 255),
            "no translucent box fill: {fills:?}"
        );
        let scrim = support::shapes(&d)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.rect.width() >= SCREEN.x - 1.0 && rect.fill.a() < 255 =>
                {
                    Some(rect.fill)
                }
                _ => None,
            });
        let scrim = scrim.expect("the scrim is painted");
        assert!(
            scrim.a() > 0 && scrim.a() < 128,
            "the scrim is part-way: {scrim:?}"
        );
    }
}
