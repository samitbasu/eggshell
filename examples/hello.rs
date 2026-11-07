//! A window with a label; the smallest thing `cargo xtask example` can run.

fn main() -> eframe::Result {
    eframe::run_ui_native("hello", eframe::NativeOptions::default(), |ui, _frame| {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Hello, bullseye");
        });
    })
}
