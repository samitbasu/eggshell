//! The drag-and-drop gallery (2D reorder in a scroll area, outside file
//! drops, auto-scroll) through the real path: pointer events, hit testing,
//! the gesture, the layout and the draw.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eggshell_ui::testing::Timeline;
use eggshell_ui::{Cause, DragEnd};
use egui::epaint::Shape;
use egui::{Event, Key, Modifiers, MouseWheelUnit, Pos2, Rect, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_gallery.rs"]
mod gallery;

use crate::support::{self, FrameRig};
use gallery::{Image, State};

type H<'a> = Harness<'a, FrameRig<State>>;

const SIZE: egui::Vec2 = vec2(520.0, 400.0);
/// The scroll area's viewport: egui's margin inside the window.
const VIEWPORT: Rect = Rect::from_min_max(Pos2::new(0.0, 8.0), Pos2::new(520.0, 392.0));

fn near_the_bottom_edge() -> Pos2 {
    Pos2::new(VIEWPORT.center().x, VIEWPORT.max.y - 4.0)
}

/// A panel's face is inset from its slot by the slot's padding.
const FACE_INSET: f32 = 4.0;

fn harness_with(
    state: State,
    frame: fn(&mut egui::Ui, &mut State, &mut eggshell_ui::LayoutState),
) -> H<'static> {
    let mut h = support::frame_harness(SIZE, common::base_text(egui::Theme::Light), state, frame);
    h.run_steps(4);
    h
}

fn harness(cols: usize) -> H<'static> {
    let mut state = State::default();
    state.cols = cols;
    harness_with(state, gallery::frame)
}

fn slot_path(key: u32) -> String {
    format!("{}/PanelKey({key})", gallery::GRID)
}

fn face_path(key: u32) -> String {
    format!("{}/\"face\"", slot_path(key))
}

/// The panel's slot as layout gave it this frame.
fn rect(h: &H<'_>, key: u32) -> Rect {
    support::layout_rect(&h.ctx, &slot_path(key))
}

/// Where the panel is painted: its face, with its slide.
fn painted(h: &H<'_>, key: u32) -> Rect {
    support::last_drawn(&h.ctx, &face_path(key))
        .expect("the face is drawn")
        .painted_rect
}

fn face_layout(h: &H<'_>, key: u32) -> Rect {
    support::layout_rect(&h.ctx, &face_path(key))
}

/// The panels in the order layout puts them, by reading order.
fn layout_order(h: &H<'_>) -> Vec<u32> {
    let mut keys: Vec<(u32, Rect)> = (0..20)
        .filter_map(|key| {
            support::last_drawn(&h.ctx, &slot_path(key)).map(|n| (key, n.layout_rect))
        })
        .collect();
    keys.sort_by(|a, b| {
        a.1.min
            .y
            .total_cmp(&b.1.min.y)
            .then(a.1.min.x.total_cmp(&b.1.min.x))
    });
    keys.into_iter().map(|(key, _)| key).collect()
}

fn order(h: &H<'_>) -> Vec<u32> {
    h.state().app.panels.iter().map(|p| p.0.0).collect()
}

fn lift(h: &mut H<'_>, key: u32) {
    let from = rect(h, key).center();
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from + vec2(0.0, 12.0));
    h.run_steps(2);
}

fn pointer_to(h: &mut H<'_>, at: Pos2) {
    h.hover_at(at);
    h.run_steps(3);
}

/// The right half of a panel's slot: the pointer has passed its middle.
fn after_middle(h: &H<'_>, key: u32) -> Pos2 {
    let r = rect(h, key);
    r.center() + vec2(r.width() / 4.0, 0.0)
}

fn move_past(h: &mut H<'_>, key: u32) {
    let at = after_middle(h, key);
    pointer_to(h, at);
}

fn carried(h: &H<'_>) -> bool {
    eggshell_ui::drag_pointer(&h.ctx).is_some()
}

#[derive(Debug)]
struct TestFile(PathBuf);

impl egui::DroppedFile for TestFile {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}

fn hover_files(h: &mut H<'_>, names: &[&str]) {
    h.input_mut().hovered_files = names
        .iter()
        .map(|n| egui::HoveredFile {
            path: Some(PathBuf::from(n)),
            mime: String::new(),
        })
        .collect();
}

fn drop_files(h: &mut H<'_>, names: &[&str]) {
    h.input_mut().hovered_files.clear();
    h.input_mut().dropped_files = names
        .iter()
        .map(|n| Arc::new(TestFile(PathBuf::from(n))) as egui::DroppedFileHandle)
        .collect();
}

#[test]
fn the_gallery_wraps_into_square_columns_and_scrolls() {
    let h = harness(3);
    let (a, b) = (rect(&h, 0), rect(&h, 1));
    assert!((a.width() - a.height()).abs() < 1.0, "square: {a:?}");
    assert!((b.min.x - a.max.x).abs() < 1.0, "next to each other");
    assert!(rect(&h, 3).min.y > a.max.y - 1.0, "the fourth wraps");
    assert!(
        rect(&h, 11).max.y > SIZE.y,
        "taller than the viewport: {:?}",
        rect(&h, 11)
    );
}

#[test]
fn reorder_across_a_row_boundary_previews_then_commits_on_release() {
    let mut h = harness(3);
    lift(&mut h, 2);
    assert!(carried(&h), "precondition: the panel is lifted");
    let r4 = rect(&h, 4);
    assert!(
        r4.min.y > rect(&h, 2).max.y - 1.0,
        "precondition: panel 4 is on the next row"
    );
    move_past(&mut h, 4);
    assert_eq!(
        layout_order(&h)[..6],
        [0, 1, 3, 4, 2, 5],
        "after 4, before 5"
    );
    assert_eq!(
        order(&h)[..6],
        [0, 1, 2, 3, 4, 5],
        "the data does not change until the drop"
    );
    let at = rect(&h, 2).center();
    h.drop_at(at);
    h.run_steps(3);
    assert_eq!(order(&h)[..6], [0, 1, 3, 4, 2, 5]);
    assert_eq!(layout_order(&h)[..6], [0, 1, 3, 4, 2, 5]);
    assert!(!carried(&h));
    assert!(
        h.state().app.status.ends_with("Dropped"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn layout_rects_are_final_while_painting_slides() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let old = rect(&h, 1);
    move_past_for_a_frame(&mut h, 4);
    let now = rect(&h, 1);
    assert!(now.min.x < old.min.x, "panel 1 moved left in layout");
    let drawn = painted(&h, 1);
    let face = support::last_drawn(&h.ctx, &face_path(1))
        .unwrap()
        .layout_rect;
    assert!(
        drawn.min.x > face.min.x + 1.0 && drawn.min.x < old.min.x + FACE_INSET + 1.0,
        "panel 1 is painted away from its final place, towards the old one: {drawn:?} vs {face:?}"
    );
    h.run_steps(20);
    assert!(
        (painted(&h, 1).min - face_layout(&h, 1).min).length() < 1.0,
        "and reaches it"
    );
}

fn move_past_for_a_frame(h: &mut H<'_>, key: u32) {
    h.hover_at(after_middle(h, key));
    h.step();
    h.step();
}

#[test]
fn escape_cancels_a_2d_reorder_with_no_undo() {
    let mut h = harness(3);
    lift(&mut h, 2);
    move_past(&mut h, 4);
    assert_ne!(layout_order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    h.key_press(Key::Escape);
    h.run_steps(8);
    assert_eq!(layout_order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    assert_eq!(order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    let far = rect(&h, 8).center();
    pointer_to(&mut h, far);
    assert_eq!(layout_order(&h)[..6], [0, 1, 2, 3, 4, 5], "stays cancelled");
    assert!(
        h.state().app.status.contains("Cancelled(Esc)"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn releasing_outside_the_viewport_cancels() {
    let mut h = harness(3);
    lift(&mut h, 2);
    move_past(&mut h, 4);
    h.drop_at(Pos2::new(SIZE.x + 40.0, SIZE.y / 2.0));
    h.run_steps(8);
    assert_eq!(order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    assert_eq!(layout_order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    assert!(
        h.state().app.status.contains("Cancelled(Outside)"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn losing_window_focus_cancels() {
    let mut h = harness(3);
    lift(&mut h, 2);
    move_past(&mut h, 4);
    h.event(Event::WindowFocused(false));
    h.run_steps(8);
    assert!(!carried(&h));
    assert_eq!(layout_order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    assert!(
        h.state()
            .app
            .status
            .contains(&format!("{:?}", DragEnd::Cancelled(Cause::FocusLost))),
        "{}",
        h.state().app.status
    );
}

#[test]
fn a_panel_removed_mid_drag_cancels_the_drag() {
    let mut h = harness(3);
    lift(&mut h, 2);
    h.state_mut().app.panels.retain(|p| p.0.0 != 2);
    h.run_steps(3);
    assert!(!carried(&h));
    assert!(
        h.state().app.status.contains("SourceGone"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn the_drag_keeps_its_identity_while_the_order_changes_under_it() {
    let mut h = harness(3);
    lift(&mut h, 2);
    for target in [4, 7, 1] {
        move_past(&mut h, target);
        assert!(carried(&h), "still dragging after crossing {target}");
    }
    assert_eq!(
        layout_order(&h).iter().position(|&k| k == 2),
        Some(2),
        "after panel 1 and before 3: a different slot, the same drag"
    );
}

#[test]
fn outside_files_show_a_gap_and_insert_at_the_pointer() {
    let mut h = harness(3);
    let at = after_middle(&h, 4);
    h.hover_at(at);
    hover_files(&mut h, &["pic.png"]);
    h.run_steps(3);
    assert!(
        support::last_drawn(&h.ctx, &format!("{}/\"insert\"", gallery::GRID)).is_some(),
        "a placeholder holds the gap"
    );
    assert!(
        layout_order(&h)[..6] == [0, 1, 2, 3, 4, 5],
        "the panels are in order, the gap after 4"
    );
    assert!(
        rect(&h, 5).min.x > rect(&h, 4).max.x + 1.0 || rect(&h, 5).min.y > rect(&h, 4).max.y - 1.0
    );
    drop_files(&mut h, &["/tmp/pic.png"]);
    h.run_steps(3);
    let app = &h.state().app;
    assert_eq!(app.panels.len(), 13);
    assert_eq!(app.panels[5].1, "pic.png");
    assert!(
        support::last_drawn(&h.ctx, &format!("{}/\"insert\"", gallery::GRID)).is_none(),
        "the gap is gone"
    );
}

#[test]
fn outside_files_of_the_wrong_type_show_nothing_and_are_refused() {
    let mut h = harness(3);
    let at = rect(&h, 4).center();
    h.hover_at(at);
    hover_files(&mut h, &["notes.txt"]);
    h.run_steps(3);
    assert!(support::last_drawn(&h.ctx, &format!("{}/\"insert\"", gallery::GRID)).is_none());
    drop_files(&mut h, &["/tmp/notes.txt"]);
    h.run_steps(3);
    assert_eq!(h.state().app.panels.len(), 12);
    assert!(
        h.state().app.status.starts_with("refused"),
        "{}",
        h.state().app.status
    );
}

#[test]
fn a_drop_with_no_position_appends_and_scrolls_the_new_panel_into_view() {
    let mut h = harness(3);
    assert!(
        rect(&h, 11).max.y > SIZE.y,
        "precondition: the end is below the fold"
    );
    let before = rect(&h, 0).min.y;
    hover_files(&mut h, &["pic.png"]);
    h.run_steps(3);
    assert!(
        support::last_drawn(&h.ctx, &format!("{}/\"insert\"", gallery::GRID)).is_none(),
        "no position, so no gap, and nothing is guessed"
    );
    drop_files(&mut h, &["/tmp/pic.png"]);
    h.run_steps(30);
    let app = &h.state().app;
    assert_eq!(app.panels.len(), 13);
    assert_eq!(app.panels[12].1, "pic.png");
    assert!(app.reveal.is_none());
    assert!(
        rect(&h, 12).min.y < SIZE.y && rect(&h, 12).max.y > 0.0,
        "the new panel is on screen"
    );
    assert!(rect(&h, 0).min.y < before - 20.0, "the view scrolled");
}

#[test]
fn a_drop_at_a_position_that_arrived_with_it_inserts_there() {
    let mut h = harness(3);
    let at = after_middle(&h, 4);
    h.hover_at(at);
    hover_files(&mut h, &["pic.png"]);
    h.step();
    drop_files(&mut h, &["/tmp/pic.png"]);
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(h.state().app.panels[5].1, "pic.png");
}

#[test]
fn dragging_near_the_bottom_edge_scrolls_and_the_gap_follows() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let grid_top = |h: &H<'_>| support::layout_rect(&h.ctx, gallery::GRID).min.y;
    let before = grid_top(&h);
    pointer_to(&mut h, near_the_bottom_edge());
    h.run_steps(10);
    assert!(
        grid_top(&h) < before - 20.0,
        "content scrolled up: {before} -> {}",
        grid_top(&h)
    );
    let slot = layout_order(&h).iter().position(|&k| k == 0).unwrap();
    assert!(slot >= 6, "the panel went to a lower row: {slot}");
}

#[test]
fn outside_drop_on_a_scrolled_away_part_via_auto_scroll() {
    let mut h = harness(3);
    h.hover_at(near_the_bottom_edge());
    hover_files(&mut h, &["far.png"]);
    h.run_steps(30);
    assert!(
        support::last_drawn(&h.ctx, &format!("{}/\"insert\"", gallery::GRID)).is_some(),
        "a placeholder holds the gap"
    );
    drop_files(&mut h, &["/tmp/far.png"]);
    h.run_steps(2);
    let at = h
        .state()
        .app
        .panels
        .iter()
        .position(|p| p.1 == "far.png")
        .unwrap();
    assert!(
        at >= 9,
        "auto-scroll brought the last rows under the pointer: {at}"
    );
}

#[test]
fn the_wheel_scrolls_during_a_drag() {
    let mut h = harness(3);
    lift(&mut h, 0);
    pointer_to(&mut h, Pos2::new(SIZE.x / 2.0, SIZE.y / 2.0));
    let before = rect(&h, 8).min.y;
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -150.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.run_steps(8);
    assert!(
        rect(&h, 8).min.y < before - 20.0,
        "the wheel scrolled while dragging"
    );
    assert!(carried(&h));
}

#[test]
fn scrolling_alone_does_not_slide_panels() {
    let mut h = harness(3);
    pointer_to(&mut h, Pos2::new(SIZE.x / 2.0, SIZE.y / 2.0));
    let before = rect(&h, 4).min.y;
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -60.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.step();
    h.step();
    assert!(rect(&h, 4).min.y < before - 10.0, "it did scroll");
    for key in [0, 4, 7] {
        assert!(
            (painted(&h, key).min
                - support::last_drawn(&h.ctx, &face_path(key))
                    .unwrap()
                    .layout_rect
                    .min)
                .length()
                < 0.5,
            "panel {key} is painted at its layout rect while scrolling"
        );
    }
}

#[test]
fn changing_the_column_count_reflows_with_final_layout_and_a_slide() {
    let mut h = harness(3);
    let old = rect(&h, 5);
    h.state_mut().app.cols = 2;
    h.step();
    let now = rect(&h, 5);
    assert!(now.min != old.min && now.width() > old.width());
    assert!(
        (painted(&h, 5).min
            - support::last_drawn(&h.ctx, &face_path(5))
                .unwrap()
                .layout_rect
                .min)
            .length()
            > 3.0,
        "panel 5 is not yet painted at its final rect"
    );
    h.run_steps(20);
    assert!((painted(&h, 5).min - face_layout(&h, 5).min).length() < 1.0);
}

/// The gallery under a toolbar strip, as `main` lays it out.
fn under_a_toolbar(ui: &mut egui::Ui, app: &mut State, layout: &mut eggshell_ui::LayoutState) {
    egui::Panel::top("toolbar")
        .exact_size(44.0)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(1, 2, 3)))
        .show(ui, |ui| {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, egui::Color32::from_rgb(1, 2, 3));
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ui, |ui| gallery::frame(ui, app, layout));
}

/// The trace painted last, which is the lifted panel's: its shapes are on the
/// layer above every other. With its shape index and clip.
fn lifted_trace(h: &H<'_>) -> (Vec<Pos2>, usize, Rect) {
    let mut last = None;
    for (i, clipped) in h.output().shapes.iter().enumerate() {
        let mut flat = Vec::new();
        support::flatten(&clipped.shape, &mut flat);
        for shape in flat {
            if let Shape::Path(p) = shape
                && p.points.len() == 64
            {
                last = Some((p.points.clone(), i, clipped.clip_rect));
            }
        }
    }
    last.expect("a trace is painted")
}

#[test]
fn the_live_image_keeps_moving_under_the_pointer() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let at = rect(&h, 4).center() + vec2(17.0, 13.0);
    pointer_to(&mut h, at);
    let (first, ..) = lifted_trace(&h);
    let centre = Rect::from_points(&first).center();
    assert!(
        (centre - at).length() < 40.0,
        "the trace is under the pointer: {centre:?}"
    );
    h.step();
    h.step();
    let (second, ..) = lifted_trace(&h);
    assert_ne!(
        first, second,
        "the lifted panel's trace changed between frames"
    );
}

#[test]
fn the_live_image_is_unclipped_and_above_the_toolbar() {
    let mut h = support::frame_harness(
        SIZE,
        common::base_text(egui::Theme::Light),
        State::default(),
        under_a_toolbar,
    );
    h.run_steps(4);
    lift(&mut h, 0);
    let viewport_top = 44.0;
    let at = Pos2::new(SIZE.x / 2.0, 10.0);
    pointer_to(&mut h, at);
    let toolbar = h
        .output()
        .shapes
        .iter()
        .position(
            |s| matches!(&s.shape, Shape::Rect(r) if r.fill == egui::Color32::from_rgb(1, 2, 3)),
        )
        .expect("the toolbar is painted");
    let (trace, index, clip) = lifted_trace(&h);
    assert!(
        index > toolbar,
        "the lifted trace is painted after the toolbar"
    );
    assert!(
        clip.min.y < viewport_top - 1.0,
        "its clip reaches past the viewport: {clip:?}"
    );
    assert!(
        Rect::from_points(&trace).min.y < viewport_top,
        "the trace is drawn over the toolbar strip, not clipped to the viewport"
    );
}

#[test]
fn the_slot_is_outlined_while_the_panel_is_lifted_and_the_image_has_no_outline() {
    let mut h = harness(3);
    let ring = common::tokens(&h.ctx).ring;
    let strokes = |h: &H<'_>| -> Vec<Rect> {
        let mut all = Vec::new();
        for clipped in &h.output().shapes {
            support::flatten(&clipped.shape, &mut all);
        }
        all.into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.stroke.width > 0.0 && r.stroke.color == ring => Some(r.rect),
                _ => None,
            })
            .collect()
    };
    assert!(strokes(&h).is_empty(), "no outline before the lift");
    lift(&mut h, 0);
    let away = rect(&h, 0).center() + vec2(0.0, 40.0);
    pointer_to(&mut h, away);
    let slot = rect(&h, 0);
    assert!(
        strokes(&h)
            .iter()
            .any(|r| support::same(*r, slot.shrink(1.0)) || support::same(*r, slot)),
        "the slot is outlined: {:?} vs {slot:?}",
        strokes(&h)
    );
    assert!(
        (painted(&h, 0).center() - slot.center()).length() < 1.0,
        "and the layout rect is still the slot"
    );
}

#[test]
fn the_chip_image_leaves_the_panel_in_its_slot_dimmed() {
    let mut state = State::default();
    state.image = Image::Chip;
    let mut h = harness_with(state, gallery::frame);
    lift(&mut h, 0);
    let pointer = rect(&h, 4).center();
    pointer_to(&mut h, pointer);
    h.run_steps(20);
    let label = |h: &H<'_>| -> Vec<(Pos2, f32)> {
        support::flatten_all(h)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) if t.galley.text() == "w0" => Some((t.pos, t.opacity_factor)),
                _ => None,
            })
            .collect()
    };
    let labels = label(&h);
    assert_eq!(labels.len(), 2, "the panel's own label and the chip's");
    let (in_slot, chip): (Vec<_>, Vec<_>) = labels
        .into_iter()
        .partition(|(at, _)| (at.x - pointer.x).abs() > 40.0 || (at.y - pointer.y).abs() > 40.0);
    assert!(
        chip.len() == 1 && chip[0].1 == 1.0,
        "the chip follows the pointer, undimmed: {chip:?}"
    );
    assert_eq!(in_slot.len(), 1, "the panel's label stays in its slot");
    let dimmed = common::tokens(&h.ctx).muted.gamma_multiply(0.35);
    assert!(
        painted_fills(&h, dimmed)
            .iter()
            .any(|face| support::same(*face, face_layout(&h, 0))),
        "the face in the slot is dimmed"
    );
    assert!(
        (painted(&h, 0).center() - face_layout(&h, 0).center()).length() < 1.0,
        "and is painted at it"
    );
}

#[test]
fn on_release_the_panel_is_opaque_in_its_slot_with_no_flash() {
    let mut h = harness(3);
    let muted = common::tokens(&h.ctx).muted;
    lift(&mut h, 0);
    move_past(&mut h, 4);
    let at = rect(&h, 0).center();
    h.drop_at(at);
    h.step();
    let slot = rect(&h, 0);
    let faces: Vec<Rect> = painted_fills(&h, muted);
    assert_eq!(
        faces.len(),
        12,
        "every panel face is painted fully opaque, once"
    );
    assert!(
        faces.iter().any(|f| f.contains(slot.center())),
        "the dropped panel's face sits at its slot on the release frame"
    );
}

fn painted_fills(h: &H<'_>, fill: egui::Color32) -> Vec<Rect> {
    support::flatten_all(h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.fill == fill => Some(r.rect),
            _ => None,
        })
        .collect()
}

/// The lifted panel is centred where the pointer puts it on every frame,
/// including the frame its slot moves in the live reorder: its offset comes
/// from this frame's layout, not last frame's slot. Its trace spans its full
/// width, so the trace's horizontal centre is the panel's.
#[test]
fn the_lifted_panel_never_lags_its_slot_when_the_order_changes() {
    let mut h = harness(3);
    lift(&mut h, 2);
    let start = h.ctx.pointer_latest_pos().expect("a pointer");
    let centre_x = |h: &H<'_>| {
        let (trace, ..) = lifted_trace(h);
        (trace[0].x + trace[trace.len() - 1].x) / 2.0
    };
    let grab_x = start.x - centre_x(&h);
    let target = rect(&h, 1).center() - vec2(rect(&h, 1).width() / 4.0, 0.0);
    assert!(
        target.x < start.x - rect(&h, 1).width() / 2.0,
        "precondition: the walk crosses into column 2"
    );
    let mut reordered = false;
    for step in 1..=20 {
        let at = start + (target - start) * (step as f32 / 20.0);
        h.hover_at(at);
        h.step();
        reordered |= layout_order(&h)[1] == 2;
        let drawn = centre_x(&h);
        assert!(
            (drawn - (at.x - grab_x)).abs() < 1.5,
            "step {step}: the lifted panel is centred at x={drawn}, not under the pointer at x={}",
            at.x - grab_x
        );
    }
    assert!(
        reordered,
        "precondition: the walk reordered the panel into column 2"
    );
}

#[test]
fn a_whole_gallery_drag_is_continuous() {
    let mut h = harness(3);
    lift(&mut h, 2);
    for target in [4, 7, 1, 10] {
        move_past(&mut h, target);
    }
    let at = rect(&h, 2).center();
    h.drop_at(at);
    h.run_steps(20);
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn an_escaped_gallery_drag_is_continuous() {
    let mut h = harness(2);
    lift(&mut h, 1);
    for target in [3, 5] {
        move_past(&mut h, target);
    }
    h.key_press(Key::Escape);
    h.run_steps(20);
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn a_resize_under_a_held_panel_keeps_the_drag() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let mut width = SIZE.x;
    while width > 400.0 {
        width -= 12.0;
        h.set_size(vec2(width, SIZE.y));
        h.step();
        assert!(carried(&h), "still dragging at width {width}");
    }
    h.run_steps(20);
    assert_eq!(
        layout_order(&h)[..3],
        [0, 1, 2],
        "the pointer stayed over the first slot"
    );
    assert!(rect(&h, 1).max.x <= width, "the grid follows the window");
    let at = rect(&h, 0).center();
    h.drop_at(at);
    h.run_steps(20);
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn a_gallery_grown_by_a_column_change_while_scrolling_slides_a_reorder() {
    let mut h = harness(3);
    lift(&mut h, 0);
    pointer_to(&mut h, near_the_bottom_edge());
    h.run_steps(4);
    let before = painted(&h, 1);
    move_past(&mut h, 3);
    let after = painted(&h, 1);
    assert!(
        (after.min - before.min).length() < 120.0,
        "a reorder while the view scrolls slides, it does not snap: {before:?} -> {after:?}"
    );
}
