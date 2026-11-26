//! Rings are painted after every other shape of their layer, in paint order
//! among rings, as CSS outlines are: no sibling covers one, and each keeps
//! the clip and the transforms of its node.

use core::time::Duration;

use eggshell_ui::overlay::{Anchor, Entrance, Modal, Shown, Side};
use eggshell_ui::testing::{Driver, PlacedFrame, Target};
use eggshell_ui::{Builder, LayoutState, TextSpec, TextStyle, Weight, tw};
use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{Color32, FontFamily, Key, Rect, StrokeKind, Ui, pos2, vec2};

use crate::support;

const RING: Color32 = Color32::from_rgb(10, 120, 240);
const FILL: Color32 = Color32::from_rgb(200, 30, 30);
const NEXT: Color32 = Color32::from_rgb(30, 200, 30);
const WIDTH: f32 = 4.0;

fn base_text() -> TextStyle {
    TextStyle::new(
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight: Weight::Normal,
        },
        Color32::BLACK,
    )
}

/// Every shape painted, groups flattened, with its clip, in paint order.
fn painted(shapes: &[ClippedShape]) -> Vec<(Rect, &Shape)> {
    let mut all = Vec::new();
    for clipped in shapes {
        for shape in support::flatten_all_of(&clipped.shape) {
            all.push((clipped.clip_rect, shape));
        }
    }
    all
}

fn rgb(color: Color32) -> [u8; 3] {
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    [r, g, b]
}

/// The one ring stroked in `RING`, with where it is in paint order and its
/// clip, whatever opacity it is painted at.
fn the_ring(shapes: &[ClippedShape]) -> (usize, Rect, RectShape) {
    let rings: Vec<_> = painted(shapes)
        .into_iter()
        .enumerate()
        .filter_map(|(at, (clip, shape))| match shape {
            Shape::Rect(r) if r.stroke.width > 0.0 && rgb(r.stroke.color) == rgb(RING) => {
                Some((at, clip, r.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rings.len(),
        1,
        "precondition: one ring is painted: {rings:?}"
    );
    rings.into_iter().next().expect("one ring")
}

/// The boxes filled with `fill`, whatever opacity they are painted at.
fn filled(shapes: &[ClippedShape], fill: Color32) -> Vec<(usize, Rect, RectShape)> {
    painted(shapes)
        .into_iter()
        .enumerate()
        .filter_map(|(at, (clip, shape))| match shape {
            Shape::Rect(r) if r.fill.a() > 0 && rgb(r.fill) == rgb(fill) => {
                Some((at, clip, r.clone()))
            }
            _ => None,
        })
        .collect()
}

fn the_fill(shapes: &[ClippedShape], fill: Color32) -> (usize, Rect, RectShape) {
    let found = filled(shapes, fill);
    assert_eq!(
        found.len(),
        1,
        "precondition: one box in {fill:?}: {found:?}"
    );
    found.into_iter().next().expect("one box")
}

/// What `ring` strokes: outside its rect or inside it.
fn ring_band(ring: &RectShape) -> Rect {
    match ring.stroke_kind {
        StrokeKind::Outside => ring.rect.expand(ring.stroke.width),
        _ => ring.rect,
    }
}

/// The shapes painted after `at` that reach into `region`.
fn painted_over(shapes: &[ClippedShape], at: usize, region: Rect) -> Vec<String> {
    painted(shapes)
        .into_iter()
        .enumerate()
        .filter(|&(i, (clip, shape))| {
            i > at && shape.visual_bounding_rect().intersects(region) && clip.intersects(region)
        })
        .map(|(i, (_, shape))| format!("{i}: {shape:?}"))
        .collect()
}

fn output(d: &Driver<()>) -> &[ClippedShape] {
    &d.harness().output().shapes
}

/// ```html
/// <div class="flex">
///   <div class="w-[60px] h-[30px] bg-[#c81e1e] focus-visible:ring-4"></div>   <!-- focused -->
///   <div class="w-[60px] h-[30px] bg-[#1ec81e]"></div>                        <!-- next -->
/// </div>
/// ```
#[test]
fn a_focused_nodes_ring_is_painted_over_its_next_siblings_fill() {
    fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.div(tw().flex().p(8.0), |b| {
            b.div_id(
                "focused",
                tw().w_px(60.0)
                    .h_px(30.0)
                    .bg(FILL)
                    .sense_click()
                    .focus(|p| p.ring(WIDTH, RING)),
                |_| {},
            );
            b.div_id("next", tw().w_px(60.0).h_px(30.0).bg(NEXT), |_| {});
        });
    }
    let mut d = support::driver(vec2(300.0, 100.0), base_text(), (), view).build();
    d.frames(2);
    support::drive_past_bar(&mut d);
    d.press(Key::Tab).frames(2);
    assert!(
        support::focused_path(&d).is_some_and(|path| path.ends_with("\"focused\"")),
        "precondition: the node has focus: {:?}",
        support::focused_path(&d)
    );
    let (at, _, ring) = the_ring(output(&d));
    let (next_at, _, next) = the_fill(output(&d), NEXT);
    let overlap = ring_band(&ring).intersect(next.rect);
    assert!(
        overlap.is_positive(),
        "precondition: the next sibling's fill covers part of the ring: {:?} and {:?}",
        ring_band(&ring),
        next.rect
    );
    assert!(next_at > 0 && next_at < at, "the fill comes first");
    let over = painted_over(output(&d), at, overlap);
    assert!(over.is_empty(), "painted over the ring: {over:?}");
}

/// A ring takes its node's clip: scrolled out of its container, its part
/// past the viewport stays hidden.
#[test]
fn a_ring_in_a_scroll_container_stays_clipped_by_it() {
    fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.scroll("list", tw().w_px(100.0).h_px(50.0)).children(|s| {
            s.div(tw().h_px(30.0), |_| {});
            s.div_id(
                "row",
                tw().w_px(100.0).h_px(30.0).bg(FILL).ring(WIDTH, RING),
                |_| {},
            );
            s.div(tw().h_px(100.0), |_| {});
        });
    }
    let mut d = support::driver(vec2(300.0, 200.0), base_text(), (), view).build();
    d.frames(2);
    let list = d.rect(Target::suffix("/\"list\""));
    let (at, clip, ring) = the_ring(output(&d));
    let (fill_at, fill_clip, _) = the_fill(output(&d), FILL);
    assert!(
        !clip.contains_rect(ring_band(&ring)),
        "precondition: the ring reaches past the viewport: {:?} in {clip:?}",
        ring_band(&ring)
    );
    assert!(fill_at < at);
    assert_eq!(clip, fill_clip, "the ring keeps its node's clip");
    assert!(list.contains_rect(clip), "within the container: {clip:?}");
}

#[derive(Default)]
struct Sheet {
    shown: Shown,
    entrance: Option<Entrance>,
}

/// A sheet whose box has a ring and a child that covers its inside.
fn sheet_view(b: &mut Builder<'_, '_, Sheet>, app: &Sheet) {
    b.div(tw().w_full().h_full(), |_| {});
    b.overlay("sheet")
        .modal(Modal::Edge(Side::Right))
        .shown(app.shown)
        .transition(Duration::from_millis(200))
        .ease_linear()
        .entrance(app.entrance.unwrap_or(Entrance::Zoom))
        .children(|s| {
            s.div_id(
                "box",
                tw().w_px(200.0).h_full().bg(FILL).ring(WIDTH, RING),
                |s| {
                    s.div(tw().w_full().h_full().bg(NEXT), |_| {});
                },
            );
        });
}

#[test]
fn a_ring_under_a_sliding_or_staged_ancestor_keeps_its_transform_and_clip() {
    for entrance in [Entrance::Zoom, Entrance::Slide(Side::Right)] {
        let mut d = support::driver(
            vec2(800.0, 600.0),
            base_text(),
            Sheet {
                entrance: Some(entrance),
                ..Sheet::default()
            },
            sheet_view,
        )
        .step(Duration::from_millis(20))
        .settle_within(Duration::from_millis(80))
        .build();
        d.frames(2);
        d.app_mut().shown = Shown::Open;
        d.frames(3);
        let layout = d.rect(Target::suffix("/\"sheet\"/\"box\""));
        let shapes = &d.harness().output().shapes;
        let (at, clip, ring) = the_ring(shapes);
        let (fill_at, fill_clip, fill) = the_fill(shapes, FILL);
        assert!(
            !support::same(fill.rect, layout),
            "{entrance:?}: precondition: the box is painted away from its layout: {:?} and {layout:?}",
            fill.rect
        );
        assert!(fill_at < at, "{entrance:?}");
        assert!(
            support::same(ring.rect, fill.rect),
            "{entrance:?}: the ring moves with its box: {:?} and {:?}",
            ring.rect,
            fill.rect
        );
        assert_eq!(clip, fill_clip, "{entrance:?}: and keeps its clip");
        let (child_at, _, _) = the_fill(shapes, NEXT);
        assert!(child_at < at, "{entrance:?}: over the box's child");
    }
}

/// An overlay is a layer of its own, above the page's: the page's rings,
/// painted last in the page's layer, stay under it.
#[test]
fn a_page_ring_stays_under_an_open_overlay() {
    fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.div_id(
            "ringed",
            tw().w_px(100.0).h_px(60.0).bg(NEXT).ring(WIDTH, RING),
            |_| {},
        );
        b.overlay("pop")
            .anchor(Anchor::Point(pos2(20.0, 20.0)))
            .children(|s| {
                s.div(tw().w_px(160.0).h_px(160.0).bg(FILL), |_| {});
            });
    }
    let mut d = support::driver(vec2(400.0, 300.0), base_text(), (), view).build();
    d.frames(12);
    let (at, _, ring) = the_ring(output(&d));
    let (pop_at, _, pop) = the_fill(output(&d), FILL);
    assert!(
        ring_band(&ring).intersect(pop.rect).is_positive()
            && !pop.rect.contains_rect(ring_band(&ring)),
        "precondition: the overlay covers part of the ring: {:?} and {:?}",
        ring_band(&ring),
        pop.rect
    );
    assert!(at < pop_at, "the overlay is painted over the ring");
}

const PREVIEW: Color32 = Color32::from_rgb(200, 200, 30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Item;

fn drag_view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("item", tw().w_px(100.0).h_px(60.0).bg(FILL), |_| {})
        .drag_source(Item)
        .drag_preview(|b| {
            b.div(
                tw().w_px(60.0)
                    .h_px(20.0)
                    .bg(PREVIEW)
                    .ring(2.0, RING)
                    .ring_inset(),
                |b| {
                    b.div(tw().w_full().h_full().bg(NEXT), |_| {});
                },
            );
        });
    b.div_id(
        "zone",
        tw().w_px(100.0).h_px(60.0).bg(Color32::from_gray(90)),
        |_| {},
    )
    .drop_target::<Item>(|_, _| {});
}

fn drag_frame(ui: &mut Ui, app: &mut (), layout: &mut LayoutState) -> PlacedFrame {
    let area = ui.max_rect();
    layout
        .build(&ui.ctx().clone(), |b| drag_view(b, app))
        .layout(area)
        .draw(app, ui)
}

#[test]
fn the_lifted_drag_preview_paints_its_rings_last() {
    let mut d = support::frame_driver(vec2(400.0, 300.0), base_text(), (), drag_frame).build();
    d.frames(2);
    let from = d.rect(Target::suffix("/\"item\"")).center();
    d.lift(from, vec2(0.0, 12.0)).frames(2);
    assert_eq!(
        filled(output(&d), PREVIEW).len(),
        1,
        "precondition: the preview is painted"
    );
    let screen = d.ctx().content_rect();
    let (at, clip, ring) = the_ring(output(&d));
    let (child_at, child_clip, child) = the_fill(output(&d), NEXT);
    assert!(
        clip.contains_rect(screen) && child_clip.contains_rect(screen),
        "precondition: both are on the drag layer, unclipped: {clip:?} and {child_clip:?}"
    );
    assert!(
        support::same(child.rect, ring.rect),
        "precondition: the child covers the inset ring: {:?} and {:?}",
        child.rect,
        ring.rect
    );
    assert!(child_at < at, "the ring is lifted last");
}
