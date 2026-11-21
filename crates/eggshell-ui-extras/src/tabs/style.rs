//! How tabs look beyond their tab list, which is drawn as the segmented
//! control is.

use eggshell_ui::Style;

/// TODO(prose): the look of [`tabs`](super::TabsExt::tabs) around the tab
/// list; the list and its tabs are drawn in the style's segmented look.
/// Lengths are in points.
///
/// ```
/// use eggshell_ui_extras::tabs::TabsStyle;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = (*style.extension::<TabsStyle>()).clone();
/// look.gap = 12.0;
/// style.set_extension(look);
/// assert_eq!(style.extension::<TabsStyle>().gap, 12.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TabsStyle {
    /// Gap between the tab list and the panel.
    pub gap: f32,
}

impl eggshell_ui::contract::Extension for TabsStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl TabsStyle {
    /// TODO(prose): an 8 pt gap between the list and the panel.
    pub fn from_style(_: &Style) -> Self {
        Self { gap: 8.0 }
    }
}
