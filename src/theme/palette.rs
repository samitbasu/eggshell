use egui::{Color32, Theme};

use super::ThemeColor;
use super::tokens::rgb;

/// TODO(prose): one of shadcn's base colours (`baseColor`), which sets the
/// greys: the page, its text, cards, popovers, borders, inputs, the muted,
/// secondary and accent surfaces and the focus ring. Neutral is shadcn's
/// default.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BaseColor {
    /// `neutral`.
    #[default]
    Neutral,
    /// `stone`.
    Stone,
    /// `zinc`.
    Zinc,
    /// `mauve`.
    Mauve,
    /// `olive`.
    Olive,
    /// `mist`.
    Mist,
    /// `taupe`.
    Taupe,
}

impl BaseColor {
    /// Every base colour, in the registry's order.
    pub const ALL: [Self; 7] = [
        Self::Neutral,
        Self::Stone,
        Self::Zinc,
        Self::Mauve,
        Self::Olive,
        Self::Mist,
        Self::Taupe,
    ];

    /// The base colour's name, as shadcn's registry titles it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::Stone => "Stone",
            Self::Zinc => "Zinc",
            Self::Mauve => "Mauve",
            Self::Olive => "Olive",
            Self::Mist => "Mist",
            Self::Taupe => "Taupe",
        }
    }

    /// shadcn's registry theme of the same name (`apps/v4/registry/themes.ts`,
    /// converted from oklch to sRGB), unchanged.
    fn scheme(self, theme: Theme) -> Scheme {
        match (self, theme) {
            (Self::Neutral, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x0a0a0a),
                card: rgb(0xffffff),
                card_foreground: rgb(0x0a0a0a),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x0a0a0a),
                primary: rgb(0x171717),
                primary_foreground: rgb(0xfafafa),
                secondary: rgb(0xf5f5f5),
                secondary_foreground: rgb(0x171717),
                muted: rgb(0xf5f5f5),
                muted_foreground: rgb(0x737373),
                accent: rgb(0xf5f5f5),
                accent_foreground: rgb(0x171717),
                border: rgb(0xe5e5e5),
                input: rgb(0xe5e5e5),
                ring: rgb(0xa1a1a1),
                charts: [
                    rgb(0xd4d4d4),
                    rgb(0x737373),
                    rgb(0x525252),
                    rgb(0x404040),
                    rgb(0x262626),
                ],
            },
            (Self::Neutral, Theme::Dark) => Scheme {
                background: rgb(0x0a0a0a),
                foreground: rgb(0xfafafa),
                card: rgb(0x171717),
                card_foreground: rgb(0xfafafa),
                popover: rgb(0x171717),
                popover_foreground: rgb(0xfafafa),
                primary: rgb(0xe5e5e5),
                primary_foreground: rgb(0x171717),
                secondary: rgb(0x262626),
                secondary_foreground: rgb(0xfafafa),
                muted: rgb(0x262626),
                muted_foreground: rgb(0xa1a1a1),
                accent: rgb(0x262626),
                accent_foreground: rgb(0xfafafa),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x737373),
                charts: [
                    rgb(0xd4d4d4),
                    rgb(0x737373),
                    rgb(0x525252),
                    rgb(0x404040),
                    rgb(0x262626),
                ],
            },
            (Self::Stone, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x0c0a09),
                card: rgb(0xffffff),
                card_foreground: rgb(0x0c0a09),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x0c0a09),
                primary: rgb(0x1c1917),
                primary_foreground: rgb(0xfafaf9),
                secondary: rgb(0xf5f5f4),
                secondary_foreground: rgb(0x1c1917),
                muted: rgb(0xf5f5f4),
                muted_foreground: rgb(0x79716b),
                accent: rgb(0xf5f5f4),
                accent_foreground: rgb(0x1c1917),
                border: rgb(0xe7e5e4),
                input: rgb(0xe7e5e4),
                ring: rgb(0xa6a09b),
                charts: [
                    rgb(0xd6d3d1),
                    rgb(0x79716b),
                    rgb(0x57534d),
                    rgb(0x44403b),
                    rgb(0x292524),
                ],
            },
            (Self::Stone, Theme::Dark) => Scheme {
                background: rgb(0x0c0a09),
                foreground: rgb(0xfafaf9),
                card: rgb(0x1c1917),
                card_foreground: rgb(0xfafaf9),
                popover: rgb(0x1c1917),
                popover_foreground: rgb(0xfafaf9),
                primary: rgb(0xe7e5e4),
                primary_foreground: rgb(0x1c1917),
                secondary: rgb(0x292524),
                secondary_foreground: rgb(0xfafaf9),
                muted: rgb(0x292524),
                muted_foreground: rgb(0xa6a09b),
                accent: rgb(0x292524),
                accent_foreground: rgb(0xfafaf9),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x79716b),
                charts: [
                    rgb(0xd6d3d1),
                    rgb(0x79716b),
                    rgb(0x57534d),
                    rgb(0x44403b),
                    rgb(0x292524),
                ],
            },
            (Self::Zinc, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x09090b),
                card: rgb(0xffffff),
                card_foreground: rgb(0x09090b),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x09090b),
                primary: rgb(0x18181b),
                primary_foreground: rgb(0xfafafa),
                secondary: rgb(0xf4f4f5),
                secondary_foreground: rgb(0x18181b),
                muted: rgb(0xf4f4f5),
                muted_foreground: rgb(0x71717b),
                accent: rgb(0xf4f4f5),
                accent_foreground: rgb(0x18181b),
                border: rgb(0xe4e4e7),
                input: rgb(0xe4e4e7),
                ring: rgb(0x9f9fa9),
                charts: [
                    rgb(0xd4d4d8),
                    rgb(0x71717b),
                    rgb(0x52525c),
                    rgb(0x3f3f46),
                    rgb(0x27272a),
                ],
            },
            (Self::Zinc, Theme::Dark) => Scheme {
                background: rgb(0x09090b),
                foreground: rgb(0xfafafa),
                card: rgb(0x18181b),
                card_foreground: rgb(0xfafafa),
                popover: rgb(0x18181b),
                popover_foreground: rgb(0xfafafa),
                primary: rgb(0xe4e4e7),
                primary_foreground: rgb(0x18181b),
                secondary: rgb(0x27272a),
                secondary_foreground: rgb(0xfafafa),
                muted: rgb(0x27272a),
                muted_foreground: rgb(0x9f9fa9),
                accent: rgb(0x27272a),
                accent_foreground: rgb(0xfafafa),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x71717b),
                charts: [
                    rgb(0xd4d4d8),
                    rgb(0x71717b),
                    rgb(0x52525c),
                    rgb(0x3f3f46),
                    rgb(0x27272a),
                ],
            },
            (Self::Mauve, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x0c090c),
                card: rgb(0xffffff),
                card_foreground: rgb(0x0c090c),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x0c090c),
                primary: rgb(0x1d161e),
                primary_foreground: rgb(0xfafafa),
                secondary: rgb(0xf3f1f3),
                secondary_foreground: rgb(0x1d161e),
                muted: rgb(0xf3f1f3),
                muted_foreground: rgb(0x79697b),
                accent: rgb(0xf3f1f3),
                accent_foreground: rgb(0x1d161e),
                border: rgb(0xe7e4e7),
                input: rgb(0xe7e4e7),
                ring: rgb(0xa89ea9),
                charts: [
                    rgb(0xd7d0d7),
                    rgb(0x79697b),
                    rgb(0x594c5b),
                    rgb(0x463947),
                    rgb(0x2a212c),
                ],
            },
            (Self::Mauve, Theme::Dark) => Scheme {
                background: rgb(0x0c090c),
                foreground: rgb(0xfafafa),
                card: rgb(0x1d161e),
                card_foreground: rgb(0xfafafa),
                popover: rgb(0x1d161e),
                popover_foreground: rgb(0xfafafa),
                primary: rgb(0xe7e4e7),
                primary_foreground: rgb(0x1d161e),
                secondary: rgb(0x2a212c),
                secondary_foreground: rgb(0xfafafa),
                muted: rgb(0x2a212c),
                muted_foreground: rgb(0xa89ea9),
                accent: rgb(0x2a212c),
                accent_foreground: rgb(0xfafafa),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x79697b),
                charts: [
                    rgb(0xd7d0d7),
                    rgb(0x79697b),
                    rgb(0x594c5b),
                    rgb(0x463947),
                    rgb(0x2a212c),
                ],
            },
            (Self::Olive, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x0c0c09),
                card: rgb(0xffffff),
                card_foreground: rgb(0x0c0c09),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x0c0c09),
                primary: rgb(0x1d1d16),
                primary_foreground: rgb(0xfbfbf9),
                secondary: rgb(0xf4f4f0),
                secondary_foreground: rgb(0x1d1d16),
                muted: rgb(0xf4f4f0),
                muted_foreground: rgb(0x7c7c67),
                accent: rgb(0xf4f4f0),
                accent_foreground: rgb(0x1d1d16),
                border: rgb(0xe8e8e3),
                input: rgb(0xe8e8e3),
                ring: rgb(0xabab9c),
                charts: [
                    rgb(0xd8d8d0),
                    rgb(0x7c7c67),
                    rgb(0x5b5b4b),
                    rgb(0x474739),
                    rgb(0x2b2b22),
                ],
            },
            (Self::Olive, Theme::Dark) => Scheme {
                background: rgb(0x0c0c09),
                foreground: rgb(0xfbfbf9),
                card: rgb(0x1d1d16),
                card_foreground: rgb(0xfbfbf9),
                popover: rgb(0x1d1d16),
                popover_foreground: rgb(0xfbfbf9),
                primary: rgb(0xe8e8e3),
                primary_foreground: rgb(0x1d1d16),
                secondary: rgb(0x2b2b22),
                secondary_foreground: rgb(0xfbfbf9),
                muted: rgb(0x2b2b22),
                muted_foreground: rgb(0xabab9c),
                accent: rgb(0x2b2b22),
                accent_foreground: rgb(0xfbfbf9),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x7c7c67),
                charts: [
                    rgb(0xd8d8d0),
                    rgb(0x7c7c67),
                    rgb(0x5b5b4b),
                    rgb(0x474739),
                    rgb(0x2b2b22),
                ],
            },
            (Self::Mist, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x090b0c),
                card: rgb(0xffffff),
                card_foreground: rgb(0x090b0c),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x090b0c),
                primary: rgb(0x161b1d),
                primary_foreground: rgb(0xf9fbfb),
                secondary: rgb(0xf1f3f3),
                secondary_foreground: rgb(0x161b1d),
                muted: rgb(0xf1f3f3),
                muted_foreground: rgb(0x67787c),
                accent: rgb(0xf1f3f3),
                accent_foreground: rgb(0x161b1d),
                border: rgb(0xe3e7e8),
                input: rgb(0xe3e7e8),
                ring: rgb(0x9ca8ab),
                charts: [
                    rgb(0xd0d6d8),
                    rgb(0x67787c),
                    rgb(0x4b585b),
                    rgb(0x394447),
                    rgb(0x22292b),
                ],
            },
            (Self::Mist, Theme::Dark) => Scheme {
                background: rgb(0x090b0c),
                foreground: rgb(0xf9fbfb),
                card: rgb(0x161b1d),
                card_foreground: rgb(0xf9fbfb),
                popover: rgb(0x161b1d),
                popover_foreground: rgb(0xf9fbfb),
                primary: rgb(0xe3e7e8),
                primary_foreground: rgb(0x161b1d),
                secondary: rgb(0x22292b),
                secondary_foreground: rgb(0xf9fbfb),
                muted: rgb(0x22292b),
                muted_foreground: rgb(0x9ca8ab),
                accent: rgb(0x22292b),
                accent_foreground: rgb(0xf9fbfb),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x67787c),
                charts: [
                    rgb(0xd0d6d8),
                    rgb(0x67787c),
                    rgb(0x4b585b),
                    rgb(0x394447),
                    rgb(0x22292b),
                ],
            },
            (Self::Taupe, Theme::Light) => Scheme {
                background: rgb(0xffffff),
                foreground: rgb(0x0c0a09),
                card: rgb(0xffffff),
                card_foreground: rgb(0x0c0a09),
                popover: rgb(0xffffff),
                popover_foreground: rgb(0x0c0a09),
                primary: rgb(0x1d1816),
                primary_foreground: rgb(0xfbfaf9),
                secondary: rgb(0xf3f1f1),
                secondary_foreground: rgb(0x1d1816),
                muted: rgb(0xf3f1f1),
                muted_foreground: rgb(0x7c6d67),
                accent: rgb(0xf3f1f1),
                accent_foreground: rgb(0x1d1816),
                border: rgb(0xe8e4e3),
                input: rgb(0xe8e4e3),
                ring: rgb(0xaba09c),
                charts: [
                    rgb(0xd8d2d0),
                    rgb(0x7c6d67),
                    rgb(0x5b4f4b),
                    rgb(0x473c39),
                    rgb(0x2b2422),
                ],
            },
            (Self::Taupe, Theme::Dark) => Scheme {
                background: rgb(0x0c0a09),
                foreground: rgb(0xfbfaf9),
                card: rgb(0x1d1816),
                card_foreground: rgb(0xfbfaf9),
                popover: rgb(0x1d1816),
                popover_foreground: rgb(0xfbfaf9),
                primary: rgb(0xe8e4e3),
                primary_foreground: rgb(0x1d1816),
                secondary: rgb(0x2b2422),
                secondary_foreground: rgb(0xfbfaf9),
                muted: rgb(0x2b2422),
                muted_foreground: rgb(0xaba09c),
                accent: rgb(0x2b2422),
                accent_foreground: rgb(0xfbfaf9),
                border: Color32::from_white_alpha(26),
                input: Color32::from_white_alpha(38),
                ring: rgb(0x7c6d67),
                charts: [
                    rgb(0xd8d2d0),
                    rgb(0x7c6d67),
                    rgb(0x5b4f4b),
                    rgb(0x473c39),
                    rgb(0x2b2422),
                ],
            },
        }
    }
}

/// TODO(prose): the colours of a preset in shadcn's terms: a
/// [`BaseColor`] for the greys and a [`ThemeColor`] for `primary` and the
/// charts.
///
/// Every [`ThemeColor`] goes with every [`BaseColor`]: shadcn offers a base
/// colour's own theme and the coloured ones, which is what
/// [`ThemeColor::Base`] and the other variants are.
///
/// ```
/// use eggshell_ui::theme::{BaseColor, Palette, ThemeColor};
///
/// let palette = Palette::new().base(BaseColor::Zinc).theme(ThemeColor::Blue);
/// # let _ = palette;
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Palette {
    base: BaseColor,
    theme: ThemeColor,
}

impl Palette {
    /// shadcn's default: neutral throughout.
    pub const fn new() -> Self {
        Self {
            base: BaseColor::Neutral,
            theme: ThemeColor::Base,
        }
    }

    /// This palette on `base`.
    #[must_use]
    pub const fn base(self, base: BaseColor) -> Self {
        Self { base, ..self }
    }

    /// This palette with `theme` for `primary` and the charts.
    #[must_use]
    pub const fn theme(self, theme: ThemeColor) -> Self {
        Self { theme, ..self }
    }

    /// The colours of this palette in `theme`.
    pub(crate) fn scheme(self, theme: Theme) -> Scheme {
        let base = self.base.scheme(theme);
        let (primary, primary_foreground) = self.theme.primary(theme, &base);
        let charts = self.theme.charts(&base);
        Scheme {
            primary,
            primary_foreground,
            charts,
            ..base
        }
    }
}

/// The colours a [`Palette`] sets, by shadcn's names.
pub(crate) struct Scheme {
    pub background: Color32,
    pub foreground: Color32,
    pub card: Color32,
    pub card_foreground: Color32,
    pub popover: Color32,
    pub popover_foreground: Color32,
    pub primary: Color32,
    pub primary_foreground: Color32,
    pub secondary: Color32,
    pub secondary_foreground: Color32,
    pub muted: Color32,
    pub muted_foreground: Color32,
    pub accent: Color32,
    pub accent_foreground: Color32,
    pub border: Color32,
    pub input: Color32,
    pub ring: Color32,
    pub charts: [Color32; 5],
}
