//! Text fields with a leading or trailing icon at the default, `sm` and `xs`
//! sizes.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::theme::size;
use eggshell_ui::{Builder, Icon, icons, tw};

#[derive(Default)]
pub struct State {
    pub query: String,
}

/// ```html
/// <div class="relative">
///   <svg class="absolute left-2.5 top-2.5 size-4 text-muted-foreground"></svg>
///   <input class="pl-8" placeholder="Search">
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(4.0).p(4.0).w_px(280.0),
        |s| {
            for (name, size) in [
                ("default", eggshell_ui::contract::Size::DEFAULT),
                ("sm", size::SM),
                ("xs", size::XS),
            ] {
                s.div_id(name, tw().control_size(size), |s| {
                    s.text_field("search", state.query.clone())
                        .named("Search")
                        .hint("Search")
                        .leading(Icon::new(icons::SEARCH))
                        .on_change(|state: &mut State, query| state.query = query);
                });
            }
            s.text_field("clearable", "lucide")
                .named("Clearable")
                .trailing(Icon::new(icons::CLOSE))
                .on_change(|_: &mut State, _| {});
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("field_icons", State::default(), layout)
}
