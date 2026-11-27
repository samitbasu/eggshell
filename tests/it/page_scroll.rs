//! The examples' shell lays every view on a page scroll (#486): a view taller
//! than the window scrolls to its bottom, and one that fits keeps its layout,
//! narrower by the scroll gutter.

use eggshell_examples::common;
use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, tw};
use egui::{Event, Modifiers, MouseWheelUnit, Rect, Theme, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const PHONE: Vec2 = vec2(390.0, 400.0);
const ROWS: usize = 40;
const INSET: f32 = 16.0;

fn rows(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(tw().flex().flex_col().gap(2.0).p(4.0), |b| {
        for row in 0..ROWS {
            b.button(("row", row), format!("Row {row}")).add();
        }
    });
}

/// A view that fills the window: a button at the top, one as wide as the
/// window and one pinned to the bottom edge.
fn fills(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(
        tw().flex()
            .flex_col()
            .justify_between()
            .h_full()
            .p_px(INSET),
        |b| {
            b.button("top", "Top").add();
            b.button("wide", "Wide").classes(tw().w_full()).add();
            b.button("bottom", "Bottom").add();
        },
    );
}

fn rect(harness: &Harness<'_>, label: &str) -> Rect {
    harness.get_by_label(label).rect()
}

fn wheel(harness: &mut Harness<'_>, at: egui::Pos2, dy: f32) {
    harness.event(Event::PointerMoved(at));
    harness.run_steps(2);
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -dy),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    harness.run_steps(60);
}

#[test]
fn a_view_taller_than_the_window_scrolls_to_its_bottom() {
    let mut harness = Harness::builder()
        .with_size(PHONE)
        .build_ui(common::app_frame((), |_, _| {}, rows));
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(4);
    let last = format!("Row {}", ROWS - 1);
    assert!(
        rect(&harness, &last).max.y > PHONE.y,
        "precondition: the last row starts below the window"
    );
    let first = rect(&harness, "Row 0");
    wheel(&mut harness, first.center(), 10_000.0);
    let scrolled = rect(&harness, &last);
    assert!(
        scrolled.max.y <= PHONE.y && scrolled.max.y > PHONE.y - 40.0,
        "the last row is at the bottom of the window: {scrolled:?}"
    );
    assert!(
        rect(&harness, "Row 0").max.y < first.min.y,
        "the first row scrolled away"
    );
}

#[test]
fn a_view_that_fits_fills_the_window_less_the_gutter() {
    let mut harness = Harness::builder()
        .with_size(PHONE)
        .build_ui(common::unit_frame((), fills));
    harness.ctx.set_theme(Theme::Light);
    harness.run_steps(4);
    let gutter = common::style(&harness.ctx).get(Theme::Light).scroll.width;
    assert!(gutter > 0.0, "precondition: the style reserves a gutter");
    let [top, wide, bottom] = ["Top", "Wide", "Bottom"].map(|label| rect(&harness, label));
    assert!(
        top.max.y < wide.min.y && wide.max.y < bottom.min.y,
        "precondition: the three buttons are laid out in a column"
    );

    // The harness's own panel may inset the window, so the insets are measured
    // from the left one.
    let inset = top.min.x;
    assert!(
        inset >= INSET,
        "precondition: the view's padding is inside the window"
    );
    assert!(
        (PHONE.y - bottom.max.y - inset).abs() < 0.5,
        "the view fills the window's height: {bottom:?}"
    );
    assert!(
        (PHONE.x - wide.max.x - inset - gutter).abs() < 0.5,
        "the view is as wide as the window less the gutter: {wide:?}"
    );

    wheel(&mut harness, wide.center(), 1_000.0);
    assert_eq!(
        rect(&harness, "Bottom"),
        bottom,
        "a page that fits does not scroll"
    );
}
