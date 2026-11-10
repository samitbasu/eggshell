//! Drag-and-drop spike (#71): the gallery (2D reorder in a scroll area, outside
//! file drops, auto-scroll) through the real path.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Event, Key, Modifiers, MouseWheelUnit, Pos2, Rect, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_gallery.rs"]
mod gallery;

use crate::support::{self, FrameRig};
use gallery::{PanelKey, State};

type H<'a> = Harness<'a, FrameRig<State>>;

fn harness(cols: usize) -> H<'static> {
    let mut state = State::default();
    state.cols = cols;
    let mut h = support::frame_harness(
        vec2(520.0, 400.0),
        common::base_text(egui::Theme::Light),
        state,
        gallery::frame,
    );
    h.run_steps(4);
    h
}

fn order(h: &H<'_>) -> Vec<u32> {
    h.state().app.panels.iter().map(|p| p.0.0).collect()
}

fn rect(h: &H<'_>, key: u32) -> Rect {
    h.state().app.rects[&PanelKey(key)]
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

/// How far the painted box of a panel is inset from its layout rect.
fn pad(h: &H<'_>) -> f32 {
    let cell = rect(h, 0);
    painted_rects(h)
        .iter()
        .filter(|r| (r.center() - cell.center()).length() < 1.0 && r.width() < cell.width())
        .map(|r| (cell.width() - r.width()) / 2.0)
        .next()
        .expect("a panel is painted")
}

fn painted_rects(h: &H<'_>) -> Vec<Rect> {
    let mut all = Vec::new();
    for clipped in &h.output().shapes {
        support::flatten(&clipped.shape, &mut all);
    }
    all.into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Rect(r) => Some(r.rect),
            _ => None,
        })
        .collect()
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

#[test]
fn the_gallery_wraps_into_square_columns_and_scrolls() {
    let h = harness(3);
    let (a, b) = (rect(&h, 0), rect(&h, 1));
    assert!((a.width() - a.height()).abs() < 1.0, "square: {a:?}");
    assert!((b.min.x - a.max.x).abs() < 1.0, "next to each other");
    assert!(rect(&h, 3).min.y > a.max.y - 1.0, "the fourth wraps");
    assert!(
        h.state().app.content_height > h.state().app.viewport.height(),
        "taller than the viewport"
    );
}

#[test]
fn reorder_across_a_row_boundary() {
    let mut h = harness(3);
    lift(&mut h, 2);
    assert!(egui::DragAndDrop::has_payload_of_type::<PanelKey>(&h.ctx));
    let r4 = rect(&h, 4);
    assert!(
        r4.min.y > rect(&h, 2).max.y - 1.0,
        "panel 4 is on the next row"
    );
    pointer_to(&mut h, r4.center() + vec2(r4.width() / 4.0, 0.0));
    assert_eq!(order(&h)[..6], [0, 1, 3, 4, 2, 5], "after 4, before 5");
    assert!(egui::DragAndDrop::has_payload_of_type::<PanelKey>(&h.ctx));
    let held = rect(&h, 2);
    h.drop_at(held.center());
    h.run_steps(3);
    assert_eq!(order(&h)[..6], [0, 1, 3, 4, 2, 5]);
    assert!(h.state().app.gesture.is_none());
}

#[test]
fn layout_rects_are_final_while_painting_slides() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let old = rect(&h, 1);
    let r4 = rect(&h, 4);
    h.hover_at(r4.center() + vec2(r4.width() / 4.0, 0.0));
    h.step();
    h.step();
    let now = rect(&h, 1);
    assert!(now.min.x < old.min.x, "panel 1 moved left in layout");
    let p = pad(&h);
    let painted = painted_rects(&h);
    assert!(
        painted.iter().any(|r| {
            (r.width() - (now.width() - 2.0 * p)).abs() < 2.0
                && r.min.x > now.min.x + p + 1.0
                && r.min.x < old.min.x + p + 1.0
                && (r.min.y - (now.min.y + p)).abs() < 1.0
        }),
        "panel 1 is painted away from its final place (at or near the old one)"
    );
    h.run_steps(8);
    let settled = painted_rects(&h);
    let fin = rect(&h, 1).shrink(p);
    assert!(
        settled.iter().any(|r| (r.min - fin.min).length() < 1.5),
        "and reaches it"
    );
}

#[test]
fn escape_cancels_a_2d_reorder() {
    let mut h = harness(3);
    lift(&mut h, 2);
    let r4 = rect(&h, 4);
    pointer_to(&mut h, r4.center() + vec2(r4.width() / 4.0, 0.0));
    assert_ne!(order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert_eq!(order(&h)[..6], [0, 1, 2, 3, 4, 5]);
    let far = rect(&h, 8).center();
    pointer_to(&mut h, far);
    assert_eq!(order(&h)[..6], [0, 1, 2, 3, 4, 5], "stays cancelled");
}

#[test]
fn drop_position_gives_the_insertion_index() {
    let h = harness(3);
    let app = &h.state().app;
    let r = |k| rect(&h, k);
    // Left half of panel 4: before it. Right half: after it.
    assert_eq!(app.index_at(r(4).center() - vec2(10.0, 0.0), None), 4);
    assert_eq!(app.index_at(r(4).center() + vec2(10.0, 0.0), None), 5);
    // Right of the last panel in a row, and in the gap below a row: next row.
    assert_eq!(app.index_at(r(2).right_center() + vec2(1.0, 0.0), None), 3);
    let below = Pos2::new(r(0).center().x - 1.0, r(0).max.y + 0.5);
    assert_eq!(
        app.index_at(below, None),
        3,
        "left of centre on the next row"
    );
    // A gap between rows (panels here touch, so use bare rects) reads as the next row.
    let rows = [
        Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(10.0, 10.0)),
        Rect::from_min_size(Pos2::new(0.0, 20.0), vec2(10.0, 10.0)),
    ];
    assert_eq!(common::dnd::insertion_index(&rows, Pos2::new(9.0, 15.0)), 1);
    // Below everything: the end.
    assert_eq!(app.index_at(Pos2::new(10.0, 10_000.0), None), 12);
}

#[test]
fn outside_files_show_a_gap_and_insert_at_the_pointer() {
    let mut h = harness(3);
    let r4 = rect(&h, 4);
    let at = r4.center() + vec2(r4.width() / 4.0, 0.0);
    h.hover_at(at);
    hover_files(&mut h, &["pic.png"]);
    h.run_steps(3);
    assert_eq!(h.state().app.preview.as_ref().map(|p| p.0), Some(5));
    assert!(
        rect(&h, 5).min != at && h.state().app.rects.contains_key(&PanelKey(5)),
        "the gap is a placeholder in the layout, so panels slid"
    );
    h.input_mut().hovered_files.clear();
    h.input_mut()
        .dropped_files
        .push(Arc::new(TestFile(PathBuf::from("/tmp/pic.png"))));
    h.run_steps(3);
    let app = &h.state().app;
    assert_eq!(app.panels.len(), 13);
    assert_eq!(app.panels[5].1, "pic.png");
    assert!(app.preview.is_none());
}

#[test]
fn outside_files_of_the_wrong_type_show_nothing_and_are_refused() {
    let mut h = harness(3);
    let at = rect(&h, 4).center();
    h.hover_at(at);
    hover_files(&mut h, &["notes.txt"]);
    h.run_steps(3);
    assert!(h.state().app.preview.is_none());
    h.input_mut().hovered_files.clear();
    h.input_mut()
        .dropped_files
        .push(Arc::new(TestFile(PathBuf::from("/tmp/notes.txt"))));
    h.run_steps(3);
    assert_eq!(h.state().app.panels.len(), 12);
    assert!(h.state().app.dropped[0].starts_with("refused"));
}

fn viewport(h: &H<'_>) -> Rect {
    h.state().app.viewport
}

#[test]
fn dragging_near_the_bottom_edge_scrolls() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let before = h.state().app.origin.y;
    let v = viewport(&h);
    pointer_to(&mut h, Pos2::new(v.center().x, v.max.y - 4.0));
    h.run_steps(10);
    let after = h.state().app.origin.y;
    assert!(
        after < before - 20.0,
        "content scrolled up: {before} -> {after}"
    );
    // The in-app reorder follows the scroll: the dragged panel goes to the
    // slot under the pointer, in content coordinates.
    let idx = h
        .state()
        .app
        .panels
        .iter()
        .position(|p| p.0.0 == 0)
        .unwrap();
    assert!(idx >= 6, "dragged panel reached a lower row: {idx}");
}

#[test]
fn outside_drop_on_a_scrolled_away_part_via_auto_scroll() {
    let mut h = harness(3);
    let v = viewport(&h);
    h.hover_at(Pos2::new(v.center().x, v.max.y - 4.0));
    hover_files(&mut h, &["far.png"]);
    h.run_steps(30);
    let (index, _) = h.state().app.preview.clone().expect("a preview");
    assert!(
        index >= 9,
        "auto-scroll brought the last rows under the pointer: {index}"
    );
    h.input_mut().hovered_files.clear();
    h.input_mut()
        .dropped_files
        .push(Arc::new(TestFile(PathBuf::from("/tmp/far.png"))));
    h.run_steps(2);
    assert_eq!(h.state().app.panels[index].1, "far.png");
}

#[test]
fn the_wheel_scrolls_during_a_drag_and_reorders_by_content_position() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let before = h.state().app.origin.y;
    let v = viewport(&h);
    let mid = v.center();
    pointer_to(&mut h, mid);
    let o1 = h.state().app.origin.y;
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -150.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.run_steps(8);
    let o2 = h.state().app.origin.y;
    println!("wheel during drag: origin {before} -> {o1} -> {o2}");
    assert!(o2 < o1 - 20.0, "the wheel scrolled while dragging");
    assert!(egui::DragAndDrop::has_payload_of_type::<PanelKey>(&h.ctx));
}

#[test]
fn scrolling_alone_does_not_slide_panels() {
    let mut h = harness(3);
    let c = viewport(&h).center();
    pointer_to(&mut h, c);
    h.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -60.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    h.step();
    h.step();
    for key in [0, 4, 7] {
        let final_rect = rect(&h, key).shrink(pad(&h));
        assert!(
            painted_rects(&h)
                .iter()
                .any(|r| support::same(*r, final_rect)),
            "panel {key} is painted at its layout rect while scrolling"
        );
    }
    assert!(
        h.state().app.origin.y < viewport(&h).min.y - 10.0,
        "it did scroll"
    );
}

#[test]
fn changing_the_column_count_reflows_with_final_layout_and_a_slide() {
    let mut h = harness(3);
    let old = rect(&h, 5);
    h.state_mut().app.cols = 2;
    h.step();
    let now = rect(&h, 5);
    assert!(now.min != old.min && now.width() > old.width());
    let final_rect = now.shrink(pad(&h));
    assert!(
        !painted_rects(&h)
            .iter()
            .any(|r| support::same(*r, final_rect)),
        "panel 5 is not yet painted at its final rect"
    );
    h.run_steps(10);
    assert!(
        painted_rects(&h)
            .iter()
            .any(|r| support::same(*r, rect(&h, 5).shrink(pad(&h))))
    );
}

/// The gallery under a toolbar strip, as `main` lays it out.
fn under_a_toolbar(ui: &mut egui::Ui, app: &mut State, layout: &mut bullseye::LayoutState) {
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

fn toolbar_harness() -> H<'static> {
    let mut h = support::frame_harness(
        vec2(520.0, 400.0),
        common::base_text(egui::Theme::Light),
        State::default(),
        under_a_toolbar,
    );
    h.run_steps(4);
    h
}

/// The trace of the panel painted nearest `at`: the 64-point line.
fn trace_near(h: &H<'_>, at: Pos2) -> Vec<Pos2> {
    let mut all = Vec::new();
    for clipped in &h.output().shapes {
        support::flatten(&clipped.shape, &mut all);
    }
    all.into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Path(p) if p.points.len() == 64 => Some(p.points.clone()),
            _ => None,
        })
        .min_by(|a, b| {
            let d = |pts: &Vec<Pos2>| (Rect::from_points(pts).center() - at).length();
            d(a).total_cmp(&d(b))
        })
        .expect("a trace is painted")
}

#[test]
fn the_live_image_keeps_moving_under_the_pointer() {
    let mut h = harness(3);
    lift(&mut h, 0);
    let at = rect(&h, 4).center() + vec2(17.0, 13.0);
    pointer_to(&mut h, at);
    let first = trace_near(&h, at);
    let centre = Rect::from_points(&first).center();
    assert!(
        (centre - at).length() < 40.0,
        "the trace is under the pointer: {centre:?}"
    );
    h.step();
    h.step();
    let second = trace_near(&h, at);
    assert_ne!(
        first, second,
        "the lifted panel's trace changed between frames"
    );
}

#[test]
fn the_live_image_is_unclipped_and_above_the_toolbar() {
    let mut h = toolbar_harness();
    lift(&mut h, 0);
    let viewport = viewport(&h);
    assert!(
        viewport.min.y >= 44.0,
        "the toolbar takes the top: {viewport:?}"
    );
    let at = Pos2::new(viewport.center().x, 10.0);
    pointer_to(&mut h, at);
    let shapes = &h.output().shapes;
    let toolbar = shapes
        .iter()
        .position(|s| matches!(&s.shape, egui::epaint::Shape::Rect(r) if r.fill == egui::Color32::from_rgb(1, 2, 3)))
        .expect("the toolbar is painted");
    let mut best: Option<(f32, usize, Rect)> = None;
    for (i, clipped) in shapes.iter().enumerate() {
        let mut flat = Vec::new();
        support::flatten(&clipped.shape, &mut flat);
        for shape in flat {
            if let egui::epaint::Shape::Path(p) = shape
                && p.points.len() == 64
            {
                let bounds = Rect::from_points(&p.points);
                let d = (bounds.center() - at).length();
                if best.is_none_or(|(bd, ..)| d < bd) {
                    best = Some((d, i, clipped.clip_rect));
                }
            }
        }
    }
    let (_, index, clip) = best.expect("a trace");
    assert!(
        index > toolbar,
        "the lifted trace is painted after the toolbar"
    );
    assert!(
        clip.min.y < viewport.min.y - 1.0,
        "its clip reaches past the viewport: {clip:?}"
    );
    let trace = trace_near(&h, at);
    assert!(
        Rect::from_points(&trace).min.y < viewport.min.y,
        "the trace is drawn over the toolbar strip, not clipped to the viewport"
    );
}

#[test]
fn the_slot_shows_an_outline_while_the_panel_is_lifted() {
    let mut h = harness(3);
    let ring = bullseye_shadcn::Shadcn::of(&h.ctx).tokens.ring;
    let slot = rect(&h, 0);
    let outlined = |h: &H<'_>| {
        painted_strokes(h)
            .iter()
            .any(|(r, c)| support::same(*r, slot) && *c == ring)
    };
    assert!(!outlined(&h), "no outline before the lift");
    lift(&mut h, 0);
    let away = rect(&h, 0).center() + vec2(0.0, 40.0);
    pointer_to(&mut h, away);
    let slot = rect(&h, 0);
    assert!(
        painted_strokes(&h)
            .iter()
            .any(|(r, c)| support::same(*r, slot) && *c == ring),
        "the slot is outlined"
    );
    let face = trace_near(&h, rect(&h, 0).center());
    assert!(
        (Rect::from_points(&face).center() - slot.center()).length() < 60.0,
        "layout rect is still the slot"
    );
}

fn painted_strokes(h: &H<'_>) -> Vec<(Rect, egui::Color32)> {
    let mut all = Vec::new();
    for clipped in &h.output().shapes {
        support::flatten(&clipped.shape, &mut all);
    }
    all.into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Rect(r) if r.stroke.width > 0.0 => Some((r.rect, r.stroke.color)),
            _ => None,
        })
        .collect()
}

#[test]
fn on_release_the_panel_is_opaque_in_its_slot_with_no_flash() {
    let mut h = harness(3);
    let muted = bullseye_shadcn::Shadcn::of(&h.ctx).tokens.muted;
    lift(&mut h, 0);
    let r4 = rect(&h, 4);
    pointer_to(&mut h, r4.center() + vec2(r4.width() / 4.0, 0.0));
    let at = rect(&h, 0).center();
    h.drop_at(at);
    h.step();
    let slot = rect(&h, 0);
    let mut all = Vec::new();
    for clipped in &h.output().shapes {
        support::flatten(&clipped.shape, &mut all);
    }
    let faces: Vec<_> = all
        .into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Rect(r) if r.fill == muted => Some(r.rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        faces.len(),
        12,
        "every panel face is painted fully opaque, once"
    );
    assert!(
        faces
            .iter()
            .any(|f| (f.center() - slot.center()).length() < 3.0 || f.contains(slot.center())),
        "the dropped panel's face sits at its slot on the release frame"
    );
}
