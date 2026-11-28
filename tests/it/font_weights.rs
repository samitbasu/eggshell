//! Font weights through the real path: the shadcn preset registers its font,
//! and widgets and weight classes draw with the face of their weight (#13).

use std::sync::Arc;

use eggshell_ui::contract::Themed;
use eggshell_ui::testing::Driver;
use eggshell_ui::theme::{FontSet, Shadcn};
use eggshell_ui::{Builder, Problem, TextSpec, Weight, tw};
use egui::epaint::Shape;
use egui::{FontFamily, Theme, Vec2};

use crate::support;

use eggshell_examples::common;

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

fn harness(fonts: impl Fn(Shadcn) -> Shadcn) -> Driver<()> {
    let half = |preset: Shadcn| Arc::new(fonts(preset).style());
    let mut d = support::driver(
        Vec2::new(300.0, 300.0),
        common::base_text(Theme::Light),
        (),
        view,
    )
    .style(&eggshell_ui::SharedStyle::new(Themed {
        light: half(Shadcn::light()),
        dark: half(Shadcn::dark()),
    }))
    .build();
    d.frames(3);
    d
}

/// The family `text` was painted in.
fn family_of(d: &Driver<()>, text: &str) -> FontFamily {
    support::shapes(d)
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
    let d = harness(|preset| preset);
    assert!(
        d.ctx()
            .fonts(|fonts| fonts.families().contains(&geist(Weight::Bold))),
        "precondition: the preset registered Geist's weights"
    );
    for (text, weight) in [
        ("normal", Weight::Normal),
        ("medium", Weight::Medium),
        ("semibold", Weight::SemiBold),
        ("bold", Weight::Bold),
    ] {
        assert_eq!(family_of(&d, text), geist(weight), "{text}");
    }
}

#[test]
fn a_shadcn_button_is_medium() {
    let d = harness(|preset| preset);
    assert_eq!(family_of(&d, "Save"), geist(Weight::Medium));
}

#[test]
fn without_a_font_every_weight_draws_the_regular_family() {
    let d = harness(|preset| preset.with_fonts(None));
    for text in ["Save", "medium", "bold"] {
        assert_eq!(family_of(&d, text), FontFamily::Proportional, "{text}");
    }
}

#[test]
fn a_set_without_semibold_draws_semibold_in_its_medium_face() {
    let font = egui::FontData::from_static(include_bytes!("../../src/fonts/Geist-Variable.ttf"));
    let partial = FontSet::faces(
        "partial",
        [Weight::Normal, Weight::Medium, Weight::Bold].map(|weight| (weight, font.clone())),
    );
    let mut d = harness(|preset| preset.with_fonts(Some(partial.clone())));
    let named = |weight: Weight| FontFamily::Name(format!("partial/{}", weight.number()).into());
    assert_eq!(
        family_of(&d, "semibold"),
        named(Weight::Medium),
        "lighter, never Bold"
    );
    assert_eq!(family_of(&d, "bold"), named(Weight::Bold));
    d.expect_reported(Problem::MissingFace);
}

#[test]
fn a_role_carries_its_weight() {
    let d = harness(|preset| preset);
    let spec = TextSpec {
        weight: Weight::SemiBold,
        ..common::sized(14.0)
    };
    assert_eq!(spec.font_id(d.ctx()).family, geist(Weight::SemiBold));
}

/// The names of the faces that have `c` in `family`, first choice first.
fn faces_for(d: &Driver<()>, family: &FontFamily, c: char) -> Vec<String> {
    d.ctx().fonts_mut(|fonts| {
        fonts
            .characters(family)
            .get(&c)
            .cloned()
            .unwrap_or_default()
    })
}

#[test]
fn geist_draws_its_own_key_symbols_and_the_symbol_face_comes_last() {
    let d = harness(|preset| preset);
    for weight in Weight::ALL {
        let family = geist(weight);
        let shift = faces_for(&d, &family, '\u{21e7}');
        let command = faces_for(&d, &family, '\u{2318}');
        assert_eq!(
            shift.first().map(String::as_str),
            Some(format!("Geist/{}", weight.number()).as_str()),
            "{weight:?}: Geist has its own ⇧: {shift:?}"
        );
        assert!(
            command
                .first()
                .is_some_and(|face| face.starts_with("eggshell-symbols")),
            "{weight:?}: ⌘, which Geist lacks, comes from the symbol face: {command:?}"
        );
        assert!(
            shift
                .last()
                .is_some_and(|face| face.starts_with("eggshell-symbols")),
            "{weight:?}: the symbol face is the last fallback: {shift:?}"
        );
    }
}

#[test]
fn a_medium_key_chip_draws_the_command_symbol() {
    let chip = "\u{2318}K";
    let mut d = support::driver(
        Vec2::new(200.0, 60.0),
        common::base_text(Theme::Light),
        (),
        |b, _| b.kbd("\u{2318}K"),
    )
    .build();
    d.frames(3);
    let drawn = support::shapes(&d)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == chip => {
                Some(text.galley.job.sections[0].format.font_id.family.clone())
            }
            _ => None,
        });
    assert_eq!(
        drawn,
        Some(geist(Weight::Medium)),
        "{chip:?} is drawn medium"
    );
}
