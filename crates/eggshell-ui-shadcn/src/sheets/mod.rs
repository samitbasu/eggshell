//! The widgets' rules in each shadcn style, generated from the pinned shadcn
//! source by `cargo xtask shadcn-import --write`; CI fails if they drift
//! (`--check`). Each rule's classes carry the shadcn class they came from.
//!
//! This file is generated too. Do not edit.

use eggshell_ui::sheet::Sheet;
use egui::Theme;

use crate::ShadcnStyle;

/// One half of a widget's rules: its light or its dark theme.
type Half = fn(&mut Sheet);

mod vega_button;
mod vega_input;
mod vega_input_group;
mod vega_select;
mod vega_dropdown_menu;
mod vega_tooltip;
mod vega_tabs;
mod lyra_button;
mod lyra_input;
mod lyra_input_group;
mod lyra_select;
mod lyra_dropdown_menu;
mod lyra_tooltip;
mod lyra_tabs;

/// Writes the rules of `style` for `theme` into `sheet`. Their colours are
/// the style's core tokens, which the sheet is resolved with.
pub(crate) fn write(sheet: &mut Sheet, style: ShadcnStyle, theme: Theme) {
    let halves: &[(Half, Half)] = match style {
        ShadcnStyle::Vega => &[
            (vega_button::light, vega_button::dark),
            (vega_input::light, vega_input::dark),
            (vega_input_group::light, vega_input_group::dark),
            (vega_select::light, vega_select::dark),
            (vega_dropdown_menu::light, vega_dropdown_menu::dark),
            (vega_tooltip::light, vega_tooltip::dark),
            (vega_tabs::light, vega_tabs::dark),
        ],
        ShadcnStyle::Lyra => &[
            (lyra_button::light, lyra_button::dark),
            (lyra_input::light, lyra_input::dark),
            (lyra_input_group::light, lyra_input_group::dark),
            (lyra_select::light, lyra_select::dark),
            (lyra_dropdown_menu::light, lyra_dropdown_menu::dark),
            (lyra_tooltip::light, lyra_tooltip::dark),
            (lyra_tabs::light, lyra_tabs::dark),
        ],
    };
    for (light, dark) in halves {
        match theme {
            Theme::Light => light(sheet),
            Theme::Dark => dark(sheet),
        }
    }
}
