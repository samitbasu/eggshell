//! The extras' badge, chip and spinner, through the real event path, in the
//! shadcn preset the rig applies.

use eggshell_ui::contract::Variant;
use eggshell_ui::testing::{Animation, Driver, Target};
use eggshell_ui::{Builder, Problem, TextStyle, tw};
use eggshell_ui_extras::badge::{BadgeExt, BadgeStyle};
use eggshell_ui_extras::spinner::SpinnerExt;
use eggshell_ui_shadcn::{Tokens, size, variant};
use egui::{Key, Role, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, box_at};

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

fn builder(view: support::View<App>, app: App) -> eggshell_ui::testing::DriverBuilder<App> {
    support::driver(vec2(400.0, 200.0), base(), app, view).theme(Theme::Light)
}

fn harness(view: support::View<App>, app: App) -> Driver<App> {
    let mut d = builder(view, app).build();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(4);
    d
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
    let mut d = harness(chips, App::default());
    let chip = d.rect(Target::suffix("/\"both\""));
    let x = d.rect(Target::suffix("/\"both\"/\"remove\""));
    let label = egui::pos2(chip.left() + 10.0, chip.center().y);
    assert!(
        chip.contains_rect(x),
        "precondition: the × is inside the chip"
    );
    assert!(
        !x.contains(label),
        "precondition: the label point misses the ×"
    );
    d.click(label);
    assert_eq!((d.app().clicks, d.app().removals), (1, 0));
    d.click(Target::suffix("/\"both\"/\"remove\""));
    assert_eq!((d.app().clicks, d.app().removals), (1, 1));
}

#[test]
fn the_x_is_a_button_named_after_the_chip() {
    let d = harness(chips, App::default());
    for name in ["Rust", "Remove Rust", "Remove Go"] {
        d.harness().get_by_role_and_label(Role::Button, name);
    }
}

#[test]
fn backspace_or_delete_removes_the_focused_chip() {
    for key in [Key::Backspace, Key::Delete] {
        let mut d = harness(chips, App::default());
        d.press(Key::Tab).frames(1);
        assert_eq!(
            support::focused_path(&d).as_deref(),
            Some("/\"both\""),
            "precondition: the clickable chip is the first Tab stop"
        );
        d.press(key).frames(1);
        assert_eq!(d.app().removals, 1, "{key:?} on the chip");
        assert_eq!(d.app().clicks, 0);
        d.press(Key::Tab).frames(1);
        assert_eq!(
            support::focused_path(&d).as_deref(),
            Some("/\"removable\"/\"remove\""),
            "precondition: a chip that is only removed is reached at its ×"
        );
        d.press(key).frames(1);
        assert_eq!(d.app().removals, 2, "{key:?} on the ×");
    }
}

#[test]
fn backspace_does_nothing_to_a_chip_without_focus() {
    let mut d = harness(chips, App::default());
    assert_eq!(support::focused_path(&d), None, "precondition");
    d.press(Key::Backspace).frames(1);
    assert_eq!(d.app().removals, 0);
}

#[test]
fn a_hovered_chip_takes_its_variants_hover_fill() {
    let mut d = harness(chips, App::default());
    let style = d.shell().style().extension::<BadgeStyle>();
    let states = style
        .variants
        .get(style.variants.default_key())
        .paint
        .states;
    assert_ne!(states.rest.fill, states.hover.fill, "precondition");
    let chip = d.rect(Target::suffix("/\"both\""));
    assert_eq!(box_at(&d, chip).fill, states.rest.fill);
    d.hover(egui::pos2(chip.left() + 6.0, chip.center().y))
        .frames(20);
    assert_eq!(box_at(&d, chip).fill, states.hover.fill);
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
    let d = harness(badges, App::default());
    let style = d.shell().style().extension::<BadgeStyle>();
    assert_eq!(style.height, 20.0, "precondition: the preset's style, h-5");
    for variant in VARIANTS {
        assert!(
            style.variants.keys().any(|key| key == variant),
            "the preset defines {variant}"
        );
        let rest = style.variants.get(variant).paint.states.rest;
        let badge = d.rect(Target::suffix(&format!("/{variant:?}")));
        assert_eq!(badge.height(), style.height, "{variant}");
        let drawn = box_at(&d, badge);
        assert_eq!(drawn.fill, rest.fill, "{variant}");
        assert_eq!(drawn.stroke.color, rest.border, "{variant}");
    }
    let plain = box_at(&d, d.rect(Target::suffix("/\"plain\"")));
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
    let mut d = harness(view, App::default());
    d.expect_reported(Problem::MissingStyleEntry);
    let style = d.shell().style().extension::<BadgeStyle>();
    let default = style.variants.get(style.variants.default_key());
    let drawn = box_at(&d, d.rect(Target::suffix("/\"odd\"")));
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
fn statuses(d: &Driver<App>) -> Vec<String> {
    let mut names: Vec<String> = d
        .harness()
        .query_all_by_role(Role::Status)
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    names.sort();
    names
}

#[test]
fn a_spinner_is_a_status_named_loading_unless_named() {
    let d = harness_named(None);
    assert_eq!(statuses(&d), ["Loading", "Loading"]);
    let d = harness_named(Some("Saving"));
    assert_eq!(statuses(&d), ["Loading", "Saving"]);
}

fn harness_named(named: Option<&'static str>) -> Driver<App> {
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
    let d = harness(spinners, App::default());
    assert_eq!(
        d.rect(Target::suffix("/\"spinner\"")).width(),
        16.0,
        "Vega size-4"
    );
    assert_eq!(
        d.rect(Target::suffix("/\"small\"")).width(),
        12.0,
        "the xs glyph"
    );
}

/// The spinner's turn, read off its icon's quad.
fn spinner_turn(d: &Driver<App>) -> f32 {
    let spinner = d.rect(Target::suffix("/\"spinner\""));
    let icons: Vec<_> = support::textured(d)
        .into_iter()
        .filter(|mesh| spinner.contains(mesh.calc_bounds().center()))
        .collect();
    assert_eq!(icons.len(), 1, "the spinner draws one icon");
    support::turn(icons[0])
}

#[test]
fn a_spinner_turns_unless_motion_is_reduced() {
    let mut d = harness(spinners, App::default());
    let before = spinner_turn(&d);
    d.frames(3);
    assert_ne!(spinner_turn(&d), before, "it turns");

    let mut d = builder(spinners, App::default())
        .motion(Animation::Reduced)
        .build();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(4);
    let before = spinner_turn(&d);
    d.frames(3);
    assert_eq!(spinner_turn(&d), before, "it stands still");
    d.settle();
}

/// A badge says shadcn's `focus-visible:` ring, which would make it a Tab
/// stop; only a chip that can be clicked is one.
#[test]
fn a_badge_is_no_tab_stop() {
    let mut d = harness(badges, App::default());
    for _ in 0..8 {
        d.press(Key::Tab).frames(2);
        assert_eq!(
            support::focused_path(&d),
            None,
            "nothing among the badges takes the focus"
        );
    }
}

/// The badge is drawn from the style's sheet: an App rule on its part
/// restyles it over the preset's look.
#[test]
fn an_app_rule_restyles_a_badge() {
    let d = builder(badges, App::default())
        .style(support::shadcn_with(|style| {
            style
                .sheet
                .as_app()
                .rule(eggshell_ui_extras::badge::BadgePart::Root, |t, _| {
                    t.bg(egui::Color32::RED)
                });
        }))
        .build();
    let mut d = d;
    d.frames(4);
    let plain = d.rect(Target::suffix("/\"plain\""));
    assert_eq!(box_at(&d, plain).fill, egui::Color32::RED);
}

/// A chip whose × is its Tab stop rings itself while the × has the focus,
/// over the preset's generated look.
#[test]
fn a_chip_rings_while_its_x_has_the_focus() {
    let mut d = harness(chips, App::default());
    let x = "/\"removable\"/\"remove\"";
    for _ in 0..8 {
        if support::focused_path(&d)
            .as_deref()
            .is_some_and(|path| path.ends_with(x))
        {
            break;
        }
        d.press(Key::Tab).frames(4);
    }
    assert!(
        support::focused_path(&d)
            .as_deref()
            .is_some_and(|path| path.ends_with(x)),
        "precondition: the × has the focus"
    );
    d.frames(8);
    let chip = d.rect(Target::suffix("/\"removable\""));
    assert!(support::ring_at(&d, chip).is_some(), "the chip is ringed");
}
