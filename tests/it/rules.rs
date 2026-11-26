//! Rules a node declares for the nodes under it (#443): which nodes they
//! reach, whose classes win, and what they do not reach.

use eggshell_ui::prelude::*;
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

struct Held {
    invalid: bool,
}

fn has(b: &mut Builder<'_, '_, Held>, held: &Held) {
    b.div_id(
        "host",
        tw().flex()
            .pl(1.0)
            .has(Match::slot("icon").attr("side", "start"), |b| b.pl(10.0))
            .has(Match::aria(Aria::Invalid), |b| b.pt(5.0)),
        |b| {
            let tint = if held.invalid {
                tw().aria(Aria::Invalid)
            } else {
                tw()
            };
            b.icon("lead", tint.slot("icon").attr("side", "start"), chevron());
        },
    );
    b.div_id(
        "bare",
        tw().flex()
            .pl(1.0)
            .has(Match::slot("icon").attr("side", "start"), |b| b.pl(10.0)),
        |b| {
            b.icon("other", tw().slot("icon").attr("side", "end"), chevron());
        },
    );
}

fn held_driver(invalid: bool) -> Driver<Held> {
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), Held { invalid }, has).build();
    d.frames(4);
    d
}

#[test]
fn has_applies_its_box_classes_over_the_nodes_own_when_a_child_matches() {
    let d = held_driver(false);
    let host = d.rect(Target::suffix("\"host\""));
    let lead = d.rect(Target::suffix("\"lead\""));
    assert_eq!(
        lead.min.x - host.min.x,
        40.0,
        "pl-10 from the matching child over its own pl-1"
    );
    let bare = d.rect(Target::suffix("\"bare\""));
    let other = d.rect(Target::suffix("\"other\""));
    assert_eq!(
        other.min.x - bare.min.x,
        4.0,
        "no child matches: its own pl-1 stands"
    );
}

#[test]
fn has_rules_match_a_childs_aria_state_and_each_rule_is_its_own() {
    let d = held_driver(true);
    let host = d.rect(Target::suffix("\"host\""));
    let lead = d.rect(Target::suffix("\"lead\""));
    assert_eq!(lead.min.y - host.min.y, 20.0, "pt-5 from the invalid child");
    assert_eq!(
        lead.min.x - host.min.x,
        40.0,
        "and the slot rule still applies"
    );
}

struct Field;

fn field() -> Target {
    Target::role(egui::accesskit::Role::TextInput, "Query")
}

fn focused(b: &mut Builder<'_, '_, Field>, _: &Field) {
    b.div_id(
        "group",
        tw().flex().has(Match::focus_visible(), |b| b.pl(10.0)),
        |b| {
            b.text_field("query", String::new())
                .named("Query")
                .on_change(|_: &mut Field, _| {});
        },
    );
}

#[test]
fn has_focus_visible_follows_the_focus_a_frame_late() {
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), Field, focused).build();
    d.frames(4);
    let at = |d: &Driver<Field>| d.rect(field()).min.x - d.rect(Target::suffix("\"group\"")).min.x;
    assert_eq!(at(&d), 0.0, "precondition: nothing is focused");
    d.click(field());
    d.frames(4);
    assert_eq!(at(&d), 40.0, "the group's pl-10 while its field has focus");
}

/// ```html
/// <div id="deep" class="flex pl-1 has-aria-checked:pl-10 has-[>[data-slot=field]]:pt-5">
///   <div class="flex has-[[data-slot=nothing]]:pr-3">
///     <svg id="checked" aria-checked="true" />
///   </div>
/// </div>
/// <div id="bare" class="flex pl-1 has-aria-checked:pl-10 has-[>[data-slot=field]]:pt-5">
///   <div class="flex"><svg id="under" aria-checked="true" /></div>
/// </div>
/// <div id="near" class="flex pl-1 has-aria-checked:pl-10 has-[>[data-slot=field]]:pt-5">
///   <div data-slot="field" class="flex"><svg id="plain" /></div>
/// </div>
/// <div id="far" class="flex pl-1 has-aria-checked:pl-10 has-[>[data-slot=field]]:pt-5">
///   <div class="flex"><div data-slot="field" class="flex"><svg id="nested" /></div></div>
/// </div>
/// ```
fn depths(b: &mut Builder<'_, '_, ()>, _: &()) {
    let host = || {
        tw().flex()
            .pl(1.0)
            .has(Match::aria(Aria::Checked), |b| b.pl(10.0))
            .has_child(Match::slot("field"), |b| b.pt(5.0))
    };
    b.div_id("deep", host(), |b| {
        b.div(
            tw().flex().has(Match::slot("nothing"), |b| b.pr(3.0)),
            |b| {
                b.icon("checked", tw().aria(Aria::Checked), chevron());
            },
        );
    });
    b.div_id("bare", host(), |b| {
        b.div(tw().flex(), |b| {
            b.icon("under", tw().aria(Aria::Checked), chevron());
        });
    });
    b.div_id("near", host(), |b| {
        b.div(tw().flex().slot("field"), |b| {
            b.icon("plain", tw(), chevron());
        });
    });
    b.div_id("far", host(), |b| {
        b.div(tw().flex(), |b| {
            b.div(tw().flex().slot("field"), |b| {
                b.icon("nested", tw(), chevron());
            });
        });
    });
}

#[test]
fn has_matches_a_node_at_any_depth_and_has_child_only_a_child() {
    let mut d = support::driver(vec2(300.0, 300.0), base_text(), (), depths).build();
    d.frames(4);
    let offset = |host: &str, leaf: &str| {
        let host = d.rect(Target::suffix(&format!("\"{host}\"")));
        let leaf = d.rect(Target::suffix(&format!("\"{leaf}\"")));
        leaf.min - host.min
    };
    assert_eq!(
        offset("deep", "checked"),
        vec2(40.0, 0.0),
        "a grandchild matches has, through a container with has rules of its own"
    );
    assert_eq!(
        offset("bare", "under"),
        vec2(40.0, 0.0),
        "and through a container with no rules"
    );
    assert_eq!(
        offset("near", "plain"),
        vec2(4.0, 20.0),
        "a child matches has_child"
    );
    assert_eq!(
        offset("far", "nested"),
        vec2(4.0, 0.0),
        "a grandchild does not match has_child"
    );
}
