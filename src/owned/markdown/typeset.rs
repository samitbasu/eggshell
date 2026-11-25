//! shadcn/typeset's parameters, and the Markdown style they generate.
//!
//! Every ratio and multiple is typeset.css's (shadcn-ui/ui,
//! `apps/v4/app/(app)/(typeset)/typeset.css`), copied, not tuned. A length
//! there in `em` is of the element's own font size, which is why the file
//! divides some margins by the element's size ratio; each length here is
//! that `em` resolved to points.

use crate::contract::Em;
use crate::{TextSpec, Tracking, Weight};
use egui::{Color32, FontFamily};

use super::style::{
    Case, CodeBlockStyle, HeadingRule, HeadingStyle, ListStyle, QuoteStyle, RichLook, Style,
    TableLook, UnsupportedStyle,
};

/// TODO(prose): shadcn/typeset's parameters, from which the rest of a
/// document's look is derived. The defaults are shadcn's.
///
/// ```
/// use eggshell_ui::owned::markdown::{Measure, Typeset};
///
/// let reading = Typeset {
///     size: 18.0,
///     leading: 1.9,
///     flow: 2.0,
///     measure: Measure::Ch70,
///     ..Typeset::default()
/// };
/// assert_eq!(reading.measure_width(), 33.0 * 18.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Typeset {
    /// TODO(prose): the family of body text.
    pub body: FontFamily,
    /// TODO(prose): the family of headings; `None` is the body's.
    pub heading: Option<FontFamily>,
    /// TODO(prose): the family of code.
    pub mono: FontFamily,
    /// TODO(prose): the size of body text, in points.
    pub size: f32,
    /// TODO(prose): the line height of body text, as a multiple of its size.
    pub leading: f32,
    /// TODO(prose): the space between blocks, in `em` of the body size.
    pub flow: f32,
    /// TODO(prose): the line length.
    pub measure: Measure,
}

impl Default for Typeset {
    fn default() -> Self {
        Self {
            body: FontFamily::Proportional,
            heading: None,
            mono: FontFamily::Monospace,
            size: 15.0,
            leading: 1.75,
            flow: 1.25,
            measure: Measure::Ch80,
        }
    }
}

/// TODO(prose): shadcn's line lengths. Each is a width in `em` of the body
/// size, as shadcn's builder sets it, not a count of characters measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Measure {
    /// TODO(prose)
    Ch60,
    /// TODO(prose)
    Ch70,
    /// TODO(prose)
    #[default]
    Ch80,
    /// TODO(prose)
    Ch90,
}

impl Measure {
    /// The width, in `em` of the body size.
    pub fn ems(self) -> f32 {
        match self {
            Self::Ch60 => 28.0,
            Self::Ch70 => 33.0,
            Self::Ch80 => 37.0,
            Self::Ch90 => 42.0,
        }
    }
}

/// TODO(prose): the theme colours a typeset takes, as typeset.css takes
/// them from the theme's variables.
#[derive(Clone, Debug, PartialEq)]
pub struct TypesetInk {
    /// `--color-foreground`: text and headings.
    pub foreground: Color32,
    /// `--color-muted-foreground`: list markers and the minor headings.
    pub muted: Color32,
    /// `--color-border`: rules and the quote's border.
    pub rule: Color32,
    /// `--color-muted`: behind code.
    pub code_fill: Color32,
    /// `--radius`, in points: a code block's corners.
    pub radius: f32,
}

/// Points in a Tailwind spacing unit, which [`Style`]'s lengths are in.
const UNIT: f32 = 4.0;

/// Inline code's size, in `em` of the text around it.
const INLINE_CODE: f32 = 0.85;

/// One heading's row of typeset.css.
struct Heading {
    /// `font-size`, in `em` of the body.
    size: f32,
    /// `line-height`, as a multiple of the heading's size.
    leading: f32,
    weight: Weight,
    /// `margin-block-start`, in flows of the heading's own `em`.
    flows: f32,
    minor: Minor,
}

/// Whether a heading is one of the two typeset.css mutes.
#[derive(Clone, Copy, PartialEq)]
enum Minor {
    No,
    Muted,
    /// h6's: also uppercase and tracked out.
    Small,
}

/// h1 to h6.
const HEADINGS: [Heading; 6] = [
    Heading {
        size: 1.75,
        leading: 1.3,
        weight: Weight::SemiBold,
        flows: 1.0,
        minor: Minor::No,
    },
    Heading {
        size: 1.25,
        leading: 1.4,
        weight: Weight::SemiBold,
        flows: 1.4,
        minor: Minor::No,
    },
    Heading {
        size: 1.125,
        leading: 1.45,
        weight: Weight::SemiBold,
        flows: 1.0,
        minor: Minor::No,
    },
    Heading {
        size: 1.0,
        leading: 1.5,
        weight: Weight::SemiBold,
        flows: 1.0,
        minor: Minor::No,
    },
    // `calc(var(--typeset-flow) / 0.875)`: one flow of the body.
    Heading {
        size: 0.875,
        leading: 1.5,
        weight: Weight::Medium,
        flows: 1.0 / 0.875,
        minor: Minor::Muted,
    },
    Heading {
        size: 0.8125,
        leading: 1.5,
        weight: Weight::Medium,
        flows: 1.0 / 0.8125,
        minor: Minor::Small,
    },
];

impl Typeset {
    /// TODO(prose): the width of the line, in points, for the layout around
    /// the document to cap it at: typeset leaves the width to the layout.
    pub fn measure_width(&self) -> f32 {
        self.measure.ems() * self.size
    }

    /// TODO(prose): the Markdown widget's style, every length and weight
    /// derived from these parameters as typeset.css derives them, in
    /// `ink`'s colours.
    pub fn markdown_style(&self, ink: &TypesetInk) -> Style {
        let base = self.size;
        // One `em` of an element `ratio` times the body's size, in units.
        let em = |ratio: f32| ratio * base / UNIT;
        let flow = |ratio: f32| self.flow * em(ratio);
        let text = |ratio: f32, leading: f32, family: &FontFamily, weight| TextSpec {
            size: ratio * base,
            line_height: leading * ratio * base,
            family: family.clone(),
            weight,
        };
        let heading_family = self.heading.as_ref().unwrap_or(&self.body);
        let headings = HEADINGS.map(|h| HeadingStyle {
            text: text(h.size, h.leading, heading_family, h.weight),
            tracking: match h.minor {
                Minor::Small => Tracking::points(0.08 * h.size * base),
                Minor::No | Minor::Muted => Tracking::points(0.0),
            },
            margin_top: self.flow * h.flows * em(h.size),
            // `h1 + *, … { margin-block-start: 1em }`
            after_heading: em(h.size),
            // `hr + h1, … hr + h4 { margin-block-start: var(--typeset-flow) }`
            after_rule: match h.minor {
                Minor::No => flow(h.size),
                Minor::Muted | Minor::Small => self.flow * h.flows * em(h.size),
            },
            color: match h.minor {
                Minor::No => Some(ink.foreground),
                Minor::Muted | Minor::Small => Some(ink.muted),
            },
            case: match h.minor {
                Minor::Small => Case::Upper,
                Minor::No | Minor::Muted => Case::AsWritten,
            },
            rule: HeadingRule::None,
        });
        // `pre { font-size: 0.875em; line-height: 1.5; padding: 0.75em 1em }`
        let code = 0.875;
        Style {
            body: Some(text(1.0, self.leading, &self.body, Weight::Normal)),
            color: Some(ink.foreground),
            block_gap: flow(1.0),
            after_heading: em(1.0),
            // `hr { margin-block-start: calc(var(--typeset-flow) * 2.4) }`
            rule_gap: 2.4 * flow(1.0),
            headings,
            // `ul, ol { padding-inline-start: 1.5em }`, `li { margin-block-start:
            // 0.5em; padding-inline-start: 0.4em }`
            list: ListStyle {
                indent: em(1.5),
                marker_gap: em(0.4),
                item_gap: em(0.5),
                marker: ink.muted,
            },
            // `blockquote { border-inline-start: 2px; padding-inline-start: 1em }`
            quote: QuoteStyle {
                border: ink.rule,
                border_width: 2.0,
                padding: em(1.0),
                text: ink.foreground,
                font_style: crate::FontStyle::Normal,
            },
            code: CodeBlockStyle {
                fill: ink.code_fill,
                border: ink.rule,
                border_width: 0.0,
                radius: ink.radius,
                padding_x: em(code),
                padding_y: 0.75 * em(code),
                text: text(code, 1.5, &self.mono, Weight::Normal),
            },
            rule: ink.rule,
            unsupported: UnsupportedStyle {
                fill: Color32::TRANSPARENT,
                border: ink.rule,
                label: ink.muted,
                text: text(code, 1.5, &self.mono, Weight::Normal),
            },
            // `thead th { padding: 0.65em 1em }`, `td { padding: 0.75em 1em }`,
            // `th:first-child, td:first-child { padding-inline-start: 0 }`,
            // `table { line-height: 1.5 }`.
            table: Some(TableLook {
                padding_x: base,
                padding_y: 0.75 * base,
                header_padding_y: 0.65 * base,
                first_column_inset: 0.0,
                header_weight: Weight::Medium,
                rule: ink.rule,
                text: text(1.0, 1.5, &self.body, Weight::Normal),
            }),
            // `a { color: inherit; font-weight: 500; text-decoration-color:
            // color-mix(currentColor 30%) }`; `:not(pre) > code { font-size:
            // 0.85em; border-radius: min(var(--radius) * 0.6, 0.35em);
            // padding: 0.125em 0.3em }`, in `em` of the code.
            rich: Some(RichLook {
                link: ink.foreground,
                link_underline: ink.foreground.gamma_multiply(0.3),
                link_weight: Weight::Medium,
                code_fill: ink.code_fill,
                code_radius: (ink.radius * 0.6).min(0.35 * INLINE_CODE * base).round() as u8,
                code_size: Em::new(INLINE_CODE),
                code_padding_x: 0.3 * INLINE_CODE * base,
                code_padding_y: 0.125 * INLINE_CODE * base,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owned::markdown::Level;

    fn ink() -> TypesetInk {
        TypesetInk {
            foreground: Color32::BLACK,
            muted: Color32::GRAY,
            rule: Color32::LIGHT_GRAY,
            code_fill: Color32::WHITE,
            radius: 10.0,
        }
    }

    #[test]
    fn heading_sizes_and_line_heights_are_typesets_ratios_of_the_base() {
        let style = Typeset::default().markdown_style(&ink());
        let levels = [
            Level::H1,
            Level::H2,
            Level::H3,
            Level::H4,
            Level::H5,
            Level::H6,
        ];
        let sizes: Vec<_> = levels
            .into_iter()
            .map(|level| {
                let text = &style.heading(level).text;
                (text.size, text.line_height)
            })
            .collect();
        let expected = [
            (26.25, 26.25 * 1.3),
            (18.75, 18.75 * 1.4),
            (16.875, 16.875 * 1.45),
            (15.0, 22.5),
            (13.125, 13.125 * 1.5),
            (12.1875, 12.1875 * 1.5),
        ];
        for (actual, expected) in sizes.into_iter().zip(expected) {
            assert!(
                (actual.0 - expected.0).abs() < 1e-4 && (actual.1 - expected.1).abs() < 1e-4,
                "{actual:?} is not {expected:?}"
            );
        }
    }

    #[test]
    fn a_minor_heading_is_medium_and_muted_and_h6_is_uppercase_and_tracked() {
        let style = Typeset::default().markdown_style(&ink());
        let (h4, h5, h6) = (
            style.heading(Level::H4),
            style.heading(Level::H5),
            style.heading(Level::H6),
        );
        assert_eq!(h4.text.weight, Weight::SemiBold);
        assert_eq!(h5.text.weight, Weight::Medium);
        assert_eq!(
            (h4.color, h5.color),
            (Some(Color32::BLACK), Some(Color32::GRAY))
        );
        assert_eq!((h5.case, h6.case), (Case::AsWritten, Case::Upper));
        assert_eq!(h6.tracking, Tracking::points(0.08 * 12.1875));
    }

    #[test]
    fn a_minor_heading_has_one_flow_of_the_body_above_it() {
        let style = Typeset::default().markdown_style(&ink());
        for level in [Level::H5, Level::H6] {
            assert!((style.heading(level).margin_top - style.block_gap).abs() < 1e-5);
        }
    }

    #[test]
    fn code_is_0_875_em_on_the_themes_muted_fill_and_radius() {
        let style = Typeset::default().markdown_style(&ink());
        assert_eq!(style.code.text.size, 13.125);
        assert_eq!(style.code.text.line_height, 13.125 * 1.5);
        assert_eq!(style.code.text.family, FontFamily::Monospace);
        assert_eq!(style.code.padding_x * UNIT, 13.125);
        assert_eq!(style.code.padding_y * UNIT, 0.75 * 13.125);
        assert_eq!((style.code.fill, style.code.radius), (Color32::WHITE, 10.0));
    }

    #[test]
    fn a_heading_family_of_none_is_the_bodys() {
        let typeset = Typeset {
            body: FontFamily::Name("Serif".into()),
            ..Typeset::default()
        };
        let style = typeset.markdown_style(&ink());
        assert_eq!(style.heading(Level::H1).text.family, typeset.body);
        let typeset = Typeset {
            heading: Some(FontFamily::Monospace),
            ..typeset
        };
        let style = typeset.markdown_style(&ink());
        assert_eq!(style.heading(Level::H1).text.family, FontFamily::Monospace);
        assert_eq!(style.body.map(|body| body.family), Some(typeset.body));
    }
}
