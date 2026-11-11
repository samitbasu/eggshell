//! The reorder example through the real path: a list that previews its new
//! order live under a drag and commits it on release.

use eggshell_ui::testing::Timeline;
use egui::epaint::Shape;
use egui::{Event, Key, Modifiers, Pos2, Rect, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_reorder.rs"]
mod reorder;

use crate::support::{self, FrameRig};
use reorder::{Image, State};

type H<'a> = Harness<'a, FrameRig<State>>;

fn harness(image: Image) -> H<'static> {
    let state = State {
        image,
        ..State::default()
    };
    let mut h = support::frame_harness(
        vec2(520.0, 320.0),
        common::base_text(egui::Theme::Light),
        state,
        reorder::frame,
    );
    h.run_steps(4);
    h
}

fn slot_path(key: u32) -> String {
    format!("{}/ItemKey({key})", reorder::LIST)
}

fn row_path(key: u32) -> String {
    format!("{}/\"row\"", slot_path(key))
}

fn row_rect(h: &H<'_>, key: u32) -> Rect {
    support::layout_rect(&h.ctx, &row_path(key))
}

/// The rows in the order layout puts them, top to bottom.
fn layout_order(h: &H<'_>) -> Vec<u32> {
    let mut rows: Vec<(u32, f32)> = (0..4)
        .map(|key| (key, support::layout_rect(&h.ctx, &slot_path(key)).min.y))
        .collect();
    rows.sort_by(|a, b| a.1.total_cmp(&b.1));
    rows.into_iter().map(|(key, _)| key).collect()
}

fn order(h: &H<'_>) -> Vec<u32> {
    h.state().app.items.iter().map(|i| i.0.0).collect()
}

fn carried(h: &H<'_>) -> bool {
    eggshell_ui::drag_pointer(&h.ctx).is_some()
}

/// Presses on row `n` and moves just far enough to start a drag.
fn lift(h: &mut H<'_>, n: u32) {
    let from = row_rect(h, n).center();
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from + vec2(0.0, 12.0));
    h.run_steps(2);
}

fn move_to_row(h: &mut H<'_>, n: u32) {
    let at = row_rect(h, n).center();
    h.hover_at(at);
    h.run_steps(3);
}

#[test]
fn dragging_a_row_previews_live_and_keeps_the_drag_under_a_new_position() {
    let mut h = harness(Image::Chip);
    lift(&mut h, 0);
    assert!(carried(&h), "the drag started");
    assert_eq!(layout_order(&h), [0, 1, 2, 3]);
    move_to_row(&mut h, 2);
    assert_eq!(
        layout_order(&h),
        [1, 2, 0, 3],
        "row 0 took the place of row 2"
    );
    assert_eq!(order(&h), [0, 1, 2, 3], "the data waits for the drop");
    assert!(carried(&h), "the drag survived the source moving");
    move_to_row(&mut h, 3);
    assert_eq!(layout_order(&h), [1, 2, 3, 0]);
    let at = row_rect(&h, 0).center();
    h.drop_at(at);
    h.run_steps(3);
    assert_eq!(order(&h), [1, 2, 3, 0], "released inside the list commits");
    assert_eq!(layout_order(&h), [1, 2, 3, 0]);
    assert!(!carried(&h));
    assert!(
        h.state().app.status.ends_with("Dropped"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn siblings_slide_while_layout_is_final() {
    let mut h = harness(Image::Chip);
    lift(&mut h, 0);
    let before = row_rect(&h, 1);
    move_to_row(&mut h, 1);
    h.step();
    let after = row_rect(&h, 1);
    assert!(after.min.y < before.min.y, "row 1 moved up in layout");
    let painted = support::last_drawn(&h.ctx, &row_path(1))
        .unwrap()
        .painted_rect;
    assert!(
        painted.min.y > after.min.y + 1.0,
        "row 1 is painted below its final place: {painted:?} vs {after:?}"
    );
}

#[test]
fn escape_restores_the_original_order_with_nothing_to_undo() {
    let mut h = harness(Image::Chip);
    lift(&mut h, 0);
    move_to_row(&mut h, 3);
    assert_eq!(layout_order(&h), [1, 2, 3, 0]);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert_eq!(layout_order(&h), [0, 1, 2, 3]);
    assert_eq!(order(&h), [0, 1, 2, 3]);
    assert!(!carried(&h));
    move_to_row(&mut h, 2);
    assert_eq!(
        layout_order(&h),
        [0, 1, 2, 3],
        "a cancelled drag stays cancelled"
    );
    let at = row_rect(&h, 2).center();
    h.drop_at(at);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
    assert!(
        h.state().app.status.contains("Cancelled(Esc)"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn releasing_outside_the_list_cancels() {
    let mut h = harness(Image::Chip);
    lift(&mut h, 0);
    move_to_row(&mut h, 3);
    assert_eq!(layout_order(&h), [1, 2, 3, 0]);
    let list = support::layout_rect(&h.ctx, reorder::LIST);
    let outside = Pos2::new(list.max.x + 20.0, list.center().y);
    h.hover_at(outside);
    h.run_steps(3);
    assert_eq!(
        layout_order(&h),
        [0, 1, 2, 3],
        "the preview goes when the pointer leaves"
    );
    h.drop_at(outside);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
    assert!(
        h.state().app.status.contains("Cancelled(Outside)"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn losing_window_focus_cancels() {
    let mut h = harness(Image::Chip);
    lift(&mut h, 0);
    move_to_row(&mut h, 3);
    h.event(Event::WindowFocused(false));
    h.run_steps(3);
    assert!(!carried(&h));
    assert_eq!(layout_order(&h), [0, 1, 2, 3]);
    assert!(
        h.state().app.status.contains("FocusLost"),
        "{}",
        h.state().app.status
    );
    let at = row_rect(&h, 0).center();
    h.drop_at(at);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
}

#[test]
fn alt_arrows_move_the_focused_row_without_a_pointer() {
    let mut h = harness(Image::Chip);
    for _ in 0..3 {
        h.key_press(Key::Tab);
        h.run_steps(2);
    }
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 2, 1, 3]);
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    h.run_steps(3);
    assert_eq!(order(&h), [2, 0, 1, 3], "focus follows the row's key");
}

#[test]
fn a_press_on_the_label_starts_the_row_drag() {
    let mut h = harness(Image::Chip);
    let r = row_rect(&h, 1);
    let at = r.min + vec2(20.0, r.height() / 2.0);
    h.hover_at(at);
    h.run_steps(3);
    h.drag_at(at);
    h.step();
    h.hover_at(at + vec2(0.0, 8.0));
    h.run_steps(3);
    assert!(carried(&h));
}

fn shapes_at(h: &H<'_>, text: &str) -> Vec<(Pos2, f32)> {
    support::flatten_all(h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Text(t) if t.galley.text() == text => Some((t.pos, t.opacity_factor)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_live_row_follows_the_pointer_and_leaves_its_slot_outlined() {
    let mut h = harness(Image::Live);
    let ring = common::tokens(&h.ctx).ring;
    lift(&mut h, 0);
    let list = support::layout_rect(&h.ctx, reorder::LIST);
    let at = Pos2::new(list.center().x, row_rect(&h, 3).center().y);
    h.hover_at(at);
    h.run_steps(3);
    let labels = shapes_at(&h, "Lay it out");
    assert_eq!(labels.len(), 1, "the row is painted once, as the image");
    assert!(
        (labels[0].0.y - at.y).abs() < 14.0,
        "at the pointer: {labels:?} vs {at:?}"
    );
    let slot = support::last_drawn(&h.ctx, &slot_path(0))
        .unwrap()
        .painted_rect;
    let outlined = support::flatten_all(&h).into_iter().any(
        |s| matches!(s, Shape::Rect(r) if support::same(r.rect, slot) && r.stroke.color == ring),
    );
    assert!(outlined, "the slot is outlined");
}

#[test]
fn a_dragged_list_is_continuous() {
    for image in [Image::Chip, Image::Live] {
        let mut h = harness(image);
        lift(&mut h, 0);
        for row in [1, 2, 3, 1] {
            move_to_row(&mut h, row);
        }
        let at = row_rect(&h, 0).center();
        h.drop_at(at);
        h.run_steps(20);
        Timeline::recorded(&h.ctx).assert_continuous();
    }
}
