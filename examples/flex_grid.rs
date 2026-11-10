//! Port of egui_taffy's "Grid demo": spans, and every align-self against
//! justify-self combination in a six-column grid.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with eggshell: taffy places, egui draws. The gap slider is a stock
//! egui slider in a leaf of the same tree.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Track, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

pub struct State {
    /// In spacing units.
    pub gap: f32,
}

impl Default for State {
    fn default() -> Self {
        Self { gap: 2.0 }
    }
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <!-- a range input for the gap, with its caption and value -->
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
}

#[derive(Clone, Copy)]
enum Align {
    Start,
    Center,
    End,
    Stretch,
}

const ALIGNS: [Align; 4] = [Align::Start, Align::Center, Align::End, Align::Stretch];

impl Align {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Center => "Center",
            Self::End => "End",
            Self::Stretch => "Stretch",
        }
    }

    fn align_self(self, tw: Tw) -> Tw {
        match self {
            Self::Start => tw.self_start(),
            Self::Center => tw.self_center(),
            Self::End => tw.self_end(),
            Self::Stretch => tw.self_stretch(),
        }
    }

    fn justify_self(self, tw: Tw) -> Tw {
        match self {
            Self::Start => tw.justify_self_start(),
            Self::Center => tw.justify_self_center(),
            Self::End => tw.justify_self_end(),
            Self::Stretch => tw.justify_self_stretch(),
        }
    }
}

/// ```html
/// <div class="flex items-center justify-center border border-[#a0a0a0] rounded-md">Start</div>
/// ```
fn cell<A>(b: &mut B<'_, '_, A>, extra: Tw, label: impl Into<String>) {
    b.div(
        extra
            .flex()
            .items_center()
            .justify_center()
            .border(1.0)
            .border_color(Color32::GRAY)
            .rounded_md(),
        |b| b.text(tw(), label),
    );
}

/// ```html
/// <div class="grid grid-cols-[1fr_1fr_1fr_1fr_1fr_1.5fr] grid-rows-6 gap-2 p-2 size-full items-stretch justify-items-stretch">
///   <div class="col-span-5 flex items-center justify-center border border-[#a0a0a0] rounded-md">Col span 5</div>
///   <div class="row-span-6 flex items-center justify-center border border-[#a0a0a0] rounded-md">Row span 6</div>
///   <div></div>
///   <!-- four column headers: Start, Center, End, Stretch -->
///   <!-- four rows: a row header, then four cells, e.g. -->
///   <div class="self-start justify-self-center p-1 flex items-center justify-center border border-[#a0a0a0] rounded-md">Start Center</div>
/// </div>
/// ```
pub fn demo<A>(b: &mut B<'_, '_, A>, state: &State) {
    let columns = [1.0, 1.0, 1.0, 1.0, 1.0, 1.5].map(Track::fr);
    b.div(
        tw().grid()
            .grid_cols_tracks(&columns)
            .grid_rows(6)
            .gap(state.gap)
            .p(2.0)
            .size_full()
            .items_stretch()
            .justify_items_stretch(),
        |b| {
            cell(b, tw().col_span(5), "Col span 5");
            cell(b, tw().row_span(6), "Row span 6");
            b.div(tw(), |_| {});
            for header in ALIGNS {
                cell(b, tw(), header.name());
            }
            for align in ALIGNS {
                cell(b, tw(), align.name());
                for justify in ALIGNS {
                    let extra = justify.justify_self(align.align_self(tw())).p(1.0);
                    cell(b, extra, format!("{} {}", align.name(), justify.name()));
                }
            }
        },
    );
}

/// The controls beside the demo.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, |b| controls(b, state), |b| demo(b, state));
}

fn main() -> eframe::Result {
    common::run_app("flex_grid", State::default(), layout)
}
