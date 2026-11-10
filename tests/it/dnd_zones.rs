//! Drag-and-drop spike (#71): list to zone, and OS files on zones.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Pos2, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_zones.rs"]
mod zones;

use crate::support::{self, FrameRig};
use zones::{State, Zone};

type H<'a> = Harness<'a, FrameRig<State>>;

fn harness() -> H<'static> {
    let mut h = support::frame_harness(
        vec2(640.0, 320.0),
        common::base_text(egui::Theme::Light),
        State::new(),
        zones::frame,
    );
    h.run_steps(4);
    h
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

fn zone_rect(h: &H<'_>, z: Zone) -> egui::Rect {
    h.state().app.zone_rects[&z]
}

/// Presses on the node whose label starts a drag; returns after a few
/// moves.
fn lift_at(h: &mut H<'_>, from: Pos2) {
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(6.0, 0.0));
    h.step();
    h.hover_at(from + vec2(12.0, 0.0));
    h.run_steps(2);
}

/// The centre of the first draggable of a list: the lists sit at the left.
fn card_at() -> Pos2 {
    Pos2::new(60.0, 40.0)
}

fn tag_at() -> Pos2 {
    Pos2::new(200.0, 40.0)
}

#[test]
fn a_card_over_the_canvas_highlights_it_and_the_drop_carries_its_position() {
    let mut h = harness();
    lift_at(&mut h, card_at());
    assert!(egui::DragAndDrop::has_payload_of_type::<zones::Card>(
        &h.ctx
    ));
    let canvas = zone_rect(&h, Zone::Canvas);
    let at = canvas.min + vec2(30.0, 20.0);
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(h.state().app.hover, Some(Zone::Canvas));
    h.drop_at(at);
    h.run_steps(3);
    let app = &h.state().app;
    assert_eq!(app.placed.len(), 1);
    assert_eq!(
        app.placed[0].1,
        vec2(30.0, 20.0),
        "position inside the zone"
    );
    assert_eq!(app.cards.len(), 2);
    assert_eq!(app.hover, None);
}

#[test]
fn an_incompatible_payload_shows_no_feedback_and_is_not_delivered() {
    let mut h = harness();
    lift_at(&mut h, tag_at());
    assert!(egui::DragAndDrop::has_payload_of_type::<zones::Tag>(&h.ctx));
    let at = zone_rect(&h, Zone::Canvas).center();
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(h.state().app.hover, None);
    assert!(h.state().app.refusing.is_empty());
    h.drop_at(at);
    h.run_steps(3);
    assert!(h.state().app.placed.is_empty());
}

#[test]
fn releasing_a_card_outside_every_zone_delivers_nothing() {
    let mut h = harness();
    lift_at(&mut h, card_at());
    h.hover_at(Pos2::new(250.0, 300.0));
    h.run_steps(2);
    h.drop_at(Pos2::new(250.0, 300.0));
    h.run_steps(3);
    assert!(h.state().app.placed.is_empty());
    assert_eq!(h.state().app.cards.len(), 3);
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
fn files_hint_the_zone_that_takes_them_and_dim_the_ones_that_refuse() {
    let mut h = harness();
    let at = zone_rect(&h, Zone::Pictures).center();
    h.hover_at(at);
    hover_files(&mut h, &["a.png"]);
    h.run_steps(3);
    assert_eq!(h.state().app.hover, Some(Zone::Pictures));
    assert!(h.state().app.refusing.contains(&Zone::Documents));
    let docs = zone_rect(&h, Zone::Documents).center();
    h.hover_at(docs);
    h.run_steps(3);
    assert_eq!(
        h.state().app.hover,
        None,
        "a png over Documents gets no hint"
    );
}

#[test]
fn dropped_files_are_accepted_or_refused_by_the_zone_under_the_pointer() {
    let mut h = harness();
    let at = zone_rect(&h, Zone::Documents).center();
    h.hover_at(at);
    h.run_steps(2);
    h.input_mut()
        .dropped_files
        .push(Arc::new(TestFile("/x/notes.md".into())));
    h.run_steps(2);
    h.input_mut()
        .dropped_files
        .push(Arc::new(TestFile("/x/pic.png".into())));
    h.run_steps(2);
    let app = &h.state().app;
    assert_eq!(app.files, vec![(Zone::Documents, "/x/notes.md".to_owned())]);
    assert!(
        app.log[1].starts_with("refused /x/pic.png"),
        "{:?}",
        app.log
    );
}
