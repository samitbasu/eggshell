//! What a layout remembers of keyboard focus is its own: two layouts of one
//! context (here also sharing a key, in passes of their own) do not answer
//! for each other's subtrees.

use eggshell_ui::{Builder, LayoutState, tw};
use egui::{Context, Id, Rect, pos2, vec2};

fn context() -> Context {
    let ctx = Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

/// What a pass found out.
struct Pass {
    x: Id,
    y: Option<Id>,
    /// Whether `y` had focus, and whether `x` was focus-within, as the pass
    /// ended.
    y_focused: bool,
    within: bool,
}

/// A pass in which `layout` is laid out and drawn: a focusable node "x",
/// holding a focusable node "y" if `with_y`; `watch` is the widget whose
/// focus is reported.
fn pass(layout: &mut LayoutState, ctx: &Context, with_y: bool, watch: Option<Id>) -> Pass {
    let mut found = None;
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let placed = layout
            .build(ctx, |b: &mut Builder<'_, '_, ()>| {
                b.div_id("x", tw().w_px(100.0).h_px(50.0).sense_click(), |b| {
                    if with_y {
                        b.div_id("y", tw().w_px(40.0).h_px(20.0).sense_click(), |_| {});
                    }
                });
            })
            .layout(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0)));
        let node = placed.at("x");
        let x = node.id().expect("x");
        let y = with_y.then(|| node.at("y").id().expect("y"));
        placed.draw(&mut (), ui);
        found = Some(Pass {
            x,
            y,
            y_focused: ctx.memory(|memory| watch.is_some_and(|w| memory.has_focus(w))),
            within: layout.shell(ctx).focus_within(x),
        });
    })
    .drop_without_applying_deltas();
    found.expect("a pass ran")
}

#[test]
fn a_layout_does_not_answer_for_the_subtrees_of_another() {
    let ctx = context();
    let (mut a, mut b) = (
        LayoutState::new(crate::support::shadcn()),
        LayoutState::new(crate::support::shadcn()),
    );
    let first = pass(&mut a, &ctx, true, None);
    let y = first.y.expect("a has a y");
    ctx.memory_mut(|memory| memory.request_focus(y));
    pass(&mut a, &ctx, true, Some(y));
    let held = pass(&mut a, &ctx, true, Some(y));
    assert!(held.y_focused && held.within, "a's x holds the focus");
    let other = pass(&mut b, &ctx, false, Some(y));
    assert_eq!(other.x, first.x, "the layouts share a key");
    assert!(other.y_focused, "y still has focus in b's pass");
    assert!(!other.within, "b's x holds no widget that has focus");
}
