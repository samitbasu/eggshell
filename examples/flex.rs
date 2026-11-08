//! Port of egui_taffy's "Flex demo": justify-content against flex-grow, with
//! four align-self items per row.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with bullseye: taffy places, egui draws. The original's buttons are
//! plain boxes here, and its rows scroll where these are clipped. The demo has
//! no controls.

#[path = "common/mod.rs"]
mod common;

use bullseye::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// One row's classes: justify-content name and class, and the items' flex-grow.
fn rows() -> [(&'static str, Tw, f32); 7] {
    [
        ("start", tw().justify_start(), 0.0),
        ("end", tw().justify_end(), 0.0),
        ("stretch", tw().justify_stretch(), 0.0),
        ("stretch", tw().justify_stretch(), 1.0),
        ("center", tw().justify_center(), 0.0),
        ("space-between", tw().justify_between(), 0.0),
        ("space-around", tw().justify_around(), 0.0),
    ]
}

/// The four align-self values, one item each.
fn aligns() -> [(&'static str, Tw); 4] {
    [
        ("start", tw().self_start()),
        ("end", tw().self_end()),
        ("center", tw().self_center()),
        ("stretch", tw().self_stretch()),
    ]
}

/// ```html
/// <div class="px-3 py-1 bg-[#dcdcdc] border border-[#a0a0a0] rounded-md self-start grow-0">start</div>
/// ```
fn button<A>(b: &mut B<'_, '_, A>, align: Tw, grow: f32, label: &str) {
    b.div(
        align
            .grow_by(grow)
            .px(3.0)
            .py(1.0)
            .bg(Color32::LIGHT_GRAY)
            .border(1.0)
            .border_color(Color32::GRAY)
            .rounded_md(),
        |b| b.text(tw(), label),
    );
}

/// One row: a caption column, then a bordered row of four items.
///
/// ```html
/// <div class="flex flex-row gap-2 p-2 min-h-[48px] border border-[#a0a0a0] rounded-md">
///   <div class="flex flex-col w-[200px] shrink-0">
///     <span>Justify content: space-between</span>
///     <span>Flex grow: 0</span>
///     <span>Align self:</span>
///   </div>
///   <div class="flex flex-row grow gap-2 p-2 min-h-[48px] border border-[#a0a0a0] rounded-md justify-between">
///     <div class="self-start grow-0 px-3 py-1 bg-[#dcdcdc] border border-[#a0a0a0] rounded-md">start</div>
///     <!-- end, center and stretch: self-end, self-center, self-stretch -->
///   </div>
/// </div>
/// ```
fn row<A>(b: &mut B<'_, '_, A>, name: &str, justify: Tw, grow: f32) {
    let framed = |base: Tw| {
        base.flex()
            .flex_row()
            .gap(2.0)
            .p(2.0)
            .min_h_px(48.0)
            .border(1.0)
            .border_color(Color32::GRAY)
            .rounded_md()
    };
    b.div(framed(tw()), |b| {
        b.div(tw().flex().flex_col().w_px(200.0).shrink_0(), |b| {
            b.text(tw(), format!("Justify content: {name}"));
            b.text(tw(), format!("Flex grow: {grow}"));
            b.text(tw(), "Align self:");
        });
        b.div(framed(justify).grow(), |b| {
            for (label, align) in aligns() {
                button(b, align, grow, label);
            }
        });
    });
}

/// ```html
/// <div class="flex flex-col gap-2 p-2 size-full bg-white">
///   <!-- seven rows -->
/// </div>
/// ```
pub fn layout<A>(b: &mut B<'_, '_, A>, _: &()) {
    b.div(
        tw().flex()
            .flex_col()
            .gap(2.0)
            .p(2.0)
            .size_full()
            .bg(Color32::WHITE),
        |b| {
            for (name, justify, grow) in rows() {
                row(b, name, justify, grow);
            }
        },
    );
}

fn main() -> eframe::Result {
    common::run("flex", (), layout)
}
