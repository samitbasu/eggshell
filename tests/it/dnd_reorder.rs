//! The reorder example through the real path: a list that previews its new
//! order live under a drag and commits it on release.

use eggshell_ui::testing::{Driver, Timeline};
use egui::epaint::Shape;
use egui::{Event, Key, KeyboardShortcut, Modifiers, Pos2, Rect, vec2};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_reorder.rs"]
mod reorder;

use crate::support;
use reorder::{Image, State};

fn harness(image: Image) -> Driver<State> {
    let state = State {
        image,
        ..State::default()
    };
    let mut d = support::recorded(support::frame_driver(
        vec2(520.0, 320.0),
        common::base_text(egui::Theme::Light),
        state,
        reorder::frame,
    ));
    d.frames(4);
    d
}

fn slot_path(key: u32) -> String {
    format!("{}/ItemKey({key})", reorder::LIST)
}

fn row_path(key: u32) -> String {
    format!("{}/\"row\"", slot_path(key))
}

fn row_rect(d: &Driver<State>, key: u32) -> Rect {
    support::layout_rect(d.ctx(), &row_path(key))
}

/// The rows in the order layout puts them, top to bottom.
fn layout_order(d: &Driver<State>) -> Vec<u32> {
    let mut rows: Vec<(u32, f32)> = (0..4)
        .map(|key| (key, support::layout_rect(d.ctx(), &slot_path(key)).min.y))
        .collect();
    rows.sort_by(|a, b| a.1.total_cmp(&b.1));
    rows.into_iter().map(|(key, _)| key).collect()
}

fn order(d: &Driver<State>) -> Vec<u32> {
    d.app().items.iter().map(|i| i.0.0).collect()
}

fn carried(d: &Driver<State>) -> bool {
    d.shell().drag_pointer().is_some()
}

/// Presses on row `n` and moves just far enough to start a drag.
fn lift(d: &mut Driver<State>, n: u32) {
    let from = row_rect(d, n).center();
    d.lift(from, vec2(0.0, 12.0)).frames(1);
}

fn move_to_row(d: &mut Driver<State>, n: u32) {
    let at = row_rect(d, n).center();
    d.hover(at);
}

#[test]
fn dragging_a_row_previews_live_and_keeps_the_drag_under_a_new_position() {
    let mut d = harness(Image::Chip);
    lift(&mut d, 0);
    assert!(carried(&d), "the drag started");
    assert_eq!(layout_order(&d), [0, 1, 2, 3]);
    move_to_row(&mut d, 2);
    assert_eq!(
        layout_order(&d),
        [1, 2, 0, 3],
        "row 0 took the place of row 2"
    );
    assert_eq!(order(&d), [0, 1, 2, 3], "the data waits for the drop");
    assert!(carried(&d), "the drag survived the source moving");
    move_to_row(&mut d, 3);
    assert_eq!(layout_order(&d), [1, 2, 3, 0]);
    let at = row_rect(&d, 0).center();
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d), [1, 2, 3, 0], "released inside the list commits");
    assert_eq!(layout_order(&d), [1, 2, 3, 0]);
    assert!(!carried(&d));
    assert!(d.app().status.ends_with("Dropped"), "{}", d.app().status);
}

#[test]
fn siblings_slide_while_layout_is_final() {
    let mut d = harness(Image::Chip);
    lift(&mut d, 0);
    let before = row_rect(&d, 1);
    move_to_row(&mut d, 1);
    d.frames(1);
    let after = row_rect(&d, 1);
    assert!(after.min.y < before.min.y, "row 1 moved up in layout");
    let painted = support::last_drawn(d.ctx(), &row_path(1))
        .unwrap()
        .painted_rect;
    assert!(
        painted.min.y > after.min.y + 1.0,
        "row 1 is painted below its final place: {painted:?} vs {after:?}"
    );
}

#[test]
fn escape_restores_the_original_order_with_nothing_to_undo() {
    let mut d = harness(Image::Chip);
    lift(&mut d, 0);
    move_to_row(&mut d, 3);
    assert_eq!(layout_order(&d), [1, 2, 3, 0]);
    d.press(Key::Escape).frames(2);
    assert_eq!(layout_order(&d), [0, 1, 2, 3]);
    assert_eq!(order(&d), [0, 1, 2, 3]);
    assert!(!carried(&d));
    move_to_row(&mut d, 2);
    assert_eq!(
        layout_order(&d),
        [0, 1, 2, 3],
        "a cancelled drag stays cancelled"
    );
    let at = row_rect(&d, 2).center();
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d), [0, 1, 2, 3]);
    assert!(
        d.app().status.contains("Cancelled(Esc)"),
        "{}",
        d.app().status
    );
}

#[test]
fn releasing_outside_the_list_cancels() {
    let mut d = harness(Image::Chip);
    lift(&mut d, 0);
    move_to_row(&mut d, 3);
    assert_eq!(layout_order(&d), [1, 2, 3, 0]);
    let list = support::layout_rect(d.ctx(), reorder::LIST);
    let outside = Pos2::new(list.max.x + 20.0, list.center().y);
    d.hover(outside);
    assert_eq!(
        layout_order(&d),
        [0, 1, 2, 3],
        "the preview goes when the pointer leaves"
    );
    d.pointer_up(outside).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d), [0, 1, 2, 3]);
    assert!(
        d.app().status.contains("Cancelled(Outside)"),
        "{}",
        d.app().status
    );
}

#[test]
fn losing_window_focus_cancels() {
    let mut d = harness(Image::Chip);
    lift(&mut d, 0);
    move_to_row(&mut d, 3);
    d.event(Event::WindowFocused(false)).frames(2);
    assert!(!carried(&d));
    assert_eq!(layout_order(&d), [0, 1, 2, 3]);
    assert!(d.app().status.contains("FocusLost"), "{}", d.app().status);
    let at = row_rect(&d, 0).center();
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d), [0, 1, 2, 3]);
}

#[test]
fn alt_arrows_move_the_focused_row_without_a_pointer() {
    let mut d = harness(Image::Chip);
    support::drive_past_bar(&mut d);
    for _ in 0..3 {
        d.press(Key::Tab).frames(1);
    }
    d.press(KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp))
        .frames(2);
    assert_eq!(order(&d), [0, 2, 1, 3]);
    d.press(KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp))
        .frames(2);
    assert_eq!(order(&d), [2, 0, 1, 3], "focus follows the row's key");
}

#[test]
fn a_press_on_the_label_starts_the_row_drag() {
    let mut d = harness(Image::Chip);
    let r = row_rect(&d, 1);
    let at = r.min + vec2(20.0, r.height() / 2.0);
    d.hover(at);
    d.event(Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    d.hover(at + vec2(0.0, 8.0));
    assert!(carried(&d));
}

fn shapes_at(d: &Driver<State>, text: &str) -> Vec<(Pos2, f32)> {
    support::shapes(d)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Text(t) if t.galley.text() == text => Some((t.pos, t.opacity_factor)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_live_row_follows_the_pointer_and_leaves_its_slot_outlined() {
    let mut d = harness(Image::Live);
    let ring = common::tokens(d.ctx()).ring;
    lift(&mut d, 0);
    let list = support::layout_rect(d.ctx(), reorder::LIST);
    let at = Pos2::new(list.center().x, row_rect(&d, 3).center().y);
    d.hover(at);
    let labels = shapes_at(&d, "Lay it out");
    assert_eq!(labels.len(), 1, "the row is painted once, as the image");
    assert!(
        (labels[0].0.y - at.y).abs() < 14.0,
        "at the pointer: {labels:?} vs {at:?}"
    );
    let slot = support::last_drawn(d.ctx(), &slot_path(0))
        .unwrap()
        .painted_rect;
    let outlined = support::shapes(&d).into_iter().any(
        |s| matches!(s, Shape::Rect(r) if support::same(r.rect, slot) && r.stroke.color == ring),
    );
    assert!(outlined, "the slot is outlined");
}

#[test]
fn a_dragged_list_is_continuous() {
    for image in [Image::Chip, Image::Live] {
        let mut d = harness(image);
        lift(&mut d, 0);
        for row in [1, 2, 3, 1] {
            move_to_row(&mut d, row);
        }
        let at = row_rect(&d, 0).center();
        d.pointer_up(at).event(Event::PointerGone).frames(19);
        Timeline::recorded(d.ctx()).assert_continuous();
    }
}
