//! A drag belongs to the `LayoutState` whose source was lifted: a second
//! layout in the same egui context does not see it, or the grab and pointer
//! it reads through its own `Shell`.

use eggshell_ui::{Builder, LayoutState, tw};
use egui::{Color32, Pos2, Ui, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support::{self, FrameRig};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Item(u32);

struct App {
    /// A second layout of the context, laid out after the one with the items.
    watcher: LayoutState,
    carried: Option<Item>,
    watched: Option<Item>,
}

type H = Harness<'static, FrameRig<App>>;

fn items(b: &mut Builder<'_, '_, App>, carried: &mut Option<Item>) {
    *carried = b.dragging::<Item>().map(|drag| *drag.payload);
    b.div_id("grid", tw().flex().gap(2.0).p(2.0), |b| {
        for key in 0..3 {
            b.div_id(
                Item(key),
                tw().w_px(100.0).h_px(60.0).bg(Color32::from_gray(90)),
                |_| {},
            )
            .drag_source(Item(key));
        }
    })
    .drop_target::<Item>(|_, _| {});
}

fn frame(ui: &mut Ui, app: &mut App, layout: &mut LayoutState) {
    let ctx = ui.ctx().clone();
    let area = ui.max_rect();
    let mut carried = None;
    let placed = layout.build(&ctx, |b| items(b, &mut carried)).layout(area);
    app.carried = carried;
    let mut watched = None;
    app.watcher
        .build(&ctx, |b: &mut Builder<'_, '_, App>| {
            watched = b.dragging::<Item>().map(|drag| *drag.payload);
            b.div_id("watcher", tw().h_px(10.0), |_| {});
        })
        .layout(area)
        .draw(app, ui);
    app.watched = watched;
    placed.draw(app, ui);
}

fn harness() -> H {
    let app = App {
        watcher: LayoutState::named("watcher"),
        carried: None,
        watched: None,
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

fn lift(h: &mut H) -> Pos2 {
    let from = support::layout_rect(&h.ctx, "/\"grid\"/Item(1)").center();
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

#[test]
fn a_drag_is_seen_only_by_the_layout_it_was_lifted_in() {
    let mut h = harness();
    lift(&mut h);
    assert_eq!(h.state().app.carried, Some(Item(1)), "the drag is live");
    assert_eq!(h.state().app.watched, None, "another layout's drag");
}

#[test]
fn the_pointer_of_a_drag_is_read_through_the_layout_that_holds_it() {
    let mut h = harness();
    lift(&mut h);
    let ctx = h.ctx.clone();
    let App { watcher, .. } = &h.state().app;
    assert!(
        h.state().layout.shell(&ctx).drag_pointer().is_some(),
        "the lifting layout reads the pointer"
    );
    assert_eq!(watcher.shell(&ctx).drag_pointer(), None);
    assert_eq!(watcher.shell(&ctx).drag_grab(), vec2(0.0, 0.0));
}
