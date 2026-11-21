//! How form sections and labelled rows look.

use eggshell_ui::{Style, TextSpec, Weight};
use egui::Color32;

/// TODO(prose): the look of [`form_section`](super::FormExt::form_section)s
/// and their [`labeled`](super::FormExt::labeled) rows. Lengths are in
/// points.
///
/// ```
/// use eggshell_ui_extras::form::FormStyle;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = (*style.extension::<FormStyle>()).clone();
/// look.radius = 4;
/// style.set_extension(look);
/// assert_eq!(style.extension::<FormStyle>().radius, 4);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FormStyle {
    /// The section's box: its fill.
    pub fill: Color32,
    /// The box's border colour.
    pub border: Color32,
    /// The box's border width.
    pub border_width: f32,
    /// The box's corner radius.
    pub radius: u8,
    /// Space above the first row and below the last, inside the box.
    pub padding_y: f32,
    /// A row's left and right padding inside a section.
    pub row_padding_x: f32,
    /// A row's top and bottom padding inside a section.
    pub row_padding_y: f32,
    /// The rule between a [`divided`](super::FormSectionChain::divided)
    /// section's rows, edge to edge.
    pub rule: Color32,
    /// The rule's width.
    pub rule_width: f32,
    /// The title's text role, over the box.
    pub title: TextSpec,
    /// The title's colour.
    pub title_color: Color32,
    /// Space between the title and the box.
    pub title_gap: f32,
    /// Space between a row's label and its value.
    pub row_gap: f32,
    /// A row's least height: a control's, so a text row and a control row
    /// are the same height.
    pub row_min_height: f32,
    /// A row's label colour.
    pub label_color: Color32,
    /// A [`labeled_text`](super::FormExt::labeled_text) value's colour.
    pub value_color: Color32,
    /// TODO(prose): the space an app puts between sections stacked in a
    /// column, as shadcn's `FieldSet` does.
    pub section_gap: f32,
}

impl eggshell_ui::contract::Extension for FormStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl FormStyle {
    /// TODO(prose): a form in `style`'s colours: the menu's surface as the
    /// box, the table's rule as its border and rules, a control's height as
    /// a row's.
    pub fn from_style(style: &Style) -> Self {
        let buttons = &style.button.sizes;
        Self {
            fill: style.menu.popup.paint.states.rest.fill,
            border: style.table.rule,
            border_width: 1.0,
            radius: 12,
            padding_y: 8.0,
            row_padding_x: 16.0,
            row_padding_y: 8.0,
            rule: style.table.rule,
            rule_width: style.table.rule_width,
            title: TextSpec {
                size: 14.0,
                line_height: 20.0,
                weight: Weight::Medium,
                ..style.text.clone()
            },
            title_color: style.label.rest.text,
            title_gap: 12.0,
            row_gap: 12.0,
            row_min_height: buttons.get(buttons.default_key()).height,
            label_color: style.label.rest.text,
            value_color: style.slider.readout.rest.text,
            section_gap: 24.0,
        }
    }
}
