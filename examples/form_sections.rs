//! Grouped form sections of labelled rows (#370): a titled, divided section
//! beside every control with a track, a plain one, and a labelled row outside
//! any section.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Progress, tw};
use eggshell_ui_extras::form::{FormExt, FormStyle};

pub struct Settings {
    pub gain: f64,
    pub mute: bool,
    pub rate: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gain: 60.0,
            mute: false,
            rate: 48,
        }
    }
}

/// ```html
/// <div class="flex flex-col gap-6 p-4 w-[420px]">
///   <fieldset><legend>Audio</legend>
///     <div class="rounded-xl border bg-card py-2 divide-y">
///       <div role="group">Gain <input type="range"></div> …
///     </div>
///   </fieldset>
///   <fieldset><legend>Device</legend> …</fieldset>
///   <div role="group">Version <span class="text-muted-foreground">0.1.0</span></div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, Settings>, settings: &'f Settings) {
    let gap = b.style().extension::<FormStyle>().section_gap;
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(gap / 4.0).p(4.0).w_px(420.0),
        |b| {
            b.form_section("audio")
                .title("Audio")
                .divided()
                .children(|b| {
                    b.labeled("Gain", |b| {
                        b.div(tw().w_px(160.0), |b| {
                            b.slider("gain", settings.gain, 0.0..=100.0)
                                .named("Gain")
                                .on_change(|s: &mut Settings, v| s.gain = v);
                        });
                    });
                    b.labeled("Mute", |b| {
                        b.switch_unlabelled("mute", settings.mute)
                            .named("Mute")
                            .on_change(|s: &mut Settings, on| s.mute = on);
                    });
                    b.labeled("Sample rate", |b| {
                        b.segmented("rate", settings.rate, [(44, "44.1 kHz"), (48, "48 kHz")])
                            .named("Sample rate")
                            .on_change(|s: &mut Settings, rate| s.rate = rate);
                    });
                    b.labeled_text("Latency", "5.3 ms");
                });
            b.form_section("device").title("Device").children(|b| {
                b.labeled_text("Name", "Built-in Output");
                b.labeled("Buffer", |b| {
                    b.progress("buffer", Progress::new(0.4))
                        .named("Buffer")
                        .classes(tw().w_px(160.0))
                        .add();
                });
                b.labeled_text(
                    "A label long enough that it wraps onto a second line",
                    "Value",
                );
            });
            b.labeled_text("Version", "0.1.0");
        },
    );
}

fn main() -> eframe::Result {
    common::trace_input();
    common::run_app("form_sections", Settings::default(), layout)
}
