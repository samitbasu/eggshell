//! Font weights through the real path: the shadcn preset registers its font,
//! and widgets and weight classes draw with the face of their weight (#13).

use eggshell_ui::{Builder, TextSpec, Weight, tw};
use eggshell_ui_shadcn::{FontSet, Shadcn};
use egui::epaint::Shape;
use egui::{FontFamily, Theme, Vec2};
use egui_kittest::Harness;

use crate::support;

#[path = "../../examples/common/mod.rs"]
mod common;

fn geist(weight: Weight) -> FontFamily {
    match weight {
        Weight::Normal => FontFamily::Proportional,
        weight => FontFamily::Name(format!("Geist/{}", weight.number()).into()),
    }
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().gap(2.0).p(4.0), |b| {
        b.button("save", "Save").on_click(|_: &mut ()| {});
        b.text(tw().font_normal(), "normal");
        b.text(tw().font_medium(), "medium");
        b.text(tw().font_semibold(), "semibold");
        b.text(tw().font_bold(), "bold");
    });
}

fn harness(prepare: fn(&egui::Context, &())) -> Harness<'static, support::Rig<()>> {
    let mut harness = support::harness_prepared(
        Vec2::new(300.0, 300.0),
        common::base_text(Theme::Light),
        (),
        view,
        prepare,
    );
    harness.run_steps(3);
    harness
}

/// The family `text` was painted in.
fn family_of(harness: &Harness<'_, support::Rig<()>>, text: &str) -> FontFamily {
    support::shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.galley.job.sections[0].format.font_id.family.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} was painted"))
}

#[test]
fn each_weight_class_draws_the_face_of_its_weight() {
    let harness = harness(|_, _| {});
    assert!(
        harness
            .ctx
            .fonts(|fonts| fonts.families().contains(&geist(Weight::Bold))),
        "precondition: the preset registered Geist's weights"
    );
    for (text, weight) in [
        ("normal", Weight::Normal),
        ("medium", Weight::Medium),
        ("semibold", Weight::SemiBold),
        ("bold", Weight::Bold),
    ] {
        assert_eq!(family_of(&harness, text), geist(weight), "{text}");
    }
}

#[test]
fn a_shadcn_button_is_medium() {
    let harness = harness(|_, _| {});
    assert_eq!(family_of(&harness, "Save"), geist(Weight::Medium));
}

#[test]
fn without_a_font_every_weight_draws_the_regular_family() {
    let harness = harness(|ctx, _| {
        for theme in [Theme::Light, Theme::Dark] {
            Shadcn::for_theme(theme).with_fonts(None).apply(ctx);
        }
    });
    for text in ["Save", "medium", "bold"] {
        assert_eq!(
            family_of(&harness, text),
            FontFamily::Proportional,
            "{text}"
        );
    }
}

#[test]
fn a_set_without_semibold_draws_semibold_in_its_medium_face() {
    let harness = harness(|ctx, _| {
        let font = egui::FontData::from_static(include_bytes!(
            "../../crates/eggshell-ui-shadcn/fonts/Geist-Variable.ttf"
        ));
        let partial = FontSet::faces(
            "partial",
            [Weight::Normal, Weight::Medium, Weight::Bold].map(|weight| (weight, font.clone())),
        );
        for theme in [Theme::Light, Theme::Dark] {
            Shadcn::for_theme(theme)
                .with_fonts(Some(partial.clone()))
                .apply(ctx);
        }
    });
    let named = |weight: Weight| FontFamily::Name(format!("partial/{}", weight.number()).into());
    assert_eq!(
        family_of(&harness, "semibold"),
        named(Weight::Medium),
        "lighter, never Bold"
    );
    assert_eq!(family_of(&harness, "bold"), named(Weight::Bold));
}

#[test]
fn a_role_carries_its_weight() {
    let harness = harness(|_, _| {});
    let spec = TextSpec {
        weight: Weight::SemiBold,
        ..common::sized(14.0)
    };
    assert_eq!(spec.font_id(&harness.ctx).family, geist(Weight::SemiBold));
}
