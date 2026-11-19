//! The extras hover card: `b.hover_card(key).trigger(..).content(..)`, on
//! the core `hover_anchored` primitive, with its style and delays.

use core::time::Duration;

use eggshell_ui::{Builder, TextStyle, tw};
use eggshell_ui_extras::hover_card::{HoverCardExt as _, HoverCardStyle};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::{Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support::{self, Rig, hover_over, rect_of};

const STEP: f32 = 0.05;
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

fn harness() -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness_scaled_stepped(
        vec2(500.0, 400.0),
        STEP,
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
        still,
    );
    harness.run_steps(4);
    harness
}

fn wait(harness: &mut Harness<'_, Rig<App>>, time: Duration) {
    harness.run_steps((time.as_secs_f32() / STEP).ceil() as usize);
}

fn shown(harness: &Harness<'_, Rig<App>>, card: &str) -> Option<egui::Rect> {
    let path = format!("/\"{card}\"/\"Card\"/\"Box\"");
    harness
        .state()
        .nodes
        .iter()
        .find(|(candidate, _)| *candidate == path)
        .map(|(_, rect)| *rect)
}

#[test]
fn the_shadcn_preset_installs_shadcns_hover_card() {
    let harness = harness();
    let installed = HoverCardStyle::of(&harness.ctx);
    let preset = Shadcn::of(&harness.ctx);
    assert_eq!(*installed, preset.hover_card());
    assert_eq!(installed.width, 256.0, "w-64");
    assert_eq!(installed.padding, 16.0, "p-4");
    assert_eq!(installed.radius, preset.tokens.radius, "rounded-lg");
    assert_eq!(
        installed.transition,
        Duration::from_millis(100),
        "duration-100"
    );
    assert_eq!(installed.open_delay, Duration::from_millis(700));
    assert_eq!(installed.close_delay, Duration::from_millis(300));
    let colors = installed.look.paint.states.rest;
    assert_eq!(colors.fill, preset.tokens.popover);
    assert_eq!(colors.text, preset.tokens.popover_foreground);
    assert_eq!(colors.border, preset.tokens.border);
}

#[test]
fn resting_on_the_trigger_opens_the_styled_box_below_it_and_its_button_clicks() {
    let mut harness = harness();
    let at = rect_of(&harness, HANDLE).center();
    hover_over(&mut harness, at);
    wait(&mut harness, Duration::from_millis(500));
    assert!(shown(&harness, "nextjs").is_none(), "waits for 700 ms");
    wait(&mut harness, Duration::from_millis(300));
    let card = shown(&harness, "nextjs").expect("open");
    let trigger = rect_of(&harness, "/\"nextjs\"");
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
    let follow = rect_of(&harness, FOLLOW).center();
    harness.hover_at(follow);
    harness.run_steps(2);
    support::press_and_release(&mut harness, follow);
    harness.run_steps(2);
    assert_eq!(harness.state().app.follows, 1);
    assert!(shown(&harness, "nextjs").is_some(), "the card stays");
}

#[test]
fn a_cards_own_open_delay_overrides_the_style() {
    let mut harness = harness();
    let at = rect_of(&harness, QUICK).center();
    hover_over(&mut harness, at);
    wait(&mut harness, Duration::from_millis(200));
    assert!(shown(&harness, "quick").is_some());
}

#[test]
fn the_cards_text_describes_the_trigger() {
    let harness = harness();
    let node = harness.get_by_label("@nextjs");
    assert_eq!(
        node.accesskit_node().description().as_deref(),
        Some("@nextjs 0 follows Follow")
    );
}

#[test]
fn hovering_a_trigger_with_nothing_focusable_in_it_opens_the_card() {
    for (key, text) in [("link", "A link's card"), ("avatar", "An avatar's card")] {
        let mut harness = harness();
        let trigger = rect_of(&harness, &format!("/\"{key}\""));
        assert!(
            trigger.width() > 0.0 && trigger.height() > 0.0,
            "precondition: {key} is laid out"
        );
        hover_over(&mut harness, trigger.center());
        wait(&mut harness, Duration::from_millis(800));
        assert!(shown(&harness, key).is_some(), "{key}: the card opens");
        harness.get_by_label(text);
    }
}
