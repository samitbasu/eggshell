//! A layout drawn from a draw closure of another: an egui `Popup` or `Area`
//! with eggshell content inside. The inner layout's guard puts the outer
//! layout's drawing shell back when it finishes.

use std::cell::Cell;
use std::rc::Rc;

use eggshell_ui::{Builder, LayoutState, ShellExt as _, tw};

/// Draws an outer layout whose first node draws an inner layout, then calls
/// `after` from a draw closure of the outer layout that runs after the inner
/// one has finished.
fn nested(after: impl Fn(&egui::Ui) + 'static) {
    let after: Rc<dyn Fn(&egui::Ui)> = Rc::new(after);
    let ctx = egui::Context::default();
    ctx.run_ui(Default::default(), |ui| {
        let mut outer = LayoutState::new(&eggshell_ui::SharedStyle::new(crate::support::shadcn()));
        let placed = outer
            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                b.div_id("host", tw().w_px(100.0).h_px(40.0), |_| {})
                    .draw(|app, ui| {
                        let mut inner = LayoutState::new(&eggshell_ui::SharedStyle::new(
                            crate::support::shadcn(),
                        ));
                        let ctx = ui.ctx().clone();
                        let placed = inner
                            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                                b.button("inner", "Inner").on_click(|_| {});
                            })
                            .layout(ui.max_rect());
                        placed.draw(app, ui);
                    });
                b.button("outer", "Outer").on_click(|_| {});
                let after = Rc::clone(&after);
                b.div_id("after", tw().w_px(10.0).h_px(10.0), |_| {})
                    .draw(move |_, ui| after(ui));
            })
            .layout(ui.max_rect());
        placed.draw(&mut (), ui);
    })
    .drop_without_applying_deltas();
}

#[test]
fn an_owned_button_after_a_nested_layout_draws() {
    // Panicked in `nav::take_activation` -> `Ui::shell` before the guard
    // restored the outer shell.
    nested(|_| {});
}

#[test]
fn the_outer_closures_see_a_shell_again_after_the_nested_layout() {
    let seen = Rc::new(Cell::new(false));
    let saw = Rc::clone(&seen);
    nested(move |ui| saw.set(ui.try_shell().is_some()));
    assert!(seen.get());
}

#[test]
fn outside_every_draw_there_is_no_shell() {
    let ctx = egui::Context::default();
    ctx.run_ui(Default::default(), |ui| {
        assert!(ui.try_shell().is_none());
    })
    .drop_without_applying_deltas();
}

/// The walk of each layout uses its own shell: an inner layout's scroll
/// container and `focus-within` nodes, and the outer layout's after it, draw
/// without a report.
#[test]
fn walks_of_nested_layouts_draw_scroll_and_focus_within_without_reports() {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        ctx.run_ui(Default::default(), |ui| {
            let mut outer =
                LayoutState::new(&eggshell_ui::SharedStyle::new(crate::support::shadcn()));
            let placed = outer
                .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                    b.div_id(
                        "host",
                        tw().w_px(100.0).h_px(60.0).focus_within(|p| p),
                        |_| {},
                    )
                    .draw(|app, ui| {
                        let mut inner = LayoutState::new(&eggshell_ui::SharedStyle::new(
                            crate::support::shadcn(),
                        ));
                        let ctx = ui.ctx().clone();
                        let placed = inner
                            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                                b.scroll("list", tw().w_px(80.0).h_px(40.0)).children(|s| {
                                    s.div_id("rows", tw().h_px(200.0).focus_within(|p| p), |_| {});
                                });
                            })
                            .layout(ui.max_rect());
                        placed.draw(app, ui);
                    });
                    b.scroll("after", tw().w_px(80.0).h_px(40.0)).children(|s| {
                        s.div_id("rows", tw().h_px(200.0).focus_within(|p| p), |_| {});
                    });
                })
                .layout(ui.max_rect());
            placed.draw(&mut (), ui);
        })
        .drop_without_applying_deltas();
    }
    eggshell_ui::testing::assert_no_reports(&ctx);
}

/// A ring painted by a nested layout waits for the outer layout's walk of the
/// same layer to end, so the outer layout's later siblings do not cover it.
#[test]
fn a_nested_layouts_ring_is_not_covered_by_a_later_outer_sibling() {
    const RING: egui::Color32 = egui::Color32::from_rgb(10, 120, 240);
    const NEXT: egui::Color32 = egui::Color32::from_rgb(30, 200, 30);
    let ctx = egui::Context::default();
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let output = ctx.run_ui(Default::default(), |ui| {
            let mut outer =
                LayoutState::new(&eggshell_ui::SharedStyle::new(crate::support::shadcn()));
            let placed = outer
                .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                    b.div(tw().flex().p(8.0), |b| {
                        b.div_id("host", tw().w_px(100.0).h_px(40.0), |_| {})
                            .draw(|app, ui| {
                                let mut inner = LayoutState::new(&eggshell_ui::SharedStyle::new(
                                    crate::support::shadcn(),
                                ));
                                let ctx = ui.ctx().clone();
                                inner
                                    .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                                        b.div(tw().w_full().h_full().ring(4.0, RING), |_| {});
                                    })
                                    .layout(ui.max_rect())
                                    .draw(app, ui);
                            });
                        b.div(tw().w_px(100.0).h_px(40.0).bg(NEXT), |_| {});
                    });
                })
                .layout(ui.max_rect());
            placed.draw(&mut (), ui);
        });
        shapes = output.shapes.clone();
        output.drop_without_applying_deltas();
    }
    let rects: Vec<(usize, &egui::epaint::RectShape)> = shapes
        .iter()
        .enumerate()
        .filter_map(|(at, clipped)| match &clipped.shape {
            egui::Shape::Rect(rect) => Some((at, rect)),
            _ => None,
        })
        .collect();
    let (ring_at, ring) = rects
        .iter()
        .find(|(_, rect)| rect.stroke.color == RING)
        .copied()
        .expect("precondition: the inner layout paints its ring");
    let (next_at, next) = rects
        .iter()
        .find(|(_, rect)| rect.fill == NEXT)
        .copied()
        .expect("precondition: the outer sibling is painted");
    assert!(
        ring.rect.expand(ring.stroke.width).intersects(next.rect)
            && ring.stroke_kind == egui::StrokeKind::Outside,
        "precondition: the sibling's fill reaches under the ring: {:?} and {:?}",
        ring.rect,
        next.rect
    );
    assert!(
        next_at < ring_at,
        "the ring is painted over the outer sibling"
    );
}
