//! The extras' skeleton, through the real draw, in the shadcn preset the
//! rig applies.

use std::time::Duration;

use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_extras::skeleton::{SkeletonExt as _, SkeletonStyle};
use eggshell_ui_shadcn::{Shadcn, ShadcnStyle, Tokens};
use egui::epaint::Shape;
use egui::{Color32, Theme, vec2};
use egui_kittest::Harness;

use crate::support::{self, ReducedMotion, Rig, rect_of};

const FRAME: f32 = 1.0;

struct App;

fn base() -> TextStyle {
    let tokens = Tokens::for_theme(Theme::Light);
    TextStyle::new(tokens.text_role(), tokens.foreground)
}

/// The skeleton in a box of its own, which the tests find by key.
fn held(b: &mut Builder<'_, '_, App>) {
    b.div_id("sk", tw().w_px(120.0).h_px(16.0), |b| {
        b.skeleton(tw().w_full().h_full());
    });
}

fn view(b: &mut Builder<'_, '_, App>, _: &App) {
    held(b);
}

fn harness(app: App, reduced: ReducedMotion) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness_stepped(vec2(300.0, 200.0), FRAME, base(), app, view);
    harness.ctx.set_theme(Theme::Light);
    harness.state_mut().reduced_motion = reduced;
    if reduced == ReducedMotion::Yes {
        harness
            .ctx
            .global_style_mut(|style| style.animation_time = 0.0);
    }
    harness.run_steps(4);
    harness
}

fn time(harness: &Harness<'_, Rig<App>>) -> f64 {
    harness.ctx.input(|input| input.time)
}

/// The alpha the skeleton's box is painted with.
fn painted_alpha(harness: &Harness<'_, Rig<App>>) -> u8 {
    let skeleton = rect_of(harness, "/\"sk\"");
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if support::same(rect.rect, skeleton) => Some(rect.fill.a()),
            _ => None,
        })
        .unwrap_or(0)
}

fn muted() -> Color32 {
    Tokens::for_theme(Theme::Light).muted
}

fn step_to_phase(harness: &mut Harness<'_, Rig<App>>, phase: f64) {
    for _ in 0..8 {
        let now = time(harness) / 2.0;
        if (now.fract() - phase).abs() < 0.01 {
            return;
        }
        harness.step();
    }
}

#[test]
fn the_preset_gives_the_skeleton_shadcns_look() {
    let light = Shadcn::light();
    let style = light.style();
    let skeleton = light.skeleton(&style);
    assert_eq!(skeleton.fill, light.tokens.muted, "bg-muted");
    assert_eq!(skeleton.radius, light.tokens.radius_md(), "rounded-md");
    assert_eq!(skeleton.period, Duration::from_secs(2), "animate-pulse");
    assert_eq!(f32::from(skeleton.low), 0.5);
    let lyra = Shadcn::light().with_style(ShadcnStyle::Lyra);
    assert_eq!(lyra.skeleton(&lyra.style()).radius, 0, "rounded-none");
}

#[test]
fn a_skeleton_pulses_from_full_fill_down_to_half() {
    let mut harness = harness(App, ReducedMotion::No);
    assert_eq!(
        SkeletonStyle::of(&harness.ctx).fill,
        muted(),
        "precondition: the preset's"
    );
    step_to_phase(&mut harness, 0.0);
    let full = painted_alpha(&harness);
    assert_eq!(full, muted().a(), "at the start of a pulse");
    step_to_phase(&mut harness, 0.5);
    let low = painted_alpha(&harness);
    assert!(
        (i32::from(low) - i32::from(full) / 2).abs() <= 2,
        "half way through: {low} of {full}"
    );
}

#[test]
fn a_skeleton_asks_for_frames_while_it_pulses() {
    let mut harness = harness(App, ReducedMotion::No);
    harness.step();
    assert!(harness.ctx.has_requested_repaint());
}

#[test]
fn under_reduced_motion_a_skeleton_stands_at_full_fill_and_asks_for_nothing() {
    let mut harness = harness(App, ReducedMotion::Yes);
    for _ in 0..4 {
        harness.step();
        assert_eq!(painted_alpha(&harness), muted().a());
    }
    assert!(!harness.ctx.has_requested_repaint());
}

#[test]
fn a_skeleton_is_not_announced() {
    let harness = harness(App, ReducedMotion::No);
    let id = support::id_of(&harness, "/\"sk\"");
    assert!(!harness.ctx.has_accesskit_node(id));
}
