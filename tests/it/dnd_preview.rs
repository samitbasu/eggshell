//! `drag_preview`: an app-built drag image, through the real harness path.

use std::cell::Cell;
use std::rc::Rc;

use eggshell_ui::testing::Timeline;
use eggshell_ui::{LayoutState, tw};
use egui::epaint::Shape;
use egui::{Color32, Key, Pos2, Rect, Ui, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support::{self, FrameRig};

const PREVIEW_FILL: Color32 = Color32::from_rgb(200, 30, 30);
const PREVIEW_SIZE: egui::Vec2 = vec2(60.0, 20.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Item(u32);

struct App {
    items: Vec<u32>,
    /// How many times the preview closure ran.
    built: Rc<Cell<u32>>,
}

type H = Harness<'static, FrameRig<App>>;

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

fn harness() -> H {
    let app = App {
        items: (0..8).collect(),
        built: Rc::new(Cell::new(0)),
    };
    let mut h = support::frame_harness(
        vec2(460.0, 320.0),
        common::base_text(egui::Theme::Light),
        app,
        frame,
    );
    h.run_steps(4);
    h
}

fn item_path(key: u32) -> String {
    format!("/\"grid\"/Item({key})")
}

/// The items in the order layout puts them.
fn reading_order(h: &H) -> Vec<u32> {
    let mut order: Vec<u32> = (0..8).collect();
    order.sort_by(|a, b| {
        let (ra, rb) = (item_rect(h, *a), item_rect(h, *b));
        ra.min
            .y
            .total_cmp(&rb.min.y)
            .then(ra.min.x.total_cmp(&rb.min.x))
    });
    order
}

fn item_rect(h: &H, key: u32) -> Rect {
    support::layout_rect(&h.ctx, &item_path(key))
}

fn layouts(h: &H) -> Vec<Rect> {
    (0..8).map(|k| item_rect(h, k)).collect()
}

fn lift(h: &mut H, key: u32) -> Pos2 {
    let from = item_rect(h, key).center();
    h.hover_at(from);
    h.run_steps(3);
    h.drag_at(from);
    h.step();
    h.hover_at(from + vec2(0.0, 6.0));
    h.step();
    h.hover_at(from + vec2(0.0, 12.0));
    h.run_steps(3);
    from
}

fn preview_rects(h: &H) -> Vec<(Rect, usize, Rect)> {
    let mut found = Vec::new();
    for (index, clipped) in h.output().shapes.iter().enumerate() {
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

fn preview_node(h: &H) -> bool {
    let timeline = Timeline::recorded(&h.ctx);
    timeline
        .frame(timeline.len() - 1)
        .iter()
        .any(|n| n.path.contains("drag preview"))
}

#[test]
fn the_preview_is_built_only_while_the_source_is_lifted() {
    let mut h = harness();
    assert_eq!(h.state().app.built.get(), 0, "nothing built at rest");
    assert!(!preview_node(&h));
    lift(&mut h, 2);
    assert!(h.state().app.built.get() > 0, "built under a drag");
    assert!(preview_node(&h));
    assert_eq!(
        preview_rects(&h).len(),
        1,
        "only the lifted source's preview: the others built nothing"
    );
    let builds = h.state().app.built.get();
    h.run_steps(3);
    assert_eq!(
        h.state().app.built.get(),
        builds + 3,
        "one build a frame, for one source"
    );
    let at = item_rect(&h, 2).center();
    h.drop_at(at);
    h.run_steps(4);
    let after = h.state().app.built.get();
    h.run_steps(4);
    assert_eq!(
        h.state().app.built.get(),
        after,
        "no build once it is dropped"
    );
    assert!(!preview_node(&h));
    assert!(preview_rects(&h).is_empty());
}

#[test]
fn the_preview_is_out_of_flow_so_no_sibling_moves() {
    let mut h = harness();
    let at_rest = layouts(&h);
    lift(&mut h, 2);
    assert!(preview_node(&h), "precondition: the preview is in the tree");
    assert_eq!(
        layouts(&h),
        at_rest,
        "the same layout with the preview built"
    );
}

#[test]
fn the_preview_is_on_the_drag_layer_anchored_under_the_pointer_every_frame() {
    let mut h = harness();
    let from = lift(&mut h, 2);
    let grab = from - item_rect(&h, 2).min;
    assert!(grab.length() > 1.0, "precondition: grabbed off the corner");
    let bottom = h
        .output()
        .shapes
        .iter()
        .position(|s| matches!(&s.shape, Shape::Rect(r) if r.fill == Color32::from_gray(90)))
        .expect("an item is painted");
    let mut reordered = false;
    // Across the first row and onto the second, so the order changes.
    let mut at = from;
    for target in [
        item_rect(&h, 0).center(),
        item_rect(&h, 5).center(),
        item_rect(&h, 3).center(),
    ] {
        for step in 1..=10 {
            let here = at + (target - at) * (step as f32 / 10.0);
            h.hover_at(here);
            h.step();
            let found = preview_rects(&h);
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
                clip.contains_rect(h.ctx.content_rect()),
                "unclipped: {clip:?}"
            );
            reordered |= reading_order(&h) != (0..8).collect::<Vec<u32>>();
        }
        at = target;
    }
    assert!(reordered, "precondition: the walk reordered the grid");
}

#[test]
fn an_animated_child_of_the_preview_keeps_animating() {
    let mut h = harness();
    lift(&mut h, 2);
    let dot = |h: &H| -> Vec<Pos2> {
        support::flatten_all(h)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Circle(c) if c.fill == Color32::WHITE => Some(c.center),
                _ => None,
            })
            .collect()
    };
    let first = dot(&h);
    assert_eq!(first.len(), 1, "the dot is painted once, in the preview");
    h.step();
    h.step();
    let second = dot(&h);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second, "the dot moved between frames");
}

#[test]
fn a_dragged_preview_is_continuous() {
    let mut h = harness();
    lift(&mut h, 2);
    for key in [0, 5, 7, 1] {
        let at = item_rect(&h, key).center();
        h.hover_at(at);
        h.run_steps(3);
    }
    h.key_press(Key::Escape);
    h.run_steps(20);
    lift(&mut h, 4);
    let at = item_rect(&h, 6).center();
    h.hover_at(at);
    h.run_steps(3);
    h.drop_at(at);
    h.run_steps(20);
    Timeline::recorded(&h.ctx).assert_continuous();
}
