//! A three-pane desktop app frame in the style of a chat assistant: a
//! navigation sidebar, a conversation with a composer, and a details panel,
//! the outer two resizable by dragging the splitters between the panes.
//!
//! The capstone of the builder pattern so far: one theme of tokens resolved
//! once, identified nodes that sense hover, clicks and drags, paint variants
//! for every interactive state, and layout that follows the widths held in
//! application state. The composer's field is a stock text edit that grows
//! with its text up to a cap, the model is a select, and the mode a segmented
//! control. Scrolling arrives with M5; for now overflow is clipped.

pub mod centre;
pub mod details;
pub mod icons;
pub mod sidebar;
pub mod state;
pub mod theme;
pub mod view;

use bullseye::LayoutState;
use egui::Color32;

use state::App;
use view::layout;

fn main() -> eframe::Result {
    let mut app = App::default();
    let mut layout_state = LayoutState::new().with_base_text(app.theme.base_text());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_ui_native("chat_shell", options, move |ui, _| {
        ui.ctx().set_theme(egui::Theme::Dark);
        view::prepare(ui.ctx(), &app);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ui, |ui| {
                let area = ui.max_rect();
                layout_state
                    .build(ui.ctx(), |b| layout(b, &app))
                    .layout(area)
                    .draw(&mut app, ui);
            });
    })
}
