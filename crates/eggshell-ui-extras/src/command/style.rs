//! How a command list, a combobox's list and an autosuggest's look beyond the
//! menu they are built from.

use eggshell_ui::Style;
use egui::Color32;

/// TODO(prose): the look of the rows and the box of a command list, and of
/// the lists of a combobox and an autosuggest.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandStyle {
    /// TODO(prose): the fill of the highlighted row, which the pointer and
    /// the arrow keys move.
    pub selected_fill: Color32,
    /// TODO(prose): the text of the highlighted row.
    pub selected_text: Color32,
    /// TODO(prose): how tall the list of rows grows before it scrolls.
    pub list_height: f32,
}

impl eggshell_ui::contract::Extension for CommandStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl CommandStyle {
    /// TODO(prose): the menu item's highlight, and a list `max-h-72` tall.
    pub fn from_style(style: &Style) -> Self {
        let hover = style.menu.item.paint.states.hover;
        Self {
            selected_fill: hover.fill,
            selected_text: hover.text,
            list_height: 288.0,
        }
    }
}
