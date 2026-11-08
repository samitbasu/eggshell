//! Port of egui_taffy's "Flex wrap demo": a column of aligned lines and a
//! wrapping row of bordered words.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with bullseye: taffy places, egui draws. The gap slider is a stock
//! egui slider and the wrap choice a segmented control, both inside the tree.
//! The original's stock egui labels and buttons are text boxes here.

#[path = "common/mod.rs"]
mod common;

use bullseye::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

const WORDS: [&str; 18] = [
    "Lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "Etiam",
    "orci",
    "velit",
    "suscipit",
    "in",
    "tortor",
    "id",
    "ornare",
    "fringilla",
    "tortor",
];

/// Whether the row of words wraps.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Wrapping {
    Wrap,
    NoWrap,
}

impl Wrapping {
    fn classes(self) -> Tw {
        match self {
            Self::Wrap => tw().flex_wrap(),
            Self::NoWrap => tw().flex_nowrap(),
        }
    }
}

pub struct State {
    /// In spacing units.
    pub gap: f32,
    pub wrapping: Wrapping,
}

impl Default for State {
    fn default() -> Self {
        Self {
            gap: 2.0,
            wrapping: Wrapping::Wrap,
        }
    }
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <!-- a range input for the gap, and a two-way choice for the words -->
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
    common::captioned(b, "Words", |b| {
        common::choice(
            b,
            "wrapping",
            state.wrapping,
            &[
                (Wrapping::Wrap, "flex-wrap"),
                (Wrapping::NoWrap, "flex-nowrap"),
            ],
            |s| &mut s.wrapping,
        );
    });
}

/// ```html
/// <div class="p-2 border border-[#a0a0a0] rounded-md grow">Text with border</div>
/// ```
fn bordered() -> Tw {
    tw().p(2.0)
        .border(1.0)
        .border_color(Color32::GRAY)
        .rounded_md()
}

/// ```html
/// <hr class="h-px bg-[#a0a0a0]">
/// ```
fn separator<A>(b: &mut B<'_, '_, A>) {
    b.rule(tw().h_px(1.0).bg(Color32::GRAY));
}

/// ```html
/// <div class="flex flex-col items-stretch justify-center gap-2 p-2 w-full">
///   <span>Hello from bullseye</span>
///   <hr class="h-px bg-[#a0a0a0]">
///   <span>Left aligned text</span>
///   <span class="self-center">Centered text</span>
///   <span class="self-end">Right aligned text</span>
///   <div class="p-2 border border-[#a0a0a0] rounded-md">Text with border</div>
///   <hr class="h-px bg-[#a0a0a0]">
///   <div class="flex flex-wrap justify-center gap-2 p-2">
///     <div class="grow p-2 border border-[#a0a0a0] rounded-md">Lorem</div>
///     <!-- one per word -->
///   </div>
/// </div>
/// ```
pub fn demo<A>(b: &mut B<'_, '_, A>, state: &State) {
    b.div(
        tw().flex()
            .flex_col()
            .items_stretch()
            .justify_center()
            .gap(2.0)
            .p(2.0)
            .w_full(),
        |b| {
            b.text(tw(), "Hello from bullseye");
            separator(b);
            b.text(tw(), "Left aligned text");
            b.text(tw().self_center(), "Centered text");
            b.text(tw().self_end(), "Right aligned text");
            b.div(bordered(), |b| b.text(tw(), "Text with border"));
            separator(b);
            b.div(
                state
                    .wrapping
                    .classes()
                    .flex()
                    .justify_center()
                    .gap(state.gap)
                    .p(2.0),
                |b| {
                    for word in WORDS {
                        b.div(bordered().grow(), |b| b.text(tw(), word));
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
    common::run_app("flex_wrap", State::default(), layout)
}
