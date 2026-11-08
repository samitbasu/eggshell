//! The todo example through the full path: pointer events, hit testing,
//! group-hover reveal, reordering by data key.

#![allow(
    clippy::duplicate_mod,
    reason = "each test crate includes the shared examples/common module"
)]

use egui::epaint::Shape;
use egui::vec2;
use egui_kittest::Harness;

#[path = "../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/todo.rs"]
mod todo;

mod support;

use support::{Rig, fill_of, hover_over, press_and_release, rect_of};
use todo::{ItemKey, State};

const ACTION_HOVER: u8 = 208;

fn harness() -> Harness<'static, Rig<State>> {
    support::harness(
        vec2(520.0, 320.0),
        common::base_text(),
        State::default(),
        todo::layout,
    )
}

fn keys(harness: &Harness<'_, Rig<State>>) -> Vec<ItemKey> {
    harness.state().app.items.iter().map(|i| i.key).collect()
}

fn row_path(key: ItemKey) -> String {
    format!("/{key:?}")
}

fn button_path(key: ItemKey, button: &str) -> String {
    format!("/{key:?}/{button:?}")
}

fn click_button(harness: &mut Harness<'_, Rig<State>>, key: ItemKey, button: &str) {
    let at = rect_of(harness, &button_path(key, button)).center();
    press_and_release(harness, at);
}

fn item(n: usize) -> ItemKey {
    State::default().items[n].key
}

fn remove_opacity(harness: &Harness<'_, Rig<State>>) -> Option<f32> {
    harness
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
    let harness = harness();
    let first = item(0);
    assert_eq!(rect_of(&harness, &row_path(first)).height(), 40.0);
    assert!(rect_of(&harness, &button_path(first, "down")).width() > 0.0);
}

#[test]
fn down_moves_a_row_and_up_moves_it_back() {
    let mut harness = harness();
    let [first, second, ..] = keys(&harness)[..] else {
        panic!("precondition: at least two items")
    };
    click_button(&mut harness, first, "down");
    harness.run_steps(2);
    assert_eq!(&keys(&harness)[..2], [second, first]);

    click_button(&mut harness, first, "up");
    harness.run_steps(2);
    assert_eq!(&keys(&harness)[..2], [first, second]);
}

#[test]
fn a_button_that_cannot_move_its_row_is_inert_and_does_not_highlight() {
    let mut harness = harness();
    let lit_by_hover = |harness: &mut Harness<'_, Rig<State>>, key: ItemKey| {
        let at = rect_of(harness, &button_path(key, "up")).center();
        hover_over(harness, at);
        harness.run_steps(10);
        fill_of(harness, &button_path(key, "up")).map(|c| c.r())
    };
    assert_eq!(
        lit_by_hover(&mut harness, item(1)),
        Some(ACTION_HOVER),
        "precondition: a live Up button highlights"
    );
    assert_ne!(lit_by_hover(&mut harness, item(0)), Some(ACTION_HOVER));
}

#[test]
fn remove_is_revealed_by_hovering_its_row_and_removes_it() {
    let mut harness = harness();
    let first = item(0);
    assert_eq!(remove_opacity(&harness), None, "hidden until hovered");

    let row = rect_of(&harness, &row_path(first));
    let remove = rect_of(&harness, &button_path(first, "remove"));
    assert!(
        !remove.contains(row.left_center() + vec2(10.0, 0.0)),
        "precondition: the pointer is on the row, not on Remove"
    );
    hover_over(&mut harness, row.left_center() + vec2(10.0, 0.0));
    assert_eq!(remove_opacity(&harness), Some(1.0));

    press_and_release(&mut harness, remove.center());
    harness.run_steps(2);
    assert!(!keys(&harness).contains(&first));
}

#[test]
fn a_hovered_row_keeps_its_highlight_when_it_moves() {
    let mut harness = harness();
    let first = item(0);
    let down = rect_of(&harness, &button_path(first, "down")).center();
    hover_over(&mut harness, down);
    harness.run_steps(10);

    let base = fill_of(&harness, &row_path(item(1))).map(|c| c.r());
    let lit = fill_of(&harness, &row_path(first)).map(|c| c.r());
    assert!(
        lit < base,
        "precondition: the hovered row is darker than the rest: {lit:?} vs {base:?}"
    );

    press_and_release(&mut harness, down);
    harness.step();
    let moved = fill_of(&harness, &row_path(first)).map(|c| c.r());
    assert!(
        rect_of(&harness, &row_path(first)).min.y > rect_of(&harness, &row_path(item(1))).min.y,
        "precondition: the row moved below its neighbour"
    );
    assert_eq!(moved, lit, "the highlight moved with the row");
}
