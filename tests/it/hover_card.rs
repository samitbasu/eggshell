//! The extras hover card: `b.hover_card(key).trigger(..).content(..)`, on
//! the core `hover_anchored` primitive, with its style and delays.

use core::time::Duration;
use eggshell_ui::prelude::*;

use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Shadcn, Tokens};
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

const HANDLE: &str = "/\"nextjs\"/\"handle\"";
const QUICK: &str = "/\"quick\"/\"handle\"";
const FOLLOW: &str = "/\"nextjs\"/\"Card\"/\"Box\"/\"follow\"";

#[derive(Default)]
struct App {
    follows: u32,
}

fn still(ctx: &egui::Context, _: &App) {
    ctx.global_style_mut(|style| style.animation_time = 0.0);
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(
        tw().flex().justify_center().gap(4.0).p(4.0).items_start(),
        |b| {
            b.hover_card("nextjs")
                .trigger(tw().flex(), |t| {
                    t.button("handle", "@nextjs").on_click(|_: &mut App| {});
                })
                .content(|c| {
                    c.text(tw().font_semibold(), "@nextjs");
                    c.text(tw(), format!("{} follows", app.follows));
                    c.button("follow", "Follow")
                        .on_click(|app: &mut App| app.follows += 1);
                });
            b.hover_card("link")
                .trigger(tw().flex(), |t| t.text(tw(), "@link"))
                .content(|c| c.text(tw(), "A link's card"));
            b.hover_card("avatar")
                .trigger(tw().size_px(40.0).rounded_full(), |_| {})
                .content(|c| c.text(tw(), "An avatar's card"));
            b.hover_card("quick")
                .open_delay(Duration::from_millis(100))
                .trigger(tw().flex(), |t| {
                    t.button("handle", "@quick").on_click(|_: &mut App| {});
                })
                .content(|c| c.text(tw(), "Quick"));
        },
    );
}

fn harness() -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::prepared_driver(
        vec2(500.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
        still,
    )
    .build();
    d.frames(4);
    d
}

fn shown(d: &Driver<App>, card: &str) -> Option<egui::Rect> {
    d.find(Target::suffix(&format!("/\"{card}\"/\"Card\"/\"Box\"")))
}

#[test]
fn the_shadcn_preset_gives_shadcns_hover_card() {
    let d = harness();
    let given = d.shell().style().hover_card.clone();
    let preset = Shadcn {
        tokens: d.shell().style().tokens.clone(),
    };
    assert_eq!(given, preset.style().hover_card);
    assert_eq!(given.width, 256.0, "w-64");
    assert_eq!(given.padding, 16.0, "p-4");
    assert_eq!(given.radius, preset.tokens.radius_lg(), "rounded-lg");
    assert_eq!(given.transition, Duration::from_millis(100), "duration-100");
    assert_eq!(given.open_delay, Duration::from_millis(700));
    assert_eq!(given.close_delay, Duration::from_millis(300));
    let colors = given.look.paint.states.rest;
    assert_eq!(colors.fill, preset.tokens.popover);
    assert_eq!(colors.text, preset.tokens.popover_foreground);
    assert_eq!(colors.border, preset.tokens.border);
}

#[test]
fn resting_on_the_trigger_opens_the_styled_box_below_it_and_its_button_clicks() {
    let mut d = harness();
    d.hover(Target::suffix(HANDLE))
        .advance(Duration::from_millis(500));
    assert!(shown(&d, "nextjs").is_none(), "waits for 700 ms");
    d.advance(Duration::from_millis(300));
    let card = shown(&d, "nextjs").expect("open");
    let trigger = d.rect(Target::suffix("/\"nextjs\""));
    assert!((card.width() - 256.0).abs() < 0.5, "w-64: {}", card.width());
    assert!(
        (card.min.y - trigger.max.y - 4.0).abs() < 0.5,
        "4 pt below: {card:?} {trigger:?}"
    );
    assert!(
        trigger.center().x >= 140.0,
        "precondition: there is room to centre it"
    );
    assert!(
        (card.center().x - trigger.center().x).abs() < 0.5,
        "centred: {card:?} {trigger:?}"
    );
    d.click(Target::suffix(FOLLOW)).frames(2);
    assert_eq!(d.app().follows, 1);
    assert!(shown(&d, "nextjs").is_some(), "the card stays");
}

#[test]
fn a_cards_own_open_delay_overrides_the_style() {
    let mut d = harness();
    d.hover(Target::suffix(QUICK))
        .advance(Duration::from_millis(200));
    assert!(shown(&d, "quick").is_some());
}

#[test]
fn the_cards_text_describes_the_trigger() {
    let d = harness();
    let node = d.harness().get_by_label("@nextjs");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("@nextjs 0 follows Follow")
    );
}

#[test]
fn hovering_a_trigger_with_nothing_focusable_in_it_opens_the_card() {
    for (key, text) in [("link", "A link's card"), ("avatar", "An avatar's card")] {
        let mut d = harness();
        let trigger = d.rect(Target::suffix(&format!("/\"{key}\"")));
        assert!(
            trigger.width() > 0.0 && trigger.height() > 0.0,
            "precondition: {key} is laid out"
        );
        d.hover(Target::suffix(&format!("/\"{key}\"")))
            .advance(Duration::from_millis(800));
        assert!(shown(&d, key).is_some(), "{key}: the card opens");
        d.harness().get_by_label(text);
    }
}

/// hover-card.tsx in `style-vega.css`: the card's box is drawn from the
/// sheet, `ring-1 ring-foreground/10` where the widget's own look is
/// bordered; an App rule on its part restyles it.
#[test]
fn the_cards_box_is_shadcns_from_the_sheet_and_an_app_rule_restyles_it() {
    use eggshell_ui::owned::hover_card;
    let open = |d: &mut Driver<App>| {
        d.hover(Target::suffix(HANDLE))
            .advance(Duration::from_millis(800));
        shown(d, "nextjs").expect("open")
    };
    let mut d = harness();
    let card = open(&mut d);
    let shape = support::box_at(&d, card);
    assert_eq!(shape.stroke.width, 0.0, "a ring, not the widget's border");
    assert_eq!(
        shape.fill,
        Tokens::for_theme(d.ctx().theme()).popover,
        "bg-popover"
    );
    let tokens = Tokens::for_theme(Theme::Light);
    // The plain driver: a prepared one sets the context's style each frame.
    let mut d = support::driver(
        vec2(500.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(support::shadcn_with(|style| {
        style
            .sheet
            .as_app()
            .rule(hover_card::Part::Content, |t, _| t.bg(egui::Color32::RED));
    }))
    .build();
    still(d.ctx(), d.app());
    d.frames(4);
    let card = open(&mut d);
    assert_eq!(support::box_at(&d, card).fill, egui::Color32::RED);
}
