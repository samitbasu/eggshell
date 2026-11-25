//! The widgets' rules in each shadcn style, generated from the pinned shadcn
//! source by `cargo xtask shadcn-import --write`; CI fails if they drift
//! (`--check`). Each rule's classes carry the shadcn class they came from.
//!
//! This file is generated too. Do not edit.

use crate::sheet::Sheet;
use egui::Theme;

use crate::theme::ShadcnStyle;

/// One half of a widget's rules: its light or its dark theme.
type Half = fn(&mut Sheet);

mod vega_button;
mod vega_input;
mod vega_input_group;
mod vega_select;
#[cfg(feature = "command")] mod vega_combobox;
#[cfg(feature = "command")] mod vega_command;
mod vega_dropdown_menu;
mod vega_separator;
mod vega_tooltip;
mod vega_checkbox;
mod vega_radio_group;
mod vega_switch;
mod vega_field;
mod vega_table;
#[cfg(feature = "hover-card")] mod vega_hover_card;
#[cfg(feature = "dialog")] mod vega_dialog;
#[cfg(feature = "dialog")] mod vega_alert_dialog;
#[cfg(feature = "dialog")] mod vega_sheet;
#[cfg(feature = "dialog")] mod vega_popover;
#[cfg(feature = "badge")] mod vega_badge;
#[cfg(feature = "form")] mod vega_card;
#[cfg(feature = "form")] mod vega_item;
mod vega_slider;
#[cfg(feature = "disclosure")] mod vega_accordion;
#[cfg(feature = "notify")] mod vega_sonner;
#[cfg(feature = "skeleton")] mod vega_skeleton;
#[cfg(feature = "spinner")] mod vega_spinner;
mod vega_progress;
mod vega_tabs;
mod lyra_button;
mod lyra_input;
mod lyra_input_group;
mod lyra_select;
#[cfg(feature = "command")] mod lyra_combobox;
#[cfg(feature = "command")] mod lyra_command;
mod lyra_dropdown_menu;
mod lyra_separator;
mod lyra_tooltip;
mod lyra_checkbox;
mod lyra_radio_group;
mod lyra_switch;
mod lyra_field;
mod lyra_table;
#[cfg(feature = "hover-card")] mod lyra_hover_card;
#[cfg(feature = "dialog")] mod lyra_dialog;
#[cfg(feature = "dialog")] mod lyra_alert_dialog;
#[cfg(feature = "dialog")] mod lyra_sheet;
#[cfg(feature = "dialog")] mod lyra_popover;
#[cfg(feature = "badge")] mod lyra_badge;
#[cfg(feature = "form")] mod lyra_card;
#[cfg(feature = "form")] mod lyra_item;
mod lyra_slider;
#[cfg(feature = "disclosure")] mod lyra_accordion;
#[cfg(feature = "notify")] mod lyra_sonner;
#[cfg(feature = "skeleton")] mod lyra_skeleton;
#[cfg(feature = "spinner")] mod lyra_spinner;
mod lyra_progress;
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
            #[cfg(feature = "command")] (vega_combobox::light, vega_combobox::dark),
            #[cfg(feature = "command")] (vega_command::light, vega_command::dark),
            (vega_dropdown_menu::light, vega_dropdown_menu::dark),
            (vega_separator::light, vega_separator::dark),
            (vega_tooltip::light, vega_tooltip::dark),
            (vega_checkbox::light, vega_checkbox::dark),
            (vega_radio_group::light, vega_radio_group::dark),
            (vega_switch::light, vega_switch::dark),
            (vega_field::light, vega_field::dark),
            (vega_table::light, vega_table::dark),
            #[cfg(feature = "hover-card")] (vega_hover_card::light, vega_hover_card::dark),
            #[cfg(feature = "dialog")] (vega_dialog::light, vega_dialog::dark),
            #[cfg(feature = "dialog")] (vega_alert_dialog::light, vega_alert_dialog::dark),
            #[cfg(feature = "dialog")] (vega_sheet::light, vega_sheet::dark),
            #[cfg(feature = "dialog")] (vega_popover::light, vega_popover::dark),
            #[cfg(feature = "badge")] (vega_badge::light, vega_badge::dark),
            #[cfg(feature = "form")] (vega_card::light, vega_card::dark),
            #[cfg(feature = "form")] (vega_item::light, vega_item::dark),
            (vega_slider::light, vega_slider::dark),
            #[cfg(feature = "disclosure")] (vega_accordion::light, vega_accordion::dark),
            #[cfg(feature = "notify")] (vega_sonner::light, vega_sonner::dark),
            #[cfg(feature = "skeleton")] (vega_skeleton::light, vega_skeleton::dark),
            #[cfg(feature = "spinner")] (vega_spinner::light, vega_spinner::dark),
            (vega_progress::light, vega_progress::dark),
            (vega_tabs::light, vega_tabs::dark),
        ],
        ShadcnStyle::Lyra => &[
            (lyra_button::light, lyra_button::dark),
            (lyra_input::light, lyra_input::dark),
            (lyra_input_group::light, lyra_input_group::dark),
            (lyra_select::light, lyra_select::dark),
            #[cfg(feature = "command")] (lyra_combobox::light, lyra_combobox::dark),
            #[cfg(feature = "command")] (lyra_command::light, lyra_command::dark),
            (lyra_dropdown_menu::light, lyra_dropdown_menu::dark),
            (lyra_separator::light, lyra_separator::dark),
            (lyra_tooltip::light, lyra_tooltip::dark),
            (lyra_checkbox::light, lyra_checkbox::dark),
            (lyra_radio_group::light, lyra_radio_group::dark),
            (lyra_switch::light, lyra_switch::dark),
            (lyra_field::light, lyra_field::dark),
            (lyra_table::light, lyra_table::dark),
            #[cfg(feature = "hover-card")] (lyra_hover_card::light, lyra_hover_card::dark),
            #[cfg(feature = "dialog")] (lyra_dialog::light, lyra_dialog::dark),
            #[cfg(feature = "dialog")] (lyra_alert_dialog::light, lyra_alert_dialog::dark),
            #[cfg(feature = "dialog")] (lyra_sheet::light, lyra_sheet::dark),
            #[cfg(feature = "dialog")] (lyra_popover::light, lyra_popover::dark),
            #[cfg(feature = "badge")] (lyra_badge::light, lyra_badge::dark),
            #[cfg(feature = "form")] (lyra_card::light, lyra_card::dark),
            #[cfg(feature = "form")] (lyra_item::light, lyra_item::dark),
            (lyra_slider::light, lyra_slider::dark),
            #[cfg(feature = "disclosure")] (lyra_accordion::light, lyra_accordion::dark),
            #[cfg(feature = "notify")] (lyra_sonner::light, lyra_sonner::dark),
            #[cfg(feature = "skeleton")] (lyra_skeleton::light, lyra_skeleton::dark),
            #[cfg(feature = "spinner")] (lyra_spinner::light, lyra_spinner::dark),
            (lyra_progress::light, lyra_progress::dark),
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
