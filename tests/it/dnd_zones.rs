//! List to zone, and OS files on zones, through the real path: two levels of
//! target feedback, delivery with a position, accept and refuse.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eggshell_ui::testing::Timeline;
use egui::epaint::Shape;
use egui::{Color32, Pos2, Rect, vec2};
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

fn find(h: &H<'_>, suffix: &str) -> Rect {
    let timeline = Timeline::recorded(&h.ctx);
    timeline
        .frame(timeline.len() - 1)
        .iter()
        .find(|n| n.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no node ending in {suffix}"))
        .layout_rect
}

fn zone_rect(h: &H<'_>, zone: Zone) -> Rect {
    find(h, &format!("/\"{}\"", zone.key()))
}

fn card_at(h: &H<'_>) -> Pos2 {
    find(h, "(\"card\", 1)").center()
}

fn tag_at(h: &H<'_>) -> Pos2 {
    find(h, "(\"tag\", 0)").center()
}

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

/// How a zone is painted: its fill and its border.
fn look(h: &H<'_>, zone: Zone) -> (Color32, Color32) {
    let rect = zone_rect(h, zone);
    support::flatten_all(h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if support::same(r.rect, rect) => Some((r.fill, r.stroke.color)),
            _ => None,
        })
        .next_back()
        .expect("the zone is painted")
}

struct Looks {
    plain: (Color32, Color32),
    accepting: (Color32, Color32),
    over: (Color32, Color32),
}

fn looks(h: &H<'_>) -> Looks {
    let t = common::tokens(&h.ctx);
    Looks {
        plain: (t.muted, t.input),
        accepting: (t.muted, t.ring),
        over: (t.accent, t.ring),
    }
}

#[test]
fn a_card_lights_the_canvas_and_fills_it_over_the_canvas() {
    let mut h = harness();
    let looks = looks(&h);
    for zone in Zone::ALL {
        assert_eq!(look(&h, zone), looks.plain, "{zone:?} at rest");
    }
    let from = card_at(&h);
    lift_at(&mut h, from);
    assert!(
        eggshell_ui::drag_pointer(&h.ctx).is_some(),
        "the card is lifted"
    );
    assert_eq!(
        look(&h, Zone::Canvas),
        looks.accepting,
        "a card would be taken here"
    );
    assert_eq!(
        look(&h, Zone::Pictures),
        looks.plain,
        "and not by the file zones"
    );
    let at = zone_rect(&h, Zone::Canvas).min + vec2(30.0, 20.0);
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(look(&h, Zone::Canvas), looks.over);
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
    assert_eq!(
        look(&h, Zone::Canvas),
        looks.plain,
        "nothing is lit once it lands"
    );
}

#[test]
fn an_incompatible_payload_shows_nothing_and_is_not_delivered() {
    let mut h = harness();
    let looks = looks(&h);
    let from = tag_at(&h);
    lift_at(&mut h, from);
    assert!(eggshell_ui::drag_pointer(&h.ctx).is_some());
    let at = zone_rect(&h, Zone::Canvas).center();
    h.hover_at(at);
    h.run_steps(3);
    for zone in Zone::ALL {
        assert_eq!(look(&h, zone), looks.plain, "{zone:?} is not lit by a tag");
    }
    h.drop_at(at);
    h.run_steps(3);
    assert!(h.state().app.placed.is_empty());
    assert_eq!(h.state().app.tags.len(), 2);
}

#[test]
fn releasing_a_card_outside_every_zone_delivers_nothing() {
    let mut h = harness();
    let from = card_at(&h);
    lift_at(&mut h, from);
    h.hover_at(Pos2::new(5.0, 300.0));
    h.run_steps(2);
    h.drop_at(Pos2::new(5.0, 300.0));
    h.run_steps(3);
    assert!(h.state().app.placed.is_empty());
    assert_eq!(h.state().app.cards.len(), 3);
}

#[test]
fn files_light_the_zones_that_take_them_and_fill_the_one_under_the_pointer() {
    let mut h = harness();
    let looks = looks(&h);
    let at = zone_rect(&h, Zone::Pictures).center();
    h.hover_at(at);
    hover_files(&mut h, &["a.png"]);
    h.run_steps(3);
    assert_eq!(look(&h, Zone::Pictures), looks.over, "a png over Pictures");
    assert_eq!(
        look(&h, Zone::Documents),
        looks.plain,
        "Documents refuses it"
    );
    assert_eq!(
        look(&h, Zone::Canvas),
        looks.plain,
        "and so does the canvas"
    );
    let docs = zone_rect(&h, Zone::Documents).center();
    h.hover_at(docs);
    h.run_steps(3);
    assert_eq!(
        look(&h, Zone::Pictures),
        looks.accepting,
        "still lit, not targeted"
    );
    assert_eq!(
        look(&h, Zone::Documents),
        looks.plain,
        "a png over Documents gets no hint"
    );
}

#[test]
fn without_a_position_files_light_the_zones_but_none_is_targeted() {
    let mut h = harness();
    let looks = looks(&h);
    let stale = zone_rect(&h, Zone::Pictures).center();
    h.hover_at(stale);
    h.run_steps(3);
    hover_files(&mut h, &["a.png"]);
    h.run_steps(3);
    assert_eq!(
        look(&h, Zone::Pictures),
        looks.accepting,
        "the pointer was there before the drag came in: that is not a position"
    );
    h.hover_at(stale + vec2(1.0, 1.0));
    h.run_steps(3);
    assert_eq!(
        look(&h, Zone::Pictures),
        looks.over,
        "pointer events are a position"
    );
}

#[test]
fn dropped_files_go_to_the_zone_under_the_pointer_or_are_refused() {
    let mut h = harness();
    let at = zone_rect(&h, Zone::Documents).center();
    h.hover_at(at);
    hover_files(&mut h, &["/x/notes.md"]);
    h.step();
    drop_files(&mut h, &["/x/notes.md"]);
    h.hover_at(at);
    h.run_steps(2);
    hover_files(&mut h, &["/x/pic.png"]);
    h.step();
    drop_files(&mut h, &["/x/pic.png"]);
    h.hover_at(at);
    h.run_steps(2);
    let app = &h.state().app;
    assert_eq!(app.files, vec![(Zone::Documents, "/x/notes.md".to_owned())]);
    assert!(
        app.log[1].starts_with("refused /x/pic.png"),
        "{:?}",
        app.log
    );
}

#[test]
fn a_drop_with_no_position_goes_to_the_zone_that_declared_the_fallback() {
    let mut h = harness();
    h.hover_at(zone_rect(&h, Zone::Canvas).center());
    h.run_steps(2);
    drop_files(&mut h, &["/x/pic.png"]);
    h.run_steps(3);
    assert_eq!(
        h.state().app.files,
        vec![(Zone::Pictures, "/x/pic.png".to_owned())],
        "the pointer was over the canvas, which is not a position"
    );
    drop_files(&mut h, &["/x/pic.png", "/x/notes.md"]);
    h.run_steps(3);
    let app = &h.state().app;
    assert_eq!(app.files.len(), 1, "no zone takes a mixed drop");
    assert!(
        app.log.iter().any(|l| l.starts_with("refused /x/pic.png")),
        "the page refuses it whole: {:?}",
        app.log
    );
}

#[test]
fn a_zone_drag_is_continuous() {
    let mut h = harness();
    let from = card_at(&h);
    lift_at(&mut h, from);
    let canvas = zone_rect(&h, Zone::Canvas).center();
    let docs = zone_rect(&h, Zone::Documents).center();
    for at in [canvas, docs, canvas] {
        h.hover_at(at);
        h.run_steps(3);
    }
    h.drop_at(canvas);
    h.run_steps(20);
    Timeline::recorded(&h.ctx).assert_continuous();
}
