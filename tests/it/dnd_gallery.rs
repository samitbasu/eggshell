//! The drag-and-drop gallery (2D reorder in a scroll area, outside file
//! drops, auto-scroll) through the real path: pointer events, hit testing,
//! the gesture, the layout and the draw.

use std::path::PathBuf;

use eggshell_ui::testing::{Driver, Timeline};
use eggshell_ui::{Cause, DragEnd};
use egui::epaint::Shape;
use egui::{Color32, Event, Key, Modifiers, MouseWheelUnit, Pos2, Rect, vec2};

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_gallery.rs"]
mod gallery;

use crate::support;
use gallery::{Image, Lift, State};

type H = Driver<State>;

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
    frame: fn(
        &mut egui::Ui,
        &mut State,
        &mut eggshell_ui::LayoutState,
    ) -> eggshell_ui::testing::PlacedFrame,
) -> H {
    let mut d = support::recorded(support::frame_driver(
        SIZE,
        common::base_text(egui::Theme::Light),
        state,
        frame,
    ));
    d.frames(4);
    d
}

/// One frame of the window exactly as `main` draws it, toolbar tree first.
/// The toolbar's tree has no animations to keep, so a fresh layout state per
/// frame is fine here.
pub fn window_frame(
    ui: &mut egui::Ui,
    app: &mut State,
    layout: &mut eggshell_ui::LayoutState,
) -> eggshell_ui::testing::PlacedFrame {
    gallery::window(
        ui,
        app,
        layout,
        &mut eggshell_ui::LayoutState::new(&eggshell_ui::SharedStyle::new(support::shadcn())),
    )
}

fn harness(cols: usize) -> H {
    let mut state = State::default();
    state.cols = cols;
    harness_with(state, window_frame)
}

fn slot_path(key: u32) -> String {
    format!("{}/PanelKey({key})", gallery::GRID)
}

fn face_path(key: u32) -> String {
    format!("{}/\"face\"", slot_path(key))
}

/// The panel's slot as layout gave it this frame.
fn rect(d: &H, key: u32) -> Rect {
    support::layout_rect(d.ctx(), &slot_path(key))
}

/// Where the panel is painted: its face, with its slide.
fn painted(d: &H, key: u32) -> Rect {
    support::last_drawn(d.ctx(), &face_path(key))
        .expect("the face is drawn")
        .painted_rect
}

fn face_layout(d: &H, key: u32) -> Rect {
    support::layout_rect(d.ctx(), &face_path(key))
}

/// The panels in the order layout puts them, by reading order.
fn layout_order(d: &H) -> Vec<u32> {
    let mut keys: Vec<(u32, Rect)> = (0..20)
        .filter_map(|key| {
            eggshell_ui::testing::laid_out_rect(d.ctx(), &slot_path(key)).map(|r| (key, r))
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

fn order(d: &H) -> Vec<u32> {
    d.app().panels.iter().map(|p| p.0.0).collect()
}

fn lift(d: &mut H, key: u32) {
    let from = rect(d, key).center();
    d.lift(from, vec2(0.0, 12.0)).frames(1);
}

/// The right half of a panel's slot: the pointer has passed its middle.
fn after_middle(d: &H, key: u32) -> Pos2 {
    let r = rect(d, key);
    r.center() + vec2(r.width() / 4.0, 0.0)
}

fn move_past(d: &mut H, key: u32) {
    let at = after_middle(d, key);
    pointer_to(d, at);
}

/// Rests the pointer on `at`, which may be below the window's edge, where
/// the driver's pointer actions do not go.
fn pointer_to(d: &mut H, at: Pos2) {
    d.harness_mut().hover_at(at);
    d.frames(3);
}

fn carried(d: &H) -> bool {
    d.shell().drag_pointer().is_some()
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn the_gallery_wraps_into_square_columns_and_scrolls() {
    let d = harness(3);
    let (a, b) = (rect(&d, 0), rect(&d, 1));
    assert!((a.width() - a.height()).abs() < 1.0, "square: {a:?}");
    assert!((b.min.x - a.max.x).abs() < 1.0, "next to each other");
    assert!(rect(&d, 3).min.y > a.max.y - 1.0, "the fourth wraps");
    assert!(
        rect(&d, 11).max.y > SIZE.y,
        "taller than the viewport: {:?}",
        rect(&d, 11)
    );
}

#[test]
fn reorder_across_a_row_boundary_previews_then_commits_on_release() {
    let mut d = harness(3);
    lift(&mut d, 2);
    assert!(carried(&d), "precondition: the panel is lifted");
    let r4 = rect(&d, 4);
    assert!(
        r4.min.y > rect(&d, 2).max.y - 1.0,
        "precondition: panel 4 is on the next row"
    );
    move_past(&mut d, 4);
    assert_eq!(
        layout_order(&d)[..6],
        [0, 1, 3, 4, 2, 5],
        "after 4, before 5"
    );
    assert_eq!(
        order(&d)[..6],
        [0, 1, 2, 3, 4, 5],
        "the data does not change until the drop"
    );
    let at = rect(&d, 2).center();
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d)[..6], [0, 1, 3, 4, 2, 5]);
    assert_eq!(layout_order(&d)[..6], [0, 1, 3, 4, 2, 5]);
    assert!(!carried(&d));
    assert!(d.app().status.ends_with("Dropped"), "{}", d.app().status);
}

#[test]
fn layout_rects_are_final_while_painting_slides() {
    let mut d = harness(3);
    lift(&mut d, 0);
    let old = rect(&d, 1);
    move_past_for_a_frame(&mut d, 4);
    let now = rect(&d, 1);
    assert!(now.min.x < old.min.x, "panel 1 moved left in layout");
    let drawn = painted(&d, 1);
    let face = support::last_drawn(d.ctx(), &face_path(1))
        .unwrap()
        .layout_rect;
    assert!(
        drawn.min.x > face.min.x + 1.0 && drawn.min.x < old.min.x + FACE_INSET + 1.0,
        "panel 1 is painted away from its final place, towards the old one: {drawn:?} vs {face:?}"
    );
    d.frames(20);
    assert!(
        (painted(&d, 1).min - face_layout(&d, 1).min).length() < 1.0,
        "and reaches it"
    );
}

fn move_past_for_a_frame(d: &mut H, key: u32) {
    let at = after_middle(d, key);
    d.carry_to(at).frames(1);
}

#[test]
fn escape_cancels_a_2d_reorder_with_no_undo() {
    let mut d = harness(3);
    lift(&mut d, 2);
    move_past(&mut d, 4);
    assert_ne!(layout_order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    d.press(Key::Escape).frames(7);
    assert_eq!(layout_order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    assert_eq!(order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    let far = rect(&d, 8).center();
    pointer_to(&mut d, far);
    assert_eq!(layout_order(&d)[..6], [0, 1, 2, 3, 4, 5], "stays cancelled");
    assert!(
        d.app().status.contains("Cancelled(Esc)"),
        "{}",
        d.app().status
    );
}

#[test]
fn releasing_outside_the_viewport_cancels() {
    let mut d = harness(3);
    lift(&mut d, 2);
    move_past(&mut d, 4);
    d.harness_mut()
        .drop_at(Pos2::new(SIZE.x + 40.0, SIZE.y / 2.0));
    d.frames(8);
    assert_eq!(order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    assert_eq!(layout_order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    assert!(
        d.app().status.contains("Cancelled(Outside)"),
        "{}",
        d.app().status
    );
}

#[test]
fn losing_window_focus_cancels() {
    let mut d = harness(3);
    lift(&mut d, 2);
    move_past(&mut d, 4);
    d.event(Event::WindowFocused(false)).frames(7);
    assert!(!carried(&d));
    assert_eq!(layout_order(&d)[..6], [0, 1, 2, 3, 4, 5]);
    assert!(
        d.app()
            .status
            .contains(&format!("{:?}", DragEnd::Cancelled(Cause::FocusLost))),
        "{}",
        d.app().status
    );
}

#[test]
fn a_panel_removed_mid_drag_cancels_the_drag() {
    let mut d = harness(3);
    lift(&mut d, 2);
    d.app_mut().panels.retain(|p| p.0.0 != 2);
    d.frames(3);
    assert!(!carried(&d));
    assert!(d.app().status.contains("SourceGone"), "{}", d.app().status);
}

#[test]
fn the_drag_keeps_its_identity_while_the_order_changes_under_it() {
    let mut d = harness(3);
    lift(&mut d, 2);
    for target in [4, 7, 1] {
        move_past(&mut d, target);
        assert!(carried(&d), "still dragging after crossing {target}");
    }
    assert_eq!(
        layout_order(&d).iter().position(|&k| k == 2),
        Some(2),
        "after panel 1 and before 3: a different slot, the same drag"
    );
}

#[test]
fn outside_files_show_a_gap_and_insert_at_the_pointer() {
    let mut d = harness(3);
    let at = after_middle(&d, 4);
    d.harness_mut().hover_at(at);
    d.hover_files(&paths(&["pic.png"])).frames(2);
    assert!(
        support::last_drawn(d.ctx(), &format!("{}/\"insert\"", gallery::GRID)).is_some(),
        "a placeholder holds the gap"
    );
    assert!(
        layout_order(&d)[..6] == [0, 1, 2, 3, 4, 5],
        "the panels are in order, the gap after 4"
    );
    assert!(
        rect(&d, 5).min.x > rect(&d, 4).max.x + 1.0 || rect(&d, 5).min.y > rect(&d, 4).max.y - 1.0
    );
    d.drop_files(&paths(&["/tmp/pic.png"])).frames(2);
    let app = &d.app();
    assert_eq!(app.panels.len(), 13);
    assert_eq!(app.panels[5].1, "pic.png");
    assert!(
        support::last_drawn(d.ctx(), &format!("{}/\"insert\"", gallery::GRID)).is_none(),
        "the gap is gone"
    );
}

#[test]
fn outside_files_of_the_wrong_type_show_nothing_and_are_refused() {
    let mut d = harness(3);
    let at = rect(&d, 4).center();
    d.harness_mut().hover_at(at);
    d.hover_files(&paths(&["notes.txt"])).frames(2);
    assert!(support::last_drawn(d.ctx(), &format!("{}/\"insert\"", gallery::GRID)).is_none());
    d.drop_files(&paths(&["/tmp/notes.txt"])).frames(2);
    assert_eq!(d.app().panels.len(), 12);
    assert!(d.app().status.starts_with("refused"), "{}", d.app().status);
}

#[test]
fn a_drop_with_no_position_appends_and_scrolls_the_new_panel_into_view() {
    let mut d = harness(3);
    assert!(
        rect(&d, 11).max.y > SIZE.y,
        "precondition: the end is below the fold"
    );
    let before = rect(&d, 0).min.y;
    d.hover_files(&paths(&["pic.png"])).frames(2);
    assert!(
        support::last_drawn(d.ctx(), &format!("{}/\"insert\"", gallery::GRID)).is_none(),
        "no position, so no gap, and nothing is guessed"
    );
    d.drop_files(&paths(&["/tmp/pic.png"])).frames(29);
    let app = &d.app();
    assert_eq!(app.panels.len(), 13);
    assert_eq!(app.panels[12].1, "pic.png");
    assert!(app.reveal.is_none());
    assert!(
        rect(&d, 12).min.y < SIZE.y && rect(&d, 12).max.y > 0.0,
        "the new panel is on screen"
    );
    assert!(rect(&d, 0).min.y < before - 20.0, "the view scrolled");
}

#[test]
fn a_drop_at_a_position_that_arrived_with_it_inserts_there() {
    let mut d = harness(3);
    let at = after_middle(&d, 4);
    d.harness_mut().hover_at(at);
    d.hover_files(&paths(&["pic.png"]));
    d.harness_mut().hover_at(at);
    d.drop_files(&paths(&["/tmp/pic.png"])).frames(2);
    assert_eq!(d.app().panels[5].1, "pic.png");
}

#[test]
fn dragging_near_the_bottom_edge_scrolls_and_the_gap_follows() {
    let mut d = harness(3);
    lift(&mut d, 0);
    let grid_top = |d: &H| support::layout_rect(d.ctx(), gallery::GRID).min.y;
    let before = grid_top(&d);
    d.hover(near_the_bottom_edge());
    d.frames(10);
    assert!(
        grid_top(&d) < before - 20.0,
        "content scrolled up: {before} -> {}",
        grid_top(&d)
    );
    let slot = layout_order(&d).iter().position(|&k| k == 0).unwrap();
    assert!(slot >= 6, "the panel went to a lower row: {slot}");
}

#[test]
fn outside_drop_on_a_scrolled_away_part_via_auto_scroll() {
    let mut d = harness(3);
    d.harness_mut().hover_at(near_the_bottom_edge());
    d.hover_files(&paths(&["far.png"])).frames(29);
    assert!(
        support::last_drawn(d.ctx(), &format!("{}/\"insert\"", gallery::GRID)).is_some(),
        "a placeholder holds the gap"
    );
    d.drop_files(&paths(&["/tmp/far.png"])).frames(1);
    let at = d
        .app()
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
    let mut d = harness(3);
    lift(&mut d, 0);
    d.hover(Pos2::new(SIZE.x / 2.0, SIZE.y / 2.0));
    let before = rect(&d, 8).min.y;
    d.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -150.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    })
    .frames(7);
    assert!(
        rect(&d, 8).min.y < before - 20.0,
        "the wheel scrolled while dragging"
    );
    assert!(carried(&d));
}

#[test]
fn scrolling_alone_does_not_slide_panels() {
    let mut d = harness(3);
    d.hover(Pos2::new(SIZE.x / 2.0, SIZE.y / 2.0));
    let before = rect(&d, 4).min.y;
    d.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -60.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    })
    .frames(1);
    assert!(rect(&d, 4).min.y < before - 10.0, "it did scroll");
    for key in [0, 4, 7] {
        assert!(
            (painted(&d, key).min
                - support::last_drawn(d.ctx(), &face_path(key))
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
    let mut d = harness(3);
    let old = rect(&d, 3);
    d.app_mut().cols = 2;
    d.frames(1);
    let now = rect(&d, 3);
    assert!(now.min != old.min && now.width() > old.width());
    assert!(
        (painted(&d, 3).min
            - support::last_drawn(d.ctx(), &face_path(3))
                .unwrap()
                .layout_rect
                .min)
            .length()
            > 3.0,
        "panel 3 is not yet painted at its final rect"
    );
    d.frames(20);
    assert!((painted(&d, 3).min - face_layout(&d, 3).min).length() < 1.0);
}

/// The gallery under a toolbar strip, as `main` lays it out.
fn under_a_toolbar(
    ui: &mut egui::Ui,
    app: &mut State,
    layout: &mut eggshell_ui::LayoutState,
) -> eggshell_ui::testing::PlacedFrame {
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
        .show(ui, |ui| gallery::frame(ui, app, layout))
        .inner
}

/// The trace painted last, which is the lifted panel's: its shapes are on the
/// layer above every other. With its shape index and clip.
fn lifted_trace(d: &H) -> (Vec<Pos2>, usize, Rect) {
    let mut last = None;
    for (i, clipped) in d.harness().output().shapes.iter().enumerate() {
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
    let mut d = harness(3);
    lift(&mut d, 0);
    let at = rect(&d, 4).center() + vec2(17.0, 13.0);
    d.hover(at);
    let (first, ..) = lifted_trace(&d);
    let centre = Rect::from_points(&first).center();
    assert!(
        (centre - at).length() < 40.0,
        "the trace is under the pointer: {centre:?}"
    );
    d.frames(1);
    d.frames(1);
    let (second, ..) = lifted_trace(&d);
    assert_ne!(
        first, second,
        "the lifted panel's trace changed between frames"
    );
}

#[test]
fn the_live_image_is_unclipped_and_above_the_toolbar() {
    let mut d = support::recorded(support::frame_driver(
        SIZE,
        common::base_text(egui::Theme::Light),
        State::default(),
        under_a_toolbar,
    ));
    d.frames(4);
    lift(&mut d, 0);
    let viewport_top = 44.0;
    let at = Pos2::new(SIZE.x / 2.0, 10.0);
    d.hover(at);
    let toolbar = d
        .harness()
        .output()
        .shapes
        .iter()
        .position(
            |s| matches!(&s.shape, Shape::Rect(r) if r.fill == egui::Color32::from_rgb(1, 2, 3)),
        )
        .expect("the toolbar is painted");
    let (trace, index, clip) = lifted_trace(&d);
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
    let mut d = harness(3);
    let ring = common::tokens(d.ctx()).ring;
    let strokes = |d: &H| -> Vec<Rect> {
        let mut all = Vec::new();
        for clipped in &d.harness().output().shapes {
            support::flatten(&clipped.shape, &mut all);
        }
        all.into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.stroke.width > 0.0 && r.stroke.color == ring => Some(r.rect),
                _ => None,
            })
            .collect()
    };
    assert!(strokes(&d).is_empty(), "no outline before the lift");
    lift(&mut d, 0);
    let away = rect(&d, 0).center() + vec2(0.0, 40.0);
    d.hover(away);
    let slot = rect(&d, 0);
    assert!(
        strokes(&d)
            .iter()
            .any(|r| support::same(*r, slot.shrink(1.0)) || support::same(*r, slot)),
        "the slot is outlined: {:?} vs {slot:?}",
        strokes(&d)
    );
    assert!(
        (painted(&d, 0).center() - slot.center()).length() < 1.0,
        "and the layout rect is still the slot"
    );
}

#[test]
fn the_chip_image_leaves_the_panel_in_its_slot_dimmed() {
    let mut state = State::default();
    state.image = Image::Chip;
    let mut d = harness_with(state, gallery::frame);
    lift(&mut d, 0);
    let pointer = rect(&d, 4).center();
    d.hover(pointer);
    d.frames(20);
    let label = |d: &H| -> Vec<(Pos2, f32)> {
        support::shapes(d)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) if t.galley.text() == "w0" => Some((t.pos, t.opacity_factor)),
                _ => None,
            })
            .collect()
    };
    let labels = label(&d);
    assert_eq!(labels.len(), 2, "the panel's own label and the chip's");
    let (in_slot, chip): (Vec<_>, Vec<_>) = labels
        .into_iter()
        .partition(|(at, _)| (at.x - pointer.x).abs() > 40.0 || (at.y - pointer.y).abs() > 40.0);
    assert!(
        chip.len() == 1 && chip[0].1 == 1.0,
        "the chip follows the pointer, undimmed: {chip:?}"
    );
    assert_eq!(in_slot.len(), 1, "the panel's label stays in its slot");
    let dimmed = common::tokens(d.ctx()).muted.gamma_multiply(0.35);
    assert!(
        painted_fills(&d, dimmed)
            .iter()
            .any(|face| support::same(*face, face_layout(&d, 0))),
        "the face in the slot is dimmed"
    );
    assert!(
        (painted(&d, 0).center() - face_layout(&d, 0).center()).length() < 1.0,
        "and is painted at it"
    );
}

#[test]
fn on_release_the_panel_is_opaque_in_its_slot_with_no_flash() {
    let mut d = harness(3);
    let muted = common::tokens(d.ctx()).muted;
    lift(&mut d, 0);
    move_past(&mut d, 4);
    let at = rect(&d, 0).center();
    d.pointer_up(at).event(Event::PointerGone);
    let slot = rect(&d, 0);
    // The toolbar's segmented controls share the fill; count panel faces only.
    let face = face_layout(&d, 1);
    let faces: Vec<Rect> = painted_fills(&d, muted)
        .into_iter()
        .filter(|r| (r.width() - face.width()).abs() < 2.0)
        .collect();
    let in_view = (0..12)
        .filter(|&key| rect(&d, key).intersects(VIEWPORT))
        .count();
    assert!(in_view < 12, "precondition: some panels are scrolled out");
    assert!(
        faces.len() >= in_view,
        "every panel face in view is painted fully opaque: {} of {in_view}",
        faces.len()
    );
    assert!(
        faces
            .iter()
            .enumerate()
            .all(|(i, f)| faces[..i].iter().all(|g| !support::same(*f, *g))),
        "and once"
    );
    assert!(
        faces.iter().any(|f| f.contains(slot.center())),
        "the dropped panel's face sits at its slot on the release frame"
    );
}

fn painted_fills(d: &H, fill: egui::Color32) -> Vec<Rect> {
    support::shapes(d)
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
    let mut d = harness(3);
    lift(&mut d, 2);
    let start = d.ctx().pointer_latest_pos().expect("a pointer");
    let centre_x = |d: &H| {
        let (trace, ..) = lifted_trace(d);
        (trace[0].x + trace[trace.len() - 1].x) / 2.0
    };
    let grab_x = start.x - centre_x(&d);
    let target = rect(&d, 1).center() - vec2(rect(&d, 1).width() / 4.0, 0.0);
    assert!(
        target.x < start.x - rect(&d, 1).width() / 2.0,
        "precondition: the walk crosses into column 2"
    );
    let mut reordered = false;
    for step in 1..=20 {
        let at = start + (target - start) * (step as f32 / 20.0);
        d.carry_to(at);
        reordered |= layout_order(&d)[1] == 2;
        let drawn = centre_x(&d);
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
    let mut d = harness(3);
    lift(&mut d, 2);
    for target in [4, 7, 1, 10] {
        move_past(&mut d, target);
    }
    let at = rect(&d, 2).center();
    d.pointer_up(at).event(Event::PointerGone).frames(19);
    Timeline::recorded(d.ctx()).assert_continuous();
}

#[test]
fn an_escaped_gallery_drag_is_continuous() {
    let mut d = harness(2);
    lift(&mut d, 1);
    for target in [3, 5] {
        move_past(&mut d, target);
    }
    d.press(Key::Escape).frames(19);
    Timeline::recorded(d.ctx()).assert_continuous();
}

#[test]
fn a_resize_under_a_held_panel_keeps_the_drag() {
    let mut d = harness(3);
    lift(&mut d, 0);
    let mut width = SIZE.x;
    while width > 400.0 {
        width -= 12.0;
        d.resize(vec2(width, SIZE.y));
        assert!(carried(&d), "still dragging at width {width}");
    }
    d.frames(20);
    assert_eq!(
        layout_order(&d)[..3],
        [0, 1, 2],
        "the pointer stayed over the first slot"
    );
    assert!(rect(&d, 1).max.x <= width, "the grid follows the window");
    let at = rect(&d, 0).center();
    d.pointer_up(at).event(Event::PointerGone).frames(19);
    Timeline::recorded(d.ctx()).assert_continuous();
}

#[test]
fn a_gallery_grown_by_a_column_change_while_scrolling_slides_a_reorder() {
    let mut d = harness(3);
    lift(&mut d, 0);
    d.hover(near_the_bottom_edge());
    d.frames(4);
    let before = painted(&d, 1);
    move_past(&mut d, 3);
    let after = painted(&d, 1);
    assert!(
        (after.min - before.min).length() < 120.0,
        "a reorder while the view scrolls slides, it does not snap: {before:?} -> {after:?}"
    );
}

/// The live image is the panel as it looks at rest, at full opacity, not
/// the dimmed in-slot look a chip drag uses.
#[test]
fn the_live_image_keeps_the_panels_look_at_rest() {
    let mut d = harness(3);
    lift(&mut d, 0);
    let over = rect(&d, 4).center() + vec2(17.0, 13.0);
    d.hover(over);
    assert!(
        carried(&d),
        "precondition: the panel is carried as a live image"
    );
    let opacity = |key: u32| {
        support::last_drawn(d.ctx(), &face_path(key))
            .expect("the face is drawn")
            .opacity
    };
    assert_eq!(
        opacity(0),
        opacity(5),
        "the live image is as opaque as a panel at rest"
    );
}

/// Another tree that doesn't own the drag must not erase where the drag would
/// land: the gallery's panels still make room. (Found by the user: with the
/// toolbar drawn first, the panels never moved.)
#[test]
fn another_tree_drawn_first_keeps_the_preview() {
    let mut d = harness(3);
    let before = layout_order(&d);
    assert_eq!(
        before[..3],
        [0, 1, 2],
        "precondition: the gallery starts in order"
    );
    lift(&mut d, 0);
    move_past(&mut d, 4);
    assert!(carried(&d), "precondition: the panel is still carried");
    assert_ne!(
        layout_order(&d)[..6],
        before[..6],
        "the other panels make room while the drag is over the grid"
    );
}

#[test]
fn the_drag_chip_wears_the_tooltip_look_in_both_themes() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut state = State::default();
        state.image = Image::Chip;
        let mut d = harness_with(state, gallery::frame);
        d.ctx().set_theme(theme);
        d.frames(4);
        lift(&mut d, 0);
        let pointer = rect(&d, 4).center();
        d.hover(pointer);
        let tokens = common::tokens(d.ctx());
        let labels: Vec<(Pos2, Color32)> = support::shapes(&d)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) if t.galley.text() == "w0" => {
                    Some((t.pos, t.override_text_color.unwrap_or(t.fallback_color)))
                }
                _ => None,
            })
            .collect();
        let chip: Vec<_> = labels
            .iter()
            .filter(|(at, _)| (*at - pointer).length() < 60.0)
            .collect();
        assert_eq!(
            chip.len(),
            1,
            "{theme:?}: one label at the pointer: {labels:?}"
        );
        assert_eq!(chip[0].1, tokens.primary_foreground, "{theme:?}: text");
        let fills: Vec<Rect> = support::shapes(&d)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) if r.fill == tokens.primary && r.rect.contains(chip[0].0) => {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            fills.len(),
            1,
            "{theme:?}: the chip's primary fill sits behind its text"
        );
        assert!(
            fills[0].width() < 80.0 && fills[0].height() < 40.0,
            "{theme:?}: a small chip: {:?}",
            fills[0]
        );
        let (primary_in_light, primary_in_dark) = (
            eggshell_ui::theme::Shadcn::light().tokens.primary,
            eggshell_ui::theme::Shadcn::dark().tokens.primary,
        );
        assert_ne!(
            primary_in_light, primary_in_dark,
            "precondition: the themes differ"
        );
    }
}

fn grip_path(key: u32) -> String {
    format!("{}/\"grip\"", face_path(key))
}

#[test]
fn h_switches_to_grip_mode_where_only_the_grip_lifts_a_panel() {
    let mut d = harness(3);
    assert_eq!(
        d.app().lift,
        Lift::Anywhere,
        "precondition: a panel lifts from anywhere by default"
    );
    assert!(
        eggshell_ui::testing::laid_out_rect(d.ctx(), &grip_path(2)).is_none(),
        "no grip by default"
    );
    d.press(Key::H).frames(2);
    assert_eq!(d.app().lift, Lift::Grip);
    let grip = support::layout_rect(d.ctx(), &grip_path(2));
    let face = face_layout(&d, 2);
    assert!(face.contains_rect(grip), "the grip is on the face");

    let body = face.center();
    assert!(
        !grip.contains(body),
        "precondition: the face's centre is not the grip"
    );
    d.lift(body, vec2(0.0, 12.0)).frames(1);
    assert!(!carried(&d), "the face's body no longer lifts the panel");
    d.pointer_up(body + vec2(0.0, 12.0))
        .event(Event::PointerGone)
        .frames(2);

    d.lift(grip.center(), vec2(0.0, 12.0)).frames(1);
    assert!(carried(&d), "the grip lifts the panel");
    move_past(&mut d, 4);
    assert_eq!(layout_order(&d)[..6], [0, 1, 3, 4, 2, 5], "the panel moves");
    let at = rect(&d, 2).center();
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert_eq!(order(&d)[..6], [0, 1, 3, 4, 2, 5]);

    d.press(Key::H).frames(2);
    assert_eq!(d.app().lift, Lift::Anywhere, "H switches back");
}

/// A resize that moves the held panel's insertion point reorders the
/// neighbours on the resize frame; they slide, as any reorder does (#86).
#[test]
fn a_resize_that_reorders_under_a_held_panel_is_continuous() {
    for cols in 3..=5 {
        let mut d = harness(cols);
        lift(&mut d, 2);
        assert!(carried(&d), "cols {cols}: precondition: lifted");
        let order = layout_order(&d);
        let mut width = SIZE.x;
        while width < 857.0 {
            width = (width + 12.0).min(857.0);
            d.resize(vec2(width, SIZE.y));
            assert!(carried(&d), "cols {cols}: still dragging at {width}");
        }
        d.frames(20);
        assert_ne!(
            layout_order(&d)[..6],
            order[..6],
            "cols {cols}: precondition: the resize reorders the preview"
        );
        Timeline::recorded(d.ctx()).assert_continuous();
    }
}

/// A resize that reorders nothing still places the nodes at once: a slide on
/// it would lag the window's edge.
#[test]
fn a_plain_resize_places_the_panels_at_once() {
    let mut d = harness(3);
    let mut width = SIZE.x;
    while width > 400.0 {
        width -= 12.0;
        d.resize(vec2(width, SIZE.y));
        for key in 0..6 {
            assert_eq!(
                painted(&d, key),
                face_layout(&d, key),
                "panel {key} at width {width}"
            );
        }
    }
}
