//! How the Markdown widget looks.

use crate::Weight;
use crate::contract::Em;
use egui::Color32;

use super::parse::Level;

/// The space between a document's blocks and its rich text, which
/// [`Typeset`](super::Typeset) derives for the widget as it builds. The
/// blocks' own look is the sheet's, on [`Part`](super::Part).
///
/// Lengths are in Tailwind spacing units (a unit is 4 points), as [`Tw`](crate::Tw)'s
/// spacing classes take them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Style {
    /// Space above every block but the first, outside lists.
    pub block_gap: f32,
    /// Space above a paragraph that follows a heading, in place
    /// of `block_gap`.
    pub after_heading: f32,
    /// Space above a thematic break (`---`), in place of
    /// `block_gap`.
    pub rule_gap: f32,
    /// Space above each level's headings, `#` first.
    pub headings: [HeadingFlow; 6],
    /// Space between a list item's blocks.
    pub item_gap: f32,
    /// The document's links and inline code, over the page's
    /// [`RichTextStyle`](crate::contract::RichTextStyle).
    pub rich: RichLook,
}

/// A document's links and inline code. Lengths are in points.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RichLook {
    pub link: Color32,
    pub link_underline: Color32,
    pub link_weight: Weight,
    /// Behind inline code.
    pub code_fill: Color32,
    pub code_radius: u8,
    /// Inline code's size, relative to the text around it.
    pub code_size: Em,
    pub code_padding_x: f32,
    pub code_padding_y: f32,
}

impl Style {
    /// The page's style with this document's rich text over it.
    pub(super) fn restyle(&self, style: &mut crate::Style) {
        let look = &self.rich;
        let rich = &mut style.rich;
        rich.link = look.link;
        rich.link_underline = Some(look.link_underline);
        rich.link_weight = look.link_weight;
        rich.code.fill = look.code_fill;
        rich.code.radius = look.code_radius;
        rich.code.size = look.code_size;
        rich.code.padding_x = look.code_padding_x;
        rich.code.padding_y = look.code_padding_y;
    }
}

/// The space above a heading.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeadingFlow {
    /// Space above the heading, in place of `block_gap`.
    pub margin_top: f32,
    /// Space above the heading when it follows another, in
    /// place of `margin_top`.
    pub after_heading: f32,
    /// Space above the heading when it follows a thematic
    /// break, in place of `margin_top`.
    pub after_rule: f32,
}

impl Style {
    /// The space above `level`'s headings.
    pub(super) fn heading(&self, level: Level) -> &HeadingFlow {
        let [h1, h2, h3, h4, h5, h6] = &self.headings;
        match level {
            Level::H1 => h1,
            Level::H2 => h2,
            Level::H3 => h3,
            Level::H4 => h4,
            Level::H5 => h5,
            Level::H6 => h6,
        }
    }
}
