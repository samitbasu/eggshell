//! The todo example through the full path: pointer events, hit testing,
//! group-hover reveal, reordering by data key.

#![allow(
    clippy::duplicate_mod,
    reason = "each test crate includes the shared examples/common module"
)]

use bullseye::LayoutState;
use egui::epaint::Shape;
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, Rect, vec2};
use egui_kittest::Harness;

#[path = "../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/todo.rs"]
mod todo;

use todo::{ItemKey, State};

const ACTION_HOVER: u8 = 208;

struct Rig {
    layout: LayoutState,
    app: State,
    nodes: Vec<(String, Rect)>,
}

fn harness() -> Harness<'static, Rig> {
    let rig = Rig {
        layout: LayoutState::new().with_base_text(common::base_text()),
        app: State::default(),
        nodes: Vec::new(),
    };
    Harness::builder()
        .with_size(vec2(520.0, 320.0))
        .with_step_dt(0.05)
        .build_ui_state(
            |ui, rig: &mut Rig| {
                let area = ui.max_rect();
                let Rig {
                    layout, app, nodes, ..
                } = rig;
                let placed = layout
                    .build(ui.ctx(), |b| todo::layout(b, &*app))
                    .layout(area);
                *nodes = placed
                    .nodes()
                    .map(|node| (node.path().to_owned(), node.rect()))
                    .collect();
                placed.draw(app, ui);
            },
            rig,
        )
}

fn rect_of(harness: &Harness<'_, Rig>, path: &str) -> Rect {
    harness
        .state()
        .nodes
        .iter()
        .find(|(candidate, _)| candidate == path)
        .unwrap_or_else(|| panic!("no node {path}: {:?}", harness.state().nodes))
        .1
}

fn keys(harness: &Harness<'_, Rig>) -> Vec<ItemKey> {
    harness.state().app.items.iter().map(|i| i.key).collect()
}

fn row_path(key: ItemKey) -> String {
    format!("/{key:?}")
}

fn button_path(key: ItemKey, button: &str) -> String {
    format!("/{key:?}/{button:?}")
}

fn hover_over(harness: &mut Harness<'_, Rig>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(3);
}

fn press_and_release(harness: &mut Harness<'_, Rig>, at: Pos2) {
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
}

fn click_button(harness: &mut Harness<'_, Rig>, key: ItemKey, button: &str) {
    let at = rect_of(harness, &button_path(key, button)).center();
    press_and_release(harness, at);
}

fn item(n: usize) -> ItemKey {
    State::default().items[n].key
}

fn remove_opacity(harness: &Harness<'_, Rig>) -> Option<f32> {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "Remove" => Some(text.opacity_factor),
            _ => None,
        })
}

fn fill_of(harness: &Harness<'_, Rig>, path: &str) -> Option<Color32> {
    let min = rect_of(harness, path).min;
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if (rect.rect.min - min).length() < 1.0 => Some(rect.fill),
            _ => None,
        })
        .next_back()
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
    let lit_by_hover = |harness: &mut Harness<'_, Rig>, key: ItemKey| {
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
