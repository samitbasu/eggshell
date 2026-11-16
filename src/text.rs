//! Text styling, and how it is inherited down the tree.

use std::borrow::Cow;

use egui::{Color32, Context, FontFamily, FontId, Id};

/// A text role: size, line height, font family and weight.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    /// Font size in points.
    pub size: f32,
    /// Height of one line in points.
    pub line_height: f32,
    /// The registered font family, drawn at [`Weight::Normal`].
    pub family: FontFamily,
    /// The weight, drawn by the face the [`Style`](crate::Style)'s
    /// [`Faces`] name for it.
    pub weight: Weight,
}

impl TextSpec {
    /// The egui font for this role in `ctx`: the family of the face the
    /// installed [`Style`](crate::Style)'s [`Faces`] give its weight.
    ///
    /// A weight with no registered face is drawn by the nearest lighter one
    /// that has a face, never by a synthesised or heavier one. A weight the
    /// style declares no face for at all is reported as
    /// [`Problem::MissingFace`](crate::Problem::MissingFace).
    pub fn font_id(&self, ctx: &Context) -> FontId {
        FontId::new(self.size, self.face(ctx))
    }

    fn face(&self, ctx: &Context) -> FontFamily {
        if self.weight == Weight::Normal {
            return self.family.clone();
        }
        let style = crate::Style::of(ctx);
        let registered = ctx.fonts(|fonts| fonts.families());
        match style
            .faces
            .resolve(&self.family, self.weight, |face| registered.contains(face))
        {
            Resolution::Face(face) | Resolution::Pending(face) => face,
            Resolution::Missing(face) => {
                report_missing(ctx, &self.family, self.weight);
                face
            }
        }
    }
}

/// A font weight, as Tailwind's `font-normal`, `font-medium`,
/// `font-semibold` and `font-bold` name them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weight {
    /// 400.
    #[default]
    Normal,
    /// 500.
    Medium,
    /// 600.
    SemiBold,
    /// 700.
    Bold,
}

impl Weight {
    /// Every weight, lightest first.
    pub const ALL: [Self; 4] = [Self::Normal, Self::Medium, Self::SemiBold, Self::Bold];

    /// The CSS weight: 400, 500, 600 or 700.
    pub fn number(self) -> u16 {
        match self {
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
        }
    }
}

/// One entry of [`Faces`]: the registered family that draws `family` at
/// `weight`.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    /// The family a [`TextSpec`] names.
    pub family: FontFamily,
    /// The weight drawn.
    pub weight: Weight,
    /// The registered family that has the face.
    pub face: FontFamily,
}

/// Which registered family draws each weight of a family. A family is its
/// own [`Weight::Normal`] face.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Faces(Vec<Face>);

/// How [`Faces`] resolved a weight.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Resolution {
    /// The weight's own face.
    Face(FontFamily),
    /// A lighter face: the weight's face is declared but egui has not
    /// registered it yet, as on the pass fonts are added.
    Pending(FontFamily),
    /// A lighter face: no face is declared for the weight.
    Missing(FontFamily),
}

impl Faces {
    /// These faces with `face` drawing `family` at `weight`, replacing any
    /// face declared for the pair before.
    #[must_use]
    pub fn with(mut self, family: FontFamily, weight: Weight, face: FontFamily) -> Self {
        self.0
            .retain(|entry| entry.family != family || entry.weight != weight);
        self.0.push(Face {
            family,
            weight,
            face,
        });
        self
    }

    /// Every declared face.
    pub fn iter(&self) -> impl Iterator<Item = &Face> {
        self.0.iter()
    }

    pub(crate) fn resolve(
        &self,
        family: &FontFamily,
        weight: Weight,
        registered: impl Fn(&FontFamily) -> bool,
    ) -> Resolution {
        let declared = |entry: &&Face| entry.family == *family && entry.weight <= weight;
        let usable = self
            .0
            .iter()
            .filter(declared)
            .filter(|entry| registered(&entry.face))
            .max_by_key(|entry| entry.weight);
        let face = usable.map_or_else(|| family.clone(), |entry| entry.face.clone());
        if usable.is_some_and(|entry| entry.weight == weight) || weight == Weight::Normal {
            Resolution::Face(face)
        } else if self
            .0
            .iter()
            .filter(declared)
            .any(|entry| entry.weight == weight)
        {
            Resolution::Pending(face)
        } else {
            Resolution::Missing(face)
        }
    }
}

fn report_missing(ctx: &Context, family: &FontFamily, weight: Weight) {
    crate::report::report(
        ctx,
        crate::Problem::MissingFace,
        Some(Id::NULL.with(("missing-face", family, weight))),
        || {
            format!(
                "no {weight:?} ({}) face of {family:?} is declared in the style's `faces`; \
                 drawing the nearest lighter weight that has one",
                weight.number()
            )
        },
    );
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
    /// A negative `spare` (text taller than its box) is kept, as CSS's default
    /// (unsafe) alignment does: centred text overflows evenly above and below,
    /// end-aligned text overflows upward.
    pub(crate) fn offset(self, spare: f32) -> f32 {
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
    /// Wrap at word boundaries over at most this many lines, the last ending
    /// with an ellipsis where the text overflows.
    Clamp(u32),
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
                weight: Weight::Normal,
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
    pub(crate) weight: Option<Weight>,
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
            weight: other.weight.or(self.weight),
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
                weight: self.weight.unwrap_or(inherited.spec.weight),
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
mod weight_tests {
    use super::*;

    fn named(name: &str) -> FontFamily {
        FontFamily::Name(name.into())
    }

    fn faces() -> Faces {
        Faces::default()
            .with(FontFamily::Proportional, Weight::Medium, named("medium"))
            .with(FontFamily::Proportional, Weight::Bold, named("bold"))
    }

    fn all(_: &FontFamily) -> bool {
        true
    }

    #[test]
    fn a_declared_registered_weight_is_its_own_face() {
        assert_eq!(
            faces().resolve(&FontFamily::Proportional, Weight::Bold, all),
            Resolution::Face(named("bold"))
        );
    }

    #[test]
    fn a_missing_weight_falls_back_to_the_nearest_lighter_face() {
        assert_eq!(
            faces().resolve(&FontFamily::Proportional, Weight::SemiBold, all),
            Resolution::Missing(named("medium")),
            "SemiBold has no face: Medium, never Bold"
        );
        assert_eq!(
            Faces::default().resolve(&FontFamily::Proportional, Weight::Medium, all),
            Resolution::Missing(FontFamily::Proportional),
            "no faces at all: the family itself, which is Normal"
        );
    }

    #[test]
    fn a_declared_weight_egui_has_not_registered_yet_is_pending() {
        let only_medium = |face: &FontFamily| *face == named("medium");
        assert_eq!(
            faces().resolve(&FontFamily::Proportional, Weight::Bold, only_medium),
            Resolution::Pending(named("medium"))
        );
    }

    #[test]
    fn faces_of_another_family_do_not_apply() {
        assert_eq!(
            faces().resolve(&FontFamily::Monospace, Weight::Bold, all),
            Resolution::Missing(FontFamily::Monospace)
        );
    }

    /// A context with egui's default fonts, plus `medium` registered as a
    /// named family, and a style that declares it for Proportional.
    fn context() -> Context {
        let ctx = Context::default();
        let mut fonts = egui::FontDefinitions::default();
        let chain = fonts.families[&FontFamily::Proportional].clone();
        fonts.families.insert(named("medium"), chain);
        ctx.set_fonts(fonts);
        let mut style = crate::Style::from_egui(&ctx.global_style());
        style.faces =
            Faces::default().with(FontFamily::Proportional, Weight::Medium, named("medium"));
        style.install(&ctx);
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        ctx
    }

    fn spec(weight: Weight) -> TextSpec {
        TextSpec {
            size: 14.0,
            line_height: 20.0,
            family: FontFamily::Proportional,
            weight,
        }
    }

    #[test]
    fn a_weight_reaches_the_font_id_of_its_registered_face() {
        let ctx = context();
        assert!(
            ctx.fonts(|fonts| fonts.families().contains(&named("medium"))),
            "precondition: the medium face is registered"
        );
        assert_eq!(
            spec(Weight::Medium).font_id(&ctx),
            FontId::new(14.0, named("medium"))
        );
        assert_eq!(
            spec(Weight::Normal).font_id(&ctx),
            FontId::new(14.0, FontFamily::Proportional)
        );
    }

    #[test]
    fn a_heavier_weight_without_a_face_draws_the_lighter_one() {
        let ctx = context();
        assert_eq!(
            spec(Weight::Bold).font_id(&ctx),
            FontId::new(14.0, named("medium"))
        );
    }

    #[test]
    fn a_missing_face_is_reported_once_per_family_and_weight() {
        let ctx = context();
        let missing = |weight| {
            spec(weight).font_id(&ctx);
            crate::report::reports(&ctx)
                .iter()
                .filter(|report| report.problem == crate::Problem::MissingFace)
                .map(|report| (report.message.clone(), report.count))
                .collect::<Vec<_>>()
        };
        let once = missing(Weight::Bold);
        assert_eq!(once.len(), 1, "{once:?}");
        assert!(once[0].0.contains("no Bold (700) face"), "{once:?}");
        let twice = missing(Weight::Bold);
        assert_eq!(twice.len(), 1, "a repeat is counted: {twice:?}");
        assert_eq!(twice[0].1, 2);
        assert_eq!(
            missing(Weight::SemiBold).len(),
            2,
            "another weight reports too"
        );
        assert_eq!(
            missing(Weight::Medium).len(),
            2,
            "a declared face never reports"
        );
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
                weight: Weight::Normal,
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
