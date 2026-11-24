use std::fmt;

use egui::{Color32, Context, Theme};

use crate::Tone;

use super::{BaseColor, Palette, ThemeColor, Tokens};

/// TODO(prose): a pair of tokens the components draw text on: the text
/// colour, then the surface under it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContrastPair {
    /// `foreground` on `background`.
    ForegroundOnBackground,
    /// `foreground` on `card`.
    ForegroundOnCard,
    /// `primary-foreground` on `primary`.
    PrimaryForegroundOnPrimary,
    /// `secondary-foreground` on `secondary`.
    SecondaryForegroundOnSecondary,
    /// `accent-foreground` on `accent`.
    AccentForegroundOnAccent,
    /// `muted-foreground` on `muted`.
    MutedForegroundOnMuted,
    /// `muted-foreground` on `background`.
    MutedForegroundOnBackground,
    /// `primary`, the colour of links, on `background`.
    LinkOnBackground,
    /// The text on `destructive` on `destructive`.
    DestructiveForegroundOnDestructive,
    /// `popover-foreground` on `popover`.
    PopoverForegroundOnPopover,
    /// TODO(prose): a tone's colour as text on `background`.
    ToneOnBackground(Tone),
    /// TODO(prose): a tone's colour as text on `card`.
    ToneOnCard(Tone),
    /// TODO(prose): a tone's foreground on a solid fill of the tone.
    ToneForegroundOnTone(Tone),
}

impl ContrastPair {
    /// Every pair.
    pub const ALL: [Self; 19] = [
        Self::ForegroundOnBackground,
        Self::ForegroundOnCard,
        Self::PrimaryForegroundOnPrimary,
        Self::SecondaryForegroundOnSecondary,
        Self::AccentForegroundOnAccent,
        Self::MutedForegroundOnMuted,
        Self::MutedForegroundOnBackground,
        Self::LinkOnBackground,
        Self::DestructiveForegroundOnDestructive,
        Self::PopoverForegroundOnPopover,
        Self::ToneOnBackground(Tone::Info),
        Self::ToneOnCard(Tone::Info),
        Self::ToneForegroundOnTone(Tone::Info),
        Self::ToneOnBackground(Tone::Success),
        Self::ToneOnCard(Tone::Success),
        Self::ToneForegroundOnTone(Tone::Success),
        Self::ToneOnBackground(Tone::Warning),
        Self::ToneOnCard(Tone::Warning),
        Self::ToneForegroundOnTone(Tone::Warning),
    ];

    /// The pair by shadcn's names, as `text on surface`.
    pub fn name(self) -> &'static str {
        match self {
            Self::ForegroundOnBackground => "foreground on background",
            Self::ForegroundOnCard => "foreground on card",
            Self::PrimaryForegroundOnPrimary => "primary-foreground on primary",
            Self::SecondaryForegroundOnSecondary => "secondary-foreground on secondary",
            Self::AccentForegroundOnAccent => "accent-foreground on accent",
            Self::MutedForegroundOnMuted => "muted-foreground on muted",
            Self::MutedForegroundOnBackground => "muted-foreground on background",
            Self::LinkOnBackground => "primary (links) on background",
            Self::DestructiveForegroundOnDestructive => "destructive-foreground on destructive",
            Self::PopoverForegroundOnPopover => "popover-foreground on popover",
            Self::ToneOnBackground(Tone::Info) => "info on background",
            Self::ToneOnCard(Tone::Info) => "info on card",
            Self::ToneForegroundOnTone(Tone::Info) => "info-foreground on info",
            Self::ToneOnBackground(Tone::Success) => "success on background",
            Self::ToneOnCard(Tone::Success) => "success on card",
            Self::ToneForegroundOnTone(Tone::Success) => "success-foreground on success",
            Self::ToneOnBackground(Tone::Warning) => "warning on background",
            Self::ToneOnCard(Tone::Warning) => "warning on card",
            Self::ToneForegroundOnTone(Tone::Warning) => "warning-foreground on warning",
        }
    }

    /// The text colour and the surface colour of the pair in `tokens`.
    pub fn colours(self, t: &Tokens) -> (Color32, Color32) {
        match self {
            Self::ForegroundOnBackground => (t.foreground, t.background),
            Self::ForegroundOnCard => (t.foreground, t.card),
            Self::PrimaryForegroundOnPrimary => (t.primary_foreground, t.primary),
            Self::SecondaryForegroundOnSecondary => (t.secondary_foreground, t.secondary),
            Self::AccentForegroundOnAccent => (t.accent_foreground, t.accent),
            Self::MutedForegroundOnMuted => (t.muted_foreground, t.muted),
            Self::MutedForegroundOnBackground => (t.muted_foreground, t.background),
            Self::LinkOnBackground => (t.primary, t.background),
            Self::DestructiveForegroundOnDestructive => (t.destructive_foreground, t.destructive),
            Self::PopoverForegroundOnPopover => (t.popover_foreground, t.popover),
            Self::ToneOnBackground(tone) => (t.tones().get(tone).tone, t.background),
            Self::ToneOnCard(tone) => (t.tones().get(tone).tone, t.card),
            Self::ToneForegroundOnTone(tone) => {
                let colours = t.tones().get(tone);
                (colours.foreground, colours.tone)
            }
        }
    }

    /// The pair's contrast in `tokens`, a translucent surface laid on
    /// `background` and translucent text on the surface.
    pub fn ratio(self, tokens: &Tokens) -> ContrastRatio {
        let (text, surface) = self.colours(tokens);
        let surface = tokens.background.blend(surface);
        ContrastRatio::between(surface.blend(text), surface)
    }
}

/// TODO(prose): a WCAG 2 contrast ratio, from 1:1 to 21:1.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ContrastRatio(f32);

impl ContrastRatio {
    /// WCAG 2's AA level for body text.
    pub const AA_TEXT: Self = Self(4.5);

    /// The ratio of two colours, whose alpha is ignored.
    pub fn between(a: Color32, b: Color32) -> Self {
        let (a, b) = (luminance(a), luminance(b));
        Self((a.max(b) + 0.05) / (a.min(b) + 0.05))
    }
}

impl From<ContrastRatio> for f32 {
    fn from(ratio: ContrastRatio) -> Self {
        ratio.0
    }
}

impl fmt::Display for ContrastRatio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}:1", self.0)
    }
}

fn luminance(c: Color32) -> f32 {
    let linear = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

/// A pair under AA in shadcn's registry colours under the preset's rules,
/// which the preset keeps as the registry has them and does not warn about.
/// `None` is every base colour or every theme colour; `ratio` is the measured
/// ratio, to 0.01. Public for the crate's contrast test.
#[doc(hidden)]
pub struct RegistryMiss {
    pub theme: Theme,
    pub base: Option<BaseColor>,
    pub colour: Option<ThemeColor>,
    pub pair: ContrastPair,
    pub ratio: f32,
}

const fn miss(
    theme: Theme,
    base: Option<BaseColor>,
    colour: Option<ThemeColor>,
    pair: ContrastPair,
    ratio: f32,
) -> RegistryMiss {
    RegistryMiss {
        theme,
        base,
        colour,
        pair,
        ratio,
    }
}

#[doc(hidden)]
pub const REGISTRY_MISSES: [RegistryMiss; 18] = {
    use BaseColor::{Mauve, Mist, Neutral, Olive, Stone, Taupe, Zinc};
    use ContrastPair::{
        DestructiveForegroundOnDestructive, LinkOnBackground, MutedForegroundOnBackground,
        MutedForegroundOnMuted,
    };
    use Theme::{Dark, Light};
    use ThemeColor::{Indigo, Lime, Violet, Yellow};
    [
        // White on dark `destructive`; the solid destructive button fills
        // `destructive/60` over the page in dark, not this.
        miss(Dark, None, None, DestructiveForegroundOnDestructive, 2.89),
        miss(Light, Some(Neutral), None, MutedForegroundOnMuted, 4.35),
        miss(Light, Some(Stone), None, MutedForegroundOnMuted, 4.39),
        miss(Light, Some(Zinc), None, MutedForegroundOnMuted, 4.39),
        miss(Light, Some(Olive), None, MutedForegroundOnMuted, 3.86),
        miss(Light, Some(Mist), None, MutedForegroundOnMuted, 4.14),
        miss(Light, Some(Taupe), None, MutedForegroundOnMuted, 4.40),
        miss(Light, Some(Olive), None, MutedForegroundOnBackground, 4.25),
        // The registry's light lime and yellow `primary` are 400 shades, made
        // for dark text on them.
        miss(Light, None, Some(Lime), LinkOnBackground, 1.53),
        miss(Light, None, Some(Yellow), LinkOnBackground, 1.57),
        // Indigo's 500 shade, dark `primary` under the documented rule.
        miss(Dark, Some(Neutral), Some(Indigo), LinkOnBackground, 4.33),
        miss(Dark, Some(Stone), Some(Indigo), LinkOnBackground, 4.32),
        miss(Dark, Some(Zinc), Some(Indigo), LinkOnBackground, 4.35),
        miss(Dark, Some(Mauve), Some(Indigo), LinkOnBackground, 4.33),
        miss(Dark, Some(Olive), Some(Indigo), LinkOnBackground, 4.28),
        miss(Dark, Some(Mist), Some(Indigo), LinkOnBackground, 4.31),
        miss(Dark, Some(Taupe), Some(Indigo), LinkOnBackground, 4.32),
        // The violet nudge was measured on the neutral page; olive's is
        // lighter.
        miss(Dark, Some(Olive), Some(Violet), LinkOnBackground, 4.48),
    ]
};

impl RegistryMiss {
    /// Whether `pair` in `tokens` has this miss's registry colours.
    fn is(&self, theme: Theme, pair: ContrastPair, tokens: &Tokens) -> bool {
        if self.theme != theme || self.pair != pair {
            return false;
        }
        let palette = Palette::new()
            .base(self.base.unwrap_or_default())
            .theme(self.colour.unwrap_or_default());
        let registry = Tokens::for_theme(theme).with_palette(palette, theme);
        pair.colours(&registry) == pair.colours(tokens)
    }
}

/// Reports, in a debug build, each pair of `tokens` under AA that is neither
/// accepted nor a registry miss, once per pair and theme. The colours are not
/// changed.
pub fn warn_low_contrast(ctx: &Context, theme: Theme, tokens: &Tokens) {
    if !cfg!(debug_assertions) {
        return;
    }
    for pair in ContrastPair::ALL {
        let ratio = pair.ratio(tokens);
        if ratio >= ContrastRatio::AA_TEXT
            || tokens.accepted_contrast.contains(&pair)
            || REGISTRY_MISSES.iter().any(|m| m.is(theme, pair, tokens))
        {
            continue;
        }
        let subject = egui::Id::NULL.with(("eggshell-ui-shadcn-contrast", theme, pair));
        crate::report(
            ctx,
            crate::Problem::LowContrast,
            Some(subject),
            || {
                format!(
                    "{} is {ratio} in {theme:?} (AA text needs {})",
                    pair.name(),
                    ContrastRatio::AA_TEXT
                )
            },
        );
    }
}
