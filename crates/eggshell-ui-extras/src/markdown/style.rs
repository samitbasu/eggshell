//! How the Markdown widget looks.

use std::sync::Arc;

use eggshell_ui::{Style, TextSpec, Tracking, Weight};
use egui::{Color32, Context, FontFamily, Id, Theme};

use super::parse::Level;

/// TODO(prose)
///
/// Lengths are in Tailwind spacing units (a unit is 4 points), as [`Tw`](eggshell_ui::Tw)'s
/// spacing classes take them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MarkdownStyle {
    /// Space above every block but the first, outside lists.
    pub block_gap: f32,
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
    /// TODO(prose)
    pub rule: HeadingRule,
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
    /// TODO(prose)
    pub padding: f32,
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
        let border = style.input.look.paint.states.rest.border;
        let muted = style.input.hint.unwrap_or(style.label.disabled.text);
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
            rule,
        };
        Self {
            block_gap: 6.0,
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
                text: style.label.rest.text,
                font_style: eggshell_ui::FontStyle::Italic,
            },
            code: CodeBlockStyle {
                fill: style.rich.code.fill,
                border,
                border_width: 0.0,
                radius: 8.0,
                padding: 4.0,
                text: mono(0.875),
            },
            rule: border,
            unsupported: UnsupportedStyle {
                fill: Color32::TRANSPARENT,
                border,
                label: muted,
                text: mono(0.875),
            },
        }
    }

    /// The style of `level`'s headings.
    pub fn heading(&self, level: Level) -> &HeadingStyle {
        &self.headings[level as usize]
    }

    /// TODO(prose)
    pub fn of(ctx: &Context) -> Arc<Self> {
        ctx.data(|data| data.get_temp::<Arc<Self>>(id(ctx.theme())))
            .unwrap_or_else(|| Arc::new(Self::from_style(&Style::of(ctx))))
    }

    /// TODO(prose)
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        ctx.data_mut(|data| data.insert_temp(id(theme), Arc::new(self)));
    }
}

fn id(theme: Theme) -> Id {
    Id::NULL.with(("eggshell-ui-extras-markdown-style", theme))
}
