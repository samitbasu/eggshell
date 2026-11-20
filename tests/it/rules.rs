//! Rules a node declares for the nodes under it (#443): which nodes they
//! reach, whose classes win, and what they do not reach.

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Aria, Builder, Icon, Match, TextSpec, TextStyle, Weight, icons, tw};
use egui::{FontFamily, vec2};

use crate::support;

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

fn chevron() -> Icon {
    Icon::new(icons::CHEVRON_DOWN)
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

fn descendants_and_children(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "outer",
        tw().descendants(Match::glyph(), |t| t.size_px(20.0)),
        |b| {
            b.icon("child", tw(), chevron());
            b.div_id(
                "inner",
                tw().children(Match::glyph(), |t| t.size_px(12.0)),
                |b| {
                    b.icon("grandchild", tw(), chevron());
                },
            );
        },
    );
    b.div_id(
        "apart",
        tw().children(Match::glyph(), |t| t.size_px(12.0)),
        |b| {
            b.div_id("between", tw(), |b| {
                b.icon("deep", tw().size_px(7.0), chevron());
            });
        },
    );
}

#[test]
fn descendants_reach_every_depth_and_the_nearest_container_wins() {
    let d = driver(descendants_and_children);
    assert_eq!(
        size_of(&d, "\"child\""),
        (20.0, 20.0),
        "a child, from outer"
    );
    assert_eq!(
        size_of(&d, "\"grandchild\""),
        (12.0, 12.0),
        "inner's children rule is nearer than outer's descendants rule"
    );
}

#[test]
fn children_do_not_reach_a_grandchild() {
    let d = driver(descendants_and_children);
    assert_eq!(
        size_of(&d, "\"deep\""),
        (7.0, 7.0),
        "precondition: the node's own size, which no rule above it reaches"
    );
}

fn own_wins(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "box",
        tw().children(Match::glyph(), |t| t.size_px(20.0).shrink_0()),
        |b| {
            b.icon("narrow", tw().w_px(5.0), chevron());
        },
    );
    b.div_id(
        "bang",
        tw().children(Match::glyph(), |t| t.w_px(20.0).important()),
        |b| {
            b.icon("loud", tw().w_px(5.0).h_px(9.0), chevron());
        },
    );
}

#[test]
fn a_nodes_own_classes_win_property_by_property_and_important_flips_it() {
    let d = driver(own_wins);
    assert_eq!(
        size_of(&d, "\"narrow\""),
        (5.0, 20.0),
        "its own width, the rule's height"
    );
    assert_eq!(
        size_of(&d, "\"loud\""),
        (20.0, 9.0),
        "the important rule's width over its own, its own height kept"
    );
}

fn facts(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "host",
        tw().children(Match::slot("icon"), |t| t.w_px(11.0))
            .children(Match::slot("icon").attr("side", "start"), |t| t.h_px(13.0))
            .children(Match::aria(Aria::Invalid), |t| t.w_px(17.0))
            .children(!Match::slot("icon"), |t| t.h_px(19.0)),
        |b| {
            b.icon("plain", tw().slot("icon"), chevron());
            b.icon("start", tw().slot("icon").attr("side", "start"), chevron());
            b.icon("end", tw().slot("icon").attr("side", "end"), chevron());
            b.icon("bad", tw().slot("other").aria(Aria::Invalid), chevron());
            b.icon("none", tw(), chevron());
        },
    );
}

#[test]
fn a_rule_matches_a_nodes_slot_attributes_aria_state_and_their_negation() {
    let d = driver(facts);
    let width = |name| size_of(&d, name).0;
    let height = |name| size_of(&d, name).1;
    assert_eq!(width("\"plain\""), 11.0, "slot");
    assert_ne!(height("\"plain\""), 13.0, "not the start attribute");
    assert_eq!(height("\"start\""), 13.0, "slot and attribute");
    assert_ne!(height("\"end\""), 13.0, "another value of the attribute");
    assert_eq!(width("\"bad\""), 17.0, "aria");
    assert_eq!(height("\"bad\""), 19.0, "not the icon slot");
    assert_eq!(height("\"none\""), 19.0, "not the icon slot");
    assert_ne!(width("\"none\""), 11.0, "no slot, no slot rule");
}

fn overlay(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id(
        "host",
        tw().descendants(Match::glyph(), |t| t.size_px(20.0)),
        |b| {
            b.overlay("popup")
                .modal(eggshell_ui::overlay::Modal::Center)
                .children(|s| {
                    s.icon("inside", tw().size_px(6.0), chevron());
                    s.icon("bare", tw(), chevron());
                });
        },
    );
}

#[test]
fn a_rule_does_not_reach_into_an_overlay() {
    let d = driver(overlay);
    assert_eq!(size_of(&d, "\"inside\""), (6.0, 6.0));
    assert_ne!(
        size_of(&d, "\"bare\""),
        (20.0, 20.0),
        "the overlay is a root of its own"
    );
}
