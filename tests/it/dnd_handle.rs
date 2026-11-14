//! `drag_handle`: only the handle lifts the source, and the whole source is
//! what moves, through the real path (kittest pointer events, egui's hit test,
//! the gesture, the layout and the draw).

use std::panic::{self, AssertUnwindSafe};

use eggshell_ui::testing::Headless;
use eggshell_ui::{Builder, DragEnd, LayoutState, NodeUi as _, tw};
use egui::epaint::Shape;
use egui::{Color32, CursorIcon, Event, Modifiers, PointerButton, Pos2, Rect, Ui, Vec2, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support::{self, FrameRig};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Pane(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Part {
    Grip,
    Button,
    Plot,
}

/// Where a drag that lifts a pane starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lifts {
    FromGrip,
    FromAnywhere,
}

struct App {
    panes: Vec<u32>,
    lifts: Lifts,
    clicks: u32,
    plot_drags: u32,
    ends: Vec<DragEnd>,
}

type H = Harness<'static, FrameRig<App>>;

const GRIP_FILL: Color32 = Color32::from_rgb(250, 200, 0);

fn pane_fill(key: u32) -> Color32 {
    Color32::from_rgb(40 + 60 * key as u8, 80, 120)
}

/// ```html
/// <div id="list" class="flex flex-col gap-2 p-2">
///   <div id="Pane(k)" class="flex items-start gap-2 p-3 w-[320px] h-[90px]">
///     <div id="Grip" class="w-5 h-5"/> <button id="Button">Press</button>
///     <canvas id="Plot" class="w-20 h-10"/>
///   </div> …
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.on_drag_end::<Pane>(|app, _, end| app.ends.push(end));
    b.div_id(
        "list",
        tw().flex().flex_col().gap(2.0).p(2.0).w_px(340.0),
        |b| {
            for &key in &app.panes {
                let source = b
                    .div_id(
                        Pane(key),
                        tw().flex()
                            .items_start()
                            .gap(2.0)
                            .p(3.0)
                            .w_px(320.0)
                            .h_px(90.0)
                            .bg(pane_fill(key)),
                        |b| {
                            b.div_id(Part::Grip, tw().w_px(20.0).h_px(20.0).bg(GRIP_FILL), |_| {});
                            b.button(Part::Button, "Press")
                                .on_click(|app: &mut App| app.clicks += 1);
                            b.custom(Part::Plot, tw().w_px(80.0).h_px(40.0).sense_drag())
                                .ui(|app: &mut App, ui| {
                                    if ui.node_response().dragged() {
                                        app.plot_drags += 1;
                                    }
                                });
                        },
                    )
                    .drag_source(Pane(key));
                if app.lifts == Lifts::FromGrip {
                    source.drag_handle(Part::Grip);
                }
            }
        },
    )
    .drop_target::<Pane>(|app, drop| {
        if let Some(from) = app.panes.iter().position(|k| *k == drop.payload.0)
            && let Some(to) = drop.index_hint
        {
            common::dnd::move_to(&mut app.panes, from, to);
        }
    });
}

fn frame(ui: &mut Ui, app: &mut App, layout: &mut LayoutState) {
    let area = ui.max_rect();
    layout
        .build(&ui.ctx().clone(), |b| view(b, app))
        .layout(area)
        .draw(app, ui);
}

fn harness(lifts: Lifts) -> H {
    let app = App {
        panes: vec![0, 1, 2],
        lifts,
        clicks: 0,
        plot_drags: 0,
        ends: Vec::new(),
    };
    let mut h = support::frame_harness(
        vec2(400.0, 340.0),
        common::base_text(egui::Theme::Light),
        app,
        frame,
    );
    h.run_steps(4);
    h
}

fn pane_path(key: u32) -> String {
    format!("/\"list\"/Pane({key})")
}

fn part_path(key: u32, part: Part) -> String {
    format!("{}/{part:?}", pane_path(key))
}

fn pane(h: &H, key: u32) -> Rect {
    support::layout_rect(&h.ctx, &pane_path(key))
}

fn part(h: &H, key: u32, part: Part) -> Rect {
    support::layout_rect(&h.ctx, &part_path(key, part))
}

/// A point of pane `key` on none of its children: its painted body.
fn body(h: &H, key: u32) -> Pos2 {
    let at = pane(h, key).max - vec2(12.0, 12.0);
    for child in [Part::Grip, Part::Button, Part::Plot] {
        assert!(
            !part(h, key, child).contains(at),
            "precondition: {at:?} is on the pane's body, not its {child:?}"
        );
    }
    assert!(pane(h, key).contains(at), "precondition: on the pane");
    at
}

/// The bottom half of pane 2: a drop there lands after it.
fn after_the_last(h: &H) -> Pos2 {
    pane(h, 2).center() + vec2(0.0, 20.0)
}

fn carried(h: &H) -> bool {
    eggshell_ui::drag_pointer(&h.ctx).is_some()
}

fn press_and_move(h: &mut H, from: Pos2, to: Pos2) {
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from.lerp(to, 0.5));
    h.step();
    h.hover_at(to);
    h.run_steps(2);
}

fn release(h: &mut H, at: Pos2) {
    h.drop_at(at);
    h.run_steps(3);
}

fn cursor(h: &H) -> CursorIcon {
    h.output().platform_output.cursor_icon
}

/// Where the rects painted with `fill` and `size` are.
fn painted(h: &H, fill: Color32, size: Vec2) -> Vec<Pos2> {
    h.output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(r) if r.fill == fill && (r.rect.size() - size).length() < 0.5 => {
                Some(r.rect.min)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_drag_from_the_handle_moves_the_panel() {
    let mut h = harness(Lifts::FromGrip);
    let grip = part(&h, 0, Part::Grip);
    assert!(
        pane(&h, 0).contains_rect(grip),
        "precondition: the grip is in the pane"
    );
    let to = after_the_last(&h);
    press_and_move(&mut h, grip.center(), to);
    assert!(carried(&h), "the pane is lifted");
    release(&mut h, to);
    assert_eq!(h.state().app.panes, [1, 2, 0]);
    assert_eq!(h.state().app.ends, [DragEnd::Dropped]);
}

#[test]
fn a_drag_from_the_panel_body_does_not_lift_it() {
    let mut h = harness(Lifts::FromGrip);
    let from = body(&h, 0);
    let to = after_the_last(&h);
    press_and_move(&mut h, from, to);
    assert!(!carried(&h), "nothing is lifted");
    release(&mut h, to);
    assert_eq!(h.state().app.panes, [0, 1, 2]);
    assert!(h.state().app.ends.is_empty(), "{:?}", h.state().app.ends);
}

#[test]
fn without_a_handle_a_drag_from_the_body_lifts_the_panel() {
    let mut h = harness(Lifts::FromAnywhere);
    let from = body(&h, 0);
    let to = after_the_last(&h);
    press_and_move(&mut h, from, to);
    assert!(carried(&h), "the pane is lifted");
    release(&mut h, to);
    assert_eq!(h.state().app.panes, [1, 2, 0]);
}

#[test]
fn a_button_in_the_panel_still_clicks() {
    let mut h = harness(Lifts::FromGrip);
    let at = part(&h, 0, Part::Button).center();
    assert!(
        pane(&h, 0).contains(at),
        "precondition: the button is in the pane"
    );
    h.hover_at(at);
    h.run_steps(3);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    assert_eq!(h.state().app.clicks, 1);
    assert!(h.state().app.ends.is_empty());
}

#[test]
fn a_custom_node_in_the_panel_gets_its_own_drag() {
    let mut h = harness(Lifts::FromGrip);
    let plot = part(&h, 0, Part::Plot);
    assert!(
        pane(&h, 0).contains_rect(plot),
        "precondition: the plot is in the pane"
    );
    let from = plot.center();
    press_and_move(&mut h, from, from + vec2(30.0, 0.0));
    assert!(!carried(&h), "the pane is not lifted");
    assert!(h.state().app.plot_drags > 0, "the plot was dragged");
    release(&mut h, from + vec2(30.0, 0.0));
    assert_eq!(h.state().app.panes, [0, 1, 2]);
    assert!(h.state().app.ends.is_empty());
}

#[test]
fn the_live_image_is_the_whole_panel_with_the_grab_kept() {
    let mut h = harness(Lifts::FromGrip);
    let (panel, grip) = (pane(&h, 0), part(&h, 0, Part::Grip));
    let press = grip.center();
    let grab = press - panel.min;
    assert!(
        grip.min != panel.min,
        "precondition: the grab from the panel differs from the grab from the grip"
    );
    let to = Pos2::new(250.0, 300.0);
    press_and_move(&mut h, press, to);
    assert!(carried(&h), "precondition: the pane is lifted");
    let image = painted(&h, pane_fill(0), panel.size());
    assert!(
        image.iter().any(|min| (*min - (to - grab)).length() < 1.0),
        "the panel is painted with its top-left {grab:?} behind the pointer {to:?}: {image:?}"
    );
    let grip_painted = painted(&h, GRIP_FILL, grip.size());
    let grip_in_image = to - grab + (grip.min - panel.min);
    assert!(
        grip_painted
            .iter()
            .any(|min| (*min - grip_in_image).length() < 1.0),
        "the grip moves with the panel: {grip_painted:?}, expected {grip_in_image:?}"
    );
    release(&mut h, to);
}

#[test]
fn the_cursor_grabs_over_the_handle_only() {
    let mut h = harness(Lifts::FromGrip);
    let grip = part(&h, 0, Part::Grip).center();
    h.hover_at(grip);
    h.run_steps(2);
    assert_eq!(cursor(&h), CursorIcon::Grab, "over the handle");
    let at = body(&h, 0);
    h.hover_at(at);
    h.run_steps(2);
    assert_eq!(cursor(&h), CursorIcon::Default, "over the body");
}

#[test]
fn without_a_handle_the_cursor_grabs_over_the_whole_source() {
    let mut h = harness(Lifts::FromAnywhere);
    let at = body(&h, 0);
    h.hover_at(at);
    h.run_steps(2);
    assert_eq!(cursor(&h), CursorIcon::Grab);
}

#[test]
fn the_cursor_is_grabbing_for_the_whole_gesture() {
    let mut h = harness(Lifts::FromGrip);
    let grip = part(&h, 0, Part::Grip).center();
    let outside = Pos2::new(380.0, 320.0);
    assert!(
        !pane(&h, 0).contains(outside) && !pane(&h, 2).contains(outside),
        "precondition: the pointer leaves the panes"
    );
    press_and_move(&mut h, grip, outside);
    assert!(carried(&h), "precondition: the pane is lifted");
    assert_eq!(cursor(&h), CursorIcon::Grabbing);
    release(&mut h, outside);
}

/// The message of the panic that `build` raises.
fn panic_message(build: impl FnOnce(&mut Builder<'_, '_, ()>)) -> String {
    let mut build = Some(build);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        Headless::new(vec2(200.0, 100.0)).layout(&(), |b, _| (build.take().unwrap())(b));
    }));
    let payload = result.expect_err("debug builds panic on a misplaced handle");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
        .expect("a string payload")
}

fn assert_says(message: &str, what: &str, line: u32) {
    assert!(message.contains(what), "expected {what:?} in: {message}");
    assert!(
        message.contains(&format!("dnd_handle.rs:{line}:")),
        "expected line {line} in: {message}"
    );
}

#[test]
fn a_handle_outside_the_source_is_a_build_error_at_the_call() {
    let mut line = 0;
    let message = panic_message(|b| {
        b.div_id(Part::Grip, tw(), |_| {});
        let source = b.div_id(Pane(0), tw(), |_| {}).drag_source(Pane(0));
        line = line!() + 1;
        source.drag_handle(Part::Grip);
    });
    assert_says(&message, "names no node in the source's subtree", line);
}

#[test]
fn a_handle_behind_another_identified_container_cannot_be_named() {
    let mut line = 0;
    let message = panic_message(|b| {
        let source = b
            .div_id(Pane(0), tw(), |b| {
                b.div_id("header", tw(), |b| {
                    b.div_id(Part::Grip, tw(), |_| {});
                });
            })
            .drag_source(Pane(0));
        line = line!() + 1;
        source.drag_handle(Part::Grip);
    });
    assert_says(&message, "names no node in the source's subtree", line);
}

#[test]
fn a_handle_on_a_node_that_is_not_a_source_is_a_build_error() {
    let mut line = 0;
    let message = panic_message(|b| {
        let panel = b.div_id(Pane(0), tw(), |b| {
            b.div_id(Part::Grip, tw(), |_| {});
        });
        line = line!() + 1;
        panel.drag_handle(Part::Grip);
    });
    assert_says(&message, "not a drag_source", line);
}

#[test]
fn a_handle_that_is_itself_a_source_is_a_build_error() {
    let mut line = 0;
    let message = panic_message(|b| {
        let source = b
            .div_id(Pane(0), tw(), |b| {
                b.div_id(Part::Grip, tw(), |_| {}).drag_source(Pane(1));
            })
            .drag_source(Pane(0));
        line = line!() + 1;
        source.drag_handle(Part::Grip);
    });
    assert_says(&message, "names a drag source", line);
}
