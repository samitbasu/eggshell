//! How a command list, a combobox's list and an autosuggest's look beyond the
//! menu they are built from.

use egui::Color32;

/// TODO(prose): the look of the rows and the box of a command list, and of
/// the lists of a combobox and an autosuggest.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// TODO(prose): the fill of the highlighted row, which the pointer and
    /// the arrow keys move.
    pub selected_fill: Color32,
    /// TODO(prose): the text of the highlighted row.
    pub selected_text: Color32,
    /// TODO(prose): how tall the list of rows grows before it scrolls.
    pub list_height: f32,
}

impl Style {
    /// TODO(prose): the accent as the highlight, and a list `max-h-72` tall.
    pub(crate) fn from_base(style: &crate::contract::Base<'_>) -> Self {
        Self {
            selected_fill: style.tokens.accent,
            selected_text: style.tokens.accent_foreground,
            list_height: 288.0,
        }
    }
}
