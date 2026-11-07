//! Spikes for egui behaviour the library depends on.
//!
//! Findings against egui 0.37:
//! - `UiBuilder::id` and `Ui::id` are deprecated; the current names are
//!   `UiBuilder::scope_id` and `Ui::scope_id`.
//! - Text overrides live in `Visuals::override_text_color` and
//!   `Style::override_font_id`; a label resolves the color
//!   into `TextShape::fallback_color` and the font into its galley job.
//! - The focused widget is `Context::memory(|m| m.focused())`.

use std::cell::{Cell, RefCell};

use egui::{Color32, FontId, Id, Label, Shape, TextEdit, Ui, UiBuilder};
use egui_kittest::{Harness, kittest::Queryable};

fn node_scope(ui: &mut Ui, id: Id, add_contents: impl FnOnce(&mut Ui)) {
    ui.scope_builder(
        UiBuilder::new().max_rect(ui.max_rect()).scope_id(id),
        add_contents,
    );
}

#[test]
fn child_ui_has_the_given_identity_under_any_parent() {
    let id = Id::unique("node").with("path");
    let seen = RefCell::new(Vec::new());
    let mut harness = Harness::new_ui(|ui| {
        seen.borrow_mut().clear();
        node_scope(ui, id, |ui| {
            seen.borrow_mut().push((ui.scope_id(), ui.unique_id()))
        });
        ui.push_id("other parent", |ui| {
            ui.scope_builder(UiBuilder::new().id_salt("salted"), |ui| {
                node_scope(ui, id, |ui| {
                    seen.borrow_mut().push((ui.scope_id(), ui.unique_id()))
                });
            });
        });
    });
    harness.run();
    drop(harness);
    assert_eq!(*seen.borrow(), vec![(id, id), (id, id)]);
}

#[test]
fn text_overrides_reach_labels_in_a_child_ui() {
    let ctx = crate::measure::testing::fonts_ready_context();
    let font = FontId::monospace(31.0);
    let red = Color32::RED;
    let output = ctx.run_ui(Default::default(), |ui| {
        ui.scope_builder(UiBuilder::new(), |ui| {
            ui.visuals_mut().override_text_color = Some(red);
            ui.style_mut().override_font_id = Some(font.clone());
            ui.add(Label::new("styled"));
        });
        ui.add(Label::new("plain"));
    });
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect();
    let find = |s: &str| {
        *texts
            .iter()
            .find(|t| t.galley.text() == s)
            .expect("label was painted")
    };
    let (styled, plain) = (find("styled"), find("plain"));
    assert_eq!(styled.fallback_color, red);
    assert_ne!(plain.fallback_color, red);
    assert_eq!(styled.galley.job.sections[0].format.font_id, font);
    assert_ne!(plain.galley.job.sections[0].format.font_id, font);
    output.drop_without_applying_deltas();
}

#[test]
fn kittest_runs_headless() {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        if ui.button("press").clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    assert!(!clicked.get(), "precondition: nothing clicked yet");
    harness.get_by_label("press").click();
    harness.run();
    drop(harness);
    assert!(clicked.get());
}

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
