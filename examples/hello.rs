//! A window with a label; the smallest thing `cargo xtask example` can run.

#[path = "common/mod.rs"]
mod common;

fn main() -> eframe::Result {
    let mut theme_bar = common::ThemeBar::new();
    eframe::run_ui_native(
        "hello",
        eframe::NativeOptions::default(),
        move |ui, _frame| {
            theme_bar.show(ui);
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Hello, bullseye");
            });
        },
    )
}
