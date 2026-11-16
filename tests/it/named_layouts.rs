//! A `LayoutState::named` salts the ids of its nodes, so layouts that share
//! an egui context cannot collide on a top-level key; in debug builds the
//! collisions that remain are reported, naming both builds.

use eggshell_ui::testing::{assert_no_reports, assert_reported};
use eggshell_ui::{Builder, LayoutState, Problem, TextSpec, TextStyle, Weight, tw};
use egui::{Color32, Context, FontFamily, Id, Rect, pos2, vec2};

use crate::support;

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

fn context() -> Context {
    let ctx = Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

fn area() -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0))
}

/// The id of the node keyed `key` at the top of a layout built in `ctx`.
#[track_caller]
fn top_id(layout: &mut LayoutState, ctx: &Context, key: &'static str) -> Id {
    layout
        .build(ctx, |b: &mut Builder<'_, '_, ()>| {
            b.div_id(key, tw(), |_| {});
        })
        .layout(area())
        .at(key)
        .id()
        .expect("an identified node")
}

#[test]
fn a_named_layout_has_different_ids_from_an_unnamed_one() {
    let unnamed = top_id(&mut LayoutState::new(), &context(), "title");
    let named = top_id(&mut LayoutState::named("bar"), &context(), "title");
    assert_ne!(unnamed, named);
    let other = top_id(&mut LayoutState::named("page"), &context(), "title");
    assert_ne!(named, other);
}

#[test]
fn an_unnamed_layout_keeps_the_ids_it_always_had() {
    let old = Id::NULL
        .with("eggshell")
        .with((std::any::type_name::<&str>(), "title"));
    let id = top_id(&mut LayoutState::new(), &context(), "title");
    assert_eq!(id, old);
    assert_eq!(id, top_id(&mut LayoutState::new(), &context(), "title"));
}

#[test]
fn a_name_gives_the_same_ids_in_every_layout_state() {
    let first = top_id(&mut LayoutState::named("bar"), &context(), "title");
    let second = top_id(&mut LayoutState::named("bar"), &context(), "title");
    assert_eq!(first, second);
}

#[cfg(debug_assertions)]
#[test]
fn two_unnamed_layouts_sharing_a_top_level_key_are_reported_naming_both_builds() {
    let ctx = context();
    let (mut a, mut b) = (LayoutState::new(), LayoutState::new());
    let first = line!() + 1;
    top_id(&mut a, &ctx, "title");
    let second = line!() + 1;
    top_id(&mut b, &ctx, "title");
    let report = assert_reported(&ctx, Problem::TopLevelKeyClash);
    let message = &report.message;
    assert!(message.contains("\"title\""), "{message}");
    for line in [first, second] {
        assert!(
            message.contains(&format!("named_layouts.rs:{line}:")),
            "expected line {line} in: {message}"
        );
    }
    assert_eq!(report.at.line(), second, "reported at the second build");
}

#[cfg(debug_assertions)]
#[test]
fn layouts_with_different_names_or_keys_report_nothing() {
    let ctx = context();
    let mut layouts = [
        LayoutState::new(),
        LayoutState::named("bar"),
        LayoutState::named("page"),
        LayoutState::new(),
    ];
    for (layout, key) in layouts.iter_mut().zip(["title", "title", "title", "other"]) {
        top_id(layout, &ctx, key);
    }
    assert_no_reports(&ctx);
}

#[cfg(debug_assertions)]
#[test]
fn one_layout_may_build_twice_in_a_pass() {
    let ctx = context();
    let mut layout = LayoutState::new();
    top_id(&mut layout, &ctx, "title");
    top_id(&mut layout, &ctx, "title");
    assert_no_reports(&ctx);
}

#[cfg(debug_assertions)]
#[test]
fn the_same_name_twice_in_a_context_is_reported() {
    let ctx = context();
    let (mut a, mut b) = (LayoutState::named("bar"), LayoutState::named("bar"));
    let first = line!() + 1;
    top_id(&mut a, &ctx, "one");
    let second = line!() + 1;
    top_id(&mut b, &ctx, "two");
    let message = assert_reported(&ctx, Problem::LayoutNameClash).message;
    assert!(message.contains("named the same"), "{message}");
    for line in [first, second] {
        assert!(
            message.contains(&format!("named_layouts.rs:{line}:")),
            "expected line {line} in: {message}"
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn claims_start_over_each_pass() {
    let ctx = context();
    let mut a = LayoutState::new();
    let mut b = LayoutState::new();
    ctx.run_ui(egui::RawInput::default(), |_| {
        top_id(&mut a, &ctx, "title");
    })
    .drop_without_applying_deltas();
    ctx.run_ui(egui::RawInput::default(), |_| {
        top_id(&mut b, &ctx, "title");
    })
    .drop_without_applying_deltas();
    assert_no_reports(&ctx);
}

/// The rig's bar has a node keyed `"bar-button"`; a view that uses the same
/// top-level key must still get an id of its own.
#[test]
fn a_view_may_reuse_the_rigs_bar_keys() {
    let mut harness = support::harness(
        vec2(200.0, 100.0),
        base_text(),
        (),
        |b: &mut Builder<'_, '_, ()>, _: &()| {
            b.button("bar-button", "View").on_click(|_: &mut ()| {});
        },
    );
    harness.run_steps(2);
    let bar_button = harness
        .state()
        .bar
        .last_stop
        .expect("the bar has a Tab stop");
    let view_button = support::id_of(&harness, "/\"bar-button\"");
    assert_ne!(bar_button, view_button);
}

#[cfg(debug_assertions)]
#[test]
fn a_dropped_layout_claims_nothing() {
    let ctx = context();
    top_id(&mut LayoutState::new(), &ctx, "title");
    top_id(&mut LayoutState::new(), &ctx, "title");
    assert_no_reports(&ctx);
}
