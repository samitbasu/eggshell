//! Numbers handed to the library that are reversed, not a number or
//! infinite: each is reported and falls back, and none panics, in debug
//! builds included.

use std::time::Duration;

use eggshell_ui::testing::{Headless, assert_no_reports, assert_reported};
use eggshell_ui::widget::Stock;
use eggshell_ui::{Builder, Problem, Style, TextStyle, Width, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Color32, Context, Key, Role, Theme, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, View, focused_path, hover_node, rect_of};

struct App {
    value: f64,
    ends: (f64, f64),
}

fn harness(app: App, view: View<App>) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(300.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.run_steps(4);
    harness
}

fn tab_to(harness: &mut Harness<'_, Rig<App>>, path: &str) {
    for _ in 0..8 {
        if focused_path(harness).as_deref() == Some(path) {
            return;
        }
        harness.key_press(Key::Tab);
        harness.run_steps(2);
    }
    panic!("Tab never reached {path}: {:?}", focused_path(harness));
}

fn press(harness: &mut Harness<'_, Rig<App>>, key: Key) {
    harness.key_press(key);
    harness.run_steps(4);
}

fn slider(b: &mut Builder<'_, '_, App>, app: &App) {
    b.slider("s", app.value, app.ends.0..=app.ends.1)
        .named("S")
        .on_change(|app: &mut App, value| app.value = value);
}

fn stepper(b: &mut Builder<'_, '_, App>, app: &App) {
    b.stepper("n", app.value, app.ends.0..=app.ends.1)
        .named("N")
        .on_change(|app: &mut App, value| app.value = value);
}

const THUMB: &str = "/\"s\"/Track/Thumb";

fn numeric_range(harness: &Harness<'_, Rig<App>>, role: Role) -> (Option<f64>, Option<f64>) {
    let node = harness.get_by_role(role);
    let node = node.accesskit_node();
    (node.min_numeric_value(), node.max_numeric_value())
}

#[test]
fn a_reversed_slider_range_is_swapped_and_reported() {
    let mut harness = harness(
        App {
            value: 40.0,
            ends: (100.0, 0.0),
        },
        slider,
    );
    let report = assert_reported(&harness.ctx, Problem::BadRange);
    assert!(report.message.contains("100.0..=0.0"), "{report:?}");
    assert_eq!(
        numeric_range(&harness, Role::Slider),
        (Some(0.0), Some(100.0))
    );
    tab_to(&mut harness, THUMB);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(harness.state().app.value, 41.0);
}

#[test]
fn a_stepper_range_with_a_nan_end_ends_at_its_other_end() {
    let mut harness = harness(
        App {
            value: 3.0,
            ends: (0.0, f64::NAN),
        },
        stepper,
    );
    assert_reported(&harness.ctx, Problem::BadRange);
    assert_eq!(
        numeric_range(&harness, Role::SpinButton),
        (Some(0.0), Some(0.0))
    );
    harness.get_by_role(Role::SpinButton).focus();
    harness.run_steps(2);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(harness.state().app.value, 0.0);
}

#[test]
fn an_infinite_stepper_end_is_mended_too() {
    let harness = harness(
        App {
            value: 3.0,
            ends: (f64::NEG_INFINITY, 10.0),
        },
        stepper,
    );
    assert_reported(&harness.ctx, Problem::BadRange);
    assert_eq!(
        numeric_range(&harness, Role::SpinButton),
        (Some(10.0), Some(10.0))
    );
}

#[test]
fn a_nan_slider_value_is_the_start_of_its_range() {
    let mut harness = harness(
        App {
            value: f64::NAN,
            ends: (0.0, 100.0),
        },
        slider,
    );
    assert_reported(&harness.ctx, Problem::NanValue);
    let track = rect_of(&harness, "/\"s\"/Track");
    let thumb = rect_of(&harness, THUMB);
    assert!(thumb.is_finite(), "{thumb:?}");
    assert_eq!(thumb.left(), track.left(), "at the start");
    tab_to(&mut harness, THUMB);
    press(&mut harness, Key::End);
    assert_eq!(harness.state().app.value, 100.0);
}

#[test]
fn a_valid_range_and_value_report_nothing() {
    let harness = harness(
        App {
            value: 4.0,
            ends: (0.0, 10.0),
        },
        slider,
    );
    assert_no_reports(&harness.ctx);
}

fn nan_lengths(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().p(2.0), |b| {
        b.div_id(
            "box",
            tw().w_px(40.0)
                .w_px(f32::NAN)
                .h_px(f32::INFINITY)
                .p(f32::NAN)
                .ring(f32::NAN, Color32::RED)
                .bg(Color32::RED),
            |b| b.text(tw(), "inside"),
        );
    });
}

#[test]
fn a_length_that_is_not_finite_is_ignored_reported_and_drawn_without_a_panic() {
    let harness = harness(
        App {
            value: 0.0,
            ends: (0.0, 1.0),
        },
        nan_lengths,
    );
    let report = assert_reported(&harness.ctx, Problem::NonFiniteLength);
    assert!(report.message.contains("box"), "{report:?}");
    let rect = rect_of(&harness, "/\"box\"");
    assert!(rect.is_finite(), "{rect:?}");
    assert_eq!(rect.width(), 40.0, "the earlier, finite width stands");
}

#[test]
fn headless_layout_ignores_an_infinite_length() {
    let mut headless = Headless::new(vec2(100.0, 100.0));
    let placed = headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
        b.div_id("box", tw().w_px(f32::INFINITY).h_px(10.0), |_| {});
    });
    let rect = placed.at("box").rect();
    assert_eq!(rect.size(), vec2(100.0, 10.0), "auto width fills the row");
    assert_reported(headless.ctx(), Problem::NonFiniteLength);
}

/// A widget whose size is not a number, as a buggy measure might give.
#[derive(Clone)]
struct Unmeasurable;

impl Stock for Unmeasurable {
    fn size(&self, _: &Context, _: &TextStyle, _: Option<Width>) -> Vec2 {
        vec2(f32::NAN, f32::INFINITY)
    }
}

fn unmeasurable(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col(), |b| {
        b.widget("w", tw(), Unmeasurable, |_, ui, _| {
            ui.label("drawn");
        });
        b.div_id("after", tw().h_px(10.0), |_| {});
    });
}

#[test]
fn a_node_laid_out_at_nan_is_placed_empty_and_drawn_without_a_panic() {
    let harness = harness(
        App {
            value: 0.0,
            ends: (0.0, 1.0),
        },
        unmeasurable,
    );
    let rect = rect_of(&harness, "/\"w\"");
    assert!(
        rect.is_finite() && rect.size() == Vec2::ZERO,
        "the precondition and the fallback: {rect:?}"
    );
    assert!(rect_of(&harness, "/\"after\"").is_finite());
}

fn endless_transition(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div_id(
        "hovered",
        tw().size_px(50.0)
            .bg(Color32::WHITE)
            .transition(Duration::MAX)
            .hover(|p| p.bg(Color32::BLACK)),
        |_| {},
    );
}

#[test]
fn a_transition_as_long_as_a_duration_gets_retargets_without_a_panic() {
    let mut harness = harness(
        App {
            value: 0.0,
            ends: (0.0, 1.0),
        },
        endless_transition,
    );
    hover_node(&mut harness, "/\"hovered\"");
    harness.hover_at(egui::pos2(250.0, 180.0));
    harness.run_steps(3);
    hover_node(&mut harness, "/\"hovered\"");
}

#[test]
fn an_infinite_egui_tooltip_delay_is_the_longest_duration() {
    let mut egui = egui::Style::default();
    egui.interaction.tooltip_delay = f32::INFINITY;
    egui.interaction.tooltip_grace_time = f32::NAN;
    let style = Style::from_egui(&egui);
    assert_eq!(style.tooltip.delay, Duration::MAX);
    assert_eq!(style.tooltip.grace, Duration::ZERO);
}
