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
#[cfg(feature = "extras")] mod vega_combobox;
mod vega_dropdown_menu;
mod vega_tooltip;
mod vega_checkbox;
mod vega_radio_group;
mod vega_switch;
mod vega_field;
mod vega_table;
#[cfg(feature = "extras")] mod vega_hover_card;
#[cfg(feature = "extras")] mod vega_dialog;
#[cfg(feature = "extras")] mod vega_alert_dialog;
#[cfg(feature = "extras")] mod vega_sheet;
#[cfg(feature = "extras")] mod vega_popover;
#[cfg(feature = "extras")] mod vega_badge;
#[cfg(feature = "extras")] mod vega_card;
mod vega_slider;
mod vega_tabs;
mod lyra_button;
mod lyra_input;
mod lyra_input_group;
mod lyra_select;
#[cfg(feature = "extras")] mod lyra_combobox;
mod lyra_dropdown_menu;
mod lyra_tooltip;
mod lyra_checkbox;
mod lyra_radio_group;
mod lyra_switch;
mod lyra_field;
mod lyra_table;
#[cfg(feature = "extras")] mod lyra_hover_card;
#[cfg(feature = "extras")] mod lyra_dialog;
#[cfg(feature = "extras")] mod lyra_alert_dialog;
#[cfg(feature = "extras")] mod lyra_sheet;
#[cfg(feature = "extras")] mod lyra_popover;
#[cfg(feature = "extras")] mod lyra_badge;
#[cfg(feature = "extras")] mod lyra_card;
mod lyra_slider;
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
            #[cfg(feature = "extras")] (vega_combobox::light, vega_combobox::dark),
            (vega_dropdown_menu::light, vega_dropdown_menu::dark),
            (vega_tooltip::light, vega_tooltip::dark),
            (vega_checkbox::light, vega_checkbox::dark),
            (vega_radio_group::light, vega_radio_group::dark),
            (vega_switch::light, vega_switch::dark),
            (vega_field::light, vega_field::dark),
            (vega_table::light, vega_table::dark),
            #[cfg(feature = "extras")] (vega_hover_card::light, vega_hover_card::dark),
            #[cfg(feature = "extras")] (vega_dialog::light, vega_dialog::dark),
            #[cfg(feature = "extras")] (vega_alert_dialog::light, vega_alert_dialog::dark),
            #[cfg(feature = "extras")] (vega_sheet::light, vega_sheet::dark),
            #[cfg(feature = "extras")] (vega_popover::light, vega_popover::dark),
            #[cfg(feature = "extras")] (vega_badge::light, vega_badge::dark),
            #[cfg(feature = "extras")] (vega_card::light, vega_card::dark),
            (vega_slider::light, vega_slider::dark),
            (vega_tabs::light, vega_tabs::dark),
        ],
        ShadcnStyle::Lyra => &[
            (lyra_button::light, lyra_button::dark),
            (lyra_input::light, lyra_input::dark),
            (lyra_input_group::light, lyra_input_group::dark),
            (lyra_select::light, lyra_select::dark),
            #[cfg(feature = "extras")] (lyra_combobox::light, lyra_combobox::dark),
            (lyra_dropdown_menu::light, lyra_dropdown_menu::dark),
            (lyra_tooltip::light, lyra_tooltip::dark),
            (lyra_checkbox::light, lyra_checkbox::dark),
            (lyra_radio_group::light, lyra_radio_group::dark),
            (lyra_switch::light, lyra_switch::dark),
            (lyra_field::light, lyra_field::dark),
            (lyra_table::light, lyra_table::dark),
            #[cfg(feature = "extras")] (lyra_hover_card::light, lyra_hover_card::dark),
            #[cfg(feature = "extras")] (lyra_dialog::light, lyra_dialog::dark),
            #[cfg(feature = "extras")] (lyra_alert_dialog::light, lyra_alert_dialog::dark),
            #[cfg(feature = "extras")] (lyra_sheet::light, lyra_sheet::dark),
            #[cfg(feature = "extras")] (lyra_popover::light, lyra_popover::dark),
            #[cfg(feature = "extras")] (lyra_badge::light, lyra_badge::dark),
            #[cfg(feature = "extras")] (lyra_card::light, lyra_card::dark),
            (lyra_slider::light, lyra_slider::dark),
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
