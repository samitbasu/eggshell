//! A scroll container's published viewport and queued requests belong to the
//! `LayoutState` that laid it out: two layouts that share an egui context
//! (and, in separate passes, a key) do not see each other's.

use eggshell_ui::{Builder, LayoutState, scroll, tw};
use egui::{Context, Id, Rect, pos2, vec2};

fn context() -> Context {
    let ctx = Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

/// Lays `layout` out in a pass of its own: a 100 pt tall scroll container
/// keyed "list" holding `content` pt.
fn lay_out(layout: &mut LayoutState, ctx: &Context, content: f32) -> Id {
    let mut id = None;
    ctx.run_ui(egui::RawInput::default(), |_| {
        let placed = layout
            .build(ctx, |b: &mut Builder<'_, '_, ()>| {
                b.scroll("list", tw().w_px(100.0).h_px(100.0))
                    .children(|s| {
                        s.div_id("rows", tw().h_px(content), |_| {});
                    });
            })
            .layout(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)));
        id = placed.at("list").id();
    })
    .drop_without_applying_deltas();
    id.expect("the list was laid out")
}

#[test]
fn a_layouts_viewport_survives_another_layout_of_the_context() {
    let ctx = context();
    let (mut a, mut b) = (LayoutState::new(), LayoutState::new());
    let list = lay_out(&mut a, &ctx, 300.0);
    assert_eq!(lay_out(&mut b, &ctx, 120.0), list, "the layouts share a key");
    assert_eq!(
        scroll::viewport(&ctx, list)
            .expect("laid out")
            .content
            .height(),
        300.0,
        "the context-wide viewport is the last layout's, not a's"
    );
}
