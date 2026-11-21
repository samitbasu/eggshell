//! A drag belongs to the `LayoutState` whose source was lifted: a second
//! layout in the same egui context does not see it, or the grab and pointer
//! it reads through its own `Shell`.

use eggshell_ui::testing::{Driver, PlacedFrame};
use eggshell_ui::{Builder, LayoutState, tw};
use egui::{Color32, Ui, vec2};

#[path = "../../examples/common/mod.rs"]
mod common;

use crate::support;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Item(u32);

struct App {
    /// A second layout of the context, laid out after the one with the items.
    watcher: LayoutState,
    carried: Option<Item>,
    watched: Option<Item>,
}

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

fn frame(ui: &mut Ui, app: &mut App, layout: &mut LayoutState) -> PlacedFrame {
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
    placed.draw(app, ui)
}

fn harness() -> Driver<App> {
    let app = App {
        watcher: LayoutState::named("watcher").with_style(support::shadcn()),
        carried: None,
        watched: None,
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

fn lift(d: &mut Driver<App>) {
    let from = support::layout_rect(d.ctx(), "/\"grid\"/Item(1)").center();
    d.lift(from, vec2(0.0, 12.0)).frames(2);
}

#[test]
fn a_drag_is_seen_only_by_the_layout_it_was_lifted_in() {
    let mut d = harness();
    lift(&mut d);
    assert_eq!(d.app().carried, Some(Item(1)), "the drag is live");
    assert_eq!(d.app().watched, None, "another layout's drag");
}

#[test]
fn the_pointer_of_a_drag_is_read_through_the_layout_that_holds_it() {
    let mut d = harness();
    lift(&mut d);
    let ctx = d.ctx().clone();
    let App { watcher, .. } = d.app();
    assert!(
        d.shell().drag_pointer().is_some(),
        "the lifting layout reads the pointer"
    );
    assert_eq!(watcher.shell(&ctx).drag_pointer(), None);
    assert_eq!(watcher.shell(&ctx).drag_grab(), vec2(0.0, 0.0));
}
