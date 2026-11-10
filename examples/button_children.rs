//! A row of icon toggles where the active one shows an accent bar under its
//! icon, built as the toggle's `.children(..)`.

#[path = "common/mod.rs"]
mod common;

use common::glyphs::{self, Glyph};
use eggshell_ui::{Builder, tw};
use eggshell_ui_shadcn::size;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Light,
    Dark,
    System,
}

const TOOLS: [(Tool, Glyph, &str); 3] = [
    (Tool::Light, Glyph::Sun, "Light"),
    (Tool::Dark, Glyph::Moon, "Dark"),
    (Tool::System, Glyph::Monitor, "System"),
];

pub struct State {
    pub active: Tool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: Tool::Light,
        }
    }
}

/// ```html
/// <div class="flex gap-2 p-4">
///   <button class="relative size-9 …"><svg/><span class="absolute inset-x-0 bottom-1 mx-auto h-0.5 w-3 rounded-full bg-primary"/></button>
///   …
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let accent = common::tokens(b.ctx()).primary;
    b.div(tw().flex().gap(2.0).p(4.0), |b| {
        for (tool, glyph, name) in TOOLS {
            let icon = glyphs::icon(b.ctx(), glyph);
            let active = state.active == tool;
            b.toggle(tool, icon, active)
                .named(name)
                .size(size::ICON)
                .children(|s| {
                    if active {
                        s.div_id(
                            "accent",
                            tw().absolute()
                                .inset_x(0.0)
                                .mx_auto()
                                .bottom(1.0)
                                .w_px(12.0)
                                .h_px(2.0)
                                .rounded_full()
                                .bg(accent),
                            |_| {},
                        );
                    }
                })
                .on_change(move |state: &mut State, _| state.active = tool);
        }
    });
}

fn main() -> eframe::Result {
    common::run_app("button_children", State::default(), layout)
}
