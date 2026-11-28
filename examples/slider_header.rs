//! Sliders with a label and a readout above the track, at the default, `sm`
//! and `xs` sizes; each readout is edited as text. Then a slider whose
//! readout types past its track, and a number edited the same way with no
//! track.

use eggshell_examples::common;

use eggshell_ui::owned::number::Step;
use eggshell_ui::prelude::*;
use eggshell_ui::theme::size;
use eggshell_ui::{Builder, tw};

pub struct Levels {
    pub volume: f64,
    pub brightness: f64,
    pub bass: f64,
    pub gain: f64,
    pub trim: f64,
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            volume: 40.0,
            brightness: 75.0,
            bass: 20.0,
            gain: -20.0,
            trim: 0.0,
        }
    }
}

/// ```html
/// <div class="flex flex-col gap-6 p-4 w-[320px]">
///   <div class="grid gap-2">
///     <div class="flex justify-between"><label>Volume</label><output>40%</output></div>
///     <input type="range">
///   </div>
/// </div>
/// ```
pub fn view<'f>(b: &mut Builder<'_, 'f, Levels>, levels: &'f Levels) {
    b.div_id(
        "Page",
        tw().flex()
            .flex_col()
            .items_stretch()
            .gap(6.0)
            .p(4.0)
            .w_px(320.0),
        |s| {
            s.slider("volume", levels.volume, 0.0..=100.0)
                .step(Step::new(5.0))
                .label("Volume")
                .readout(|v| format!("{v:.0}%"))
                .editable(|text| text.trim().trim_end_matches('%').parse().ok())
                .on_change(|app: &mut Levels, v| app.volume = v);
            s.div_id("sm", tw().control_size(size::SM), |s| {
                s.slider("brightness", levels.brightness, 0.0..=100.0)
                    .label("Brightness")
                    .readout(|v| format!("{v:.0}"))
                    .editable(|text| text.trim().parse().ok())
                    .on_change(|app: &mut Levels, v| app.brightness = v);
            });
            s.div_id("xs", tw().control_size(size::XS), |s| {
                s.slider("bass", levels.bass, 0.0..=100.0)
                    .label("Bass")
                    .readout(|v| format!("{v:.0}"))
                    .editable(|text| text.trim().parse().ok())
                    .on_change(|app: &mut Levels, v| app.bass = v);
            });
            s.slider("gain", levels.gain, -60.0..=0.0)
                .label("Gain")
                .readout(|v| format!("{v:.0} dB"))
                .editable(|text| text.trim().trim_end_matches("dB").trim().parse().ok())
                .typed_range(-380.0..=380.0)
                .on_change(|app: &mut Levels, v| app.gain = v);
            s.div(tw().flex().items_center().justify_between(), |s| {
                s.text(tw(), "Trim");
                s.number("trim", levels.trim, -12.0..=12.0)
                    .format(|v| format!("{v:+.1} dB"))
                    .editable(|text| text.trim().trim_end_matches("dB").trim().parse().ok())
                    .step(Step::new(0.5))
                    .named("Trim")
                    .on_change(|app: &mut Levels, v| app.trim = v);
            });
            s.div(tw().flex().items_center().justify_between(), |s| {
                s.text(tw(), "Trim, refused");
                s.number("trim_refused", levels.trim, -12.0..=12.0)
                    .format(|v| format!("{v:+.1} dB"))
                    .editable(|text| text.trim().trim_end_matches("dB").trim().parse().ok())
                    .step(Step::new(0.5))
                    .named("Trim, refused")
                    .refused("Locked while the channel is linked")
                    .on_change(|app: &mut Levels, v| app.trim = v);
            });
        },
    );
}

fn main() -> eframe::Result {
    common::trace_input();
    common::run_app("slider_header", Levels::default(), view)
}
