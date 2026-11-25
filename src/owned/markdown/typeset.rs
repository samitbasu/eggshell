//! shadcn/typeset's parameters, and the Markdown style they generate.
//!
//! Every ratio and multiple is typeset.css's (shadcn-ui/ui,
//! `apps/v4/app/(app)/(typeset)/typeset.css`), copied, not tuned. A length
//! there in `em` is of the element's own font size, which is why the file
//! divides some margins by the element's size ratio; each length here is
//! that `em` resolved to points.

use crate::contract::Em;
use crate::{TextSpec, Tracking, Tw, Weight};
use egui::FontFamily;

use super::style::{HeadingFlow, RichLook, Style};
use super::{Level, Part};
use crate::owned::table;
use crate::sheet::{Sheet, When};
use crate::theme::Tokens;

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

/// Points in a Tailwind spacing unit, which [`Style`]'s lengths are in.
const UNIT: f32 = 4.0;

/// The attribute of a document's tables, which its table rules are under.
pub(super) const TABLE: (&str, &str) = ("markdown", "true");

/// A code block's size, in `em` of the body.
const CODE: f32 = 0.875;

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

    /// TODO(prose): the Markdown widget's look in `style`: its parts' rules
    /// in `style`'s sheet, at the layer it writes at, in place of the ones
    /// there, and the space between its blocks. Every length and weight is
    /// derived from these parameters as typeset.css derives them, in the
    /// style's colours.
    pub fn apply(&self, style: &mut crate::Style) {
        self.write_rules(&mut style.sheet);
        style.markdown = self.flow(&style.tokens);
    }

    /// One `em` of an element `ratio` times the body's size, in units.
    fn em(&self, ratio: f32) -> f32 {
        ratio * self.size / UNIT
    }

    /// The text of an element `ratio` times the body's size.
    fn text(&self, ratio: f32, leading: f32, family: &FontFamily, weight: Weight) -> TextSpec {
        TextSpec {
            size: ratio * self.size,
            line_height: leading * ratio * self.size,
            family: family.clone(),
            weight,
        }
    }

    /// `pre { font-size: 0.875em; line-height: 1.5 }`
    fn code(&self) -> TextSpec {
        self.text(CODE, 1.5, &self.mono, Weight::Normal)
    }

    pub(crate) fn write_rules(&self, sheet: &mut Sheet) {
        let body = self.text(1.0, self.leading, &self.body, Weight::Normal);
        let em = |ratio: f32| self.em(ratio);
        sheet.replace(Part::Root, |t, _| t.flex().flex_col());
        let text = body.clone();
        sheet.replace(Part::Paragraph, move |t, k| {
            t.role(text.clone()).text(k.foreground)
        });
        self.write_headings(sheet);
        // `blockquote { border-inline-start: 2px; padding-inline-start: 1em }`
        let padding = em(1.0);
        sheet.replace(Part::Blockquote, move |t, k| {
            t.flex()
                .flex_col()
                .border_l(2.0)
                .border_color(k.border)
                .pl(padding)
                .text(k.foreground)
        });
        // `pre { padding: 0.75em 1em }`, in `em` of the code.
        let (x, y) = (em(CODE), 0.75 * em(CODE));
        sheet.replace(Part::CodeBlock, move |t, k| {
            t.bg(k.muted).rounded_px(k.radius.lg()).px(x).py(y)
        });
        let code = self.code();
        sheet.replace(Part::Code, move |t, _| t.role(code.clone()));
        // `ul, ol { padding-inline-start: 1.5em }`, `li { margin-block-start:
        // 0.5em; padding-inline-start: 0.4em }`
        let gap = em(0.5);
        sheet.replace(Part::List, move |t, _| t.flex().flex_col().gap(gap));
        let gap = em(0.4);
        sheet.replace(Part::ListItem, move |t, _| {
            t.flex().flex_row().items_start().gap(gap)
        });
        let indent = em(1.5);
        sheet.replace(Part::ListMarker, move |t, k| {
            t.role(body.clone())
                .text(k.muted_foreground)
                .w(indent)
                .shrink_0()
                .text_right()
        });
        sheet.replace(Part::Unsupported, |t, k| {
            t.flex()
                .flex_col()
                .gap(1.0)
                .p(3.0)
                .border(1.0)
                .border_color(k.border)
                .rounded_px(k.radius.lg())
        });
        sheet.replace(Part::UnsupportedLabel, |t, k| t.text(k.muted_foreground));
        self.write_tables(sheet);
    }

    /// A document's tables, over the page's: a rule under the header and
    /// every row, and typeset.css's `thead th { padding: 0.65em 1em }`,
    /// `td { padding: 0.75em 1em }`, `th:first-child, td:first-child {
    /// padding-inline-start: 0 }` and `table { line-height: 1.5 }`.
    fn write_tables(&self, sheet: &mut Sheet) {
        let base = self.size;
        let ours = When::always().attr(TABLE.0, TABLE.1);
        let ruled = |t: Tw, k: &Tokens| t.border_b(1.0).border_b_color(k.border);
        sheet.replace_when(table::Part::Root, ours, |t, _| t.grid().w_full());
        sheet.replace_when(table::Part::Header, ours, ruled);
        sheet.replace_when(table::Part::Row, ours, ruled);
        let text = self.text(1.0, 1.5, &self.body, Weight::Normal);
        let cell = move |t: Tw, k: &Tokens| {
            t.flex()
                .items_center()
                .px_px(base)
                .role(text.clone())
                .text(k.foreground)
                .tabular_nums()
        };
        let body = cell.clone();
        sheet.replace_when(table::Part::Cell, ours, move |t, k| {
            body(t, k).py_px(0.75 * base)
        });
        sheet.replace_when(table::Part::Head, ours, move |t, k| {
            cell(t, k).font_weight(Weight::Medium).py_px(0.65 * base)
        });
        for part in [table::Part::Head, table::Part::Cell] {
            let first = table::Part::first_column().attr(TABLE.0, TABLE.1);
            sheet.rule_when(part, first, |t, _| t.pl_px(0.0));
        }
    }

    /// Each heading's row of typeset.css, under its level's key.
    fn write_headings(&self, sheet: &mut Sheet) {
        let family = self.heading.as_ref().unwrap_or(&self.body);
        sheet.replace(Part::Heading, |t, _| t);
        for (level, h) in Level::ALL.into_iter().zip(HEADINGS) {
            let text = self.text(h.size, h.leading, family, h.weight);
            let minor = h.minor;
            let tracking = match minor {
                Minor::Small => Tracking::points(0.08 * h.size * self.size),
                Minor::No | Minor::Muted => Tracking::points(0.0),
            };
            sheet.rule_when(
                Part::Heading,
                When::always().attr("level", level.attr()),
                move |t, k| {
                    let t = t.role(text.clone()).tracking(tracking);
                    match minor {
                        Minor::No => t.text(k.foreground),
                        Minor::Muted => t.text(k.muted_foreground),
                        Minor::Small => t.text(k.muted_foreground).uppercase(),
                    }
                },
            );
        }
    }

    /// The space between a document's blocks, and its tables and rich text
    /// in `t`'s colours.
    pub(crate) fn flow(&self, t: &Tokens) -> Style {
        let base = self.size;
        let em = |ratio: f32| self.em(ratio);
        let flow = |ratio: f32| self.flow * em(ratio);
        let headings = HEADINGS.map(|h| HeadingFlow {
            margin_top: self.flow * h.flows * em(h.size),
            // `h1 + *, … { margin-block-start: 1em }`
            after_heading: em(h.size),
            // `hr + h1, … hr + h4 { margin-block-start: var(--typeset-flow) }`
            after_rule: match h.minor {
                Minor::No => flow(h.size),
                Minor::Muted | Minor::Small => self.flow * h.flows * em(h.size),
            },
        });
        Style {
            block_gap: flow(1.0),
            after_heading: em(1.0),
            // `hr { margin-block-start: calc(var(--typeset-flow) * 2.4) }`
            rule_gap: 2.4 * flow(1.0),
            headings,
            item_gap: em(0.5),
            // `a { color: inherit; font-weight: 500; text-decoration-color:
            // color-mix(currentColor 30%) }`; `:not(pre) > code { font-size:
            // 0.85em; border-radius: min(var(--radius) * 0.6, 0.35em);
            // padding: 0.125em 0.3em }`, in `em` of the code.
            rich: Some(RichLook {
                link: t.foreground,
                link_underline: t.foreground.gamma_multiply(0.3),
                link_weight: Weight::Medium,
                code_fill: t.muted,
                code_radius: (t.radius.lg() * 0.6).min(0.35 * INLINE_CODE * base).round() as u8,
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
    use crate::sheet::Key;
    use crate::{TextCase, TextStyle};
    use egui::Color32;

    /// The text `typeset` gives `part` under `keys`, in the light tokens.
    fn text_of(typeset: &Typeset, part: Part, keys: &[Key]) -> TextStyle {
        let mut sheet = Sheet::new();
        typeset.write_rules(&mut sheet);
        let page = TextStyle::new(
            Typeset::default().text(1.0, 1.0, &FontFamily::Proportional, Weight::Normal),
            Color32::PLACEHOLDER,
        );
        sheet
            .resolve(part, keys, &Tokens::light())
            .text_classes()
            .over(&page)
    }

    fn heading(typeset: &Typeset, level: Level) -> TextStyle {
        text_of(typeset, Part::Heading, &[Key::Attr("level", level.attr())])
    }

    #[test]
    fn heading_sizes_and_line_heights_are_typesets_ratios_of_the_base() {
        let sizes: Vec<_> = Level::ALL
            .into_iter()
            .map(|level| {
                let text = heading(&Typeset::default(), level).spec;
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
        let typeset = Typeset::default();
        let tokens = Tokens::light();
        let (h4, h5, h6) = (
            heading(&typeset, Level::H4),
            heading(&typeset, Level::H5),
            heading(&typeset, Level::H6),
        );
        assert_eq!(h4.spec.weight, Weight::SemiBold);
        assert_eq!(h5.spec.weight, Weight::Medium);
        assert_eq!(
            (h4.color, h5.color),
            (tokens.foreground, tokens.muted_foreground)
        );
        assert_eq!((h5.case, h6.case), (TextCase::Normal, TextCase::Upper));
        assert_eq!(h6.tracking, Tracking::points(0.08 * 12.1875));
    }

    #[test]
    fn a_minor_heading_has_one_flow_of_the_body_above_it() {
        let style = Typeset::default().flow(&Tokens::light());
        for level in [Level::H5, Level::H6] {
            assert!((style.heading(level).margin_top - style.block_gap).abs() < 1e-5);
        }
    }

    #[test]
    fn code_is_0_875_em_on_the_themes_muted_fill_and_radius() {
        let typeset = Typeset::default();
        let code = text_of(&typeset, Part::Code, &[]).spec;
        assert_eq!(code.size, 13.125);
        assert_eq!(code.line_height, 13.125 * 1.5);
        assert_eq!(code.family, FontFamily::Monospace);
        let mut sheet = Sheet::new();
        typeset.write_rules(&mut sheet);
        let tokens = Tokens::light();
        let block = sheet.resolve(Part::CodeBlock, &[], &tokens);
        let expected = crate::tw()
            .bg(tokens.muted)
            .rounded_px(tokens.radius.lg())
            .px(13.125 / UNIT)
            .py(0.75 * 13.125 / UNIT);
        assert_eq!(block.style(), expected.style());
        assert_eq!(block.paint_classes(), expected.paint_classes());
    }

    #[test]
    fn a_heading_family_of_none_is_the_bodys() {
        let typeset = Typeset {
            body: FontFamily::Name("Serif".into()),
            ..Typeset::default()
        };
        assert_eq!(heading(&typeset, Level::H1).spec.family, typeset.body);
        let typeset = Typeset {
            heading: Some(FontFamily::Monospace),
            ..typeset
        };
        assert_eq!(
            heading(&typeset, Level::H1).spec.family,
            FontFamily::Monospace
        );
        let paragraph = text_of(&typeset, Part::Paragraph, &[]);
        assert_eq!(paragraph.spec.family, typeset.body);
    }
}
