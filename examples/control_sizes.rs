//! Every owned control at the default, `sm` and `xs` sizes, side by side:
//! each column is a subtree under `tw().control_size(..)`.

use eggshell_examples::common;

use eggshell_ui::prelude::*;
use eggshell_ui::theme::size;
use eggshell_ui::{Builder, tw};

/// ```html
/// <div class="flex gap-6 p-4">
///   <div class="flex flex-col gap-3"> default: button, checkbox, ... </div>
///   <div class="flex flex-col gap-3"> sm </div>
///   <div class="flex flex-col gap-3"> xs </div>
/// </div>
/// ```
pub fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().gap(6.0).p(4.0), |s| {
        for (name, size) in [
            ("default", eggshell_ui::contract::Size::DEFAULT),
            ("sm", size::SM),
            ("xs", size::XS),
        ] {
            s.div_id(
                name,
                tw().flex()
                    .flex_col()
                    .items_start()
                    .gap(3.0)
                    .w_px(180.0)
                    .control_size(size),
                |s| {
                    s.span("title", tw().font_semibold(), name);
                    s.button("button", "Button")
                        .variant(eggshell_ui::theme::variant::OUTLINE)
                        .on_click(|_| {});
                    s.checkbox("checkbox", "Checkbox", true)
                        .on_change(|_, _| {});
                    s.radio_group("radio", 0, [(0, "One"), (1, "Two")])
                        .on_change(|_, _| {});
                    s.switch("switch", "Switch", true).on_change(|_, _| {});
                    s.segmented("segmented", 0, [(0, "Day"), (1, "Week")])
                        .on_change(|_, _| {});
                    s.slider("slider", 0.5, 0.0..=1.0)
                        .named("Slider")
                        .classes(tw().w_px(160.0))
                        .on_change(|_, _| {});
                    s.select("select", 0, [(0, "One"), (1, "Two")])
                        .named("Select")
                        .on_change(|_, _| {});
                    s.stepper("stepper", 3.0, 0.0..=9.0)
                        .named("Stepper")
                        .on_change(|_, _| {});
                    s.text_field("field", "Text")
                        .named("Field")
                        .classes(tw().w_px(160.0))
                        .on_change(|_, _| {});
                },
            );
        }
    });
}

fn main() -> eframe::Result {
    common::run("control_sizes", (), view)
}
