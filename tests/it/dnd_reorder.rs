//! Drag-and-drop spike (#71): the reorder example through the real path.

use egui::{Event, Key, Modifiers, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_reorder.rs"]
mod reorder;

use crate::support::{self, FrameRig};

use reorder::{ItemKey, State};

fn harness() -> Harness<'static, FrameRig<State>> {
    let mut h = support::frame_harness(
        vec2(520.0, 320.0),
        common::base_text(egui::Theme::Light),
        State::default(),
        reorder::frame,
    );
    h.run_steps(2);
    h
}

fn rect_of(h: &Harness<'_, FrameRig<State>>, key: u32) -> egui::Rect {
    h.state().app.rects[&ItemKey(key)]
}

fn hover_over(h: &mut Harness<'_, FrameRig<State>>, at: egui::Pos2) {
    h.hover_at(at);
    h.run_steps(3);
}

fn order(h: &Harness<'_, FrameRig<State>>) -> Vec<u32> {
    h.state().app.items.iter().map(|i| i.0.0).collect()
}

/// Presses on row `n` and moves just far enough to start a drag.
fn lift(h: &mut Harness<'_, FrameRig<State>>, n: u32) {
    let from = rect_of(h, n).center();
    hover_over(h, from);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from + vec2(0.0, 12.0));
    h.run_steps(2);
}

fn move_pointer_to_row(h: &mut Harness<'_, FrameRig<State>>, n: u32) {
    let at = rect_of(h, n).center();
    h.hover_at(at);
    h.run_steps(3);
}

#[test]
fn dragging_a_row_reorders_live_and_keeps_the_drag_under_a_new_position() {
    let mut h = harness();
    lift(&mut h, 0);
    assert!(
        egui::DragAndDrop::has_payload_of_type::<ItemKey>(&h.ctx),
        "the drag started and carries the key"
    );
    assert_eq!(order(&h), [0, 1, 2, 3]);
    move_pointer_to_row(&mut h, 2);
    assert_eq!(order(&h), [1, 2, 0, 3], "row 0 took the place of row 2");
    assert!(
        egui::DragAndDrop::has_payload_of_type::<ItemKey>(&h.ctx),
        "the drag survived the source being rebuilt at another position"
    );
    assert!(h.ctx.dragged_id().is_some());
    move_pointer_to_row(&mut h, 3);
    assert_eq!(order(&h), [1, 2, 3, 0]);
    h.drop_at(rect_of(&h, 0).center());
    h.run_steps(3);
    assert_eq!(order(&h), [1, 2, 3, 0], "released inside the list commits");
    assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
    assert!(h.state().app.gesture.is_none());
}

#[test]
fn siblings_slide_while_layout_is_final() {
    let mut h = harness();
    lift(&mut h, 0);
    let before = rect_of(&h, 1);
    move_pointer_to_row(&mut h, 1);
    h.step();
    let after = rect_of(&h, 1);
    assert!(after.min.y < before.min.y, "row 1 moved up in layout");
    let painted = all_shapes(&h)
        .into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Rect(r) if (r.rect.width() - after.width()).abs() < 1.0 => {
                Some(r.rect.min.y)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        painted.iter().any(|y| (y - after.min.y).abs() > 1.0),
        "something is painted off its final place: {painted:?}"
    );
}

#[test]
fn escape_restores_the_original_order() {
    let mut h = harness();
    lift(&mut h, 0);
    move_pointer_to_row(&mut h, 3);
    assert_eq!(order(&h), [1, 2, 3, 0]);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
    assert!(!egui::DragAndDrop::has_any_payload(&h.ctx));
    // The pointer is still down; moving over rows must not reorder again.
    move_pointer_to_row(&mut h, 2);
    assert_eq!(order(&h), [0, 1, 2, 3], "a cancelled drag stays cancelled");
    h.drop_at(rect_of(&h, 2).center());
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
    assert!(h.state().app.gesture.is_none());
}

#[test]
fn releasing_outside_the_list_cancels() {
    let mut h = harness();
    lift(&mut h, 0);
    move_pointer_to_row(&mut h, 3);
    assert_eq!(order(&h), [1, 2, 3, 0]);
    let list = h.state().app.list_rect;
    let outside = egui::pos2(list.max.x + 20.0, list.center().y);
    h.hover_at(outside);
    h.run_steps(2);
    h.drop_at(outside);
    h.run_steps(3);
    assert_eq!(order(&h), [0, 1, 2, 3]);
}

#[test]
fn window_focus_loss_does_not_end_the_drag_by_itself() {
    // Records what egui does today; see docs/dnd.md "Cancel".
    let mut h = harness();
    lift(&mut h, 0);
    h.event(Event::WindowFocused(false));
    h.run_steps(2);
    let still = egui::DragAndDrop::has_any_payload(&h.ctx);
    let gesture = h.state().app.gesture.is_some();
    println!("after focus loss: payload={still} gesture={gesture}");
    h.drop_at(rect_of(&h, 0).center());
    h.run_steps(3);
    assert!(h.state().app.gesture.is_none());
}

#[test]
fn alt_arrows_move_the_focused_row_without_a_pointer() {
    let mut h = harness();
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
    let mut h = harness();
    let r = rect_of(&h, 1);
    let at = r.min + vec2(20.0, r.height() / 2.0);
    hover_over(&mut h, at);
    h.drag_at(at);
    h.step();
    h.hover_at(at + vec2(0.0, 8.0));
    h.run_steps(3);
    assert!(egui::DragAndDrop::has_payload_of_type::<ItemKey>(&h.ctx));
}

fn all_shapes<'a>(h: &'a Harness<'_, FrameRig<State>>) -> Vec<&'a egui::epaint::Shape> {
    let mut all = Vec::new();
    for clipped in &h.output().shapes {
        support::flatten(&clipped.shape, &mut all);
    }
    all
}
