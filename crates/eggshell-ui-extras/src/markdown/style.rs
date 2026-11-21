//! How the Markdown widget looks.

use eggshell_ui::contract::Em;
use eggshell_ui::{Style, TextSpec, Tracking, Weight};
use egui::{Color32, FontFamily};

use super::parse::Level;

/// TODO(prose)
///
/// Lengths are in Tailwind spacing units (a unit is 4 points), as [`Tw`](eggshell_ui::Tw)'s
/// spacing classes take them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MarkdownStyle {
    /// TODO(prose): the text of paragraphs, list items and table cells;
    /// `None` keeps the text of the page around the document.
    pub body: Option<TextSpec>,
    /// TODO(prose): the colour of that text; `None` keeps the page's.
    pub color: Option<Color32>,
    /// Space above every block but the first, outside lists.
    pub block_gap: f32,
    /// TODO(prose): space above a paragraph that follows a heading, in place
    /// of `block_gap`.
    pub after_heading: f32,
    /// TODO(prose): space above a thematic break (`---`), in place of
    /// `block_gap`.
    pub rule_gap: f32,
    /// Headings, `#` first.
    pub headings: [HeadingStyle; 6],
    /// Lists.
    pub list: ListStyle,
    /// Block quotes.
    pub quote: QuoteStyle,
    /// Code blocks.
    pub code: CodeBlockStyle,
    /// The line of a thematic break (`---`), and of a heading's rule.
    pub rule: Color32,
    /// What a construct the widget does not show looks like.
    pub unsupported: UnsupportedStyle,
    /// TODO(prose): the document's tables, over the page's
    /// [`TableStyle`](eggshell_ui::contract::TableStyle); `None` keeps the
    /// page's.
    pub table: Option<TableLook>,
    /// TODO(prose): the document's links and inline code, over the page's
    /// [`RichTextStyle`](eggshell_ui::contract::RichTextStyle); `None` keeps
    /// the page's.
    pub rich: Option<RichLook>,
}

/// TODO(prose): a document's tables. Lengths are in points.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableLook {
    /// TODO(prose): beside a cell's content.
    pub padding_x: f32,
    /// TODO(prose): above and below a body cell's content.
    pub padding_y: f32,
    /// TODO(prose): above and below a header cell's content.
    pub header_padding_y: f32,
    /// TODO(prose): before the first column's content.
    pub first_column_inset: f32,
    /// TODO(prose)
    pub header_weight: Weight,
    /// TODO(prose): the lines between rows and under the table.
    pub rule: Color32,
    /// TODO(prose): the text of the cells.
    pub text: TextSpec,
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

impl eggshell_ui::contract::Extension for MarkdownStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl MarkdownStyle {
    /// The page's style with this document's tables and rich text over it.
    pub(super) fn restyle(&self, style: &mut Style) {
        if let Some(look) = &self.table {
            let mut table = style.table().clone();
            table.cell_padding = look.padding_x;
            table.cell_padding_y = Some(look.padding_y);
            table.header_padding_y = Some(look.header_padding_y);
            table.first_column_inset = Some(look.first_column_inset);
            table.header_weight = look.header_weight;
            table.rule = look.rule;
            table.rule_width = 1.0;
            table.rule_below_last = true;
            for fill in [
                &mut table.header_fill,
                &mut table.hover,
                &mut table.selected,
                &mut table.footer_fill,
            ] {
                *fill = Color32::TRANSPARENT;
            }
            style.set_table(table);
        }
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
        self.table.is_some() || self.rich.is_some()
    }
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HeadingStyle {
    /// TODO(prose)
    pub text: TextSpec,
    /// TODO(prose)
    pub tracking: Tracking,
    /// Space above the heading, in place of `block_gap`.
    pub margin_top: f32,
    /// TODO(prose): space above the heading when it follows another, in
    /// place of `margin_top`.
    pub after_heading: f32,
    /// TODO(prose): space above the heading when it follows a thematic
    /// break, in place of `margin_top`.
    pub after_rule: f32,
    /// TODO(prose): `None` keeps the colour of the document's text.
    pub color: Option<Color32>,
    /// TODO(prose)
    pub case: Case,
    /// TODO(prose)
    pub rule: HeadingRule,
}

/// TODO(prose): the case a heading's text is shown in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Case {
    /// TODO(prose)
    #[default]
    AsWritten,
    /// TODO(prose)
    Upper,
}

/// TODO(prose)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeadingRule {
    /// TODO(prose)
    None,
    /// A line under the heading, `padding` units below its text.
    Below {
        /// TODO(prose)
        padding: f32,
    },
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListStyle {
    /// The width of the marker column.
    pub indent: f32,
    /// Space between the marker and the item.
    pub marker_gap: f32,
    /// Space above every item but the first, and between an item's blocks.
    pub item_gap: f32,
    /// TODO(prose)
    pub marker: Color32,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct QuoteStyle {
    /// TODO(prose)
    pub border: Color32,
    /// In points.
    pub border_width: f32,
    /// Space between the border and the text.
    pub padding: f32,
    /// TODO(prose)
    pub text: Color32,
    /// TODO(prose)
    pub font_style: eggshell_ui::FontStyle,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CodeBlockStyle {
    /// TODO(prose)
    pub fill: Color32,
    /// TODO(prose)
    pub border: Color32,
    /// In points; zero for none.
    pub border_width: f32,
    /// In points.
    pub radius: f32,
    /// TODO(prose): space beside the code.
    pub padding_x: f32,
    /// TODO(prose): space above and below the code.
    pub padding_y: f32,
    /// TODO(prose)
    pub text: TextSpec,
}

/// TODO(prose)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct UnsupportedStyle {
    /// TODO(prose)
    pub fill: Color32,
    /// TODO(prose)
    pub border: Color32,
    /// The label naming the construct.
    pub label: Color32,
    /// The construct's source.
    pub text: TextSpec,
}

impl MarkdownStyle {
    /// TODO(prose)
    pub fn from_style(style: &Style) -> Self {
        let border = style.input().look.paint.states.rest.border;
        let muted = style.input().hint.unwrap_or(style.label().disabled.text);
        let body = &style.text;
        let mono = |scale: f32| TextSpec {
            size: body.size * scale,
            line_height: body.line_height * scale,
            family: FontFamily::Monospace,
            weight: Weight::Normal,
        };
        let heading = |size: f32, line_height: f32, weight, margin_top, rule| HeadingStyle {
            text: TextSpec {
                size,
                line_height,
                family: body.family.clone(),
                weight,
            },
            tracking: Tracking::points(-0.025 * size),
            margin_top,
            after_heading: margin_top,
            after_rule: margin_top,
            color: None,
            case: Case::AsWritten,
            rule,
        };
        let block_gap = 6.0;
        Self {
            body: None,
            color: None,
            block_gap,
            after_heading: block_gap,
            rule_gap: block_gap,
            headings: [
                heading(36.0, 40.0, Weight::Bold, 6.0, HeadingRule::None),
                heading(
                    30.0,
                    36.0,
                    Weight::SemiBold,
                    10.0,
                    HeadingRule::Below { padding: 2.0 },
                ),
                heading(24.0, 32.0, Weight::SemiBold, 8.0, HeadingRule::None),
                heading(20.0, 28.0, Weight::SemiBold, 6.0, HeadingRule::None),
                heading(18.0, 28.0, Weight::SemiBold, 6.0, HeadingRule::None),
                heading(16.0, 24.0, Weight::SemiBold, 6.0, HeadingRule::None),
            ],
            list: ListStyle {
                indent: 6.0,
                marker_gap: 2.0,
                item_gap: 2.0,
                marker: muted,
            },
            quote: QuoteStyle {
                border,
                border_width: 2.0,
                padding: 6.0,
                text: style.label().rest.text,
                font_style: eggshell_ui::FontStyle::Italic,
            },
            code: CodeBlockStyle {
                fill: style.rich.code.fill,
                border,
                border_width: 0.0,
                radius: 8.0,
                padding_x: 4.0,
                padding_y: 4.0,
                text: mono(0.875),
            },
            rule: border,
            unsupported: UnsupportedStyle {
                fill: Color32::TRANSPARENT,
                border,
                label: muted,
                text: mono(0.875),
            },
            table: None,
            rich: None,
        }
    }

    /// The style of `level`'s headings.
    pub fn heading(&self, level: Level) -> &HeadingStyle {
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
