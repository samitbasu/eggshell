//! A layout drawn from a draw closure of another: an egui `Popup` or `Area`
//! with eggshell content inside. The inner layout's guard puts the outer
//! layout's drawing shell back when it finishes.

use std::cell::Cell;
use std::rc::Rc;

use eggshell_ui::{Builder, LayoutState, ShellExt as _, tw};

/// Draws an outer layout whose first node draws an inner layout, then calls
/// `after` from a draw closure of the outer layout that runs after the inner
/// one has finished.
fn nested(after: impl Fn(&egui::Ui) + 'static) {
    let after: Rc<dyn Fn(&egui::Ui)> = Rc::new(after);
    let ctx = egui::Context::default();
    ctx.run_ui(Default::default(), |ui| {
        let mut outer = LayoutState::new();
        let placed = outer
            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                b.div_id("host", tw().w_px(100.0).h_px(40.0), |_| {})
                    .draw(|app, ui| {
                        let mut inner = LayoutState::new();
                        let ctx = ui.ctx().clone();
                        let placed = inner
                            .build(&ctx, |b: &mut Builder<'_, '_, ()>| {
                                b.button("inner", "Inner").on_click(|_| {});
                            })
                            .layout(ui.max_rect());
                        placed.draw(app, ui);
                    });
                b.button("outer", "Outer").on_click(|_| {});
                let after = Rc::clone(&after);
                b.div_id("after", tw().w_px(10.0).h_px(10.0), |_| {})
                    .draw(move |_, ui| after(ui));
            })
            .layout(ui.max_rect());
        placed.draw(&mut (), ui);
    })
    .drop_without_applying_deltas();
}

#[test]
fn an_owned_button_after_a_nested_layout_draws() {
    // Panicked in `nav::take_activation` -> `Ui::shell` before the guard
    // restored the outer shell.
    nested(|_| {});
}

#[test]
fn the_outer_closures_see_a_shell_again_after_the_nested_layout() {
    let seen = Rc::new(Cell::new(false));
    let saw = Rc::clone(&seen);
    nested(move |ui| saw.set(ui.try_shell().is_some()));
    assert!(seen.get());
}

#[test]
fn outside_every_draw_there_is_no_shell() {
    let ctx = egui::Context::default();
    ctx.run_ui(Default::default(), |ui| {
        assert!(ui.try_shell().is_none());
    })
    .drop_without_applying_deltas();
}
