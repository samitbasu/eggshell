//! How form sections and labelled rows look.

use crate::{TextSpec, Weight};
use egui::Color32;

/// TODO(prose): the look of [`form_section`](super::Ext::form_section)s
/// and their [`labeled`](super::Ext::labeled) rows. Lengths are in
/// points.
///
/// ```
/// use eggshell_ui::owned::form::Style;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = style.form.clone();
/// look.radius = 4;
/// style.form = look;
/// assert_eq!(style.form.radius, 4);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
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
    /// The rule between a [`divided`](super::SectionChain::divided)
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
    /// A [`labeled_text`](super::Ext::labeled_text) value's colour.
    pub value_color: Color32,
    /// TODO(prose): the space an app puts between sections stacked in a
    /// column, as shadcn's `FieldSet` does.
    pub section_gap: f32,
}

impl Style {
    /// TODO(prose): a form in `style`'s colours: the menu's surface as the
    /// box, the table's rule as its border and rules, a control's height as
    /// a row's.
    pub(crate) fn from_base(style: &crate::contract::Base<'_>) -> Self {
        let buttons = &style.toggle.sizes;
        Self {
            fill: style.tokens.popover,
            border: style.tokens.border,
            border_width: 1.0,
            radius: 12,
            padding_y: 8.0,
            row_padding_x: 16.0,
            row_padding_y: 8.0,
            rule: style.tokens.border,
            rule_width: 1.0,
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
