//! A style's sheet (#443): the classes it gives a widget's parts, which
//! layer's win, and where the rules of the containers above come in.

use eggshell_ui::contract::Variant;
use eggshell_ui::sheet::{Key, Part, When};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Aria, Builder, Icon, Match, Problem, TextSpec, TextStyle, Weight, icons, tw};
use egui::{FontFamily, vec2};

use crate::support;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Chip {
    Root,
    Icon,
}

impl Part for Chip {
    const SOURCE: &'static str = "badge.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "chip",
            Self::Icon => "chip-icon",
        }
    }
}

const WIDE: Variant = Variant("wide");

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        egui::Color32::BLACK,
    )
}

fn driver(view: fn(&mut Builder<'_, '_, ()>, &())) -> Driver<()> {
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), (), view).build();
    d.frames(4);
    d
}

fn size_of(d: &Driver<()>, name: &str) -> (f32, f32) {
    let rect = d.rect(Target::suffix(name));
    (rect.width(), rect.height())
}

fn chevron() -> Icon {
    Icon::new(icons::CHEVRON_DOWN)
}

fn chip_rules(style: &mut eggshell_ui::Style) {
    style
        .sheet
        .rule(Chip::Root, |t, _| t.w_px(100.0).h_px(30.0))
        .rule_when(Chip::Root, When::always().variant(WIDE), |t, _| {
            t.w_px(160.0)
        })
        .rule_when(Chip::Root, When::always().aria(Aria::Invalid), |t, _| {
            t.h_px(36.0)
        })
        .rule(Chip::Icon, |t, _| t.size_px(16.0));
}

fn parts(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(chip_rules, |b| {
        let plain = b.part(Chip::Root, &[], tw());
        b.div_id("plain", plain, |_| {});
        let wide = b.part(Chip::Root, &[Key::Variant(WIDE)], tw());
        b.div_id("wide", wide, |_| {});
        let invalid = b.part(Chip::Root, &[Key::Aria(Aria::Invalid)], tw());
        b.div_id("invalid", invalid, |_| {});
        let called = b.part(Chip::Root, &[], tw().h_px(50.0));
        b.div_id("called", called, |_| {});
    });
}

#[test]
fn a_part_gets_the_classes_of_the_rules_whose_keys_hold() {
    let d = driver(parts);
    assert_eq!(size_of(&d, "\"plain\""), (100.0, 30.0));
    assert_eq!(size_of(&d, "\"wide\""), (160.0, 30.0), "a variant");
    assert_eq!(size_of(&d, "\"invalid\""), (100.0, 36.0), "an ARIA state");
}

#[test]
fn the_call_site_wins_over_the_sheet_property_by_property() {
    let d = driver(parts);
    assert_eq!(
        size_of(&d, "\"called\""),
        (100.0, 50.0),
        "the height is the call site's, the width the sheet's"
    );
}

fn icons_in_containers(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(chip_rules, |b| {
        let alone = b.part(Chip::Icon, &[], tw());
        b.icon("alone", alone, chevron());
        b.div_id(
            "container",
            tw().descendants(Match::glyph(), |t| t.size_px(12.0)),
            |b| {
                let from_sheet = b.part(Chip::Icon, &[], tw());
                b.icon("from-sheet", from_sheet, chevron());
                let called = b.part(Chip::Icon, &[], tw().size_px(20.0));
                b.icon("called", called, chevron());
            },
        );
    });
}

#[test]
fn the_rules_of_a_container_go_over_the_sheet_and_under_the_call_site() {
    let d = driver(icons_in_containers);
    assert_eq!(size_of(&d, "\"alone\""), (16.0, 16.0), "the sheet's own");
    assert_eq!(
        size_of(&d, "\"from-sheet\""),
        (12.0, 12.0),
        "the container's rule beats the sheet's"
    );
    assert_eq!(
        size_of(&d, "\"called\""),
        (20.0, 20.0),
        "an explicit class beats the container's rule"
    );
}

fn slots(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(chip_rules, |b| {
        b.div_id(
            "container",
            tw().children(Match::slot("chip-icon"), |t| t.size_px(9.0)),
            |b| {
                let icon = b.part(Chip::Icon, &[], tw());
                b.icon("icon", icon, chevron());
                b.icon("other", tw().size_px(16.0), chevron());
            },
        );
    });
}

#[test]
fn a_part_declares_its_slot_for_the_rules_that_match_one() {
    let d = driver(slots);
    assert_eq!(size_of(&d, "\"icon\""), (9.0, 9.0));
    assert_eq!(size_of(&d, "\"other\""), (16.0, 16.0), "no slot, no match");
}

fn layers(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(
        |style| {
            style.sheet.as_app().rule(Chip::Root, |t, _| t.w_px(80.0));
            style
                .sheet
                .as_base()
                .rule(Chip::Root, |t, _| t.w_px(100.0).h_px(30.0));
        },
        |b| {
            let root = b.part(Chip::Root, &[], tw());
            b.div_id("root", root, |_| {});
        },
    );
}

#[test]
fn a_later_layer_wins_whatever_order_its_rules_were_written_in() {
    let d = driver(layers);
    assert_eq!(size_of(&d, "\"root\""), (80.0, 30.0));
}

fn replaced(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(
        |style| {
            style.sheet.as_base();
            chip_rules(style);
            style
                .sheet
                .as_app()
                .replace_when(Chip::Root, When::always().variant(WIDE), |t, _| {
                    t.h_px(44.0)
                });
        },
        |b| {
            let wide = b.part(Chip::Root, &[Key::Variant(WIDE)], tw());
            b.div_id("wide", wide, |_| {});
            let plain = b.part(Chip::Root, &[], tw());
            b.div_id("plain", plain, |_| {});
        },
    );
}

#[test]
fn replace_discards_the_rules_below_it_where_it_holds() {
    let d = driver(replaced);
    let (width, height) = size_of(&d, "\"wide\"");
    assert_eq!(height, 44.0);
    assert_ne!(width, 160.0, "the variant's width is gone with the rest");
    assert_eq!(
        size_of(&d, "\"plain\""),
        (100.0, 30.0),
        "elsewhere untouched"
    );
}

fn unnamed(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(chip_rules, |b| {
        let odd = b.part(Chip::Root, &[Key::Variant(Variant("missing"))], tw());
        b.div_id("odd", odd, |_| {});
    });
}

#[test]
fn a_variant_the_sheet_leaves_out_is_reported_and_drawn_without_it() {
    let mut d = driver(unnamed);
    d.expect_reported(Problem::MissingStyleEntry);
    assert_eq!(size_of(&d, "\"odd\""), (100.0, 30.0));
}

fn negated(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(
        |style| {
            style.sheet.rule_when(
                Chip::Root,
                When::always().not_aria(Aria::HasPopup),
                |t, _| t.w_px(70.0).h_px(20.0),
            );
        },
        |b| {
            let free = b.part(Chip::Root, &[], tw());
            b.div_id("free", free, |_| {});
            let popup = b.part(Chip::Root, &[Key::Aria(Aria::HasPopup)], tw());
            b.div_id("popup", popup, |_| {});
        },
    );
}

#[test]
fn a_negated_key_applies_while_the_node_lacks_the_state() {
    let d = driver(negated);
    assert_eq!(size_of(&d, "\"free\""), (70.0, 20.0));
    assert_ne!(size_of(&d, "\"popup\""), (70.0, 20.0));
}

#[test]
fn a_preset_hands_out_the_sheet_of_each_theme_at_the_base_layer() {
    use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle};
    let themed = Shadcn::themed(ShadcnStyle::Vega, Palette::new());
    assert_eq!(themed.light.sheet.layer(), eggshell_ui::sheet::Layer::Base);
    assert_eq!(themed.dark.sheet.layer(), eggshell_ui::sheet::Layer::Base);
    assert_eq!(
        eggshell_ui::Style::default().sheet.layer(),
        eggshell_ui::sheet::Layer::Base
    );
}

fn by_token(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.styled(
        |style| {
            style
                .sheet
                .rule(Chip::Root, |t, k| t.w_px(k.control_height()).h_px(10.0));
        },
        |b| {
            let before = b.part(Chip::Root, &[], tw());
            b.div_id("before", before, |_| {});
            b.styled(
                |style| style.tokens.control_units = 77.0 / style.tokens.spacing.points(1.0),
                |b| {
                    let scoped = b.part(Chip::Root, &[], tw());
                    b.div_id("scoped", scoped, |_| {});
                },
            );
            let after = b.part(Chip::Root, &[], tw());
            b.div_id("after", after, |_| {});
        },
    );
}

#[test]
fn a_rule_body_is_given_the_tokens_of_the_style_it_is_resolved_in() {
    let d = driver(by_token);
    let standard = size_of(&d, "\"before\"");
    assert_ne!(standard.0, 77.0, "precondition");
    assert_eq!(size_of(&d, "\"scoped\""), (77.0, 10.0));
    assert_eq!(
        size_of(&d, "\"after\""),
        standard,
        "the same sheet resolves again for other tokens, and back"
    );
}

#[test]
fn the_preset_fills_the_shadow_scale() {
    use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle};
    let themed = Shadcn::themed(ShadcnStyle::Vega, Palette::new());
    let tokens = &themed.light.tokens;
    assert_ne!(tokens.shadow_xs, egui::Shadow::NONE);
    assert!(tokens.shadow_lg.offset[1] > tokens.shadow_md.offset[1]);
}
