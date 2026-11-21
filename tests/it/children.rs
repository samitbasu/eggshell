//! `.children(..)` on buttons and toggles: nodes built inside the button,
//! after its face, through the real layout and draw path.

use std::sync::Arc;

use eggshell_ui::contract::{Disabled, Themed};
use eggshell_ui::owned::{Enablement, Face};
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::{Builder, Opacity, TextStyle, tw};
use eggshell_ui_shadcn::{Shadcn, Tokens};
use egui::{Color32, Rect, Theme, pos2, vec2};

use crate::support::{self, box_at, shapes, text_color};

const BAR: Color32 = Color32::from_rgb(220, 20, 20);

#[derive(Default)]
struct App {
    clicks: Vec<&'static str>,
    toggled: bool,
}

fn bar() -> eggshell_ui::Tw {
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
        b.button("card", Face::Empty("Card".into()))
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

fn driver(faded: bool) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut style = Shadcn::for_theme(Theme::Light).style();
    if faded {
        style.set_disabled(Disabled::Faded(Opacity::new(0.4)));
    }
    let mut d = support::driver(
        vec2(500.0, 200.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        view,
    )
    .style(Themed::both(Arc::new(style)))
    .build();
    d.ctx().set_theme(Theme::Light);
    d.frames(12);
    d
}

fn label_rect(d: &Driver<App>, inside: Rect, text: &str) -> Rect {
    shapes(d)
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
    let d = driver(false);
    let button = d.rect(Target::suffix("/\"abs\""));
    let bar = d.rect(Target::suffix("/\"abs\"/\"bar\""));
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
    let d = driver(false);
    let button = d.rect(Target::suffix("/\"flow\""));
    let tag = d.rect(Target::suffix("/\"flow\"/\"tag\""));
    let label = label_rect(&d, button, "Flow");
    assert!(tag.left() >= label.right(), "{tag:?} after {label:?}");
    assert!(button.contains_rect(tag), "{button:?} holds {tag:?}");
    let alone = d.rect(Target::suffix("/\"other\""));
    let alone_label = label_rect(&d, alone, "Other");
    assert!(
        button.width() > label.width() + tag.width() - 0.01,
        "the button is wider than label and tag together (alone: {} for {})",
        alone.width(),
        alone_label.width()
    );
}

#[test]
fn children_move_with_the_press_shift() {
    let mut d = driver(false);
    let bar = d.rect(Target::suffix("/\"abs\"/\"bar\""));
    assert_eq!(box_at(&d, bar).fill, BAR, "precondition: drawn at rest");
    d.pointer_down(Target::suffix("/\"abs\"")).frames(11);
    assert_eq!(
        box_at(&d, bar.translate(vec2(0.0, 1.0))).fill,
        BAR,
        "the child is painted one point lower"
    );
    assert_eq!(
        d.rect(Target::suffix("/\"abs\"/\"bar\"")),
        bar,
        "layout stays"
    );
}

#[test]
fn a_disabled_buttons_children_fade_with_it_under_faded() {
    let d = driver(true);
    let bar = d.rect(Target::suffix("/\"off\"/\"bar\""));
    let enabled = d.rect(Target::suffix("/\"abs\"/\"bar\""));
    assert_eq!(box_at(&d, enabled).fill, BAR, "precondition");
    let faded = box_at(&d, bar).fill;
    assert_eq!(faded, BAR.gamma_multiply(0.4));
    let off = d.rect(Target::suffix("/\"off\""));
    assert_eq!(
        text_color(&d, off, "Off").a(),
        text_color(&d, d.rect(Target::suffix("/\"abs\"")), "Save")
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
    let mut d = support::driver(
        vec2(300.0, 100.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        App::default(),
        clipped,
    )
    .build();
    d.frames(12);
    let parent = d.rect(Target::suffix("/<div>"));
    let bar = d.rect(Target::suffix("/\"abs\"/\"bar\""));
    assert!(
        bar.right() > parent.right(),
        "precondition: the bar overflows"
    );
    let clip = d
        .harness()
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
    let mut d = driver(false);
    d.click(Target::suffix("/\"abs\"/\"bar\""));
    assert_eq!(d.app().clicks, ["abs"]);
}

#[test]
fn keyed_children_of_two_buttons_do_not_collide() {
    let d = driver(false);
    let first = d.rect(Target::suffix("/\"abs\"/\"bar\""));
    let second = d.rect(Target::suffix("/\"other\"/\"bar\""));
    assert_ne!(first, second);
}

#[test]
fn a_toggle_with_children_draws_them_and_toggles() {
    let mut d = driver(false);
    let bar = d.rect(Target::suffix("/\"tog\"/\"bar\""));
    assert_eq!(box_at(&d, bar).fill, BAR);
    d.click(Target::suffix("/\"tog\"/\"bar\""));
    d.frames(2);
    assert!(d.app().toggled);
}

#[test]
fn an_empty_face_with_children_and_a_name_draws_and_clicks() {
    let mut d = driver(false);
    let card = d.rect(Target::suffix("/\"card\""));
    let content = d.rect(Target::suffix("/\"card\"/\"box\""));
    assert_eq!(box_at(&d, content).fill, BAR);
    assert!(
        card.width() >= content.width(),
        "{card:?} holds {content:?}"
    );
    assert!(card.contains_rect(content));
    d.click(Target::suffix("/\"card\"/\"box\""));
    assert_eq!(d.app().clicks, ["card"]);
}
