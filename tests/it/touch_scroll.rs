//! A touch drag on a scroll container, through the pointer and touch events
//! as eframe web sends them: how far the content moves for how far the finger
//! does, from the background and from a control, and that it coasts on.

use eggshell_ui::prelude::*;
use eggshell_ui::{Builder, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Event, FontFamily, Pos2, TouchDeviceId, TouchId, TouchPhase, Vec2, vec2};
use egui_kittest::Harness;

use crate::support::{self, Rig, rect_of};

const VIEWPORT: Vec2 = vec2(200.0, 300.0);
const LEAD: f32 = 220.0;
const CONTENT: f32 = 2000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Start {
    Background,
    Button,
    Field,
    Slider,
}

struct App {
    start: Start,
    text: String,
    value: f64,
    clicks: u32,
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.scroll("list", tw().w_px(VIEWPORT.x).h_px(VIEWPORT.y))
        .children(|s| {
            s.div(tw().flex().flex_col().items_stretch(), |s| {
                s.div(tw().h_px(LEAD).shrink_0().bg(Color32::LIGHT_GRAY), |_| {});
                match app.start {
                    Start::Background => {}
                    Start::Button => {
                        s.button("ctl", "Button")
                            .on_click(|app: &mut App| app.clicks += 1);
                    }
                    Start::Field => {
                        s.text_field("ctl", app.text.clone())
                            .named("Field")
                            .on_change(|app: &mut App, text| app.text = text);
                    }
                    Start::Slider => {
                        s.slider("ctl", app.value, 0.0..=100.0)
                            .named("Slider")
                            .on_change(|app: &mut App, value| app.value = value);
                    }
                }
                s.div(tw().h_px(CONTENT).shrink_0().bg(Color32::GRAY), |_| {});
            });
        });
}

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

type H = Harness<'static, Rig<App>>;

fn harness(start: Start) -> H {
    let mut h = support::harness_stepped(
        vec2(300.0, 400.0),
        1.0 / 120.0,
        base(),
        App {
            start,
            text: String::new(),
            value: 50.0,
            clicks: 0,
        },
        view,
    );
    h.run_steps(4);
    h
}

fn offset(h: &H) -> f32 {
    let id = support::id_of(h, "/\"list\"");
    support::shell(h)
        .viewport(id)
        .map_or(f32::NAN, |viewport| viewport.offset().y)
}

fn touch(h: &mut H, phase: TouchPhase, pos: Pos2) {
    h.event(Event::Touch {
        device_id: TouchDeviceId(0),
        id: TouchId(0),
        phase,
        pos,
        force: None,
    });
}

fn finger_down(h: &mut H, pos: Pos2) {
    h.event(Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    touch(h, TouchPhase::Start, pos);
    h.step();
}

fn finger_move(h: &mut H, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    touch(h, TouchPhase::Move, pos);
    h.step();
}

fn finger_up(h: &mut H, pos: Pos2) {
    h.event(Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(Event::PointerGone);
    touch(h, TouchPhase::End, pos);
    h.step();
}

struct Drag {
    /// The offset after the last move, with the finger still down.
    held: f32,
    /// The offset long after the release.
    settled: f32,
}

/// A finger that goes down at `from` and moves up by `distance` at `speed`
/// points a frame, and lets go.
fn drag(start: Start, distance: f32, speed: f32, warm: bool) -> Drag {
    let mut h = harness(start);
    if warm {
        let tap = Pos2::new(150.0, 100.0);
        finger_down(&mut h, tap);
        finger_up(&mut h, tap);
        h.run_steps(4);
    }
    let from = match start {
        Start::Background => Pos2::new(100.0, 120.0),
        _ => rect_of_control(&h).center(),
    };
    assert_eq!(offset(&h), 0.0, "precondition: at the top");
    finger_down(&mut h, from);
    let mut y = from.y;
    while from.y - y < distance {
        y -= speed.min(distance - (from.y - y));
        finger_move(&mut h, Pos2::new(from.x, y));
    }
    let held = offset(&h);
    finger_up(&mut h, Pos2::new(from.x, y));
    h.run_steps(120);
    Drag {
        held,
        settled: offset(&h),
    }
}

fn rect_of_control(h: &H) -> egui::Rect {
    let path = h
        .state()
        .nodes
        .iter()
        .map(|(path, _)| path.clone())
        .find(|path| path.ends_with("\"ctl\"") || path.contains("\"ctl\"/"))
        .expect("the control was laid out");
    rect_of(h, &path)
}

const DISTANCES: [f32; 3] = [10.0, 50.0, 200.0];
const FLICK: f32 = 10.0;

/// The content follows the finger from its first movement on: that movement
/// is taken up, as a phone holds a touch's moves back until the finger is
/// past its slop and then sends them at once.
fn assert_follows_finger(start: Start, warm: bool) {
    for distance in DISTANCES {
        let held = drag(start, distance, FLICK, warm).held;
        let followed = distance - FLICK.min(distance);
        assert!(
            (held - followed).abs() < 0.5,
            "{start:?} (warm: {warm}): a finger that moved {distance}, its first {FLICK} taken up, moved the content {held}"
        );
    }
}

#[test]
fn a_touch_drag_from_the_background_moves_the_content_by_the_finger() {
    assert_follows_finger(Start::Background, true);
}

#[test]
fn a_touch_drag_from_a_button_moves_the_content_by_the_finger() {
    assert_follows_finger(Start::Button, true);
}

#[test]
fn a_touch_drag_from_a_text_field_moves_the_content_by_the_finger() {
    assert_follows_finger(Start::Field, true);
}

#[test]
fn letting_go_of_a_drag_coasts_on_in_its_direction() {
    let d = drag(Start::Background, 50.0, FLICK, true);
    assert!(
        d.settled > d.held + 1.0,
        "held at {}, settled at {}",
        d.held,
        d.settled
    );
}

#[test]
#[ignore = "the first touch of a session scrolls nothing (#416)"]
fn the_first_touch_of_a_session_scrolls() {
    for start in [
        Start::Background,
        Start::Button,
        Start::Field,
        Start::Slider,
    ] {
        assert_follows_finger(start, false);
    }
}

#[test]
#[ignore = "a drag that starts on a slider is taken by the slider, so the container never scrolls (#416)"]
fn a_touch_drag_from_a_slider_moves_the_content_by_the_finger() {
    assert_follows_finger(Start::Slider, true);
}
