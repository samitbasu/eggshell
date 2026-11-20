//! `drag_handle`: only the handle lifts the source, and the whole source is
//! what moves, through the real path (kittest pointer events, egui's hit test,
//! the gesture, the layout and the draw).

use eggshell_ui::testing::{Driver, Headless, PlacedFrame, assert_reported};
use eggshell_ui::{Builder, DragEnd, LayoutState, NodeUi as _, tw};
use eggshell_ui::{Problem, Report};
use egui::epaint::Shape;
use egui::{Color32, CursorIcon, Event, Modifiers, PointerButton, Pos2, Rect, Ui, Vec2, vec2};

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support;

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

type H = Driver<App>;

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

fn frame(ui: &mut Ui, app: &mut App, layout: &mut LayoutState) -> PlacedFrame {
    let area = ui.max_rect();
    layout
        .build(&ui.ctx().clone(), |b| view(b, app))
        .layout(area)
        .draw(app, ui)
}

fn harness(lifts: Lifts) -> H {
    let app = App {
        panes: vec![0, 1, 2],
        lifts,
        clicks: 0,
        plot_drags: 0,
        ends: Vec::new(),
    };
    let mut d = support::recorded(support::frame_driver(
        vec2(400.0, 340.0),
        common::base_text(egui::Theme::Light),
        app,
        frame,
    ));
    d.frames(4);
    d
}

fn pane_path(key: u32) -> String {
    format!("/\"list\"/Pane({key})")
}

fn part_path(key: u32, part: Part) -> String {
    format!("{}/{part:?}", pane_path(key))
}

fn pane(d: &H, key: u32) -> Rect {
    support::layout_rect(d.ctx(), &pane_path(key))
}

fn part(d: &H, key: u32, part: Part) -> Rect {
    support::layout_rect(d.ctx(), &part_path(key, part))
}

/// A point of pane `key` on none of its children: its painted body.
fn body(d: &H, key: u32) -> Pos2 {
    let at = pane(d, key).max - vec2(12.0, 12.0);
    for child in [Part::Grip, Part::Button, Part::Plot] {
        assert!(
            !part(d, key, child).contains(at),
            "precondition: {at:?} is on the pane's body, not its {child:?}"
        );
    }
    assert!(pane(d, key).contains(at), "precondition: on the pane");
    at
}

/// The bottom half of pane 2: a drop there lands after it.
fn after_the_last(d: &H) -> Pos2 {
    pane(d, 2).center() + vec2(0.0, 20.0)
}

fn carried(d: &H) -> bool {
    d.shell().drag_pointer().is_some()
}

fn press_and_move(d: &mut H, from: Pos2, to: Pos2) {
    d.hover(from);
    d.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    d.carry_to(from + vec2(0.0, 6.0));
    d.carry_to(from.lerp(to, 0.5));
    d.carry_to(to).frames(1);
}

fn release(d: &mut H, at: Pos2) {
    d.pointer_up(at).event(Event::PointerGone).frames(2);
}

fn cursor(d: &H) -> CursorIcon {
    d.harness().output().platform_output.cursor_icon
}

/// Where the rects painted with `fill` and `size` are.
fn painted(d: &H, fill: Color32, size: Vec2) -> Vec<Pos2> {
    d.harness()
        .output()
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
    let mut d = harness(Lifts::FromGrip);
    let grip = part(&d, 0, Part::Grip);
    assert!(
        pane(&d, 0).contains_rect(grip),
        "precondition: the grip is in the pane"
    );
    let to = after_the_last(&d);
    press_and_move(&mut d, grip.center(), to);
    assert!(carried(&d), "the pane is lifted");
    release(&mut d, to);
    assert_eq!(d.app().panes, [1, 2, 0]);
    assert_eq!(d.app().ends, [DragEnd::Dropped]);
}

#[test]
fn a_drag_from_the_panel_body_does_not_lift_it() {
    let mut d = harness(Lifts::FromGrip);
    let from = body(&d, 0);
    let to = after_the_last(&d);
    press_and_move(&mut d, from, to);
    assert!(!carried(&d), "nothing is lifted");
    release(&mut d, to);
    assert_eq!(d.app().panes, [0, 1, 2]);
    assert!(d.app().ends.is_empty(), "{:?}", d.app().ends);
}

#[test]
fn without_a_handle_a_drag_from_the_body_lifts_the_panel() {
    let mut d = harness(Lifts::FromAnywhere);
    let from = body(&d, 0);
    let to = after_the_last(&d);
    press_and_move(&mut d, from, to);
    assert!(carried(&d), "the pane is lifted");
    release(&mut d, to);
    assert_eq!(d.app().panes, [1, 2, 0]);
}

#[test]
fn a_button_in_the_panel_still_clicks() {
    let mut d = harness(Lifts::FromGrip);
    let at = part(&d, 0, Part::Button).center();
    assert!(
        pane(&d, 0).contains(at),
        "precondition: the button is in the pane"
    );
    d.hover(at);
    for pressed in [true, false] {
        d.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    assert_eq!(d.app().clicks, 1);
    assert!(d.app().ends.is_empty());
}

#[test]
fn a_click_on_a_button_in_a_whole_panel_source_lifts_nothing() {
    let mut d = harness(Lifts::FromAnywhere);
    let at = part(&d, 0, Part::Button).center();
    assert!(
        pane(&d, 0).contains(at),
        "precondition: the button is in the pane"
    );
    d.hover(at);
    for pressed in [true, false] {
        d.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        assert!(!carried(&d), "nothing is lifted (pressed: {pressed})");
    }
    d.frames(3);
    assert_eq!(d.app().clicks, 1);
    assert!(d.app().ends.is_empty(), "{:?}", d.app().ends);
}

#[test]
fn a_custom_node_in_the_panel_gets_its_own_drag() {
    let mut d = harness(Lifts::FromGrip);
    let plot = part(&d, 0, Part::Plot);
    assert!(
        pane(&d, 0).contains_rect(plot),
        "precondition: the plot is in the pane"
    );
    let from = plot.center();
    press_and_move(&mut d, from, from + vec2(30.0, 0.0));
    assert!(!carried(&d), "the pane is not lifted");
    assert!(d.app().plot_drags > 0, "the plot was dragged");
    release(&mut d, from + vec2(30.0, 0.0));
    assert_eq!(d.app().panes, [0, 1, 2]);
    assert!(d.app().ends.is_empty());
}

#[test]
fn the_live_image_is_the_whole_panel_with_the_grab_kept() {
    let mut d = harness(Lifts::FromGrip);
    let (panel, grip) = (pane(&d, 0), part(&d, 0, Part::Grip));
    let press = grip.center();
    let grab = press - panel.min;
    assert!(
        grip.min != panel.min,
        "precondition: the grab from the panel differs from the grab from the grip"
    );
    let to = Pos2::new(250.0, 300.0);
    press_and_move(&mut d, press, to);
    assert!(carried(&d), "precondition: the pane is lifted");
    let image = painted(&d, pane_fill(0), panel.size());
    assert!(
        image.iter().any(|min| (*min - (to - grab)).length() < 1.0),
        "the panel is painted with its top-left {grab:?} behind the pointer {to:?}: {image:?}"
    );
    let grip_painted = painted(&d, GRIP_FILL, grip.size());
    let grip_in_image = to - grab + (grip.min - panel.min);
    assert!(
        grip_painted
            .iter()
            .any(|min| (*min - grip_in_image).length() < 1.0),
        "the grip moves with the panel: {grip_painted:?}, expected {grip_in_image:?}"
    );
    release(&mut d, to);
}

#[test]
fn the_cursor_grabs_over_the_handle_only() {
    let mut d = harness(Lifts::FromGrip);
    let grip = part(&d, 0, Part::Grip).center();
    d.carry_to(grip).frames(1);
    assert_eq!(cursor(&d), CursorIcon::Grab, "over the handle");
    let at = body(&d, 0);
    d.carry_to(at).frames(1);
    assert_eq!(cursor(&d), CursorIcon::Default, "over the body");
}

#[test]
fn without_a_handle_the_cursor_grabs_over_the_whole_source() {
    let mut d = harness(Lifts::FromAnywhere);
    let at = body(&d, 0);
    d.carry_to(at).frames(1);
    assert_eq!(cursor(&d), CursorIcon::Grab);
}

#[test]
fn the_cursor_is_grabbing_for_the_whole_gesture() {
    let mut d = harness(Lifts::FromGrip);
    let grip = part(&d, 0, Part::Grip).center();
    let outside = Pos2::new(380.0, 320.0);
    assert!(
        !pane(&d, 0).contains(outside) && !pane(&d, 2).contains(outside),
        "precondition: the pointer leaves the panes"
    );
    press_and_move(&mut d, grip, outside);
    assert!(carried(&d), "precondition: the pane is lifted");
    assert_eq!(cursor(&d), CursorIcon::Grabbing);
    release(&mut d, outside);
}

/// The report of a misplaced handle, which must be the only report.
fn reported(build: impl FnOnce(&mut Builder<'_, '_, ()>)) -> Report {
    let mut build = Some(build);
    let mut headless = Headless::new(vec2(200.0, 100.0));
    headless.layout(&(), |b, _| {
        if let Some(build) = build.take() {
            build(b);
        }
    });
    let report = assert_reported(headless.ctx(), Problem::DragHandle);
    assert_eq!(headless.reports().len(), 1, "{:#?}", headless.reports());
    report
}

fn assert_says(report: &Report, what: &str, line: u32) {
    assert!(
        report.message.contains(what),
        "expected {what:?} in: {}",
        report.message
    );
    assert_eq!((report.at.file(), report.at.line()), (file!(), line));
}

#[test]
fn a_handle_outside_the_source_is_reported_at_the_call() {
    let mut line = 0;
    let report = reported(|b| {
        b.div_id(Part::Grip, tw(), |_| {});
        let source = b.div_id(Pane(0), tw(), |_| {}).drag_source(Pane(0));
        line = line!() + 1;
        source.drag_handle(Part::Grip);
    });
    assert_says(&report, "names no node in the source's subtree", line);
}

#[test]
fn a_handle_behind_another_identified_container_cannot_be_named() {
    let mut line = 0;
    let report = reported(|b| {
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
    assert_says(&report, "names no node in the source's subtree", line);
}

#[test]
fn a_handle_that_is_itself_a_source_is_reported() {
    let mut line = 0;
    let report = reported(|b| {
        let source = b
            .div_id(Pane(0), tw(), |b| {
                b.div_id(Part::Grip, tw(), |_| {}).drag_source(Pane(1));
            })
            .drag_source(Pane(0));
        line = line!() + 1;
        source.drag_handle(Part::Grip);
    });
    assert_says(&report, "names a drag source", line);
}
