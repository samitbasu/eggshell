//! How the Markdown widget looks.

use crate::Weight;
use crate::contract::Em;
use egui::Color32;

use super::parse::Level;

/// TODO(prose): the space between a document's blocks. The blocks' look is
/// the sheet's, on [`Part`](super::Part).
///
/// Lengths are in Tailwind spacing units (a unit is 4 points), as [`Tw`](crate::Tw)'s
/// spacing classes take them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// Space above every block but the first, outside lists.
    pub block_gap: f32,
    /// TODO(prose): space above a paragraph that follows a heading, in place
    /// of `block_gap`.
    pub after_heading: f32,
    /// TODO(prose): space above a thematic break (`---`), in place of
    /// `block_gap`.
    pub rule_gap: f32,
    /// TODO(prose): space above each level's headings, `#` first.
    pub headings: [HeadingFlow; 6],
    /// TODO(prose): space between a list item's blocks.
    pub item_gap: f32,
    /// TODO(prose): the document's links and inline code, over the page's
    /// [`RichTextStyle`](crate::contract::RichTextStyle); `None` keeps
    /// the page's.
    pub rich: Option<RichLook>,
    /// TODO(prose): the space between blocks on a narrow screen, and the
    /// width below which it applies.
    pub narrow: Option<Box<NarrowStyle>>,
}

/// TODO(prose): a document's spacing and rich text below a viewport width.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NarrowStyle {
    /// TODO(prose): the viewport width, in points.
    pub below: f32,
    /// TODO(prose)
    pub style: Style,
}

/// TODO(prose): a document's links and inline code. Lengths are in points.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RichLook {
    /// TODO(prose)
    pub link: Color32,
    /// TODO(prose)
    pub link_underline: Color32,
    /// TODO(prose)
    pub link_weight: Weight,
    /// TODO(prose): behind inline code.
    pub code_fill: Color32,
    /// TODO(prose)
    pub code_radius: u8,
    /// TODO(prose): inline code's size, relative to the text around it.
    pub code_size: Em,
    /// TODO(prose)
    pub code_padding_x: f32,
    /// TODO(prose)
    pub code_padding_y: f32,
}

impl Style {
    /// The page's style with this document's rich text over it.
    pub(super) fn restyle(&self, style: &mut crate::Style) {
        if let Some(look) = &self.rich {
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

    /// Whether the document changes the page's style at all.
    pub(super) fn restyles(&self) -> bool {
        self.rich.is_some()
    }
}

/// TODO(prose): the space above a heading.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HeadingFlow {
    /// Space above the heading, in place of `block_gap`.
    pub margin_top: f32,
    /// TODO(prose): space above the heading when it follows another, in
    /// place of `margin_top`.
    pub after_heading: f32,
    /// TODO(prose): space above the heading when it follows a thematic
    /// break, in place of `margin_top`.
    pub after_rule: f32,
}

impl Style {
    /// The space above `level`'s headings.
    pub fn heading(&self, level: Level) -> &HeadingFlow {
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
