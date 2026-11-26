//! `Tw::invisible` (#129): Tailwind's `invisible`, CSS `visibility: hidden`.
//! The box keeps its place in the layout; nothing of its subtree is painted,
//! hit, focused, tooltipped or put in the AccessKit tree, and a `visible`
//! descendant is shown again.

use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::epaint::Shape;
use egui::{Color32, FontFamily, Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, click_node, rect_of};

/// A fill nothing else in the view paints.
const MARK: Color32 = Color32::from_rgb(0x12, 0x34, 0x56);

#[derive(Default)]
struct App {
    hidden: bool,
    under: u32,
    over: u32,
}

fn text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let middle = if app.hidden { tw().invisible() } else { tw() };
    b.div(tw().flex().flex_col().gap(4.0).p(4.0), |b| {
        b.div_id("row", tw().flex().gap(2.0), |b| {
            b.button("first", "First").add();
            b.div_id("middle", middle.bg(MARK).p(2.0), |b| {
                b.button("hidden", "Hidden").tooltip("Never shown").add();
            });
            b.button("last", "Last").add();
        });
        b.div_id("stack", tw().relative().w_px(120.0).h_px(40.0), |b| {
            b.button("under", "Under")
                .classes(tw().w_full().h_full())
                .on_click(|app: &mut App| app.under += 1);
            b.div_id(
                "cover",
                tw().absolute().inset(0.0).invisible().bg(MARK),
                |b| {
                    b.button("over", "Over")
                        .classes(tw().w_full().h_full())
                        .on_click(|app: &mut App| app.over += 1);
                },
            );
        });
        b.div_id("parent", tw().invisible().bg(MARK).p(2.0), |b| {
            b.span("shown", tw().visible(), "Shown again");
            b.span("gone", tw(), "Gone too");
        });
    });
}

fn harness(hidden: bool) -> Harness<'static, Rig<App>> {
    let mut harness = support::harness(
        vec2(400.0, 300.0),
        text(),
        App {
            hidden,
            ..App::default()
        },
        view,
    );
    harness.run_steps(4);
    harness
}

fn painted_text(harness: &Harness<'_, Rig<App>>, text: &str) -> bool {
    support::shapes(harness).iter().any(|shape| match shape {
        Shape::Text(shape) => shape.galley.text() == text,
        _ => false,
    })
}

fn painted_mark(harness: &Harness<'_, Rig<App>>) -> bool {
    support::shapes(harness).iter().any(|shape| match shape {
        Shape::Rect(rect) => rect.fill == MARK,
        _ => false,
    })
}

#[test]
fn an_invisible_box_keeps_its_place_in_the_layout() {
    let (shown, hidden) = (harness(false), harness(true));
    assert_eq!(
        rect_of(&shown, "/\"row\"/\"middle\""),
        rect_of(&hidden, "/\"row\"/\"middle\""),
        "the same box"
    );
    assert_eq!(
        rect_of(&shown, "/\"row\"/\"last\"").min.x,
        rect_of(&hidden, "/\"row\"/\"last\"").min.x,
        "its neighbour does not move"
    );
}

#[test]
fn nothing_of_an_invisible_subtree_is_painted() {
    let shown = harness(false);
    assert!(
        painted_text(&shown, "Hidden"),
        "precondition: shown, it paints"
    );
    let hidden = harness(true);
    assert!(!painted_text(&hidden, "Hidden"));
    assert!(!painted_text(&hidden, "Over"));
    assert!(!painted_text(&hidden, "Gone too"));
    assert!(
        !painted_mark(&hidden),
        "no box of an invisible node is painted"
    );
}

#[test]
fn an_invisible_subtree_is_not_in_the_accesskit_tree() {
    let shown = harness(false);
    assert!(shown.query_by_label("Hidden").is_some(), "precondition");
    let hidden = harness(true);
    assert!(hidden.query_by_label("Hidden").is_none());
    assert!(hidden.query_by_label("Over").is_none());
}

#[test]
fn tab_skips_an_invisible_subtree() {
    let mut harness = harness(true);
    let first = support::id_of(&harness, "/\"row\"/\"first\"");
    harness.ctx.memory_mut(|memory| memory.request_focus(first));
    harness.run_steps(2);
    assert_eq!(
        harness.ctx.memory(|memory| memory.focused()),
        Some(first),
        "precondition: the first button has the focus"
    );
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    let focused = harness.ctx.memory(|memory| memory.focused());
    assert_eq!(
        focused,
        Some(support::id_of(&harness, "/\"row\"/\"last\"")),
        "from first to last, past the invisible button"
    );
}

#[test]
fn a_press_on_an_invisible_box_reaches_what_is_beneath() {
    let mut harness = harness(true);
    let (under, cover) = (
        rect_of(&harness, "/\"stack\"/\"under\""),
        rect_of(&harness, "/\"stack\"/\"cover\""),
    );
    assert!(
        cover.contains(under.center()),
        "precondition: the invisible box covers the button"
    );
    click_node(&mut harness, "/\"stack\"/\"under\"");
    harness.run_steps(2);
    assert_eq!(harness.state().app.under, 1);
    assert_eq!(harness.state().app.over, 0);
}

#[test]
fn an_invisible_buttons_tooltip_never_shows() {
    let mut harness = harness(true);
    support::hover_node(&mut harness, "/\"row\"/\"middle\"");
    support::rest_for_tooltip(&mut harness);
    assert!(!painted_text(&harness, "Never shown"));
}

#[test]
fn a_visible_descendant_of_an_invisible_box_is_shown() {
    let harness = harness(false);
    assert!(painted_text(&harness, "Shown again"));
    assert!(harness.query_by_label("Shown again").is_some());
    assert!(!painted_text(&harness, "Gone too"));
}
