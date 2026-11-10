//! `.children(..)` on buttons and toggles: nodes built inside the button,
//! after its face, through the real layout and draw path.

use bullseye::contract::Disabled;
use bullseye::owned::{Enablement, Face};
use bullseye::{Builder, Opacity, TextStyle, tw};
use bullseye_shadcn::{Shadcn, Tokens};
use egui::{Color32, Rect, Theme, pos2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, box_at, click_node, press, rect_of, shapes, text_color};

const BAR: Color32 = Color32::from_rgb(220, 20, 20);

#[derive(Default)]
struct App {
    clicks: Vec<&'static str>,
    toggled: bool,
    faded: bool,
}

fn bar() -> bullseye::Tw {
    tw().absolute()
        .inset_x(0.0)
        .mx_auto()
        .bottom(0.5)
        .w_px(16.0)
        .h_px(4.0)
        .rounded_full()
        .bg(BAR)
}

/// ```html
/// <div class="flex items-start gap-4 p-4">
///   <button>Save <span class="absolute inset-x-0 bottom-0.5 mx-auto w-4 h-1 rounded-full bg-red"/></button>
///   <button>Flow <span class="w-6 h-2"/></button>
///   <button disabled>Off <span class="absolute …"/></button>
///   <button aria-pressed>Tog <span class="absolute …"/></button>
///   <button aria-label="Card"><span class="w-8 h-5"/></button>
///   <button>Other <span class="absolute …"/></button>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().items_start().gap(4.0).p(4.0), |b| {
        b.button("abs", "Save")
            .children(|s| {
                s.div_id("bar", bar(), |_| {});
            })
            .on_click(|app: &mut App| app.clicks.push("abs"));
        b.button("flow", "Flow")
            .children(|s| {
                s.div_id("tag", tw().w_px(24.0).h_px(8.0).bg(BAR), |_| {});
            })
            .add();
        b.button("off", "Off")
            .enablement(Enablement::Disabled)
            .children(|s| {
                s.div_id("bar", bar(), |_| {});
            })
            .add();
        b.toggle("tog", "Tog", app.toggled)
            .children(|s| {
                s.div_id("bar", bar(), |_| {});
            })
            .on_change(|app: &mut App, on| app.toggled = on);
        b.button("card", Face::Empty)
            .named("Card")
            .children(|s| {
                s.div_id("box", tw().w_px(32.0).h_px(20.0).bg(BAR), |_| {});
            })
            .on_click(|app: &mut App| app.clicks.push("card"));
        b.button("other", "Other")
            .children(|s| {
                s.div_id("bar", bar(), |_| {});
            })
            .add();
    });
}

fn harness(faded: bool) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let app = App {
        faded,
        ..App::default()
    };
    let mut harness = support::harness_prepared(
        vec2(500.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
        |ctx, app| {
            let mut style = Shadcn::for_theme(Theme::Light).style();
            if app.faded {
                style.disabled = Disabled::Faded(Opacity::new(0.4));
            }
            style.install(ctx);
        },
    );
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(12);
    harness
}

fn label_rect(harness: &Harness<'_, Rig<App>>, inside: Rect, text: &str) -> Rect {
    shapes(harness)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(t) if t.galley.text() == text && inside.contains(t.pos) => Some(
                Rect::from_min_size(t.pos + t.galley.rect.min.to_vec2(), t.galley.rect.size()),
            ),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text {text:?} in {inside:?}"))
}

#[test]
fn an_absolute_child_is_placed_against_the_buttons_border_box() {
    let harness = harness(false);
    let button = rect_of(&harness, "/\"abs\"");
    let bar = rect_of(&harness, "/\"abs\"/\"bar\"");
    let style = Shadcn::for_theme(Theme::Light).style();
    assert_eq!(
        style
            .button
            .variants
            .get(style.button.variants.default_key())
            .border_width,
        0.0,
        "precondition: no border, so border and padding boxes agree"
    );
    let expected = Rect::from_min_size(
        pos2(button.center().x - 8.0, button.bottom() - 2.0 - 4.0),
        vec2(16.0, 4.0),
    );
    assert_eq!(bar, expected);
}

#[test]
fn a_flow_child_follows_the_label_and_the_button_grows_to_fit_it() {
    let harness = harness(false);
    let button = rect_of(&harness, "/\"flow\"");
    let tag = rect_of(&harness, "/\"flow\"/\"tag\"");
    let label = label_rect(&harness, button, "Flow");
    assert!(tag.left() >= label.right(), "{tag:?} after {label:?}");
    assert!(button.contains_rect(tag), "{button:?} holds {tag:?}");
    let alone = rect_of(&harness, "/\"other\"");
    let alone_label = label_rect(&harness, alone, "Other");
    assert!(
        button.width() > label.width() + tag.width() - 0.01,
        "the button is wider than label and tag together (alone: {} for {})",
        alone.width(),
        alone_label.width()
    );
}

#[test]
fn children_move_with_the_press_shift() {
    let mut harness = harness(false);
    let bar = rect_of(&harness, "/\"abs\"/\"bar\"");
    assert_eq!(
        box_at(&harness, bar).fill,
        BAR,
        "precondition: drawn at rest"
    );
    press(&mut harness, "/\"abs\"");
    assert_eq!(
        box_at(&harness, bar.translate(vec2(0.0, 1.0))).fill,
        BAR,
        "the child is painted one point lower"
    );
    assert_eq!(rect_of(&harness, "/\"abs\"/\"bar\""), bar, "layout stays");
}

#[test]
fn a_disabled_buttons_children_fade_with_it_under_faded() {
    let harness = harness(true);
    let bar = rect_of(&harness, "/\"off\"/\"bar\"");
    let enabled = rect_of(&harness, "/\"abs\"/\"bar\"");
    assert_eq!(box_at(&harness, enabled).fill, BAR, "precondition");
    let faded = box_at(&harness, bar).fill;
    assert_eq!(faded, BAR.gamma_multiply(0.4));
    let off = rect_of(&harness, "/\"off\"");
    assert_eq!(
        text_color(&harness, off, "Off").a(),
        text_color(&harness, rect_of(&harness, "/\"abs\""), "Save")
            .gamma_multiply(0.4)
            .a(),
        "the face fades by the same factor"
    );
}

#[test]
fn a_child_is_clipped_where_the_button_is() {
    fn clipped(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div(
            tw().w_px(60.0)
                .h_px(60.0)
                .overflow_hidden()
                .flex()
                .items_start(),
            |b| {
                b.button("abs", "Save")
                    .children(|s| {
                        s.div_id(
                            "bar",
                            tw().absolute()
                                .left_px(0.0)
                                .top_px(0.0)
                                .w_px(200.0)
                                .h_px(10.0)
                                .bg(BAR),
                            |_| {},
                        );
                    })
                    .add();
            },
        );
    }
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(300.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        clipped,
    );
    harness.run_steps(12);
    let parent = rect_of(&harness, "/<div>");
    let bar = rect_of(&harness, "/\"abs\"/\"bar\"");
    assert!(
        bar.right() > parent.right(),
        "precondition: the bar overflows"
    );
    let clip = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| {
            let mut all = Vec::new();
            support::flatten(&clipped.shape, &mut all);
            all.iter()
                .any(|shape| matches!(shape, egui::Shape::Rect(r) if r.fill == BAR))
                .then_some(clipped.clip_rect)
        })
        .expect("the bar is painted");
    assert!(
        clip.right() <= parent.right() + 0.01,
        "clip {clip:?} within {parent:?}"
    );
}

#[test]
fn a_click_on_an_absolute_childs_area_clicks_the_button() {
    let mut harness = harness(false);
    click_node(&mut harness, "/\"abs\"/\"bar\"");
    assert_eq!(harness.state().app.clicks, ["abs"]);
}

#[test]
fn keyed_children_of_two_buttons_do_not_collide() {
    let harness = harness(false);
    let first = rect_of(&harness, "/\"abs\"/\"bar\"");
    let second = rect_of(&harness, "/\"other\"/\"bar\"");
    assert_ne!(first, second);
}

#[test]
fn a_toggle_with_children_draws_them_and_toggles() {
    let mut harness = harness(false);
    let bar = rect_of(&harness, "/\"tog\"/\"bar\"");
    assert_eq!(box_at(&harness, bar).fill, BAR);
    click_node(&mut harness, "/\"tog\"/\"bar\"");
    harness.run_steps(2);
    assert!(harness.state().app.toggled);
}

#[test]
fn an_empty_face_with_children_and_a_name_draws_and_clicks() {
    let mut harness = harness(false);
    let card = rect_of(&harness, "/\"card\"");
    let content = rect_of(&harness, "/\"card\"/\"box\"");
    assert_eq!(box_at(&harness, content).fill, BAR);
    assert!(
        card.width() >= content.width(),
        "{card:?} holds {content:?}"
    );
    assert!(card.contains_rect(content));
    click_node(&mut harness, "/\"card\"/\"box\"");
    assert_eq!(harness.state().app.clicks, ["card"]);
}

#[test]
#[should_panic(expected = "no label and no name")]
fn an_empty_face_without_a_name_is_a_bug_in_a_debug_build() {
    fn unnamed(b: &mut Builder<'_, '_, App>, _: &App) {
        b.button("x", Face::Empty).children(|_| {}).add();
    }
    let tokens = Tokens::for_theme(Theme::Light);
    let _ = support::harness(
        vec2(100.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        unnamed,
    );
}
