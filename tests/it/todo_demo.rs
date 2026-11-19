//! The todo example through the full path: pointer events, hit testing,
//! group-hover reveal, reordering by data key.

use eggshell_ui::testing::{Driver, Target, Timeline, record};
use egui::epaint::Shape;
use egui::vec2;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/todo.rs"]
mod todo;

use crate::support;

use support::fill_of;
use todo::{ItemKey, State};

fn harness() -> Driver<State> {
    support::driver(
        vec2(520.0, 320.0),
        common::base_text(egui::Theme::Light),
        State::default(),
        todo::layout,
    )
    .build()
}

fn keys(d: &Driver<State>) -> Vec<ItemKey> {
    d.app().items.iter().map(|i| i.key).collect()
}

fn row_path(key: ItemKey) -> String {
    format!("/{key:?}")
}

fn button_path(key: ItemKey, button: &str) -> String {
    format!("/{key:?}/{button:?}")
}

fn click_button(d: &mut Driver<State>, key: ItemKey, button: &str) {
    let at = d.rect(Target::suffix(&button_path(key, button))).center();
    press_and_release(d, at);
}

/// A primary press and release at `at`, a frame each, with no hover first.
fn press_and_release(d: &mut Driver<State>, at: egui::Pos2) {
    for pressed in [true, false] {
        d.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

fn item(n: usize) -> ItemKey {
    State::default().items[n].key
}

fn remove_opacity(d: &Driver<State>) -> Option<f32> {
    d.harness()
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Remove" => Some(text.opacity_factor),
            _ => None,
        })
}

#[test]
fn paths_are_what_the_tests_expect() {
    let d = harness();
    let first = item(0);
    assert_eq!(d.rect(Target::suffix(&row_path(first))).height(), 40.0);
    assert!(d.rect(Target::suffix(&button_path(first, "down"))).width() > 0.0);
}

#[test]
fn down_moves_a_row_and_up_moves_it_back() {
    let mut d = harness();
    record(d.ctx());
    let [first, second, ..] = keys(&d)[..] else {
        panic!("precondition: at least two items")
    };
    click_button(&mut d, first, "down");
    d.frames(2);
    assert_eq!(&keys(&d)[..2], [second, first]);

    click_button(&mut d, first, "up");
    d.frames(2);
    assert_eq!(&keys(&d)[..2], [first, second]);

    d.frames(6);
    Timeline::recorded(d.ctx()).assert_continuous();
}

#[test]
fn a_button_that_cannot_move_its_row_is_inert_and_does_not_highlight() {
    let mut d = harness();
    let lit_by_hover = |d: &mut Driver<State>, key: ItemKey| {
        let at = d.rect(Target::suffix(&button_path(key, "up"))).center();
        d.hover(at).frames(10);
        fill_of(d, &button_path(key, "up"))
    };
    let action_hover = common::ring(&common::tokens(d.ctx()));
    assert_eq!(
        lit_by_hover(&mut d, item(1)),
        Some(action_hover),
        "precondition: a live Up button highlights"
    );
    assert_ne!(lit_by_hover(&mut d, item(0)), Some(action_hover));
}

#[test]
fn remove_is_revealed_by_hovering_its_row_and_removes_it() {
    let mut d = harness();
    let first = item(0);
    assert_eq!(remove_opacity(&d), None, "hidden until hovered");

    let row = d.rect(Target::suffix(&row_path(first)));
    let remove = d.rect(Target::suffix(&button_path(first, "remove")));
    assert!(
        !remove.contains(row.left_center() + vec2(10.0, 0.0)),
        "precondition: the pointer is on the row, not on Remove"
    );
    d.hover(row.left_center() + vec2(10.0, 0.0));
    assert_eq!(remove_opacity(&d), Some(1.0));

    press_and_release(&mut d, remove.center());
    d.frames(2);
    assert!(!keys(&d).contains(&first));
}

#[test]
fn a_hovered_row_keeps_its_highlight_when_it_moves() {
    let mut d = harness();
    let first = item(0);
    let down = d.rect(Target::suffix(&button_path(first, "down"))).center();
    d.hover(down).frames(10);

    let base = fill_of(&d, &row_path(item(1))).map(|c| c.r());
    let lit = fill_of(&d, &row_path(first)).map(|c| c.r());
    assert!(
        lit < base,
        "precondition: the hovered row is darker than the rest: {lit:?} vs {base:?}"
    );

    let old = d.rect(Target::suffix(&row_path(first))).min;
    press_and_release(&mut d, down);
    d.frames(1);
    let painted_there = |at: egui::Pos2| {
        support::shapes(&d)
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Rect(rect) if (rect.rect.min - at).length() < 1.0 => Some(rect.fill.r()),
                _ => None,
            })
            .next_back()
    };
    let moved = painted_there(old);
    assert!(
        d.rect(Target::suffix(&row_path(first))).min.y
            > d.rect(Target::suffix(&row_path(item(1)))).min.y,
        "precondition: the row moved below its neighbour"
    );
    assert_eq!(moved, lit, "the highlight moved with the row");
}

#[test]
fn dragging_the_window_edge_keeps_every_row_continuous() {
    let mut d = harness();
    let widths = (0..=320)
        .map(|k| 240.0 + 3.0 * k as f32)
        .chain((0..=320).rev().map(|k| 240.0 + 3.0 * k as f32));
    record(d.ctx());
    for width in widths {
        d.resize(vec2(width, 320.0));
    }
    d.frames(12);
    let timeline = Timeline::recorded(d.ctx());
    assert!(timeline.len() > 600, "precondition: a long sweep");
    timeline.assert_continuous();
}

/// The fuzzer's minimal repro of #80: a row swap starts sliding and the next
/// frames resize the window.
#[test]
fn a_resize_mid_slide_does_not_snap_the_row() {
    let mut d = harness();
    record(d.ctx());
    let [first, second, ..] = keys(&d)[..] else {
        panic!("precondition: at least two items")
    };
    click_button(&mut d, second, "up");
    assert_eq!(
        &keys(&d)[..2],
        [second, first],
        "precondition: the rows swapped"
    );
    d.frames(1);
    let mut width = 520.0;
    while width > 240.0 {
        width = (width - 12.0_f32).max(240.0);
        d.resize(vec2(width, 320.0));
    }
    d.frames(16);
    Timeline::recorded(d.ctx()).assert_continuous();
}
