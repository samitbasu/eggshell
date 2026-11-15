//! The standard vector icons at 12, 16, 20 and 24 pt,
//! and in a button, a segmented cell and a menu trigger.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::icons::{self, Strokes};
use eggshell_ui::{Builder, Icon, tw};

const SET: [&Strokes; 16] = [
    &icons::CLOSE,
    &icons::CHECK,
    &icons::CHEVRON_DOWN,
    &icons::CHEVRON_UP,
    &icons::CHEVRON_LEFT,
    &icons::CHEVRON_RIGHT,
    &icons::PLUS,
    &icons::MINUS,
    &icons::MAXIMIZE,
    &icons::POP_OUT,
    &icons::PAUSE,
    &icons::PLAY,
    &icons::WARNING,
    &icons::SEARCH,
    &icons::HELP,
    &icons::GEAR,
];

const SIZES: [f32; 4] = [12.0, 16.0, 20.0, 24.0];

/// ```html
/// <div class="flex flex-col gap-4 p-4">
///   <div class="flex gap-3"> icons at 12 pt </div> ...
///   <div class="flex gap-3"> button, segmented, menu </div>
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div_id("Page", tw().flex().flex_col().gap(3.0).p(4.0), |s| {
        for size in SIZES {
            s.div_id(size as u32, tw().flex().gap(3.0).items_center(), |s| {
                for strokes in SET {
                    s.paint(tw().size_px(size), move |painter, rect, color| {
                        strokes.paint(painter, rect, color);
                    });
                }
            });
        }
        s.div_id("context", tw().flex().gap(3.0).items_center(), |s| {
            s.button("button", "Settings")
                .icon(Icon::vector(&icons::GEAR))
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .on_click(|_| {});
            s.segmented(
                "segmented",
                0,
                [
                    (0, Icon::vector(&icons::PLAY), "Play"),
                    (1, Icon::vector(&icons::PAUSE), "Pause"),
                ],
            )
            .on_change(|_, _| {});
            s.dropdown_menu("menu", "Menu")
                .icon(Icon::vector(&icons::CHEVRON_DOWN))
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .items(|m| {
                    m.item("settings", "Settings")
                        .icon(Icon::vector(&icons::GEAR))
                        .on_click(|_| {});
                    m.item("search", "Search")
                        .icon(Icon::vector(&icons::SEARCH))
                        .on_click(|_| {});
                    m.item("help", "Help").inset().on_click(|_| {});
                });
        });
    });
}

fn main() -> eframe::Result {
    common::run("icons", (), layout)
}
