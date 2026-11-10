use egui::{Color32, Theme};

use crate::tokens::rgb;

/// One of shadcn's theme colours, which re-tints `primary`,
/// `primary-foreground` and `ring`. Neutral is shadcn's default.
///
/// In the dark theme an accent departs from shadcn's registry, which uses
/// its 700/800 shade for `primary`: that is dim on near-black, so `primary`
/// is the 500 shade (the `ring`) with a black `primary-foreground`.
///
/// Dark violet is nudged from its 500 shade `#8e51ff` to `#8e52ff`, the
/// smallest step that lifts links (which use `primary`) from 4.498:1 to
/// 4.53:1 on the dark page; black on it is 4.8:1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeColor {
    /// The default: black in light, light grey in dark.
    #[default]
    Neutral,
    /// `blue`.
    Blue,
    /// `violet`.
    Violet,
    /// `green`.
    Green,
    /// `orange`.
    Orange,
    /// `rose`.
    Rose,
}

impl ThemeColor {
    /// Every accent, in the order of the type.
    pub const ALL: [Self; 6] = [
        Self::Neutral,
        Self::Blue,
        Self::Violet,
        Self::Green,
        Self::Orange,
        Self::Rose,
    ];

    /// The accent's name, as shadcn's themes page spells it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::Blue => "Blue",
            Self::Violet => "Violet",
            Self::Green => "Green",
            Self::Orange => "Orange",
            Self::Rose => "Rose",
        }
    }

    /// shadcn's registry themes (`apps/v4/registry/themes.ts`, converted from
    /// oklch to sRGB), with two departures. The registry leaves `ring`
    /// neutral; here it is the theme's 500 shade (its `chart-2`) so that
    /// focus follows the accent. And in dark the registry's `primary` is the
    /// 700/800 shade, which on near-black leaves a send button or a meter
    /// dim; here dark `primary` is that 500 shade too, with black for
    /// `primary-foreground`, because the theme's own 50 and 950 shades reach
    /// 4.5:1 on it for only some of the colours and black does for all of
    /// them. Neutral dark stays as the registry has it.
    pub(crate) fn hues(self, theme: Theme) -> Hues {
        let (primary, foreground, ring) = match (self, theme) {
            (Self::Neutral, Theme::Light) => (0x171717, 0xfafafa, 0xa1a1a1),
            (Self::Neutral, Theme::Dark) => (0xe5e5e5, 0x171717, 0x737373),
            (Self::Blue, Theme::Light) => (0x1447e6, 0xeff6ff, 0x2b7fff),
            (Self::Blue, Theme::Dark) => (0x2b7fff, BRIGHT_ON, 0x2b7fff),
            (Self::Violet, Theme::Light) => (0x7008e7, 0xf5f3ff, 0x8e51ff),
            (Self::Violet, Theme::Dark) => (0x8e52ff, BRIGHT_ON, 0x8e52ff),
            (Self::Green, Theme::Light) => (0x008236, 0xf0fdf4, 0x00c950),
            (Self::Green, Theme::Dark) => (0x00c950, BRIGHT_ON, 0x00c950),
            (Self::Orange, Theme::Light) => (0xca3500, 0xfff7ed, 0xff6900),
            (Self::Orange, Theme::Dark) => (0xff6900, BRIGHT_ON, 0xff6900),
            (Self::Rose, Theme::Light) => (0xc70036, 0xfff1f2, 0xff2056),
            (Self::Rose, Theme::Dark) => (0xff2056, BRIGHT_ON, 0xff2056),
        };
        Hues {
            primary: rgb(primary),
            primary_foreground: rgb(foreground),
            ring: rgb(ring),
        }
    }
}

/// The text on a dark accent's 500 shade.
const BRIGHT_ON: u32 = 0x000000;

pub(crate) struct Hues {
    pub primary: Color32,
    pub primary_foreground: Color32,
    pub ring: Color32,
}
