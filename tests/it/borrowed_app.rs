//! An app state that borrows (`App<'a>`) builds, lays out and draws: the
//! pipeline (and the tooltips it declares for every layout) adds no
//! `A: 'static` bound. `NodeHandle`'s handler methods still require it.

use eggshell_ui::testing::Headless;
use eggshell_ui::{LayoutState, tw};
use egui::{Pos2, RawInput, Rect, vec2};

struct App<'a> {
    label: &'a str,
}

fn view<'f>(b: &mut eggshell_ui::Builder<'_, 'f, App<'_>>, app: &'f App<'_>) {
    b.span("label", tw(), app.label);
}

#[test]
fn a_borrowing_app_lays_out_headless() {
    let text = String::from("borrowed");
    let app = App { label: &text };
    let placed = Headless::new(vec2(200.0, 100.0)).layout(&app, view);
    assert!(placed.at("label").rect().width() > 0.0);
}

#[test]
fn a_borrowing_app_builds_lays_out_and_draws() {
    let text = String::from("borrowed");
    let mut app = App { label: &text };
    let ctx = egui::Context::default();
    let mut layout = LayoutState::new();
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0));
    let input = RawInput {
        screen_rect: Some(screen),
        ..RawInput::default()
    };
    ctx.run_ui(input.clone(), |_| {})
        .drop_without_applying_deltas();
    let mut output = ctx.run_ui(input, |ui| {
        let view_app = App { label: app.label };
        layout
            .build(ui.ctx(), |b| view(b, &view_app))
            .layout(screen)
            .draw(&mut app, ui);
    });
    output.textures_delta.clear();
    assert!(!output.shapes.is_empty(), "the label was drawn");
}
