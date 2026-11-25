//! The extras' skeleton, through the real draw, in the shadcn preset the
//! rig applies.

use std::time::Duration;

use eggshell_ui::owned::skeleton::{SkeletonExt as _, SkeletonStyle};
use eggshell_ui::testing::{Animation, Driver, Target};
use eggshell_ui::theme::{Shadcn, ShadcnStyle, Tokens};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::epaint::Shape;
use egui::{Color32, Theme, vec2};

use crate::support;

const FRAME: Duration = Duration::from_secs(1);

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

fn harness(app: App, motion: Animation) -> Driver<App> {
    let mut d = support::driver(vec2(300.0, 200.0), base(), app, view)
        .step(FRAME)
        .theme(Theme::Light)
        .motion(motion)
        .build();
    d.frames(4);
    d
}

fn time(d: &Driver<App>) -> f64 {
    d.ctx().input(|input| input.time)
}

/// The alpha the skeleton's box is painted with.
fn painted_alpha(d: &Driver<App>) -> u8 {
    let skeleton = d.rect(Target::suffix("/\"sk\""));
    support::shapes(d)
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

fn step_to_phase(d: &mut Driver<App>, phase: f64) {
    for _ in 0..8 {
        let now = time(d) / 2.0;
        if (now.fract() - phase).abs() < 0.01 {
            return;
        }
        d.frames(1);
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
    let mut d = harness(App, Animation::Full);
    assert_eq!(
        d.shell().style().extension::<SkeletonStyle>().fill,
        muted(),
        "precondition: the preset's"
    );
    step_to_phase(&mut d, 0.0);
    let full = painted_alpha(&d);
    assert_eq!(full, muted().a(), "at the start of a pulse");
    step_to_phase(&mut d, 0.5);
    let low = painted_alpha(&d);
    assert!(
        (i32::from(low) - i32::from(full) / 2).abs() <= 2,
        "half way through: {low} of {full}"
    );
}

#[test]
fn a_skeleton_asks_for_frames_while_it_pulses() {
    let mut d = harness(App, Animation::Full);
    d.frames(1);
    assert!(d.ctx().has_requested_repaint());
}

#[test]
fn under_reduced_motion_a_skeleton_stands_at_full_fill_and_asks_for_nothing() {
    let mut d = harness(App, Animation::Reduced);
    for _ in 0..4 {
        d.frames(1);
        assert_eq!(painted_alpha(&d), muted().a());
    }
    assert!(!d.ctx().has_requested_repaint());
}

#[test]
fn a_skeleton_is_not_announced() {
    let d = harness(App, Animation::Full);
    let id = support::id_of(&d, "/\"sk\"");
    assert!(!d.ctx().has_accesskit_node(id));
}

/// A skeleton draws from the style's sheet: an App rule restyles it.
#[test]
fn an_app_rule_restyles_a_skeleton() {
    let mut d = support::driver(vec2(300.0, 200.0), base(), App, view)
        .step(FRAME)
        .theme(Theme::Light)
        .motion(Animation::Reduced)
        .style(support::shadcn_with(|style| {
            style
                .sheet
                .as_app()
                .rule(eggshell_ui::owned::skeleton::SkeletonPart::Root, |t, _| {
                    t.bg(Color32::RED)
                });
        }))
        .build();
    d.frames(4);
    let skeleton = d.rect(Target::suffix("/\"sk\""));
    let fill = support::shapes(&d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if support::same(rect.rect, skeleton) => Some(rect.fill),
            _ => None,
        });
    assert_eq!(fill, Some(Color32::RED));
}
