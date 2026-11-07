//! Spikes for egui behaviour the library will depend on.
//!
//! Findings against egui 0.37:
//! - `UiBuilder::id` and `Ui::id` are deprecated; the current names are
//!   `UiBuilder::scope_id` and `Ui::scope_id`.
//! - Text overrides live in `Visuals::override_text_color` and
//!   `Style::override_font_id`; a label resolves the color
//!   into `TextShape::fallback_color` and the font into its galley job.
//! - The focused widget is `Context::memory(|m| m.focused())`.
//!
//! The first two are exercised by `tests/draw.rs`; the focus spike waits for
//! the interactive primitives.

use egui::{Id, TextEdit};
use egui_kittest::Harness;

#[test]
fn focused_widget_id_is_queried_from_memory() {
    let id = Id::unique("field");
    let mut text = String::new();
    let mut harness = Harness::new_ui(|ui| {
        let name = ui.label("field");
        ui.add(TextEdit::singleline(&mut text).id(id))
            .labelled_by(name.id);
    });
    harness.run();
    assert_eq!(harness.ctx.memory(|m| m.focused()), None);

    harness.ctx.memory_mut(|m| m.request_focus(id));
    harness.run();
    assert_eq!(harness.ctx.memory(|m| m.focused()), Some(id));
    assert!(harness.ctx.memory(|m| m.has_focus(id)));
}
