//! The widgets' rules in each shadcn style, generated from the pinned shadcn
//! source by `cargo xtask shadcn-import --write`; CI fails if they drift
//! (`--check`). Each rule's classes carry the shadcn class they came from.

use eggshell_ui::sheet::Sheet;
use egui::Theme;

use crate::ShadcnStyle;

#[rustfmt::skip]
mod lyra_button;
#[rustfmt::skip]
mod vega_button;

/// Writes the rules of `style` for `theme` into `sheet`. Their colours are
/// the style's core tokens, which the sheet is resolved with.
pub(crate) fn write(sheet: &mut Sheet, style: ShadcnStyle, theme: Theme) {
    match (style, theme) {
        (ShadcnStyle::Vega, Theme::Light) => vega_button::light(sheet),
        (ShadcnStyle::Vega, Theme::Dark) => vega_button::dark(sheet),
        (ShadcnStyle::Lyra, Theme::Light) => lyra_button::light(sheet),
        (ShadcnStyle::Lyra, Theme::Dark) => lyra_button::dark(sheet),
    }
}
