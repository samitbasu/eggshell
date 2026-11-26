//! The alert (#372), through the real event path, in the shadcn preset the
//! rig applies.

use std::time::Duration;

use eggshell_ui::owned::alert::Announce;
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::{Tokens, size, variant};
use eggshell_ui::{Builder, Style, TextStyle, tw};
use egui::accesskit::Live;
use egui::{Role, Shape, Theme, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::support;

const DELAY: Duration = Duration::from_millis(500);
const GRACE: Duration = Duration::from_millis(300);

#[derive(Default)]
struct App {
    rearmed: u32,
}

fn timed(style: &mut Style) {
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = DELAY;
    tooltip.grace = GRACE;
    style.tooltip = tooltip;
}

fn harness(view: support::View<App>) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(480.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(timed)))
    .theme(Theme::Light)
    .build();
    support::set_companion(&d, support::Companion::Alone);
    d.frames(12);
    d
}

/// ```html
/// <div data-slot="alert" class="… cn-alert-variant-destructive">
///   <svg/> <div data-slot="alert-title">Input clipped</div>
///   <div data-slot="alert-description">Lower the gain, or re-arm.</div>
///   <div data-slot="alert-action"><button>Re-arm</button></div>
/// </div>
/// ```
fn trip(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().p(4.0).w_px(400.0), |b| {
        b.alert("trip")
            .variant(variant::DESTRUCTIVE)
            .title("Input clipped")
            .description("Lower the gain, or re-arm.")
            .action("Re-arm", |button| {
                button.on_click(|app: &mut App| app.rearmed += 1);
            })
            .add();
    });
}

#[test]
fn the_action_s_click_reaches_the_app() {
    let mut d = harness(trip);
    d.click(Target::suffix("/\"action\"/\"button\""));
    assert_eq!(d.app().rearmed, 1);
    d.harness()
        .get_by_role_and_label(Role::Button, "Re-arm")
        .click();
    d.frames(2);
    assert_eq!(d.app().rearmed, 2, "through assistive technology too");
}

#[test]
fn the_action_is_an_outline_xs_button_unless_its_closure_says_otherwise() {
    fn both(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div(tw().flex().flex_col().gap(4.0).p(4.0).w_px(400.0), |b| {
            b.alert("preset")
                .title("Preset")
                .action("Undo", |button| button.add())
                .add();
            b.alert("own")
                .title("Own")
                .action("Undo", |button| button.size(size::SM).add())
                .add();
            b.button("xs", "Undo")
                .variant(variant::OUTLINE)
                .size(size::XS)
                .add();
        });
    }
    let d = harness(both);
    let height = |path: &str| d.rect(Target::suffix(path)).height();
    let xs = height("/\"xs\"");
    assert_eq!(height("/\"preset\"/\"action\"/\"button\""), xs);
    assert!(
        height("/\"own\"/\"action\"/\"button\"") > xs,
        "the closure's size wins"
    );
}

const CUT: &str = "Input clipped on channel 2: lower the gain, or re-arm the trip.";
const FITS: &str = "Input clipped";

fn truncated(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(
        tw().flex().flex_col().gap(4.0).p(4.0).pt(20.0).w_px(300.0),
        |b| {
            b.alert("cut").title(CUT).truncate().add();
            b.alert("fits").title(FITS).truncate().add();
        },
    );
}

/// For each time `text` is drawn, whether it was cut short.
fn drawn_elided(d: &Driver<App>, text: &str) -> Vec<bool> {
    support::shapes(d)
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text(shape) if shape.galley.text() == text => Some(shape.galley.elided),
            _ => None,
        })
        .collect()
}

fn tooltip_shown(d: &Driver<App>) -> bool {
    support::nodes(d)
        .iter()
        .any(|(path, _)| path.contains("Tooltip("))
}

#[test]
fn a_cut_title_shows_its_whole_text_in_a_tooltip_and_one_that_fits_does_not() {
    let mut d = harness(truncated);
    assert_eq!(
        drawn_elided(&d, CUT),
        [true],
        "precondition: the title is cut"
    );
    assert_eq!(drawn_elided(&d, FITS), [false], "precondition: it fits");

    d.hover(Target::suffix("/\"fits\"/\"title\""));
    d.advance(DELAY + GRACE);
    assert!(!tooltip_shown(&d), "a title that fits has no tooltip");

    d.hover(Target::suffix("/\"cut\"/\"title\""));
    d.advance(DELAY + GRACE);
    assert!(tooltip_shown(&d), "the cut title opens a tooltip");
    assert_eq!(
        drawn_elided(&d, CUT),
        [true, false],
        "the tooltip draws the whole title over the cut one"
    );
    d.harness().get_by_label(CUT);
}

fn announced(b: &mut Builder<'_, '_, App>, _: &App) {
    b.div(tw().flex().flex_col().gap(4.0).p(4.0).w_px(400.0), |b| {
        b.alert("polite")
            .title("Polite")
            .description("Saved.")
            .add();
        b.alert("assertive")
            .title("Assertive")
            .announce(Announce::Assertive)
            .add();
        b.alert("off")
            .title("Heads up")
            .announce(Announce::Off)
            .add();
    });
}

#[test]
fn announce_sets_the_role_and_the_live_region() {
    let d = harness(announced);
    let h = d.harness();
    let status = h.get_by_role(Role::Status);
    assert_eq!(status.accesskit_node().data().live(), Some(Live::Polite));
    assert_eq!(
        status.accesskit_node().description().as_deref(),
        Some("Saved."),
        "the description is the node's accessible description"
    );
    let alert = h.get_by_role(Role::Alert);
    assert_eq!(alert.accesskit_node().data().live(), Some(Live::Assertive));
    let group = h.get_by_role_and_label(Role::Group, "Heads up");
    assert_eq!(group.accesskit_node().data().live(), None);
}

#[test]
fn an_alert_may_have_only_a_description() {
    let d = harness(|b, _| {
        b.div(tw().p(4.0).w_px(400.0), |b| {
            b.alert("note")
                .description("This one has a description only.")
                .add();
        });
    });
    let alert = d.rect(Target::suffix("/\"note\""));
    let text = d.rect(Target::suffix("/\"note\"/\"description\""));
    assert!(alert.contains_rect(text));
    assert!(
        support::nodes(&d)
            .iter()
            .all(|(path, _)| !path.ends_with("/\"title\"")),
        "no title node"
    );
}
