//! Live readouts that never move the layout: right-aligned monospace values
//! in cells a fixed number of `ch` wide, beside labels and swatches.
//!
//! The values change every frame in sign and digit count; nothing around them
//! shifts. The styling is plain shadcn.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, tw};
use egui::FontFamily;

type B<'b, 'f> = Builder<'b, 'f, ()>;

/// Digits and sign a value cell has room for.
const WIDTH_CH: f32 = 9.0;

const CHANNELS: [(&str, &str); 4] = [
    ("Temperature", "°C"),
    ("Pressure", "kPa"),
    ("Flow", "L/min"),
    ("Position", "mm"),
];

/// A value of 1 to 8 characters, signed or not, from `seed`.
fn reading(seed: u64) -> String {
    let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 20;
    let digits = 1 + (mixed % 7) as u32;
    let magnitude = (mixed >> 8) % 10u64.pow(digits);
    if mixed & 1 == 1 {
        format!("-{magnitude}")
    } else {
        magnitude.to_string()
    }
}

/// ```html
/// <div class="flex flex-col gap-2 p-6 text-sm">
///   <div class="flex items-center gap-3">
///     <span class="w-32 text-muted-foreground">Temperature</span>
///     <span class="w-[9ch] font-mono text-right whitespace-nowrap">-1234</span>
///     <span class="text-muted-foreground">°C</span>
///   </div>
///   <!-- … -->
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, _: &()) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    let frame = b.ctx().cumulative_pass_nr();
    b.ctx().request_repaint();
    b.div(
        tw().flex()
            .flex_col()
            .gap(2.0)
            .p(6.0)
            .role(common::sized(14.0)),
        |b| {
            for (i, (label, unit)) in CHANNELS.into_iter().enumerate() {
                b.div(tw().flex().items_center().gap(3.0), |b| {
                    b.text(tw().w_px(128.0).text(muted), label);
                    b.span(
                        ("value", i),
                        tw().w_ch(WIDTH_CH)
                            .font(FontFamily::Monospace)
                            .text_right()
                            .whitespace_nowrap(),
                        reading(frame + i as u64),
                    );
                    b.text(tw().text(muted), unit);
                });
            }
        },
    );
}

fn main() -> eframe::Result {
    common::run("live_values", (), layout)
}
