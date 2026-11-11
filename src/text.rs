//! Text styling, and how it is inherited down the tree.

use std::borrow::Cow;

use egui::{Color32, FontFamily, FontId};

/// A text role: size, line height and font family, all resolved by the
/// application. Font weight is expressed by the family, because egui does not
/// synthesize weights.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    /// Font size in points.
    pub size: f32,
    /// Height of one line in points.
    pub line_height: f32,
    /// The registered font family.
    pub family: FontFamily,
}

impl TextSpec {
    /// The egui font for this role.
    pub fn font_id(&self) -> FontId {
        FontId::new(self.size, self.family.clone())
    }
}

/// Where a text node places its lines when its box is taller than they are.
///
/// A node's text behaves as the single item of the node's own flex (or grid)
/// container, so the node's own classes decide: `items-center` on a row,
/// `justify-center` on a column, and their `end` forms. Anything else, such
/// as a block, puts the text at the top. A parent's alignment never matters
/// here: it places the node's box, which is already as tall as the text
/// unless a height class says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

impl VAlign {
    pub(crate) fn of(style: &taffy::Style) -> Self {
        use taffy::style::{AlignContent, AlignItems, Display, FlexDirection};
        let from_items = |items: AlignItems| match items {
            AlignItems::CENTER => Self::Middle,
            AlignItems::END | AlignItems::FLEX_END => Self::Bottom,
            _ => Self::Top,
        };
        let from_content = |content: AlignContent| match content {
            AlignContent::CENTER => Self::Middle,
            AlignContent::END | AlignContent::FLEX_END => Self::Bottom,
            _ => Self::Top,
        };
        let flipped = |align| match align {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Middle => Self::Middle,
        };
        match (style.display, style.flex_direction) {
            (Display::Grid, _) => from_items(style.align_items),
            (Display::Flex, FlexDirection::Row | FlexDirection::RowReverse) => {
                from_items(style.align_items)
            }
            (Display::Flex, FlexDirection::Column) => from_content(style.justify_content),
            (Display::Flex, FlexDirection::ColumnReverse) => {
                flipped(from_content(style.justify_content))
            }
            _ => Self::Top,
        }
    }

    /// How far down a box `spare` points taller than the text the text starts.
    pub(crate) fn offset(self, spare: f32) -> f32 {
        let spare = spare.max(0.0);
        match self {
            Self::Top => 0.0,
            Self::Middle => spare / 2.0,
            Self::Bottom => spare,
        }
    }
}

/// How text wraps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhiteSpace {
    /// Wrap at word boundaries.
    #[default]
    Normal,
    /// Never wrap.
    NoWrap,
    /// Never wrap, and end with an ellipsis where the text overflows.
    Truncate,
}

/// Horizontal alignment of text within its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Aligned to the left edge.
    #[default]
    Left,
    /// Centred.
    Center,
    /// Aligned to the right edge.
    Right,
}

/// Upright or slanted glyphs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontStyle {
    /// Upright.
    #[default]
    Normal,
    /// Slanted.
    Italic,
}

/// A line drawn with the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDecoration {
    /// No line.
    #[default]
    None,
    /// A line under the text.
    Underline,
}

/// Capitalisation applied to text before it is measured and drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextCase {
    /// Text as given.
    #[default]
    Normal,
    /// Every letter upper case.
    Upper,
    /// The first letter of every word upper case.
    Capitalize,
}

impl TextCase {
    /// `text` with this capitalisation applied.
    pub fn apply<'t>(self, text: &'t str) -> Cow<'t, str> {
        match self {
            Self::Normal => Cow::Borrowed(text),
            Self::Upper => Cow::Owned(text.to_uppercase()),
            Self::Capitalize => {
                let mut at_word_start = true;
                Cow::Owned(
                    text.chars()
                        .flat_map(|c| {
                            let mapped: Vec<char> = if at_word_start {
                                c.to_uppercase().collect()
                            } else {
                                vec![c]
                            };
                            at_word_start = c.is_whitespace();
                            mapped
                        })
                        .collect(),
                )
            }
        }
    }
}

/// Whether the pointer selects a node's text, like CSS `user-select`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UserSelect {
    /// Selectable unless the node, or an ancestor, senses the pointer.
    #[default]
    Auto,
    /// Always selectable, even inside a node that senses the pointer.
    Text,
    /// Never selectable.
    None,
}

/// Whether a node is, or is inside, one that senses the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interactive {
    Within,
    Outside,
}

impl UserSelect {
    pub(crate) fn selects(self, interactive: Interactive) -> bool {
        match self {
            Self::Auto => interactive == Interactive::Outside,
            Self::Text => true,
            Self::None => false,
        }
    }
}

/// How wide digits are, like CSS `font-variant-numeric`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Numerals {
    /// Each digit has the width the font gives it.
    #[default]
    Proportional,
    /// Every digit takes the width of the font's widest digit.
    Tabular,
}

/// Extra space between letters, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tracking(f32);

impl Tracking {
    /// No extra space.
    pub const ZERO: Self = Self(0.0);

    /// Extra space of `points` between letters; non-finite input is no extra space.
    pub fn points(points: f32) -> Self {
        if points.is_finite() {
            Self(points)
        } else {
            Self::ZERO
        }
    }
}

impl From<Tracking> for f32 {
    fn from(tracking: Tracking) -> f32 {
        tracking.0
    }
}

/// Fully resolved text style: what a node's text is measured and drawn with.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// Size, line height and family.
    pub spec: TextSpec,
    /// Text colour.
    pub color: Color32,
    /// Upright or italic.
    pub font_style: FontStyle,
    /// Underline.
    pub decoration: TextDecoration,
    /// Wrapping behaviour.
    pub white_space: WhiteSpace,
    /// Horizontal alignment.
    pub align: TextAlign,
    /// Extra letter spacing.
    pub tracking: Tracking,
    /// Whether the pointer selects the text.
    pub select: UserSelect,
    /// Capitalisation.
    pub case: TextCase,
    /// Digit widths.
    pub numerals: Numerals,
}

impl TextStyle {
    /// A style with the given role and colour and no other classes.
    pub fn new(spec: TextSpec, color: Color32) -> Self {
        Self {
            spec,
            color,
            font_style: FontStyle::default(),
            decoration: TextDecoration::default(),
            white_space: WhiteSpace::default(),
            align: TextAlign::default(),
            tracking: Tracking::default(),
            case: TextCase::default(),
            select: UserSelect::default(),
            numerals: Numerals::default(),
        }
    }

    /// The style egui itself would give body text: [`egui::TextStyle::Body`]'s
    /// font, the visuals' text colour, and egui's row height for that font as
    /// the line height.
    ///
    /// This is the default base of every [`LayoutState`](crate::LayoutState)
    /// build. It reads the context's current style, so a theme change is
    /// reflected on the next call. Fonts must be available: run at least one
    /// egui pass on `ctx` first.
    pub fn from_egui(ctx: &egui::Context) -> Self {
        let style = ctx.global_style();
        let font_id = style
            .text_styles
            .get(&egui::TextStyle::Body)
            .cloned()
            .unwrap_or_default();
        let line_height = ctx.fonts_mut(|fonts| fonts.row_height(&font_id));
        Self::new(
            TextSpec {
                size: font_id.size,
                line_height,
                family: font_id.family,
            },
            style.visuals.text_color(),
        )
    }
}

/// The text classes one node sets; fields it leaves unset are inherited.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextClasses {
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) family: Option<FontFamily>,
    pub(crate) color: Option<Color32>,
    pub(crate) font_style: Option<FontStyle>,
    pub(crate) decoration: Option<TextDecoration>,
    pub(crate) white_space: Option<WhiteSpace>,
    pub(crate) align: Option<TextAlign>,
    pub(crate) tracking: Option<Tracking>,
    pub(crate) case: Option<TextCase>,
    pub(crate) select: Option<UserSelect>,
    pub(crate) numerals: Option<Numerals>,
}

impl TextClasses {
    /// These classes with those `other` sets on top.
    #[must_use]
    pub(crate) fn merge(self, other: Self) -> Self {
        Self {
            size: other.size.or(self.size),
            line_height: other.line_height.or(self.line_height),
            family: other.family.or(self.family),
            color: other.color.or(self.color),
            font_style: other.font_style.or(self.font_style),
            decoration: other.decoration.or(self.decoration),
            white_space: other.white_space.or(self.white_space),
            align: other.align.or(self.align),
            tracking: other.tracking.or(self.tracking),
            case: other.case.or(self.case),
            select: other.select.or(self.select),
            numerals: other.numerals.or(self.numerals),
        }
    }

    /// These classes merged over `inherited`, field by field.
    pub fn over(&self, inherited: &TextStyle) -> TextStyle {
        TextStyle {
            spec: TextSpec {
                size: self.size.unwrap_or(inherited.spec.size),
                line_height: self.line_height.unwrap_or(inherited.spec.line_height),
                family: self
                    .family
                    .clone()
                    .unwrap_or_else(|| inherited.spec.family.clone()),
            },
            color: self.color.unwrap_or(inherited.color),
            font_style: self.font_style.unwrap_or(inherited.font_style),
            decoration: self.decoration.unwrap_or(inherited.decoration),
            white_space: self.white_space.unwrap_or(inherited.white_space),
            align: self.align.unwrap_or(inherited.align),
            tracking: self.tracking.unwrap_or(inherited.tracking),
            case: self.case.unwrap_or(inherited.case),
            select: self.select.unwrap_or(inherited.select),
            numerals: self.numerals.unwrap_or(inherited.numerals),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn base() -> TextStyle {
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 18.0,
                family: FontFamily::Proportional,
            },
            Color32::BLACK,
        )
    }

    #[test]
    fn no_classes_inherit_everything() {
        assert_eq!(TextClasses::default().over(&base()), base());
    }

    #[test]
    fn each_class_overrides_only_its_field() {
        let classes = TextClasses {
            color: Some(Color32::RED),
            font_style: Some(FontStyle::Italic),
            ..Default::default()
        };
        let resolved = classes.over(&base());
        assert_eq!(resolved.color, Color32::RED);
        assert_eq!(resolved.font_style, FontStyle::Italic);
        assert_eq!(
            TextStyle {
                color: base().color,
                font_style: FontStyle::Normal,
                ..resolved
            },
            base()
        );
    }

    #[test]
    fn explicit_unset_overrides_an_inherited_flag() {
        let inherited = TextStyle {
            font_style: FontStyle::Italic,
            ..base()
        };
        let classes = TextClasses {
            font_style: Some(FontStyle::Normal),
            ..Default::default()
        };
        assert_eq!(classes.over(&inherited).font_style, FontStyle::Normal);
    }

    #[test]
    fn case_transforms() {
        assert_eq!(TextCase::Normal.apply("a b"), "a b");
        assert_eq!(TextCase::Upper.apply("a b"), "A B");
        assert_eq!(
            TextCase::Capitalize.apply("hello big  world"),
            "Hello Big  World"
        );
    }
}
