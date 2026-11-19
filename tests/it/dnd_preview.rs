//! `drag_preview`: an app-built drag image, through the real harness path.

use std::cell::Cell;
use std::rc::Rc;

use eggshell_ui::testing::{Driver, Timeline};
use eggshell_ui::{LayoutState, tw};
use egui::epaint::Shape;
use egui::{Color32, Event, Key, Pos2, Rect, Ui, vec2};

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support;

const PREVIEW_FILL: Color32 = Color32::from_rgb(200, 30, 30);
const PREVIEW_SIZE: egui::Vec2 = vec2(60.0, 20.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Item(u32);

struct App {
    items: Vec<u32>,
    /// How many times the preview closure ran.
    built: Rc<Cell<u32>>,
}

fn view(b: &mut eggshell_ui::Builder<'_, '_, App>, app: &App) {
    let carried = b.dragging::<Item>();
    b.div_id(
        "grid",
        tw().flex()
            .flex_row()
            .flex_wrap()
            .w_px(440.0)
            .gap(2.0)
            .p(2.0),
        |b| {
            let mut order: Vec<u32> = app.items.clone();
            if let Some(drag) = &carried
                && let Some(to) = drag.index_hint()
                && let Some(from) = order.iter().position(|k| *k == drag.payload.0)
            {
                common::dnd::move_to(&mut order, from, to);
            }
            for key in order {
                let built = app.built.clone();
                b.div_id(
                    Item(key),
                    tw().w_px(100.0).h_px(60.0).bg(Color32::from_gray(90)),
                    |_| {},
                )
                .drag_source(Item(key))
                .drag_preview(move |b| {
                    built.set(built.get() + 1);
                    b.div(
                        tw().w_px(PREVIEW_SIZE.x)
                            .h_px(PREVIEW_SIZE.y)
                            .bg(PREVIEW_FILL),
                        |b| {
                            b.custom("pulse", tw().w_px(10.0).h_px(10.0))
                                .ui(move |_, ui| {
                                    let t = ui.input(|i| i.time) as f32;
                                    let at =
                                        ui.max_rect().min + vec2(5.0 + 4.0 * (t * 9.0).sin(), 5.0);
                                    ui.painter().circle_filled(at, 3.0, Color32::WHITE);
                                    ui.ctx().request_repaint();
                                });
                        },
                    );
                });
            }
        },
    )
    .drop_target::<Item>(|app, drop| {
        if let Some(from) = app.items.iter().position(|k| *k == drop.payload.0)
            && let Some(to) = drop.index_hint
        {
            common::dnd::move_to(&mut app.items, from, to);
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

fn harness() -> Driver<App> {
    let app = App {
        items: (0..8).collect(),
        built: Rc::new(Cell::new(0)),
    };
    let mut d = support::recorded(support::frame_driver(
        vec2(460.0, 320.0),
        common::base_text(egui::Theme::Light),
        app,
        frame,
    ));
    d.frames(4);
    d
}

fn item_path(key: u32) -> String {
    format!("/\"grid\"/Item({key})")
}

/// The items in the order layout puts them.
fn reading_order(d: &Driver<App>) -> Vec<u32> {
    let mut order: Vec<u32> = (0..8).collect();
    order.sort_by(|a, b| {
        let (ra, rb) = (item_rect(d, *a), item_rect(d, *b));
        ra.min
            .y
            .total_cmp(&rb.min.y)
            .then(ra.min.x.total_cmp(&rb.min.x))
    });
    order
}

fn item_rect(d: &Driver<App>, key: u32) -> Rect {
    support::layout_rect(d.ctx(), &item_path(key))
}

fn layouts(d: &Driver<App>) -> Vec<Rect> {
    (0..8).map(|k| item_rect(d, k)).collect()
}

fn lift(d: &mut Driver<App>, key: u32) -> Pos2 {
    let from = item_rect(d, key).center();
    d.lift(from, vec2(0.0, 12.0)).frames(2);
    from
}

fn preview_rects(d: &Driver<App>) -> Vec<(Rect, usize, Rect)> {
    let mut found = Vec::new();
    for (index, clipped) in d.harness().output().shapes.iter().enumerate() {
        for shape in support::flatten_all_of(&clipped.shape) {
            if let Shape::Rect(r) = shape
                && r.fill == PREVIEW_FILL
            {
                found.push((r.rect, index, clipped.clip_rect));
            }
        }
    }
    found
}

fn preview_node(d: &Driver<App>) -> bool {
    let timeline = Timeline::recorded(d.ctx());
    timeline
        .frame(timeline.len() - 1)
        .expect("a recorded frame")
        .iter()
        .any(|n| n.path.contains("drag preview"))
}

#[test]
fn the_preview_is_built_only_while_the_source_is_lifted() {
    let mut d = harness();
    assert_eq!(d.app().built.get(), 0, "nothing built at rest");
    assert!(!preview_node(&d));
    lift(&mut d, 2);
    assert!(d.app().built.get() > 0, "built under a drag");
    assert!(preview_node(&d));
    assert_eq!(
        preview_rects(&d).len(),
        1,
        "only the lifted source's preview: the others built nothing"
    );
    let builds = d.app().built.get();
    d.frames(3);
    assert_eq!(
        d.app().built.get(),
        builds + 3,
        "one build a frame, for one source"
    );
    let at = item_rect(&d, 2).center();
    d.pointer_up(at).event(Event::PointerGone).frames(3);
    let after = d.app().built.get();
    d.frames(4);
    assert_eq!(d.app().built.get(), after, "no build once it is dropped");
    assert!(!preview_node(&d));
    assert!(preview_rects(&d).is_empty());
}

#[test]
fn the_preview_is_out_of_flow_so_no_sibling_moves() {
    let mut d = harness();
    let at_rest = layouts(&d);
    lift(&mut d, 2);
    assert!(preview_node(&d), "precondition: the preview is in the tree");
    assert_eq!(
        layouts(&d),
        at_rest,
        "the same layout with the preview built"
    );
}

#[test]
fn the_preview_is_on_the_drag_layer_anchored_under_the_pointer_every_frame() {
    let mut d = harness();
    let from = lift(&mut d, 2);
    let grab = from - item_rect(&d, 2).min;
    assert!(grab.length() > 1.0, "precondition: grabbed off the corner");
    let bottom = d
        .harness()
        .output()
        .shapes
        .iter()
        .position(|s| matches!(&s.shape, Shape::Rect(r) if r.fill == Color32::from_gray(90)))
        .expect("an item is painted");
    let mut reordered = false;
    // Across the first row and onto the second, so the order changes.
    let mut at = from;
    for target in [
        item_rect(&d, 0).center(),
        item_rect(&d, 5).center(),
        item_rect(&d, 3).center(),
    ] {
        for step in 1..=10 {
            let here = at + (target - at) * (step as f32 / 10.0);
            d.carry_to(here);
            let found = preview_rects(&d);
            assert_eq!(found.len(), 1, "one preview at step {step}");
            let (rect, index, clip) = &found[0];
            assert!(
                (rect.min - (here - grab)).length() < 0.6,
                "step {step}: at {:?}, not under the pointer at {:?}",
                rect.min,
                here - grab
            );
            assert!(*index > bottom, "painted above the items");
            assert!(
                clip.contains_rect(d.ctx().content_rect()),
                "unclipped: {clip:?}"
            );
            reordered |= reading_order(&d) != (0..8).collect::<Vec<u32>>();
        }
        at = target;
    }
    assert!(reordered, "precondition: the walk reordered the grid");
}

#[test]
fn an_animated_child_of_the_preview_keeps_animating() {
    let mut d = harness();
    lift(&mut d, 2);
    let dot = |d: &Driver<App>| -> Vec<Pos2> {
        support::shapes(d)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Circle(c) if c.fill == Color32::WHITE => Some(c.center),
                _ => None,
            })
            .collect()
    };
    let first = dot(&d);
    assert_eq!(first.len(), 1, "the dot is painted once, in the preview");
    d.frames(1);
    d.frames(1);
    let second = dot(&d);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second, "the dot moved between frames");
}

#[test]
fn a_dragged_preview_is_continuous() {
    let mut d = harness();
    lift(&mut d, 2);
    for key in [0, 5, 7, 1] {
        let at = item_rect(&d, key).center();
        d.hover(at);
    }
    d.press(Key::Escape).frames(19);
    lift(&mut d, 4);
    let at = item_rect(&d, 6).center();
    d.hover(at);
    d.pointer_up(at).event(Event::PointerGone).frames(19);
    Timeline::recorded(d.ctx()).assert_continuous();
}
