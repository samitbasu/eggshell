//! A widget's own look for a part, given at its call site with
//! `Builder::part_over`: the widgets' defaults, under every layer of the
//! sheet, and cut by a rule that replaces the part's lower layers.

use eggshell_ui::sheet::{Defaults, Part};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Style, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, FontFamily, vec2};

use crate::support;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Chip;

impl Part for Chip {
    const SOURCE: &'static str = "chip.tsx";

    fn slot(self) -> &'static str {
        "chip"
    }
}

fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    let classes = b.part_over(
        Chip,
        &[],
        Defaults(tw().w_px(40.0).h_px(20.0).bg(Color32::RED)),
        tw(),
    );
    b.div_id("chip", classes, |_| {});
}

fn driver(edit: fn(&mut Style)) -> Driver<()> {
    let text = TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    );
    let mut d = support::driver(vec2(200.0, 100.0), text, (), view)
        .style(support::shadcn_with(edit))
        .build();
    d.frames(3);
    d
}

#[test]
fn an_app_rule_goes_over_a_widgets_defaults() {
    let d = driver(|style| {
        style.sheet.as_app().rule(Chip, |t, _| t.w_px(60.0));
    });
    let chip = d.rect(Target::suffix("\"chip\""));
    assert_eq!(
        chip.size(),
        vec2(60.0, 20.0),
        "the app's width, the defaults' height"
    );
    assert_eq!(
        support::box_at(&d, chip).fill,
        Color32::RED,
        "the defaults' fill"
    );
}

#[test]
fn a_rule_that_replaces_the_part_cuts_its_defaults() {
    let d = driver(|style| {
        style
            .sheet
            .as_preset()
            .replace(Chip, |t, _| t.w_px(30.0).h_px(10.0));
    });
    let chip = d.rect(Target::suffix("\"chip\""));
    assert_eq!(chip.size(), vec2(30.0, 10.0));
    assert!(
        support::shapes(&d).iter().all(|shape| !matches!(
            shape,
            egui::epaint::Shape::Rect(r) if r.fill == Color32::RED
        )),
        "no defaults' fill"
    );
}

#[cfg(debug_assertions)]
#[test]
fn the_inspector_shows_the_defaults_as_their_layer_at_the_widgets_lines() {
    use eggshell_ui::sheet::Layer;
    use eggshell_ui::{Property, Source};
    let mut d = driver(|_| {});
    let height = d
        .why(Target::suffix("\"chip\""), Property::height)
        .expect("the defaults set it");
    assert_eq!(height.winner.origin.layer, Layer::Defaults);
    let Source::Rust(at) = height.winner.origin.source else {
        panic!("a Rust call");
    };
    assert!(at.file().ends_with("part_over.rs"), "{at}");
}
