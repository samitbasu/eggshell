//! A style whose animation time is zero is "reduced motion": every kind of
//! eggshell motion snaps. Each test shows the motion running as a control and
//! then not running.

use core::time::Duration;

use eggshell_ui::overlay::{Entrance, Modal, Shown, Side};
use eggshell_ui::{Builder, TextSpec, TextStyle, tw};
use egui::{Color32, FontFamily, vec2};
use egui_kittest::Harness;

use crate::support::{self, ReducedMotion, Rig, rect_of};

const SLOW: Duration = Duration::from_millis(1000);
const BASE: Color32 = Color32::from_rgb(20, 20, 20);
const LIT: Color32 = Color32::from_rgb(220, 20, 20);
const FIRST: Color32 = Color32::from_rgb(10, 200, 10);
const BOX: Color32 = Color32::from_rgb(10, 20, 30);
const BOX_WIDTH: f32 = 200.0;

#[derive(Default)]
struct App {
    swapped: bool,
    sheet: Shown,
}

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

/// ```html
/// <div class="w-[100px] h-[40px] bg-[#141414] hover:bg-[#dc1414] transition duration-1000 ease-linear"></div>
/// <div class="w-[100px] h-[30px] animate-layout duration-1000 ease-linear"></div> <!-- twice, swapped -->
/// ```
fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App) {
    b.div_id("page", tw().flex().flex_col(), |s| {
        s.div_id(
            "hover",
            tw().w_px(100.0)
                .h_px(40.0)
                .bg(BASE)
                .hover(|p| p.bg(LIT))
                .transition(SLOW)
                .ease_linear(),
            |_| {},
        );
        let rows = if app.swapped {
            ["second", "first"]
        } else {
            ["first", "second"]
        };
        for key in rows {
            let fill = if key == "first" { FIRST } else { BASE };
            s.div_id(
                key,
                tw().w_px(100.0)
                    .h_px(30.0)
                    .bg(fill)
                    .animate_layout(SLOW)
                    .ease_linear(),
                |_| {},
            );
        }
    });
    b.overlay("sheet")
        .modal(Modal::Edge(Side::Right))
        .shown(app.sheet)
        .transition(SLOW)
        .ease_linear()
        .entrance(Entrance::Slide(Side::Right))
        .on_dismiss(|app: &mut App| app.sheet = Shown::Closed)
        .children(|s| {
            s.div_id("box", tw().w_px(BOX_WIDTH).h_full().bg(BOX), |_| {});
        });
}

fn harness(reduced: ReducedMotion) -> Harness<'static, Rig<App>> {
    let mut harness =
        support::harness_stepped(vec2(500.0, 400.0), 0.02, base_text(), App::default(), view);
    harness.state_mut().reduced_motion = reduced;
    if reduced == ReducedMotion::Yes {
        harness
            .ctx
            .global_style_mut(|style| style.animation_time = 0.0);
    }
    harness.run_steps(4);
    harness
}

fn painted_min(harness: &Harness<'_, Rig<App>>, fill: Color32) -> egui::Pos2 {
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect.min),
            _ => None,
        })
        .unwrap_or_else(|| panic!("nothing is painted in {fill:?}"))
}

#[test]
fn a_state_transition_runs_unless_motion_is_reduced() {
    for (reduced, lit_after_one_frame) in [(ReducedMotion::No, false), (ReducedMotion::Yes, true)] {
        let mut harness = harness(reduced);
        let at = rect_of(&harness, r#"/"page"/"hover""#).center();
        assert_eq!(support::fill_of(&harness, r#"/"page"/"hover""#), Some(BASE));
        harness.hover_at(at);
        harness.step();
        harness.step();
        let fill = support::fill_of(&harness, r#"/"page"/"hover""#);
        assert_eq!(fill == Some(LIT), lit_after_one_frame, "{fill:?}");
    }
}

#[test]
fn a_position_slide_runs_unless_motion_is_reduced() {
    for (reduced, slides) in [(ReducedMotion::No, true), (ReducedMotion::Yes, false)] {
        let mut harness = harness(reduced);
        harness.state_mut().app.swapped = true;
        harness.step();
        let laid_out = rect_of(&harness, r#"/"page"/"first""#).min;
        let painted = painted_min(&harness, FIRST);
        assert_eq!(
            (painted - laid_out).length() > 5.0,
            slides,
            "laid out at {laid_out:?}, painted at {painted:?}"
        );
    }
}

#[test]
fn an_overlay_enters_unless_motion_is_reduced() {
    for (reduced, enters) in [(ReducedMotion::No, true), (ReducedMotion::Yes, false)] {
        let mut harness = harness(reduced);
        harness.state_mut().app.sheet = Shown::Open;
        harness.step();
        let laid_out = rect_of(&harness, r#"/"sheet"/"box""#).min;
        let painted = painted_min(&harness, BOX);
        assert_eq!(
            (painted.x - laid_out.x) > 5.0,
            enters,
            "laid out at {laid_out:?}, painted at {painted:?}"
        );
    }
}

#[test]
fn an_overlay_leaves_at_once_when_motion_is_reduced() {
    let mut harness = harness(ReducedMotion::Yes);
    harness.state_mut().app.sheet = Shown::Open;
    harness.run_steps(3);
    harness.state_mut().app.sheet = Shown::Closed;
    harness.run_steps(2);
    assert!(
        !harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.starts_with(r#"/"sheet""#)),
        "it is gone"
    );
}
