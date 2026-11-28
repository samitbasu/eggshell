//! List to zone, and OS files on zones, through the real path: two levels of
//! target feedback, delivery with a position, accept and refuse.

use std::path::PathBuf;

use eggshell_ui::testing::{Driver, Timeline};
use egui::epaint::Shape;
use egui::{Color32, Event, Pos2, Rect, vec2};

use eggshell_examples::common;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_zones.rs"]
mod zones;

use crate::support;
use zones::{State, Zone};

type H = Driver<State>;

fn harness() -> H {
    let mut d = support::recorded(support::frame_driver(
        vec2(640.0, 320.0),
        common::base_text(egui::Theme::Light),
        State::new(),
        zones::frame,
    ));
    d.frames(4);
    d
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

fn find(d: &H, suffix: &str) -> Rect {
    let timeline = Timeline::recorded(d.ctx());
    timeline
        .frame(timeline.len() - 1)
        .expect("a recorded frame")
        .iter()
        .find(|n| n.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no node ending in {suffix}"))
        .layout_rect
}

fn zone_rect(d: &H, zone: Zone) -> Rect {
    find(d, &format!("/\"{}\"", zone.key()))
}

fn card_at(d: &H) -> Pos2 {
    find(d, "(\"card\", 1)").center()
}

fn tag_at(d: &H) -> Pos2 {
    find(d, "(\"tag\", 0)").center()
}

fn lift_at(d: &mut H, from: Pos2) {
    d.lift(from, vec2(12.0, 0.0)).frames(1);
}

/// How a zone is painted: its fill and its border.
fn look(d: &H, zone: Zone) -> (Color32, Color32) {
    let rect = zone_rect(d, zone);
    support::shapes(d)
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

fn looks(d: &H) -> Looks {
    let t = common::tokens(d.ctx());
    Looks {
        plain: (t.muted, t.input),
        accepting: (t.muted, t.ring),
        over: (t.accent, t.ring),
    }
}

#[test]
fn a_card_lights_the_canvas_and_fills_it_over_the_canvas() {
    let mut d = harness();
    let looks = looks(&d);
    for zone in Zone::ALL {
        assert_eq!(look(&d, zone), looks.plain, "{zone:?} at rest");
    }
    let from = card_at(&d);
    lift_at(&mut d, from);
    assert!(d.shell().drag_pointer().is_some(), "the card is lifted");
    assert_eq!(
        look(&d, Zone::Canvas),
        looks.accepting,
        "a card would be taken here"
    );
    assert_eq!(
        look(&d, Zone::Pictures),
        looks.plain,
        "and not by the file zones"
    );
    let at = zone_rect(&d, Zone::Canvas).min + vec2(30.0, 20.0);
    d.hover(at);
    assert_eq!(look(&d, Zone::Canvas), looks.over);
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    let app = &d.app();
    assert_eq!(app.placed.len(), 1);
    assert_eq!(
        app.placed[0].1,
        vec2(30.0, 20.0),
        "position inside the zone"
    );
    assert_eq!(app.cards.len(), 2);
    assert_eq!(
        look(&d, Zone::Canvas),
        looks.plain,
        "nothing is lit once it lands"
    );
}

#[test]
fn an_incompatible_payload_shows_nothing_and_is_not_delivered() {
    let mut d = harness();
    let looks = looks(&d);
    let from = tag_at(&d);
    lift_at(&mut d, from);
    assert!(d.shell().drag_pointer().is_some());
    let at = zone_rect(&d, Zone::Canvas).center();
    d.hover(at);
    for zone in Zone::ALL {
        assert_eq!(look(&d, zone), looks.plain, "{zone:?} is not lit by a tag");
    }
    d.pointer_up(at).event(Event::PointerGone).frames(2);
    assert!(d.app().placed.is_empty());
    assert_eq!(d.app().tags.len(), 2);
}

#[test]
fn releasing_a_card_outside_every_zone_delivers_nothing() {
    let mut d = harness();
    let from = card_at(&d);
    lift_at(&mut d, from);
    d.carry_to(Pos2::new(5.0, 300.0)).frames(1);
    d.pointer_up(Pos2::new(5.0, 300.0))
        .event(Event::PointerGone)
        .frames(2);
    assert!(d.app().placed.is_empty());
    assert_eq!(d.app().cards.len(), 3);
}

#[test]
fn files_light_the_zones_that_take_them_and_fill_the_one_under_the_pointer() {
    let mut d = harness();
    let looks = looks(&d);
    let at = zone_rect(&d, Zone::Pictures).center();
    d.harness_mut().hover_at(at);
    d.hover_files(&paths(&["a.png"])).frames(2);
    assert_eq!(look(&d, Zone::Pictures), looks.over, "a png over Pictures");
    assert_eq!(
        look(&d, Zone::Documents),
        looks.plain,
        "Documents refuses it"
    );
    assert_eq!(
        look(&d, Zone::Canvas),
        looks.plain,
        "and so does the canvas"
    );
    let docs = zone_rect(&d, Zone::Documents).center();
    d.hover(docs);
    assert_eq!(
        look(&d, Zone::Pictures),
        looks.accepting,
        "still lit, not targeted"
    );
    assert_eq!(
        look(&d, Zone::Documents),
        looks.plain,
        "a png over Documents gets no hint"
    );
}

#[test]
fn without_a_position_files_light_the_zones_but_none_is_targeted() {
    let mut d = harness();
    let looks = looks(&d);
    let stale = zone_rect(&d, Zone::Pictures).center();
    d.hover(stale);
    d.hover_files(&paths(&["a.png"])).frames(2);
    assert_eq!(
        look(&d, Zone::Pictures),
        looks.accepting,
        "the pointer was there before the drag came in: that is not a position"
    );
    d.hover(stale + vec2(1.0, 1.0));
    assert_eq!(
        look(&d, Zone::Pictures),
        looks.over,
        "pointer events are a position"
    );
}

#[test]
fn dropped_files_go_to_the_zone_under_the_pointer_or_are_refused() {
    let mut d = harness();
    let at = zone_rect(&d, Zone::Documents).center();
    d.harness_mut().hover_at(at);
    d.hover_files(&paths(&["/x/notes.md"]));
    d.harness_mut().hover_at(at);
    d.drop_files(&paths(&["/x/notes.md"])).frames(1);
    d.hover_files(&paths(&["/x/pic.png"]));
    d.harness_mut().hover_at(at);
    d.drop_files(&paths(&["/x/pic.png"])).frames(1);
    let app = &d.app();
    assert_eq!(app.files, vec![(Zone::Documents, "/x/notes.md".to_owned())]);
    assert!(
        app.log[1].starts_with("refused /x/pic.png"),
        "{:?}",
        app.log
    );
}

#[test]
fn a_drop_with_no_position_goes_to_the_zone_that_declared_the_fallback() {
    let mut d = harness();
    let canvas = zone_rect(&d, Zone::Canvas).center();
    d.carry_to(canvas).frames(1);
    d.drop_files(&paths(&["/x/pic.png"])).frames(2);
    assert_eq!(
        d.app().files,
        vec![(Zone::Pictures, "/x/pic.png".to_owned())],
        "the pointer was over the canvas, which is not a position"
    );
    d.drop_files(&paths(&["/x/pic.png", "/x/notes.md"]))
        .frames(2);
    let app = &d.app();
    assert_eq!(app.files.len(), 1, "no zone takes a mixed drop");
    assert!(
        app.log.iter().any(|l| l.starts_with("refused /x/pic.png")),
        "the page refuses it whole: {:?}",
        app.log
    );
}

#[test]
fn a_zone_drag_is_continuous() {
    let mut d = harness();
    let from = card_at(&d);
    lift_at(&mut d, from);
    let canvas = zone_rect(&d, Zone::Canvas).center();
    let docs = zone_rect(&d, Zone::Documents).center();
    for at in [canvas, docs, canvas] {
        d.hover(at);
    }
    d.pointer_up(canvas).event(Event::PointerGone).frames(19);
    Timeline::recorded(d.ctx()).assert_continuous();
}
