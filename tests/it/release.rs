//! The checks gated on `debug_assertions` are off in release (#336). These
//! tests are ignored in a debug build; `cargo xtask ci`'s `release` step runs
//! them in the `release-gates` profile, which has no debug assertions.
//!
//! A test of what release builds do belongs here, under `release::`, so the
//! step's filter finds it.

use eggshell_ui::testing::{Headless, assert_no_reports};
use eggshell_ui::{Builder, LayoutState, tw};
use egui::{Context, Rect, pos2, vec2};

fn context() -> Context {
    let ctx = Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .drop_without_applying_deltas();
    ctx
}

#[cfg_attr(
    debug_assertions,
    ignore = "release-only: `cargo xtask ci` runs it in the release-gates profile"
)]
#[test]
fn an_unnamed_button_is_not_reported() {
    let mut headless = Headless::new(vec2(200.0, 100.0));
    headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _| {
        b.button("nameless", "").add();
    });
    assert_no_reports(headless.ctx());
}

#[cfg_attr(
    debug_assertions,
    ignore = "release-only: `cargo xtask ci` runs it in the release-gates profile"
)]
#[test]
fn a_duplicate_key_path_is_not_reported() {
    let mut headless = Headless::new(vec2(200.0, 100.0));
    headless.layout(&(), |b: &mut Builder<'_, '_, ()>, _| {
        b.scope("outer", |b| {
            b.span("dup", tw(), "x");
            b.span("dup", tw(), "y");
        });
    });
    assert_no_reports(headless.ctx());
}

#[cfg_attr(
    debug_assertions,
    ignore = "release-only: `cargo xtask ci` runs it in the release-gates profile"
)]
#[test]
fn two_unnamed_layouts_sharing_a_top_level_key_are_not_reported() {
    let ctx = context();
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
    for mut layout in [LayoutState::new(), LayoutState::new()] {
        layout
            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                b.div_id("title", tw(), |_| {});
            })
            .layout(area);
    }
    assert_no_reports(&ctx);
}
