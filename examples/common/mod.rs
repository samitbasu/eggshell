//! What the egui_taffy and flexbox ports share: the base text role and the
//! eframe shell with a controls panel beside the bullseye layout.
#![allow(
    dead_code,
    reason = "each crate that includes this file uses a subset of it"
)]

use bullseye::{Builder, LayoutState, TextSpec, TextStyle};
use egui::{Color32, FontFamily};

/// The base text role.
pub const BODY: TextSpec = TextSpec {
    size: 16.0,
    line_height: 20.0,
    family: FontFamily::Proportional,
};

/// A proportional text role of `size` points.
pub fn sized(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Proportional,
    }
}

/// A monospace text role of `size` points.
pub fn mono(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Monospace,
    }
}

/// The text every layout starts from.
pub fn base_text() -> TextStyle {
    TextStyle::new(BODY, Color32::BLACK)
}

/// Builds one frame of a layout from the example's state.
pub type Layout<S> = for<'b, 'f> fn(&mut Builder<'b, 'f, ()>, &S);

/// Egui widgets that edit the example's state.
pub type Controls<S> = fn(&mut egui::Ui, &mut S);

/// Runs `layout` in the central panel, with `controls` in a side panel.
///
/// The controls are plain egui widgets outside the bullseye tree: stock
/// widgets inside bullseye leaves need same-frame measuring.
pub fn run<S: 'static>(
    title: &str,
    mut state: S,
    controls: Option<Controls<S>>,
    layout: Layout<S>,
) -> eframe::Result {
    let mut layout_state = LayoutState::new().with_base_text(base_text());
    eframe::run_ui_native(title, eframe::NativeOptions::default(), move |ui, _| {
        ui.ctx().set_theme(egui::Theme::Light);
        if let Some(controls) = controls {
            egui::Panel::left("controls").show(ui, |ui| controls(ui, &mut state));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show(ui, |ui| {
                let area = ui.max_rect();
                layout_state
                    .build(ui.ctx(), |b| layout(b, &state))
                    .layout(area)
                    .draw(&mut (), ui);
            });
    })
}
