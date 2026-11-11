//! Scroll containers through build, layout, draw and the pointer: sizes,
//! culling, clipped hit testing, the wheel, nesting, programmatic scrolling,
//! the drag's auto-scroll and the absence of animation.

use core::time::Duration;

use eggshell_ui::scroll::{self, Axis};
use eggshell_ui::testing::{Timeline, record};
use eggshell_ui::{Builder, EdgeBand, EdgeScroll, ScrollSpeed, TextSpec, TextStyle, tw};
use egui::epaint::Shape;
use egui::{
    Align, Color32, Event, FontFamily, Id, Modifiers, MouseWheelUnit, Pos2, Rect, Vec2, vec2,
};
use egui_kittest::Harness;

use crate::support::{self, Rig, rect_of};

const ROW: f32 = 30.0;
const HEADER: f32 = 50.0;
const VIEWPORT_HEIGHT: f32 = 100.0;
const SIZE: Vec2 = vec2(400.0, 400.0);
/// Frames egui takes to apply a wheel event it smooths.
const SMOOTHING: usize = 30;
/// Seconds between the frames of the harness.
const STEP_DT: f32 = 0.05;

fn base() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 18.0,
            family: FontFamily::Proportional,
        },
        Color32::BLACK,
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Edge {
    Default,
    Off,
    Custom(EdgeScroll),
}

/// What the list is made of.
#[derive(Clone, Copy, PartialEq)]
enum Rows {
    Plain,
    Sources,
    Animated,
}

struct App {
    rows: u32,
    width: f32,
    axis: Axis,
    present: bool,
    edge: Edge,
    kind: Rows,
    drawn: Vec<u32>,
    clicked: Vec<u32>,
    header_clicks: u32,
    /// A spacer before the rows.
    lead: f32,
    /// A row to scroll into view, asked for in the build that adds it.
    reveal: Option<u32>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            rows: 10,
            width: 200.0,
            axis: Axis::Y,
            present: true,
            edge: Edge::Default,
            kind: Rows::Plain,
            drawn: Vec::new(),
            clicked: Vec::new(),
            header_clicks: 0,
            lead: 0.0,
            reveal: None,
        }
    }
}

fn color(i: u32) -> Color32 {
    Color32::from_rgb(10 + 20 * (i % 10) as u8, 100, 200 - 10 * (i % 10) as u8)
}

fn row_path(i: u32) -> String {
    format!("/\"list\"/{i}")
}

const LIST: &str = "/\"list\"";
const CONTENT: &str = "/\"list\"/<scrollcontent>";

/// ```html
/// <div class="flex flex-col">
///   <div class="h-[50px] w-[200px]">header</div>
///   <div class="h-[100px] overflow-y-auto" style="width: …">
///     <div class="flex flex-col"> <div class="h-[30px] bg-…">row</div> … </div>
///   </div>
/// </div>
/// ```
fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    if app.kind == Rows::Sources {
        b.on_drag_end::<u32>(|_, _, _| {});
    }
    b.div(tw().flex().flex_col(), |b| {
        b.div_id(
            "header",
            tw().h_px(HEADER).w_px(200.0).sense_click(),
            |_| {},
        )
        .on_click(|app: &mut App| app.header_clicks += 1);
        if !app.present {
            return;
        }
        let list = b.scroll("list", tw().w_px(app.width).h_px(VIEWPORT_HEIGHT));
        let (kind, vertical) = (app.kind, app.axis == Axis::Y);
        let list = match app.edge {
            Edge::Default => list,
            Edge::Off => list.no_edge_scroll(),
            Edge::Custom(edge) => list.edge_scroll(edge),
        };
        list.axis(app.axis).children(|s| {
            let strip = if vertical {
                tw().flex().flex_col()
            } else {
                tw().flex()
            };
            s.div(strip, |s| {
                s.div(tw().h_px(app.lead).shrink_0(), |_| {});
                for i in 0..app.rows {
                    if app.reveal == Some(i) {
                        scroll::scroll_into_view(s.ctx(), s.id_of(i), None);
                    }
                    let size = if vertical {
                        tw().h_px(ROW).w_full()
                    } else {
                        tw().w_px(80.0).h_full().shrink_0()
                    };
                    let mut row = size.bg(color(i)).sense_click();
                    if kind == Rows::Animated {
                        row = row.animate_layout(Duration::from_millis(200));
                    }
                    let node = s
                        .div_id(i, row, |_| {})
                        .draw(move |app: &mut App, _| app.drawn.push(i))
                        .on_click(move |app: &mut App| app.clicked.push(i));
                    if kind == Rows::Sources {
                        node.drag_source(i);
                    }
                }
            });
        });
    });
}

type H<'a> = Harness<'a, Rig<App>>;

fn harness_with(app: App) -> H<'static> {
    let mut h = support::harness(SIZE, base(), app, view);
    h.step();
    h
}

fn harness(rows: u32) -> H<'static> {
    harness_with(App {
        rows,
        ..App::default()
    })
}

fn list_id(h: &H<'_>) -> Id {
    support::id_of(h, LIST)
}

fn viewport(h: &H<'_>) -> scroll::Viewport {
    scroll::viewport(&h.ctx, list_id(h)).expect("the list was laid out")
}

fn offset(h: &H<'_>) -> Vec2 {
    viewport(h).offset()
}

fn wheel_event(delta: Vec2) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta,
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    }
}

fn wheel(h: &mut H<'_>, at: Pos2, delta: Vec2) {
    h.hover_at(at);
    h.step();
    h.event(wheel_event(delta));
    h.run_steps(SMOOTHING);
}

/// A point inside the list's viewport.
fn inside(h: &H<'_>) -> Pos2 {
    viewport(h).rect.center()
}

/// Scrolls the list down by `points` with the wheel.
fn scroll_down(h: &mut H<'_>, points: f32) {
    let at = inside(h);
    wheel(h, at, vec2(0.0, -points));
}

fn drawn(h: &mut H<'_>) -> Vec<u32> {
    h.state_mut().app.drawn.clear();
    h.step();
    let mut rows = h.state().app.drawn.clone();
    rows.sort_unstable();
    rows
}

fn lift_row(h: &mut H<'_>, row: u32) {
    let from = rect_of(h, &row_path(row)).center();
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from + vec2(0.0, 12.0));
    h.run_steps(2);
    assert!(
        eggshell_ui::drag_pointer(&h.ctx).is_some(),
        "precondition: row {row} is lifted"
    );
}

#[test]
fn the_content_is_as_wide_as_the_viewport_and_as_tall_as_it_needs_on_the_first_frame() {
    let mut h = support::harness(
        SIZE,
        base(),
        App {
            rows: 10,
            ..App::default()
        },
        view,
    );
    h.step();
    let (content, list) = (rect_of(&h, CONTENT), rect_of(&h, LIST));
    assert_eq!(list.size(), vec2(200.0, VIEWPORT_HEIGHT));
    assert_eq!(content.width(), list.width());
    assert_eq!(content.height(), 10.0 * ROW);
    assert!(
        content.height() > list.height(),
        "precondition: it overflows"
    );
}

#[test]
fn the_content_follows_the_viewports_width_in_the_frame_it_changes() {
    let mut h = harness(10);
    h.state_mut().app.width = 120.0;
    h.step();
    assert_eq!(rect_of(&h, LIST).width(), 120.0);
    assert_eq!(rect_of(&h, CONTENT).width(), 120.0);
    assert_eq!(rect_of(&h, &row_path(3)).width(), 120.0);
}

#[test]
fn the_content_height_follows_the_rows_in_the_frame_they_change() {
    let mut h = harness(10);
    h.state_mut().app.rows = 25;
    h.step();
    assert_eq!(rect_of(&h, CONTENT).height(), 25.0 * ROW);
}

#[test]
fn content_shorter_than_the_viewport_is_laid_out_at_its_height() {
    let h = harness(2);
    const { assert!(2.0 * ROW < VIEWPORT_HEIGHT) };
    assert_eq!(
        rect_of(&h, CONTENT).height(),
        VIEWPORT_HEIGHT,
        "it fills the viewport"
    );
    assert_eq!(rect_of(&h, &row_path(1)).height(), ROW);
}

#[test]
fn a_horizontal_container_is_as_tall_as_the_viewport_and_as_wide_as_it_needs() {
    let h = harness_with(App {
        axis: Axis::X,
        rows: 10,
        ..App::default()
    });
    let content = rect_of(&h, CONTENT);
    assert_eq!(content.height(), VIEWPORT_HEIGHT);
    assert_eq!(content.width(), 10.0 * 80.0);
}

#[test]
fn a_container_that_scrolls_both_ways_is_as_large_as_it_needs_in_both() {
    fn grid(b: &mut Builder<'_, '_, App>, _: &App) {
        b.scroll("list", tw().w_px(200.0).h_px(100.0))
            .axis(Axis::Both)
            .children(|s| {
                s.div(tw().flex().flex_col(), |s| {
                    for r in 0..10u32 {
                        s.div_id(r, tw().flex().h_px(ROW), |s| {
                            for c in 0..10u32 {
                                s.div_id(c, tw().w_px(80.0).h_px(ROW).shrink_0(), |_| {});
                            }
                        });
                    }
                });
            });
    }
    let mut h = support::harness(SIZE, base(), App::default(), grid);
    h.step();
    assert_eq!(rect_of(&h, CONTENT).size(), vec2(800.0, 300.0));
}

#[test]
fn padding_scrolls_with_the_content_and_the_border_stays_put() {
    fn padded(b: &mut Builder<'_, '_, App>, _: &App) {
        b.scroll(
            "list",
            tw().w_px(200.0)
                .h_px(100.0)
                .p(2.5)
                .border(2.0)
                .border_color(Color32::RED),
        )
        .children(|s| {
            for i in 0..10u32 {
                s.div_id(i, tw().h_px(ROW), |_| {});
            }
        });
    }
    let mut h = support::harness(SIZE, base(), App::default(), padded);
    h.step();
    let (list, content, first) = (
        rect_of(&h, LIST),
        rect_of(&h, CONTENT),
        rect_of(&h, &row_path(0)),
    );
    assert_eq!(content.min, list.min + vec2(2.0, 2.0), "inside the border");
    assert_eq!(
        first.min,
        content.min + vec2(10.0, 10.0),
        "inside the padding"
    );
    assert_eq!(
        content.height(),
        10.0 * ROW + 20.0,
        "the padding is part of the content"
    );
    assert_eq!(content.width(), 196.0, "the viewport is inside the border");
}

#[test]
fn the_scroll_bar_does_not_narrow_the_content() {
    let mut h = harness(10);
    let at = inside(&h);
    h.hover_at(at);
    h.run_steps(4);
    assert_eq!(rect_of(&h, CONTENT).width(), 200.0);
    assert_eq!(rect_of(&h, &row_path(0)).width(), 200.0);
}

#[test]
fn nodes_outside_the_viewport_are_not_drawn_and_their_closures_do_not_run() {
    let mut h = harness(10);
    assert!(
        rect_of(&h, &row_path(8)).min.y > rect_of(&h, LIST).max.y,
        "precondition: row 8 is far below the viewport"
    );
    assert_eq!(
        drawn(&mut h),
        vec![0, 1, 2, 3],
        "only the rows that reach it"
    );
    let fills: Vec<Color32> = support::shapes(&h)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.fill),
            _ => None,
        })
        .collect();
    assert!(fills.contains(&color(0)));
    assert!(
        !fills.contains(&color(8)),
        "and nothing of row 8 is painted"
    );
}

#[test]
fn scrolling_changes_which_nodes_are_drawn() {
    let mut h = harness(10);
    scroll_down(&mut h, 60.0);
    assert_eq!(offset(&h), vec2(0.0, 60.0), "precondition: it scrolled");
    let rows = drawn(&mut h);
    assert!(!rows.contains(&0), "row 0 is gone: {rows:?}");
    assert!(
        rows.starts_with(&[1, 2]) || rows.starts_with(&[2]),
        "{rows:?}"
    );
    for row in 2..=5 {
        assert!(rows.contains(&row), "row {row} is in view: {rows:?}");
    }
    assert!(!rows.contains(&7), "row 7 is not: {rows:?}");
}

#[test]
fn the_node_with_the_keyboard_focus_is_drawn_while_scrolled_out() {
    let mut h = harness(10);
    let id = support::id_of(&h, &row_path(0));
    h.key_press(egui::Key::Tab);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    assert!(
        h.ctx.memory(|memory| memory.has_focus(id)),
        "precondition: row 0 has the focus"
    );
    scroll_down(&mut h, 200.0);
    assert!(
        rect_of(&h, &row_path(0)).max.y < rect_of(&h, LIST).min.y,
        "precondition: row 0 is scrolled out"
    );
    assert!(drawn(&mut h).contains(&0), "the focused row is still drawn");
    assert!(
        !drawn(&mut h).contains(&1),
        "while an unfocused row that is scrolled out is not"
    );
}

#[test]
fn the_offset_is_clamped_to_the_content() {
    let mut h = harness(10);
    scroll_down(&mut h, 5000.0);
    assert_eq!(offset(&h).y, 10.0 * ROW - VIEWPORT_HEIGHT);
    assert_eq!(
        rect_of(&h, &row_path(9)).max.y,
        rect_of(&h, LIST).max.y,
        "the last row ends at the bottom"
    );
}

#[test]
fn the_offset_follows_a_shrinking_content_in_the_same_frame() {
    let mut h = harness(20);
    scroll_down(&mut h, 5000.0);
    assert_eq!(offset(&h).y, 20.0 * ROW - VIEWPORT_HEIGHT, "precondition");
    h.state_mut().app.rows = 5;
    h.step();
    assert_eq!(offset(&h).y, 5.0 * ROW - VIEWPORT_HEIGHT);
    assert_eq!(rect_of(&h, &row_path(4)).max.y, rect_of(&h, LIST).max.y);
}

#[test]
fn egui_keeps_the_scroll_state_through_rebuilds_and_absence() {
    let mut h = harness(10);
    scroll_down(&mut h, 80.0);
    let scrolled = offset(&h);
    assert_ne!(scrolled, Vec2::ZERO, "precondition");
    h.run_steps(3);
    assert_eq!(
        offset(&h),
        scrolled,
        "frames rebuild the tree and keep the offset"
    );
    h.state_mut().app.present = false;
    h.run_steps(2);
    h.state_mut().app.present = true;
    h.step();
    assert_eq!(
        offset(&h),
        scrolled,
        "also for a container that was gone for a while"
    );
}

#[test]
fn scrolled_out_nodes_take_no_clicks() {
    let mut h = harness(10);
    scroll_down(&mut h, 60.0);
    let (list, row1, header) = (
        rect_of(&h, LIST),
        rect_of(&h, &row_path(1)),
        rect_of(&h, "/\"header\""),
    );
    assert!(
        row1.max.y <= list.min.y,
        "precondition: row 1 is above the viewport"
    );
    assert!(
        header.contains(row1.center()),
        "precondition: row 1's layout rect is over the header"
    );
    support::press_and_release(&mut h, row1.center());
    h.run_steps(2);
    assert_eq!(h.state().app.header_clicks, 1, "the header gets the click");
    assert!(h.state().app.clicked.is_empty(), "the clipped row does not");
}

#[test]
fn a_half_clipped_node_takes_clicks_only_on_its_visible_part() {
    let mut h = harness(10);
    scroll_down(&mut h, 15.0);
    let (list, row0) = (rect_of(&h, LIST), rect_of(&h, &row_path(0)));
    assert!(
        row0.min.y < list.min.y && row0.max.y > list.min.y,
        "precondition: row 0 is cut by the top of the viewport"
    );
    support::press_and_release(&mut h, Pos2::new(row0.center().x, list.min.y + 3.0));
    h.run_steps(2);
    assert_eq!(
        h.state().app.clicked,
        vec![0],
        "its visible part takes the click"
    );
    support::press_and_release(&mut h, Pos2::new(row0.center().x, list.min.y - 3.0));
    h.run_steps(2);
    assert_eq!(h.state().app.clicked, vec![0], "its clipped part does not");
    assert_eq!(h.state().app.header_clicks, 1, "the header does");
}

#[test]
fn the_wheel_scrolls_while_another_widget_is_dragged() {
    let mut h = harness_with(App {
        kind: Rows::Sources,
        ..App::default()
    });
    lift_row(&mut h, 1);
    let at = h
        .ctx
        .input(|i| i.pointer.interact_pos())
        .expect("the pointer is down");
    assert!(
        viewport(&h).rect.contains(at),
        "precondition: the pointer is over the viewport"
    );
    h.event(wheel_event(vec2(0.0, -50.0)));
    h.run_steps(SMOOTHING);
    assert!(offset(&h).y > 20.0, "the wheel scrolled: {:?}", offset(&h));
    assert!(
        eggshell_ui::drag_pointer(&h.ctx).is_some(),
        "and the drag goes on"
    );
}

#[test]
fn scrolling_alone_starts_no_slide_and_the_timeline_is_continuous() {
    let mut h = harness_with(App {
        kind: Rows::Animated,
        rows: 20,
        ..App::default()
    });
    record(&h.ctx);
    h.run_steps(2);
    let at = inside(&h);
    h.hover_at(at);
    h.step();
    for delta in [-40.0, -40.0, 30.0, -90.0, 25.0] {
        h.event(wheel_event(vec2(0.0, delta)));
        h.run_steps(8);
    }
    h.run_steps(30);
    assert!(offset(&h).y > 0.0, "precondition: it scrolled");
    Timeline::recorded(&h.ctx).assert_continuous();
    let mut checked = 0;
    for i in 0..20 {
        if let Some(node) = support::last_drawn(&h.ctx, &row_path(i)) {
            checked += 1;
            assert!(
                node.slide.is_none_or(|slide| slide.offset == Vec2::ZERO),
                "row {i} slid"
            );
            assert_eq!(
                node.painted_rect, node.layout_rect,
                "row {i} is painted where it is laid out"
            );
        }
    }
    assert!(checked >= 3, "precondition: rows were checked: {checked}");
}

#[test]
fn rows_that_move_inside_a_scrolled_container_still_slide() {
    let mut h = harness_with(App {
        kind: Rows::Animated,
        rows: 20,
        ..App::default()
    });
    record(&h.ctx);
    scroll_down(&mut h, 60.0);
    h.run_steps(20);
    let row = |h: &H<'_>| support::last_drawn(&h.ctx, &row_path(4)).expect("row 4 is in view");
    let rest = row(&h);
    assert_eq!(rest.painted_rect, rest.layout_rect, "precondition: at rest");
    h.state_mut().app.lead = 30.0;
    h.step();
    let moved = row(&h);
    assert_eq!(
        moved.layout_rect.min.y,
        rest.layout_rect.min.y + 30.0,
        "layout moved at once"
    );
    assert!(
        (moved.painted_rect.min.y - rest.layout_rect.min.y).abs() < 10.0,
        "and the row is still painted near where it was: {:?} vs {:?}",
        moved.painted_rect,
        moved.layout_rect
    );
    h.run_steps(30);
    let settled = row(&h);
    assert_eq!(settled.painted_rect, settled.layout_rect, "it arrives");
    Timeline::recorded(&h.ctx).assert_continuous();
}

#[test]
fn scroll_by_moves_the_viewport_and_stays_within_the_content() {
    let mut h = harness(10);
    let id = list_id(&h);
    scroll::scroll_by(&h.ctx, id, vec2(0.0, 40.0));
    h.step();
    assert_eq!(offset(&h), vec2(0.0, 40.0));
    scroll::scroll_by(&h.ctx, id, vec2(0.0, -100.0));
    h.step();
    assert_eq!(offset(&h), Vec2::ZERO, "not above the start");
    scroll::scroll_by(&h.ctx, id, vec2(0.0, 1e6));
    h.step();
    assert_eq!(
        offset(&h).y,
        10.0 * ROW - VIEWPORT_HEIGHT,
        "not past the end"
    );
}

#[test]
fn the_content_origin_and_viewport_are_readable() {
    let mut h = harness(10);
    scroll::scroll_by(&h.ctx, list_id(&h), vec2(0.0, 45.0));
    h.step();
    let v = viewport(&h);
    assert_eq!(v.rect, rect_of(&h, LIST));
    assert_eq!(v.content, rect_of(&h, CONTENT));
    assert_eq!(v.content.min, v.rect.min - vec2(0.0, 45.0));
    let row3 = rect_of(&h, &row_path(3));
    assert_eq!(
        v.to_content(row3),
        Rect::from_min_size(Pos2::new(0.0, 3.0 * ROW), vec2(200.0, ROW))
    );
    assert_eq!(v.to_screen(v.to_content(row3)), row3);
}

#[test]
fn scroll_to_rect_aligns_a_rect_in_content_coordinates() {
    let mut h = harness(10);
    let id = list_id(&h);
    let row = Rect::from_min_size(Pos2::new(0.0, 6.0 * ROW), vec2(200.0, ROW));
    scroll::scroll_to_rect(&h.ctx, id, row, Some(Align::Min));
    h.step();
    assert_eq!(
        rect_of(&h, &row_path(6)).min.y,
        rect_of(&h, LIST).min.y,
        "its top at the top"
    );
    scroll::scroll_to_rect(&h.ctx, id, row, Some(Align::Max));
    h.step();
    assert_eq!(
        rect_of(&h, &row_path(6)).max.y,
        rect_of(&h, LIST).max.y,
        "its bottom at the bottom"
    );
    scroll::scroll_to_rect(&h.ctx, id, row, Some(Align::Center));
    h.step();
    assert!(
        (rect_of(&h, &row_path(6)).center().y - rect_of(&h, LIST).center().y).abs() < 0.5,
        "its centre in the middle"
    );
}

#[test]
fn scroll_to_rect_without_alignment_moves_the_least() {
    let mut h = harness(10);
    let id = list_id(&h);
    let row = |n: u32| Rect::from_min_size(Pos2::new(0.0, n as f32 * ROW), vec2(200.0, ROW));
    scroll::scroll_to_rect(&h.ctx, id, row(2), None);
    h.step();
    assert_eq!(offset(&h), Vec2::ZERO, "a rect in view does not move it");
    scroll::scroll_to_rect(&h.ctx, id, row(6), None);
    h.step();
    assert_eq!(
        rect_of(&h, &row_path(6)).max.y,
        rect_of(&h, LIST).max.y,
        "below: its bottom shows"
    );
    scroll::scroll_to_rect(&h.ctx, id, row(0), None);
    h.step();
    assert_eq!(offset(&h), Vec2::ZERO, "above: its top shows");
}

#[test]
fn scroll_into_view_shows_a_node_the_same_frame_adds() {
    let mut h = harness(10);
    h.state_mut().app.rows = 30;
    h.state_mut().app.reveal = Some(29);
    h.step();
    assert_eq!(
        rect_of(&h, &row_path(29)).max.y,
        rect_of(&h, LIST).max.y,
        "row 29 is in view in the frame that built it"
    );
}

#[test]
fn a_drag_near_the_edge_scrolls_the_container_by_itself() {
    let mut h = harness_with(App {
        kind: Rows::Sources,
        rows: 20,
        ..App::default()
    });
    lift_row(&mut h, 1);
    let list = rect_of(&h, LIST);
    h.hover_at(Pos2::new(list.center().x, list.max.y - 3.0));
    h.run_steps(20);
    assert!(offset(&h).y > 30.0, "scrolled down: {:?}", offset(&h));
    h.hover_at(Pos2::new(list.center().x, list.min.y + 3.0));
    h.run_steps(60);
    assert!(offset(&h).y < 5.0, "and back up: {:?}", offset(&h));
    assert!(
        eggshell_ui::drag_pointer(&h.ctx).is_some(),
        "the drag is still live"
    );
}

#[test]
fn the_drag_scrolls_at_the_speed_of_the_edge_scroll_and_requests_frames() {
    let mut h = harness_with(App {
        kind: Rows::Sources,
        rows: 40,
        ..App::default()
    });
    lift_row(&mut h, 1);
    let list = rect_of(&h, LIST);
    h.hover_at(Pos2::new(list.center().x, list.max.y));
    h.run_steps(2);
    let before = offset(&h).y;
    let steps = 12;
    h.run_steps(steps);
    let travelled = offset(&h).y - before;
    let expected = 600.0 * steps as f32 * STEP_DT;
    assert!(
        (travelled - expected).abs() < expected * 0.35,
        "about 600 points per second at the edge: {travelled} vs {expected}"
    );
    assert!(
        h.ctx.requested_repaint_last_pass(),
        "a frame is requested while it scrolls"
    );
}

#[test]
fn the_edge_scroll_can_be_changed_or_turned_off() {
    let held_near_the_bottom = |edge: Edge| {
        let mut h = harness_with(App {
            kind: Rows::Sources,
            rows: 400,
            edge,
            ..App::default()
        });
        lift_row(&mut h, 1);
        let list = rect_of(&h, LIST);
        h.hover_at(Pos2::new(list.center().x, list.max.y - 2.0));
        h.run_steps(30);
        offset(&h).y
    };
    let default = held_near_the_bottom(Edge::Default);
    assert!(
        default > 50.0,
        "precondition: the default scrolls: {default}"
    );
    assert_eq!(held_near_the_bottom(Edge::Off), 0.0, "off does not");
    let faster = held_near_the_bottom(Edge::Custom(EdgeScroll {
        band: EdgeBand::new(48.0),
        speed: ScrollSpeed::new(1800.0),
    }));
    assert!(
        faster > default * 2.0,
        "faster scrolls farther: {faster} vs {default}"
    );
    let narrow = held_near_the_bottom(Edge::Custom(EdgeScroll {
        band: EdgeBand::new(1.0),
        speed: ScrollSpeed::new(600.0),
    }));
    assert_eq!(narrow, 0.0, "a band of a point does not reach the pointer");
}

#[test]
fn without_a_drag_the_edge_does_not_scroll() {
    let mut h = harness(40);
    let list = rect_of(&h, LIST);
    h.hover_at(Pos2::new(list.center().x, list.max.y - 2.0));
    h.run_steps(30);
    assert_eq!(offset(&h), Vec2::ZERO);
}

mod nested {
    use super::*;

    #[derive(Clone, Copy, PartialEq)]
    enum Inner {
        Vertical,
        Horizontal,
    }

    struct Page {
        inner: Inner,
    }

    const OUTER: &str = "/\"outer\"";
    const INNER: &str = "/\"outer\"/\"inner\"";

    /// ```html
    /// <div class="h-[160px] w-[200px] overflow-y-auto">
    ///   <div class="h-[40px]"/>
    ///   <div class="h-[80px] w-[160px] overflow-…"> 20 boxes of 80 by 30 </div>
    ///   <div class="h-[400px]"/>
    /// </div>
    /// ```
    fn page(b: &mut Builder<'_, '_, Page>, page: &Page) {
        let inner = page.inner;
        b.scroll("outer", tw().w_px(200.0).h_px(160.0))
            .children(|s| {
                s.div(tw().h_px(40.0), |_| {});
                let list = s.scroll("inner", tw().w_px(160.0).h_px(80.0));
                let list = match inner {
                    Inner::Vertical => list,
                    Inner::Horizontal => list.axis(Axis::X),
                };
                list.children(|s| {
                    let strip = match inner {
                        Inner::Vertical => tw().flex().flex_col(),
                        Inner::Horizontal => tw().flex(),
                    };
                    s.div(strip, |s| {
                        for i in 0..20u32 {
                            s.div_id(i, tw().w_px(80.0).h_px(30.0).shrink_0(), |_| {});
                        }
                    });
                });
                s.div(tw().h_px(400.0), |_| {});
            });
    }

    type P<'a> = Harness<'a, Rig<Page>>;

    fn harness(inner: Inner) -> P<'static> {
        let mut h = support::harness(SIZE, base(), Page { inner }, page);
        h.run_steps(2);
        h
    }

    fn offset_of(h: &P<'_>, path: &str) -> Vec2 {
        let id = support::id_of(h, path);
        scroll::viewport(&h.ctx, id).expect("laid out").offset()
    }

    fn wheel_over_inner(h: &mut P<'_>, delta: Vec2) {
        let at = rect_of(h, INNER).center();
        h.hover_at(at);
        h.step();
        h.event(wheel_event(delta));
        h.run_steps(SMOOTHING);
    }

    #[test]
    fn the_inner_container_is_laid_out_inside_the_outer_content() {
        let h = harness(Inner::Vertical);
        let (outer, inner) = (rect_of(&h, OUTER), rect_of(&h, INNER));
        assert_eq!(inner.min, outer.min + vec2(0.0, 40.0));
        assert_eq!(
            rect_of(&h, &format!("{INNER}/<scrollcontent>")).height(),
            20.0 * 30.0
        );
        assert_eq!(
            rect_of(&h, &format!("{OUTER}/<scrollcontent>")).height(),
            40.0 + 80.0 + 400.0
        );
    }

    #[test]
    fn on_the_same_axis_the_inner_scrolls_first_then_the_outer_takes_over() {
        let mut h = harness(Inner::Vertical);
        wheel_over_inner(&mut h, vec2(0.0, -40.0));
        assert_eq!(offset_of(&h, INNER).y, 40.0, "the inner moved");
        assert_eq!(offset_of(&h, OUTER), Vec2::ZERO, "and the outer did not");
        for _ in 0..8 {
            wheel_over_inner(&mut h, vec2(0.0, -200.0));
        }
        assert_eq!(
            offset_of(&h, INNER).y,
            20.0 * 30.0 - 80.0,
            "the inner reached its end"
        );
        assert!(
            offset_of(&h, OUTER).y > 0.0,
            "and the outer scrolled on from there"
        );
    }

    #[test]
    fn a_horizontal_inner_container_takes_horizontal_gestures_and_leaves_vertical_ones() {
        let mut h = harness(Inner::Horizontal);
        wheel_over_inner(&mut h, vec2(-40.0, 0.0));
        assert_eq!(offset_of(&h, INNER).x, 40.0, "the carousel moved sideways");
        assert_eq!(offset_of(&h, OUTER), Vec2::ZERO, "the page did not");
        wheel_over_inner(&mut h, vec2(0.0, -40.0));
        assert_eq!(
            offset_of(&h, INNER).y,
            0.0,
            "a vertical gesture does not move the carousel"
        );
        assert_eq!(offset_of(&h, OUTER).y, 40.0, "it moves the page");
    }

    #[test]
    fn a_drag_scrolls_the_innermost_container_that_can_move() {
        let mut h = harness(Inner::Vertical);
        let inner = rect_of(&h, INNER);
        h.input_mut().hovered_files = vec![egui::HoveredFile {
            path: Some("a.png".into()),
            mime: String::new(),
        }];
        h.hover_at(Pos2::new(inner.center().x, inner.max.y - 2.0));
        h.run_steps(20);
        assert!(
            offset_of(&h, INNER).y > 20.0,
            "the inner scrolled: {:?}",
            offset_of(&h, INNER)
        );
        assert_eq!(offset_of(&h, OUTER), Vec2::ZERO, "the outer did not");
    }

    #[test]
    fn a_drag_at_the_end_of_the_inner_container_scrolls_the_outer_one() {
        let mut h = harness(Inner::Vertical);
        scroll::scroll_by(&h.ctx, support::id_of(&h, INNER), vec2(0.0, 1e6));
        h.step();
        assert_eq!(
            offset_of(&h, INNER).y,
            20.0 * 30.0 - 80.0,
            "precondition: at its end"
        );
        let inner = rect_of(&h, INNER);
        h.input_mut().hovered_files = vec![egui::HoveredFile {
            path: Some("a.png".into()),
            mime: String::new(),
        }];
        h.hover_at(Pos2::new(inner.center().x, inner.max.y - 2.0));
        h.run_steps(20);
        assert!(offset_of(&h, OUTER).y > 10.0, "the outer took over");
    }

    #[test]
    fn scroll_into_view_crosses_every_ancestor() {
        let mut h = harness(Inner::Vertical);
        let row = support::id_of(&h, &format!("{INNER}/15"));
        scroll::scroll_into_view(&h.ctx, row, Some(Align::Min));
        h.step();
        let (row, inner, outer) = (
            rect_of(&h, &format!("{INNER}/15")),
            rect_of(&h, INNER),
            rect_of(&h, OUTER),
        );
        assert_eq!(
            row.min.y, inner.min.y,
            "shown at the top of the inner container"
        );
        assert_eq!(
            inner.min.y, outer.min.y,
            "whose top is shown at the top of the outer one"
        );
    }
}
