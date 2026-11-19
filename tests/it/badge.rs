//! The extras' badge, chip and spinner, through the real event path, in the
//! shadcn preset the rig applies.

use eggshell_ui::contract::Variant;
use eggshell_ui::testing::assert_reported;
use eggshell_ui::{Builder, Problem, TextStyle, tw};
use eggshell_ui_extras::badge::{BadgeExt, BadgeStyle};
use eggshell_ui_extras::spinner::SpinnerExt;
use eggshell_ui_shadcn::{Tokens, size, variant};
use egui::{Key, Role, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, ReducedMotion, Rig, box_at, rect_of};

#[derive(Default)]
struct App {
    clicks: u32,
    removals: u32,
    named: Option<&'static str>,
}

fn base() -> TextStyle {
    let tokens = Tokens::for_theme(Theme::Light);
    TextStyle::new(tokens.text_role(), tokens.foreground)
}

fn harness(view: support::View<App>, app: App) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(vec2(400.0, 200.0), base(), app, view);
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(4);
    harness
}

/// ```html
/// <div class="flex gap-2 p-2">
///   <button class="badge">Rust <button aria-label="Remove Rust">×</button></button>
///   <span class="badge"><button aria-label="Remove Go">×</button></span>
/// </div>
/// ```
fn chips(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().gap(2.0).p(2.0), |b| {
        b.chip("both", "Rust")
            .on_click(|app: &mut App| app.clicks += 1)
            .on_remove(|app: &mut App| app.removals += 1)
            .add();
        b.chip("removable", "Go")
            .on_remove(|app: &mut App| app.removals += 1)
            .add();
    });
}

#[test]
fn a_chip_click_and_a_click_on_its_x_reach_different_handlers() {
    let mut harness = harness(chips, App::default());
    let chip = rect_of(&harness, "/\"both\"");
    let x = rect_of(&harness, "/\"both\"/\"remove\"");
    let label = egui::pos2(chip.left() + 10.0, chip.center().y);
    assert!(
        chip.contains_rect(x),
        "precondition: the × is inside the chip"
    );
    assert!(
        !x.contains(label),
        "precondition: the label point misses the ×"
    );
    support::hover_over(&mut harness, label);
    support::press_and_release(&mut harness, label);
    assert_eq!(
        (harness.state().app.clicks, harness.state().app.removals),
        (1, 0)
    );
    support::click_node(&mut harness, "/\"both\"/\"remove\"");
    assert_eq!(
        (harness.state().app.clicks, harness.state().app.removals),
        (1, 1)
    );
}

#[test]
fn the_x_is_a_button_named_after_the_chip() {
    let harness = harness(chips, App::default());
    harness.get_by_role_and_label(Role::Button, "Rust");
    harness.get_by_role_and_label(Role::Button, "Remove Rust");
    harness.get_by_role_and_label(Role::Button, "Remove Go");
}

#[test]
fn backspace_or_delete_removes_the_focused_chip() {
    for key in [Key::Backspace, Key::Delete] {
        let mut harness = harness(chips, App::default());
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_eq!(
            support::focused_path(&harness).as_deref(),
            Some("/\"both\""),
            "precondition: the clickable chip is the first Tab stop"
        );
        harness.key_press(key);
        harness.run_steps(2);
        assert_eq!(harness.state().app.removals, 1, "{key:?} on the chip");
        assert_eq!(harness.state().app.clicks, 0);
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        assert_eq!(
            support::focused_path(&harness).as_deref(),
            Some("/\"removable\"/\"remove\""),
            "precondition: a chip that is only removed is reached at its ×"
        );
        harness.key_press(key);
        harness.run_steps(2);
        assert_eq!(harness.state().app.removals, 2, "{key:?} on the ×");
    }
}

#[test]
fn backspace_does_nothing_to_a_chip_without_focus() {
    let mut harness = harness(chips, App::default());
    assert_eq!(support::focused_path(&harness), None, "precondition");
    harness.key_press(Key::Backspace);
    harness.run_steps(2);
    assert_eq!(harness.state().app.removals, 0);
}

#[test]
fn a_hovered_chip_takes_its_variants_hover_fill() {
    let mut harness = harness(chips, App::default());
    let style = BadgeStyle::of(&harness.ctx);
    let states = style
        .variants
        .get(style.variants.default_key())
        .paint
        .states;
    assert_ne!(states.rest.fill, states.hover.fill, "precondition");
    let chip = rect_of(&harness, "/\"both\"");
    assert_eq!(box_at(&harness, chip).fill, states.rest.fill);
    support::hover_over(&mut harness, egui::pos2(chip.left() + 6.0, chip.center().y));
    harness.run_steps(20);
    assert_eq!(box_at(&harness, chip).fill, states.hover.fill);
}

const VARIANTS: [Variant; 4] = [
    variant::PRIMARY,
    variant::SECONDARY,
    variant::DESTRUCTIVE,
    variant::OUTLINE,
];

fn badges(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().gap(2.0).p(2.0), |b| {
        for variant in VARIANTS {
            b.badge(variant, "Badge").variant(variant).add();
        }
        b.badge("plain", "Badge").add();
    });
}

#[test]
fn each_badge_variant_draws_its_styles_rest_colours() {
    let harness = harness(badges, App::default());
    let style = BadgeStyle::of(&harness.ctx);
    assert_eq!(style.height, 20.0, "precondition: the preset's style, h-5");
    for variant in VARIANTS {
        assert!(
            style.variants.keys().any(|key| key == variant),
            "the preset defines {variant}"
        );
        let rest = style.variants.get(variant).paint.states.rest;
        let badge = rect_of(&harness, &format!("/{variant:?}"));
        assert_eq!(badge.height(), style.height, "{variant}");
        let drawn = box_at(&harness, badge);
        assert_eq!(drawn.fill, rest.fill, "{variant}");
        assert_eq!(drawn.stroke.color, rest.border, "{variant}");
    }
    let plain = box_at(&harness, rect_of(&harness, "/\"plain\""));
    let default = style.variants.get(style.variants.default_key());
    assert_eq!(plain.fill, default.paint.states.rest.fill);
    assert_eq!(
        style.variants.default_key(),
        variant::PRIMARY,
        "shadcn's default"
    );
}

#[test]
fn a_variant_the_style_lacks_is_reported_and_drawn_as_the_default() {
    fn view(b: &mut Builder<'_, '_, App>, _: &App) {
        b.badge("odd", "Odd").variant(Variant("missing")).add();
    }
    let harness = harness(view, App::default());
    assert_reported(&harness.ctx, Problem::MissingStyleEntry);
    let style = BadgeStyle::of(&harness.ctx);
    let default = style.variants.get(style.variants.default_key());
    let drawn = box_at(&harness, rect_of(&harness, "/\"odd\""));
    assert_eq!(drawn.fill, default.paint.states.rest.fill);
}

fn spinners(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().gap(2.0).p(2.0), |b| {
        match app.named {
            Some(name) => b.spinner("spinner").named(name).add(),
            None => b.spinner("spinner").add(),
        }
        b.div(tw().control_size(size::XS), |b| b.spinner("small").add());
    });
}

/// The names of the frame's status nodes.
fn statuses(harness: &Harness<'_, Rig<App>>) -> Vec<String> {
    let mut names: Vec<String> = harness
        .query_all_by_role(Role::Status)
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    names.sort();
    names
}

#[test]
fn a_spinner_is_a_status_named_loading_unless_named() {
    let harness = harness_named(None);
    assert_eq!(statuses(&harness), ["Loading", "Loading"]);
    let harness = harness_named(Some("Saving"));
    assert_eq!(statuses(&harness), ["Loading", "Saving"]);
}

fn harness_named(named: Option<&'static str>) -> Harness<'static, Rig<App>> {
    harness(
        spinners,
        App {
            named,
            ..App::default()
        },
    )
}

#[test]
fn a_spinner_takes_the_control_size_it_is_built_in() {
    let harness = harness(spinners, App::default());
    assert_eq!(
        rect_of(&harness, "/\"spinner\"").width(),
        16.0,
        "Vega size-4"
    );
    assert_eq!(
        rect_of(&harness, "/\"small\"").width(),
        12.0,
        "the xs glyph"
    );
}

/// The spinner's turn, read off its icon's quad.
fn spinner_turn(harness: &Harness<'_, Rig<App>>) -> f32 {
    let spinner = rect_of(harness, "/\"spinner\"");
    let icons: Vec<_> = support::textured(harness)
        .into_iter()
        .filter(|mesh| spinner.contains(mesh.calc_bounds().center()))
        .collect();
    assert_eq!(icons.len(), 1, "the spinner draws one icon");
    support::turn(icons[0])
}

#[test]
fn a_spinner_turns_unless_motion_is_reduced() {
    let mut harness = harness(spinners, App::default());
    let before = spinner_turn(&harness);
    harness.run_steps(3);
    assert_ne!(spinner_turn(&harness), before, "it turns");

    let mut harness = support::harness(vec2(400.0, 200.0), base(), App::default(), spinners);
    harness.state_mut().reduced_motion = ReducedMotion::Yes;
    harness
        .ctx
        .global_style_mut(|style| style.animation_time = 0.0);
    harness.run_steps(4);
    let before = spinner_turn(&harness);
    harness.run_steps(3);
    assert_eq!(spinner_turn(&harness), before, "it stands still");
    harness.run();
}
