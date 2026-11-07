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
    /// Italic text.
    pub italic: bool,
    /// Underlined text.
    pub underline: bool,
    /// Wrapping behaviour.
    pub white_space: WhiteSpace,
    /// Horizontal alignment.
    pub align: TextAlign,
    /// Extra letter spacing.
    pub tracking: Tracking,
    /// Capitalisation.
    pub case: TextCase,
}

impl TextStyle {
    /// A style with the given role and colour and no other classes.
    pub fn new(spec: TextSpec, color: Color32) -> Self {
        Self {
            spec,
            color,
            italic: false,
            underline: false,
            white_space: WhiteSpace::default(),
            align: TextAlign::default(),
            tracking: Tracking::default(),
            case: TextCase::default(),
        }
    }
}

/// The text classes one node sets; fields it leaves unset are inherited.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextClasses {
    pub(crate) size: Option<f32>,
    pub(crate) line_height: Option<f32>,
    pub(crate) family: Option<FontFamily>,
    pub(crate) color: Option<Color32>,
    pub(crate) italic: Option<bool>,
    pub(crate) underline: Option<bool>,
    pub(crate) white_space: Option<WhiteSpace>,
    pub(crate) align: Option<TextAlign>,
    pub(crate) tracking: Option<Tracking>,
    pub(crate) case: Option<TextCase>,
}

impl TextClasses {
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
            italic: self.italic.unwrap_or(inherited.italic),
            underline: self.underline.unwrap_or(inherited.underline),
            white_space: self.white_space.unwrap_or(inherited.white_space),
            align: self.align.unwrap_or(inherited.align),
            tracking: self.tracking.unwrap_or(inherited.tracking),
            case: self.case.unwrap_or(inherited.case),
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
            italic: Some(true),
            ..Default::default()
        };
        let resolved = classes.over(&base());
        assert_eq!(resolved.color, Color32::RED);
        assert!(resolved.italic);
        assert_eq!(
            TextStyle {
                color: base().color,
                italic: false,
                ..resolved
            },
            base()
        );
    }

    #[test]
    fn explicit_unset_overrides_an_inherited_flag() {
        let inherited = TextStyle {
            italic: true,
            ..base()
        };
        let classes = TextClasses {
            italic: Some(false),
            ..Default::default()
        };
        assert!(!classes.over(&inherited).italic);
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
