//! Port of the backgrounds half of egui_taffy's "Custom background demo":
//! boxes with default, transparent, custom, bordered and rounded backgrounds.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with bullseye: taffy places, egui draws. The buttons half (button,
//! selectable and clickable backgrounds with hover-dependent borders) needs M3
//! interaction and is not ported yet. The box size slider is a plain egui
//! widget beside the layout; it moves inside once M4 measures stock egui
//! widgets in the same frame.

#[path = "common/mod.rs"]
mod common;

use bullseye::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// The fill and border egui's dark theme gives a non-interactive widget.
const DEFAULT_FILL: Color32 = Color32::from_gray(27);
const DEFAULT_BORDER: Color32 = Color32::from_gray(60);

pub struct State {
    /// Side of each box, in points.
    pub size: f32,
}

impl Default for State {
    fn default() -> Self {
        Self { size: 200.0 }
    }
}

fn controls(ui: &mut egui::Ui, state: &mut State) {
    ui.label("Box size");
    ui.add(egui::Slider::new(&mut state.size, 80.0..=300.0));
}

/// ```html
/// <div class="flex items-center justify-center size-[200px] p-5 bg-[#1b1b1b] uppercase">bg default</div>
/// ```
fn background_box<A>(b: &mut B<'_, '_, A>, state: &State, paint: Tw, text: Color32, label: &str) {
    b.div(
        paint
            .flex()
            .items_center()
            .justify_center()
            .size_px(state.size)
            .p(5.0)
            .text(text),
        |b| b.text(tw().uppercase(), label),
    );
}

/// ```html
/// <div class="flex flex-col items-center justify-center gap-2.5 size-full">
///   <h2 class="uppercase">backgrounds</h2>
///   <div class="flex flex-wrap justify-center gap-2.5 p-2.5 bg-black">
///     <div class="... bg-[#1b1b1b]">bg default</div>
///     <div class="... bg-[#1b1b1b] border border-[#3c3c3c]">bg + border default</div>
///     <div class="...">bg transparent</div>
///     <div class="... bg-[#606060]">bg custom</div>
///     <div class="... bg-[#dcdcdc] border-[20px] border-white rounded-[40px] text-black">ext 1</div>
///     <div class="... border-[40px] border-[#3c3c3c] rounded-[200px]">ext 2</div>
///   </div>
/// </div>
/// ```
pub fn layout<A>(b: &mut B<'_, '_, A>, state: &State) {
    let white = Color32::WHITE;
    b.div(
        tw().flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(2.5)
            .size_full(),
        |b| {
            b.text(tw().role(common::sized(24.0)).uppercase(), "backgrounds");
            b.div(
                tw().flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(2.5)
                    .p(2.5)
                    .bg(Color32::BLACK),
                |b| {
                    background_box(b, state, tw().bg(DEFAULT_FILL), white, "bg default");
                    background_box(
                        b,
                        state,
                        tw().bg(DEFAULT_FILL)
                            .border(1.0)
                            .border_color(DEFAULT_BORDER),
                        white,
                        "bg + border default",
                    );
                    background_box(
                        b,
                        state,
                        tw().bg(Color32::TRANSPARENT),
                        white,
                        "bg transparent",
                    );
                    background_box(b, state, tw().bg(Color32::DARK_GRAY), white, "bg custom");
                    background_box(
                        b,
                        state,
                        tw().bg(Color32::LIGHT_GRAY)
                            .border(20.0)
                            .border_color(white)
                            .rounded_px(40.0),
                        Color32::BLACK,
                        "ext 1",
                    );
                    background_box(
                        b,
                        state,
                        tw().border(40.0)
                            .border_color(DEFAULT_BORDER)
                            .rounded_px(200.0),
                        white,
                        "ext 2",
                    );
                },
            );
        },
    );
}

fn main() -> eframe::Result {
    common::run(
        "custom_background",
        State::default(),
        Some(controls),
        layout,
    )
}
