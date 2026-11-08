//! Port of egui_taffy's "Button demo": buttons that hold other layouts, a plain
//! one that counts clicks and a selectable one that toggles.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with bullseye. A button is a `div_id` that senses clicks, with
//! hover and active paint as variants; being selected is app state known at
//! build time, so it is base paint. The reset button is a stock egui button in
//! a leaf of the same tree.

#[path = "common/mod.rs"]
mod common;

use bullseye::widget::Button;
use bullseye::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

pub const FILL: Color32 = Color32::from_gray(232);
pub const FILL_HOVER: Color32 = Color32::from_gray(218);
pub const FILL_PRESSED: Color32 = Color32::from_gray(200);
pub const FILL_SELECTED: Color32 = Color32::from_rgb(170, 196, 232);
const EDGE: Color32 = Color32::from_gray(176);
const INK: Color32 = Color32::from_gray(20);

#[derive(Default)]
pub struct State {
    pub clicks: u32,
    pub selected: bool,
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]"><button>Reset</button></aside>
/// ```
fn controls(b: &mut B<'_, '_, State>) {
    b.widget(
        "reset",
        tw().self_start(),
        Button::new("Reset"),
        |state, ui, button| {
            if button.show(ui).clicked() {
                *state = State::default();
            }
        },
    );
}

type Align = fn(Tw) -> Tw;

const ALIGNMENTS: [(&str, Align); 4] = [
    ("START", Tw::self_start),
    ("CENTER", Tw::self_center),
    ("END", Tw::self_end),
    ("STRETCH", Tw::self_stretch),
];

/// A button's look. A selected one keeps its fill under the pointer.
///
/// ```html
/// <div class="flex flex-col items-center p-2 rounded border border-[#b0b0b0] text-[#141414]
///             bg-[#e8e8e8] hover:bg-[#dadada] active:bg-[#c8c8c8]">...</div>
/// <div class="... bg-[#aac4e8] active:bg-[#c8c8c8]">...</div>
/// ```
fn button(selected: bool) -> Tw {
    let look = tw()
        .flex()
        .flex_col()
        .items_center()
        .p(2.0)
        .rounded()
        .border(1.0)
        .border_color(EDGE)
        .text(INK)
        .active(|p| p.bg(FILL_PRESSED));
    if selected {
        look.bg(FILL_SELECTED)
    } else {
        look.bg(FILL).hover(|p| p.bg(FILL_HOVER))
    }
}

/// The button's content: a title and one row per cross-axis alignment, each
/// aligned within the button as its name says.
///
/// ```html
/// <span>Button</span>
/// <div class="flex flex-col p-1 self-start"><span class="self-center">START</span></div>
/// <div class="flex flex-col p-1 self-center"><span class="self-center">CENTER</span></div>
/// ...
/// ```
fn content<A>(b: &mut B<'_, '_, A>, title: &str) {
    b.text(tw(), title);
    for (name, align) in ALIGNMENTS {
        b.div(align(tw().flex().flex_col().p(1.0)), |b| {
            b.text(tw().self_center(), name);
        });
    }
}

/// ```html
/// <div class="flex flex-col items-stretch gap-2 w-full max-w-full p-2">
///   <div class="button">...</div>
///   <span>Button clicked 0 times</span>
///   <hr>
///   <div class="button selected">...</div>
///   <span>Selected: false</span>
/// </div>
/// ```
pub fn demo(b: &mut B<'_, '_, State>, state: &State) {
    b.div(
        tw().flex()
            .flex_col()
            .items_stretch()
            .gap(2.0)
            .w_full()
            .max_w_full()
            .p(2.0),
        |b| {
            b.div_id(
                "button",
                button(false),
                common::on_click::<State>(|state| state.clicks += 1),
                |b| content(b, "Button"),
            );
            b.text(tw(), format!("Button clicked {} times", state.clicks));
            b.rule(tw().h_px(1.0).bg(EDGE));
            b.div_id(
                "selectable",
                button(state.selected),
                common::on_click::<State>(|state| state.selected = !state.selected),
                |b| content(b, "Selectable button"),
            );
            b.text(tw(), format!("Selected: {}", state.selected));
        },
    );
}

/// The controls beside the demo.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, controls, |b| demo(b, state));
}

fn main() -> eframe::Result {
    common::run_app("button", State::default(), layout)
}
