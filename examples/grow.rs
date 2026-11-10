//! Port of egui_taffy's "Grow demo": flex-grow factors 0 to 3 in a column, and
//! the same four nested inside a grow-6 box.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with eggshell: taffy places, egui draws. The gap, margin and
//! padding sliders are stock egui sliders in leaves of the same tree, in
//! spacing units.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// The spacing the demo's slider controls, in spacing units.
pub struct State {
    pub gap: f32,
    pub margin: f32,
    pub padding: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            gap: 2.0,
            margin: 0.0,
            padding: 2.0,
        }
    }
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <!-- three range inputs, each with its caption and value -->
/// </aside>
/// ```
fn controls(b: &mut B<'_, '_, State>, state: &State) {
    common::slider_row(
        b,
        "gap",
        "Gap (spacing units)",
        state.gap,
        0.0..=12.0,
        |s| &mut s.gap,
    );
    common::slider_row(
        b,
        "margin",
        "Margin (spacing units)",
        state.margin,
        0.0..=12.0,
        |s| &mut s.margin,
    );
    common::slider_row(
        b,
        "padding",
        "Padding (spacing units)",
        state.padding,
        0.0..=12.0,
        |s| &mut s.padding,
    );
}

/// ```html
/// <div class="flex items-center grow-[2] gap-2 m-0 p-2 border border-[#a0a0a0] rounded-md">Grow 2</div>
/// ```
fn item<A>(b: &mut B<'_, '_, A>, state: &State, grow: f32, base: Tw) {
    b.div(
        spaced(state, base).flex().items_center().grow_by(grow),
        |b| b.text(tw(), format!("Grow {grow}")),
    );
}

/// The spacing every box shares.
fn spaced(state: &State, base: Tw) -> Tw {
    base.gap(state.gap)
        .m(state.margin)
        .p(state.padding)
        .border(1.0)
        .border_color(Color32::GRAY)
        .rounded_md()
}

/// ```html
/// <div class="flex flex-col items-end size-full gap-2 m-0 p-2">
///   <!-- grow-0 to grow-3, as above -->
///   <div class="flex flex-col grow-[6] self-stretch items-stretch gap-2 m-0 p-2 border border-[#a0a0a0] rounded-md">
///     <!-- grow-0 to grow-3, each with justify-center -->
///   </div>
/// </div>
/// ```
pub fn demo<A>(b: &mut B<'_, '_, A>, state: &State) {
    b.div(
        tw().flex()
            .flex_col()
            .items_end()
            .size_full()
            .gap(state.gap)
            .m(state.margin)
            .p(state.padding),
        |b| {
            for grow in 0..4 {
                item(b, state, grow as f32, tw());
            }
            b.div(
                spaced(state, tw())
                    .flex()
                    .flex_col()
                    .grow_by(6.0)
                    .self_stretch()
                    .items_stretch(),
                |b| {
                    for grow in 0..4 {
                        item(b, state, grow as f32, tw().justify_center());
                    }
                },
            );
        },
    );
}

/// The controls beside the demo.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, |b| controls(b, state), |b| demo(b, state));
}

fn main() -> eframe::Result {
    common::run_app("grow", State::default(), layout)
}
