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

#[cfg(feature = "extras")] mod vega_combobox;
#[cfg(feature = "extras")] mod vega_command;
#[cfg(feature = "extras")] mod vega_hover_card;
#[cfg(feature = "extras")] mod vega_dialog;
#[cfg(feature = "extras")] mod vega_alert_dialog;
#[cfg(feature = "extras")] mod vega_sheet;
#[cfg(feature = "extras")] mod vega_popover;
#[cfg(feature = "extras")] mod vega_badge;
#[cfg(feature = "extras")] mod vega_card;
#[cfg(feature = "extras")] mod vega_accordion;
#[cfg(feature = "extras")] mod vega_sonner;
#[cfg(feature = "extras")] mod vega_skeleton;
#[cfg(feature = "extras")] mod vega_spinner;
#[cfg(feature = "extras")] mod lyra_combobox;
#[cfg(feature = "extras")] mod lyra_command;
#[cfg(feature = "extras")] mod lyra_hover_card;
#[cfg(feature = "extras")] mod lyra_dialog;
#[cfg(feature = "extras")] mod lyra_alert_dialog;
#[cfg(feature = "extras")] mod lyra_sheet;
#[cfg(feature = "extras")] mod lyra_popover;
#[cfg(feature = "extras")] mod lyra_badge;
#[cfg(feature = "extras")] mod lyra_card;
#[cfg(feature = "extras")] mod lyra_accordion;
#[cfg(feature = "extras")] mod lyra_sonner;
#[cfg(feature = "extras")] mod lyra_skeleton;
#[cfg(feature = "extras")] mod lyra_spinner;

/// Writes the rules of `style` for `theme` into `sheet`. Their colours are
/// the style's core tokens, which the sheet is resolved with.
pub(crate) fn write(sheet: &mut Sheet, style: ShadcnStyle, theme: Theme) {
    let halves: &[(Half, Half)] = match style {
        ShadcnStyle::Vega => &[
            #[cfg(feature = "extras")] (vega_combobox::light, vega_combobox::dark),
            #[cfg(feature = "extras")] (vega_command::light, vega_command::dark),
            #[cfg(feature = "extras")] (vega_hover_card::light, vega_hover_card::dark),
            #[cfg(feature = "extras")] (vega_dialog::light, vega_dialog::dark),
            #[cfg(feature = "extras")] (vega_alert_dialog::light, vega_alert_dialog::dark),
            #[cfg(feature = "extras")] (vega_sheet::light, vega_sheet::dark),
            #[cfg(feature = "extras")] (vega_popover::light, vega_popover::dark),
            #[cfg(feature = "extras")] (vega_badge::light, vega_badge::dark),
            #[cfg(feature = "extras")] (vega_card::light, vega_card::dark),
            #[cfg(feature = "extras")] (vega_accordion::light, vega_accordion::dark),
            #[cfg(feature = "extras")] (vega_sonner::light, vega_sonner::dark),
            #[cfg(feature = "extras")] (vega_skeleton::light, vega_skeleton::dark),
            #[cfg(feature = "extras")] (vega_spinner::light, vega_spinner::dark),
        ],
        ShadcnStyle::Lyra => &[
            #[cfg(feature = "extras")] (lyra_combobox::light, lyra_combobox::dark),
            #[cfg(feature = "extras")] (lyra_command::light, lyra_command::dark),
            #[cfg(feature = "extras")] (lyra_hover_card::light, lyra_hover_card::dark),
            #[cfg(feature = "extras")] (lyra_dialog::light, lyra_dialog::dark),
            #[cfg(feature = "extras")] (lyra_alert_dialog::light, lyra_alert_dialog::dark),
            #[cfg(feature = "extras")] (lyra_sheet::light, lyra_sheet::dark),
            #[cfg(feature = "extras")] (lyra_popover::light, lyra_popover::dark),
            #[cfg(feature = "extras")] (lyra_badge::light, lyra_badge::dark),
            #[cfg(feature = "extras")] (lyra_card::light, lyra_card::dark),
            #[cfg(feature = "extras")] (lyra_accordion::light, lyra_accordion::dark),
            #[cfg(feature = "extras")] (lyra_sonner::light, lyra_sonner::dark),
            #[cfg(feature = "extras")] (lyra_skeleton::light, lyra_skeleton::dark),
            #[cfg(feature = "extras")] (lyra_spinner::light, lyra_spinner::dark),
        ],
    };
    for (light, dark) in halves {
        match theme {
            Theme::Light => light(sheet),
            Theme::Dark => dark(sheet),
        }
    }
}
