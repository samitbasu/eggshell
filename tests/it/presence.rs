//! Overlay presence through the real path: events dispatched by
//! `egui_kittest::Harness`, build, layout, draw and the judgement of the
//! frame's input, with a fixed time step so that a transition lasts a known
//! number of frames.

use core::time::Duration;
use std::cell::Cell;

use eggshell_ui::overlay::{Entrance, Modal, Shown, Side};
use eggshell_ui::testing::{Motion, Timeline, record};
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, Key, Pos2, pos2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, focused_node, id_of, rect_of};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dialog.rs"]
mod dialog;

const SCREEN: egui::Vec2 = vec2(800.0, 600.0);
/// Frames are this far apart.
const STEP: f32 = 0.02;
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

fn harness(view: support::View<App>) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_stepped(SCREEN, STEP, base_text(), App::default(), view);
    harness.run_steps(2);
    harness
}

fn open(harness: &mut Harness<'_, Rig<App>>) {
    harness.state_mut().app.shown = Shown::Open;
    harness.step();
}

fn close(harness: &mut Harness<'_, Rig<App>>) {
    harness.state_mut().app.shown = Shown::Closed;
    harness.step();
}

fn painted_box(harness: &Harness<'_, Rig<App>>) -> Option<egui::epaint::RectShape> {
    support::shapes(harness)
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
fn painted_offset(harness: &Harness<'_, Rig<App>>) -> f32 {
    let painted = painted_box(harness).expect("the box is painted");
    painted.rect.min.x - rect_of(harness, "/Sheet/Box").min.x
}

fn built(harness: &Harness<'_, Rig<App>>) -> u32 {
    harness.state().app.built.get()
}

fn page_button_at() -> Pos2 {
    pos2(30.0, 20.0)
}

#[test]
fn children_are_not_built_while_the_overlay_is_closed() {
    let mut harness = harness(slide_view);
    harness.run_steps(5);
    assert_eq!(built(&harness), 0);
    assert!(
        harness
            .state()
            .nodes
            .iter()
            .all(|(path, _)| !path.starts_with("/Sheet")),
        "{:?}",
        harness.state().nodes
    );
}

#[test]
fn the_box_slides_in_by_its_own_size_and_arrives() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    let first = painted_offset(&harness);
    assert!(
        (first - BOX_WIDTH).abs() < 1.0,
        "starts a box width off: {first}"
    );
    let mut seen = vec![first];
    for _ in 0..12 {
        harness.step();
        seen.push(painted_offset(&harness));
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
    let mut harness = harness(slide_view);
    open(&mut harness);
    let first = rect_of(&harness, "/Sheet/Box");
    harness.run_steps(12);
    assert_eq!(rect_of(&harness, "/Sheet/Box"), first);
    assert_eq!(first.max.x, SCREEN.x);
}

#[test]
fn the_phase_runs_entering_open_exiting_closed() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    assert!(painted_offset(&harness) > 0.0, "entering");
    harness.run_steps(12);
    assert_eq!(painted_offset(&harness), 0.0, "open");
    let open_builds = built(&harness);
    close(&mut harness);
    harness.step();
    let leaving = painted_offset(&harness);
    assert!(leaving > 0.0 && leaving < BOX_WIDTH, "exiting: {leaving}");
    assert!(built(&harness) > open_builds, "still built while it exits");
    harness.run_steps(12);
    let after = built(&harness);
    harness.run_steps(3);
    assert_eq!(built(&harness), after, "not built once closed");
    assert!(painted_box(&harness).is_none(), "closed");
    assert!(
        harness
            .state()
            .nodes
            .iter()
            .all(|(path, _)| !path.starts_with("/Sheet")),
        "gone from the frame"
    );
}

#[test]
fn on_exited_fires_once_when_the_exit_has_run() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    harness.run_steps(12);
    close(&mut harness);
    assert_eq!(harness.state().app.exited, 0, "not yet: it is leaving");
    harness.run_steps(4);
    assert_eq!(harness.state().app.exited, 0, "still leaving");
    harness.run_steps(12);
    assert_eq!(harness.state().app.exited, 1);
    harness.run_steps(12);
    assert_eq!(harness.state().app.exited, 1, "once");
}

#[test]
fn reopening_mid_exit_reverses_from_where_the_box_is() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    harness.run_steps(12);
    close(&mut harness);
    harness.run_steps(4);
    let before = painted_offset(&harness);
    assert!(before > 0.0 && before < BOX_WIDTH, "precondition: {before}");
    open(&mut harness);
    let after = painted_offset(&harness);
    assert!(
        (after - before).abs() < BOX_WIDTH / 4.0,
        "continues from {before}, not a jump to {after}"
    );
    harness.run_steps(12);
    assert_eq!(painted_offset(&harness), 0.0, "open again");
    assert_eq!(harness.state().app.exited, 0, "it never left");
}

#[test]
fn a_leaving_overlay_takes_no_input_and_still_blocks_the_page() {
    let mut harness = harness(slow_view);
    open(&mut harness);
    harness.run_steps(60);
    let inner = rect_of(&harness, "/Sheet/Box/Inner").center();
    close(&mut harness);
    harness.run_steps(2);
    assert!(painted_offset(&harness) > 0.0, "precondition: leaving");
    let exited = harness.state().app.exited;
    assert_eq!(exited, 0, "precondition: not gone");

    support::press_and_release(&mut harness, page_button_at());
    assert_eq!(harness.state().app.page_clicks, 0, "the page is blocked");
    assert_eq!(harness.state().app.dismissed, 0);

    support::press_and_release(&mut harness, inner);
    assert_eq!(harness.state().app.inner_clicks, 0, "the box is inert");
    assert_eq!(harness.state().app.dismissed, 0, "not judged again");
    assert_eq!(harness.state().app.shown, Shown::Closed);
}

#[test]
fn the_page_takes_the_pointer_again_once_the_overlay_is_gone() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    harness.run_steps(12);
    close(&mut harness);
    harness.run_steps(14);
    support::press_and_release(&mut harness, page_button_at());
    assert_eq!(harness.state().app.page_clicks, 1);
}

#[test]
fn esc_while_entering_dismisses_and_the_box_turns_round() {
    let mut harness = harness(slide_view);
    open(&mut harness);
    harness.run_steps(3);
    let entering = painted_offset(&harness);
    assert!(
        entering > 0.0 && entering < BOX_WIDTH,
        "precondition: {entering}"
    );
    harness.key_press(Key::Escape);
    harness.step();
    assert_eq!(harness.state().app.dismissed, 1);
    assert_eq!(harness.state().app.shown, Shown::Closed);
    harness.run_steps(2);
    let leaving = painted_offset(&harness);
    harness.run_steps(2);
    assert!(
        painted_offset(&harness) > leaving,
        "moving out: {leaving} then {}",
        painted_offset(&harness)
    );
    harness.run_steps(14);
    assert!(painted_box(&harness).is_none());
    assert_eq!(harness.state().app.exited, 1);
}

#[test]
fn esc_does_not_dismiss_a_leaving_overlay_again() {
    let mut harness = harness(slow_view);
    open(&mut harness);
    harness.run_steps(60);
    close(&mut harness);
    harness.key_press(Key::Escape);
    harness.step();
    assert_eq!(harness.state().app.dismissed, 0);
}

#[test]
fn focus_returns_to_the_opener_after_an_animated_close() {
    let mut harness = harness(slide_view);
    let opener = id_of(&harness, "/Page/Opener");
    harness
        .ctx
        .memory_mut(|memory| memory.request_focus(opener));
    harness.run_steps(2);
    assert_eq!(focused_node(&harness).as_deref(), Some("/Page/Opener"));
    open(&mut harness);
    harness.run_steps(12);
    assert_eq!(
        focused_node(&harness).as_deref(),
        Some("/Sheet/Box/Inner"),
        "focus entered the sheet"
    );
    harness.key_press(Key::Escape);
    harness.run_steps(20);
    assert!(painted_box(&harness).is_none(), "precondition: closed");
    assert_eq!(focused_node(&harness).as_deref(), Some("/Page/Opener"));
}

#[test]
fn nothing_requests_a_repaint_once_the_motion_has_settled() {
    let mut harness = harness(slide_view);
    assert!(!harness.ctx.has_requested_repaint(), "closed and idle");
    open(&mut harness);
    assert!(harness.ctx.has_requested_repaint(), "entering");
    harness.run_steps(12);
    harness.step();
    assert!(!harness.ctx.has_requested_repaint(), "open and idle");
    close(&mut harness);
    assert!(harness.ctx.has_requested_repaint(), "exiting");
    harness.run_steps(14);
    assert!(!harness.ctx.has_requested_repaint(), "closed and idle");
}

#[test]
fn a_fade_fades_the_box_and_the_scrim_with_the_progress() {
    let mut harness = harness(fade_view);
    open(&mut harness);
    harness.run_steps(4);
    let part_way = painted_box(&harness).expect("painted").fill;
    assert!(
        part_way.a() > 0 && part_way.a() < 255,
        "translucent mid-fade: {part_way:?}"
    );
    assert_eq!(painted_offset(&harness), 0.0, "a fade does not move");
    harness.run_steps(12);
    assert_eq!(painted_box(&harness).expect("painted").fill, BOX_FILL);
}

#[test]
fn a_zoom_scales_the_box_about_its_centre() {
    let mut harness = harness(zoom_view);
    open(&mut harness);
    harness.run_steps(3);
    let layout = rect_of(&harness, "/Sheet/Box");
    let part_way = painted_box(&harness).expect("painted").rect;
    assert!(
        part_way.width() > layout.width() * 0.95 && part_way.width() < layout.width() * 0.99,
        "between 95% and full: {part_way:?} of {layout:?}"
    );
    assert!(
        (part_way.center() - layout.center()).length() < 1.0,
        "about the centre"
    );
    harness.run_steps(12);
    let settled = painted_box(&harness).expect("painted").rect;
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
    let mut harness = harness(instant_view);
    eggshell_ui::Style::default().install_for(&harness.ctx, harness.ctx.theme());
    open(&mut harness);
    assert_eq!(painted_offset(&harness), 0.0);
    close(&mut harness);
    assert!(painted_box(&harness).is_none());
    assert_eq!(harness.state().app.exited, 1);
}

#[test]
fn the_continuity_checker_accepts_a_sheet_opening_and_closing() {
    let mut harness = harness(slide_view);
    record(&harness.ctx);
    open(&mut harness);
    harness.run_steps(14);
    close(&mut harness);
    harness.run_steps(16);
    let timeline = Timeline::recorded(&harness.ctx);
    let presenting = (0..timeline.len())
        .filter(|&frame| {
            timeline
                .frame(frame)
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

    fn harness() -> Harness<'static, Rig<State>> {
        let mut harness = support::harness_scaled(
            SCREEN,
            1.0,
            common::base_text(egui::Theme::Light),
            State::default(),
            dialog::layout,
            common::use_accent,
        );
        harness.state_mut().page = support::Page::Themed;
        harness.run_steps(4);
        harness
    }

    #[test]
    fn the_sheet_slides_in_over_the_shadcn_duration_and_slides_out() {
        let mut harness = harness();
        harness.state_mut().app.sheet = true;
        harness.step();
        let layout = rect_of(&harness, "/Sheet/Box");
        let painted = |harness: &Harness<'_, Rig<State>>| {
            support::shapes(harness)
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
            painted(&harness) > layout.min.x + layout.width() / 2.0,
            "starts off to the right"
        );
        harness.run_steps(4);
        let part_way = painted(&harness);
        assert!(part_way > layout.min.x, "still on its way after 0.2 s");
        harness.run_steps(14);
        assert!((painted(&harness) - layout.min.x).abs() < 1.0, "arrived");
        harness.state_mut().app.sheet = false;
        harness.run_steps(2);
        assert!(
            harness.state().nodes.iter().any(|(p, _)| p == "/Sheet/Box"),
            "still there while it leaves"
        );
        harness.run_steps(14);
        assert!(
            harness
                .state()
                .nodes
                .iter()
                .all(|(p, _)| !p.starts_with("/Sheet")),
            "gone"
        );
    }

    #[test]
    fn the_dialog_fades_and_zooms_in() {
        let mut harness = harness();
        harness.state_mut().app.dialog = true;
        harness.step();
        let layout = rect_of(&harness, "/Dialog/Box");
        let width = |harness: &Harness<'_, Rig<State>>| {
            support::shapes(harness)
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
        let first = width(&harness);
        assert!(first < layout.width() * 0.97, "zoomed out: {first}");
        harness.run_steps(14);
        assert!(width(&harness) >= layout.width() - 1.0, "settled");
    }
}
