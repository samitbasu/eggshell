use egui::{Color32, Theme};

use crate::palette::Scheme;
use crate::tokens::rgb;

/// One of shadcn's theme colours, which re-tints `primary` and
/// `primary-foreground`, and the charts. [`Base`](Self::Base), the base colour's own theme, is
/// shadcn's default. `ring` is not tinted:
/// the registry's coloured themes leave it to the base colour, so focus
/// rings are grey in every theme colour.
///
/// In the dark theme an accent departs from shadcn's registry, which uses
/// its 700/800 shade for `primary`: that is dim on near-black, so `primary`
/// is the 500 shade with a black `primary-foreground`.
///
/// The registry's coloured themes also set `secondary` to a near-neutral
/// zinc (`oklch(0.967 0.001 286.375)` in light); the preset keeps the base
/// colour's `secondary`, which differs by one to three 8-bit steps.
///
/// Dark violet is nudged from its 500 shade `#8e51ff` to `#8e52ff`, the
/// smallest step that lifts links (which use `primary`) from 4.498:1 to
/// 4.53:1 on the dark page; black on it is 4.8:1.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeColor {
    /// TODO(prose): the base colour's own theme, the default: near-black in
    /// light, light grey in dark, and grey charts.
    #[default]
    Base,
    /// `amber`.
    Amber,
    /// `blue`.
    Blue,
    /// `cyan`.
    Cyan,
    /// `emerald`.
    Emerald,
    /// `fuchsia`.
    Fuchsia,
    /// `green`.
    Green,
    /// `indigo`.
    Indigo,
    /// `lime`.
    Lime,
    /// `orange`.
    Orange,
    /// `pink`.
    Pink,
    /// `purple`.
    Purple,
    /// `red`.
    Red,
    /// `rose`.
    Rose,
    /// `sky`.
    Sky,
    /// `teal`.
    Teal,
    /// `violet`.
    Violet,
    /// `yellow`.
    Yellow,
}

impl ThemeColor {
    /// Every theme colour, in the registry's order.
    pub const ALL: [Self; 18] = [
        Self::Base,
        Self::Amber,
        Self::Blue,
        Self::Cyan,
        Self::Emerald,
        Self::Fuchsia,
        Self::Green,
        Self::Indigo,
        Self::Lime,
        Self::Orange,
        Self::Pink,
        Self::Purple,
        Self::Red,
        Self::Rose,
        Self::Sky,
        Self::Teal,
        Self::Violet,
        Self::Yellow,
    ];

    /// The theme colour's name, as shadcn's registry titles it.
    /// [`Base`](Self::Base) is titled after the base colour, which this
    /// does not know: it is "Base".
    pub fn name(self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Amber => "Amber",
            Self::Blue => "Blue",
            Self::Cyan => "Cyan",
            Self::Emerald => "Emerald",
            Self::Fuchsia => "Fuchsia",
            Self::Green => "Green",
            Self::Indigo => "Indigo",
            Self::Lime => "Lime",
            Self::Orange => "Orange",
            Self::Pink => "Pink",
            Self::Purple => "Purple",
            Self::Red => "Red",
            Self::Rose => "Rose",
            Self::Sky => "Sky",
            Self::Teal => "Teal",
            Self::Violet => "Violet",
            Self::Yellow => "Yellow",
        }
    }

    /// shadcn's registry themes (`apps/v4/registry/themes.ts`, converted from
    /// oklch to sRGB), with one departure: in dark the registry's `primary`
    /// is the 700/800 shade, which on near-black leaves a send button or a
    /// meter dim; here dark `primary` is the theme's 500 shade (its
    /// `chart-2`), with black for `primary-foreground`, because the theme's
    /// own 50 and 950 shades reach 4.5:1 on it for only some of the colours
    /// and black does for all of them. The base colour's own theme stays as
    /// the registry has it.
    pub(crate) fn primary(self, theme: Theme, base: &Scheme) -> (Color32, Color32) {
        match (self.shades(), theme) {
            (None, _) => (base.primary, base.primary_foreground),
            (Some(shades), Theme::Light) => (rgb(shades.primary), rgb(shades.primary_foreground)),
            (Some(shades), Theme::Dark) => (rgb(self.dark_primary(&shades)), rgb(BRIGHT_ON)),
        }
    }

    /// `chart-1` to `chart-5`, as the registry has them in both themes.
    pub(crate) fn charts(self, base: &Scheme) -> [Color32; 5] {
        self.shades()
            .map_or(base.charts, |shades| shades.charts.map(rgb))
    }

    fn dark_primary(self, shades: &Shades) -> u32 {
        let [_, five_hundred, ..] = shades.charts;
        match self {
            Self::Violet => 0x8e52ff,
            _ => five_hundred,
        }
    }

    /// The registry's light `primary` and `primary-foreground` and its
    /// charts; `None` for the base colour's own theme.
    fn shades(self) -> Option<Shades> {
        Some(match self {
            Self::Base => return None,
            Self::Amber => Shades {
                primary: 0xbb4d00,
                primary_foreground: 0xfffbeb,
                charts: [0xffd230, 0xfe9a00, 0xe17100, 0xbb4d00, 0x973c00],
            },
            Self::Blue => Shades {
                primary: 0x1447e6,
                primary_foreground: 0xeff6ff,
                charts: [0x8ec5ff, 0x2b7fff, 0x155dfc, 0x1447e6, 0x193cb8],
            },
            Self::Cyan => Shades {
                primary: 0x007595,
                primary_foreground: 0xecfeff,
                charts: [0x53eafd, 0x00b8db, 0x0092b8, 0x007595, 0x005f78],
            },
            Self::Emerald => Shades {
                primary: 0x007a55,
                primary_foreground: 0xecfdf5,
                charts: [0x5ee9b5, 0x00bc7d, 0x009966, 0x007a55, 0x006045],
            },
            Self::Fuchsia => Shades {
                primary: 0xa800b7,
                primary_foreground: 0xfdf4ff,
                charts: [0xf4a8ff, 0xe12afb, 0xc800de, 0xa800b7, 0x8a0194],
            },
            Self::Green => Shades {
                primary: 0x008236,
                primary_foreground: 0xf0fdf4,
                charts: [0x7bf1a8, 0x00c950, 0x00a63e, 0x008236, 0x016630],
            },
            Self::Indigo => Shades {
                primary: 0x432dd7,
                primary_foreground: 0xeef2ff,
                charts: [0xa3b3ff, 0x615fff, 0x4f39f6, 0x432dd7, 0x372aac],
            },
            Self::Lime => Shades {
                primary: 0x9ae600,
                primary_foreground: 0x35530e,
                charts: [0xbbf451, 0x7ccf00, 0x5ea500, 0x497d00, 0x3c6300],
            },
            Self::Orange => Shades {
                primary: 0xca3500,
                primary_foreground: 0xfff7ed,
                charts: [0xffb86a, 0xff6900, 0xf54900, 0xca3500, 0x9f2d00],
            },
            Self::Pink => Shades {
                primary: 0xc6005c,
                primary_foreground: 0xfdf2f8,
                charts: [0xfda5d5, 0xf6339a, 0xe60076, 0xc6005c, 0xa3004c],
            },
            Self::Purple => Shades {
                primary: 0x8200db,
                primary_foreground: 0xfaf5ff,
                charts: [0xdab2ff, 0xad46ff, 0x9810fa, 0x8200db, 0x6e11b0],
            },
            Self::Red => Shades {
                primary: 0xc10007,
                primary_foreground: 0xfef2f2,
                charts: [0xffa2a2, 0xfb2c36, 0xe7000b, 0xc10007, 0x9f0712],
            },
            Self::Rose => Shades {
                primary: 0xc70036,
                primary_foreground: 0xfff1f2,
                charts: [0xffa1ad, 0xff2056, 0xec003f, 0xc70036, 0xa50036],
            },
            Self::Sky => Shades {
                primary: 0x0069a8,
                primary_foreground: 0xf0f9ff,
                charts: [0x74d4ff, 0x00a6f4, 0x0084d1, 0x0069a8, 0x00598a],
            },
            Self::Teal => Shades {
                primary: 0x00786f,
                primary_foreground: 0xf0fdfa,
                charts: [0x46ecd5, 0x00bba7, 0x009689, 0x00786f, 0x005f5a],
            },
            Self::Violet => Shades {
                primary: 0x7008e7,
                primary_foreground: 0xf5f3ff,
                charts: [0xc4b4ff, 0x8e51ff, 0x7f22fe, 0x7008e7, 0x5d0ec0],
            },
            Self::Yellow => Shades {
                primary: 0xfdc700,
                primary_foreground: 0x733e0a,
                charts: [0xffdf20, 0xf0b100, 0xd08700, 0xa65f00, 0x894b00],
            },
        })
    }
}

/// The text on a dark accent's 500 shade.
const BRIGHT_ON: u32 = 0x000000;

struct Shades {
    primary: u32,
    primary_foreground: u32,
    charts: [u32; 5],
}
