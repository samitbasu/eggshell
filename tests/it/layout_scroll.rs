//! A scroll container's published viewport and queued requests belong to the
//! `LayoutState` that laid it out: two layouts that share an egui context
//! (and, in separate passes, a key) do not see each other's.

use eggshell_ui::{Builder, LayoutState, tw};
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
    let (mut a, mut b) = (
        LayoutState::new(crate::support::shadcn()),
        LayoutState::new(crate::support::shadcn()),
    );
    let list = lay_out(&mut a, &ctx, 300.0);
    assert_eq!(
        lay_out(&mut b, &ctx, 120.0),
        list,
        "the layouts share a key"
    );
    let height = |layout: &LayoutState| {
        layout
            .shell(&ctx)
            .viewport(list)
            .expect("laid out")
            .content
            .height()
    };
    assert_eq!((height(&a), height(&b)), (300.0, 120.0));
}

/// A pass in which `layout` is laid out and drawn: a scroll container keyed
/// "list" and a node "pad" whose draw closure runs `ask` with the list's id.
fn draw_pass(layout: &mut LayoutState, ctx: &Context, ask: fn(&egui::Ui, Id)) -> Id {
    let mut list = None;
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let placed = layout
            .build(ctx, |b: &mut Builder<'_, '_, ()>| {
                let id = b.id_of("list");
                b.scroll("list", tw().w_px(100.0).h_px(100.0))
                    .children(|s| {
                        s.div_id("rows", tw().h_px(300.0), |_| {});
                    });
                b.div_id("pad", tw().h_px(10.0), |_| {})
                    .draw(move |_, ui| ask(ui, id));
            })
            .layout(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)));
        list = placed.at("list").id();
        placed.draw(&mut (), ui);
    })
    .drop_without_applying_deltas();
    list.expect("the list was laid out")
}

#[test]
fn a_draw_closure_scrolls_its_own_layout_through_the_ui() {
    let ctx = context();
    let (mut a, mut b) = (
        LayoutState::named("a", crate::support::shadcn()),
        LayoutState::named("b", crate::support::shadcn()),
    );
    let (list_a, list_b) = (
        draw_pass(&mut a, &ctx, |ui, list| {
            use eggshell_ui::ShellExt;
            let Some(shell) = ui.try_shell() else { return };
            shell.scroll_by(list, vec2(0.0, 15.0));
        }),
        draw_pass(&mut b, &ctx, |_, _| {}),
    );
    for _ in 0..3 {
        draw_pass(&mut a, &ctx, |_, _| {});
        draw_pass(&mut b, &ctx, |_, _| {});
    }
    let offset = |layout: &LayoutState, list| {
        layout
            .shell(&ctx)
            .viewport(list)
            .expect("laid out")
            .offset()
    };
    assert_eq!(offset(&a, list_a), vec2(0.0, 15.0));
    assert_eq!(offset(&b, list_b), vec2(0.0, 0.0));
}
