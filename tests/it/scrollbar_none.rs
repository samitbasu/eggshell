//! `Tw::scrollbar_none`, Tailwind's `no-scrollbar`, through build, layout and
//! the wheel: no gutter and no bar, and the container still scrolls.

use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::epaint::Shape;
use egui::{Color32, Event, FontFamily, Modifiers, MouseWheelUnit, Pos2, Vec2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, rect_of};

const WIDTH: f32 = 200.0;
const HEIGHT: f32 = 100.0;
const CONTENT: f32 = 400.0;

#[derive(Clone, Copy, PartialEq)]
enum Bar {
    Default,
    None,
    /// `scrollbar_none` from a style's sheet, as a part's look.
    FromSheet,
}

/// A part whose look says `no-scrollbar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Lane;

impl eggshell_ui::sheet::Part for Lane {
    const SOURCE: &'static str = "lane.tsx";

    fn slot(self) -> &'static str {
        "lane"
    }
}

struct App {
    bar: Bar,
}

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    let classes = tw().w_px(WIDTH).h_px(HEIGHT);
    let classes = match app.bar {
        Bar::Default => classes,
        Bar::None => classes.scrollbar_none(),
        Bar::FromSheet => b.part_over(
            Lane,
            &[],
            eggshell_ui::sheet::Defaults(tw().scrollbar_none()),
            classes,
        ),
    };
    b.scroll("list", classes).children(|s| {
        s.div_id(
            "row",
            tw().w_full().h_px(CONTENT).bg(Color32::LIGHT_BLUE),
            |_| {},
        );
    });
}

type H = Harness<'static, Rig<App>>;

fn harness(bar: Bar) -> H {
    let mut harness = support::harness(vec2(300.0, 300.0), base(), App { bar }, view);
    harness.run_steps(4);
    harness
}

fn offset(h: &H) -> Vec2 {
    let id = support::id_of(h, "/\"list\"");
    support::shell(h)
        .viewport(id)
        .map_or(Vec2::NAN, |viewport| viewport.offset())
}

fn wheel(h: &mut H, delta: Vec2) {
    let at: Pos2 = rect_of(h, "/\"list\"").center();
    h.hover_at(at);
    h.step();
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta,
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.run_steps(12);
}

#[test]
fn a_container_reserves_a_gutter_unless_it_has_no_scrollbar() {
    let with = rect_of(&harness(Bar::Default), "/\"list\"/\"row\"").width();
    let without = rect_of(&harness(Bar::None), "/\"list\"/\"row\"").width();
    assert!(
        with < WIDTH,
        "precondition: the default reserves a gutter: {with}"
    );
    assert_eq!(
        without, WIDTH,
        "no gutter: the content is as wide as the box"
    );
}

/// A sheet's `no-scrollbar` is the container's, as its own is: the sheet is
/// applied before the bars are laid out.
#[test]
fn a_container_whose_sheet_says_no_scrollbar_reserves_no_gutter() {
    let from_sheet = rect_of(&harness(Bar::FromSheet), "/\"list\"/\"row\"").width();
    assert_eq!(
        from_sheet, WIDTH,
        "no gutter: the content is as wide as the box"
    );
}

#[test]
fn a_container_with_no_scrollbar_paints_no_bar() {
    let bar_strip = |h: &H| {
        let list = rect_of(h, "/\"list\"");
        support::shapes(h)
            .into_iter()
            .filter(|shape| match shape {
                Shape::Rect(rect) => {
                    rect.rect.right() > list.right() - 12.0
                        && rect.rect.right() <= list.right() + 0.5
                        && rect.rect.height() < list.height()
                        && rect.rect.width() < 12.0
                }
                _ => false,
            })
            .count()
    };
    assert!(
        bar_strip(&harness(Bar::Default)) > 0,
        "precondition: the default paints a bar in the gutter"
    );
    assert_eq!(bar_strip(&harness(Bar::None)), 0);
}

#[test]
fn it_still_scrolls_by_the_wheel() {
    let mut harness = harness(Bar::None);
    assert_eq!(offset(&harness).y, 0.0, "precondition: at the top");
    wheel(&mut harness, vec2(0.0, -60.0));
    assert!(
        offset(&harness).y > 30.0,
        "scrolled: {:?}",
        offset(&harness)
    );
}

#[test]
fn it_still_scrolls_into_view() {
    let mut harness = harness(Bar::None);
    let id = support::id_of(&harness, "/\"list\"");
    let row = support::id_of(&harness, "/\"list\"/\"row\"");
    support::shell(&harness).scroll_by(id, vec2(0.0, 150.0));
    harness.run_steps(4);
    let by = offset(&harness).y;
    assert!((by - 150.0).abs() < 1.0, "scroll_by: {by}");
    support::shell(&harness).scroll_into_view(row, Some(egui::Align::Min));
    harness.run_steps(4);
    assert!(
        offset(&harness).y < by,
        "scroll_into_view moved it back to the row's top"
    );
}
