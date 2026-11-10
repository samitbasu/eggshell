//! Port of egui_taffy's "Background holy grail demo": a header and footer
//! spanning two sidebars and a content area in a three-by-three grid.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with eggshell: taffy places, egui draws. The demo has no controls.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Track, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// A grid area whose label is centred.
///
/// ```html
/// <div class="flex items-center justify-center bg-[#dcdcdc]">header</div>
/// ```
fn area<A>(b: &mut B<'_, '_, A>, place: Tw, fill: Color32, text: Color32, label: &str) {
    b.div(
        place
            .flex()
            .items_center()
            .justify_center()
            .bg(fill)
            .text(text),
        |b| b.text(tw(), label),
    );
}

/// ```html
/// <div class="grid size-full grid-cols-[100px_1fr_100px] grid-rows-[100px_1fr_100px] bg-white">
///   <div class="row-start-1 col-span-3 ...">header</div>
///   <div class="row-start-2 col-start-1 ...">left</div>
///   <div class="row-start-2 col-start-2 ...">content</div>
///   <div class="row-start-2 col-start-3 ...">right</div>
///   <div class="row-start-3 col-span-3 ...">footer</div>
/// </div>
/// ```
pub fn layout<A>(b: &mut B<'_, '_, A>, _: &()) {
    let edge = Track::px(100.0);
    let middle = Track::fr(1.0);
    b.div(
        tw().grid()
            .size_full()
            .grid_cols_tracks(&[edge, middle, edge])
            .grid_rows_tracks(&[edge, middle, edge])
            .bg(Color32::WHITE),
        |b| {
            let (light, mid, dark) = (Color32::LIGHT_GRAY, Color32::GRAY, Color32::DARK_GRAY);
            let (black, white) = (Color32::BLACK, Color32::WHITE);
            area(b, tw().row_start(1).col_span(3), light, black, "header");
            area(b, tw().row_start(2).col_start(1), mid, white, "left");
            area(b, tw().row_start(2).col_start(2), white, black, "content");
            area(b, tw().row_start(2).col_start(3), dark, white, "right");
            area(b, tw().row_start(3).col_span(3), mid, white, "footer");
        },
    );
}

fn main() -> eframe::Result {
    common::run("holy_grail", (), layout)
}
