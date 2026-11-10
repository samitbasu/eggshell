use eggshell_ui::{Opacity, TextSpec};
use egui::{Color32, Shadow, Theme, Vec2, vec2};

use crate::styles::{DestructiveLook, Elevation, HoverSurface, SecondaryHover, ShadcnStyle};
use crate::theme_color::{Hues, ThemeColor};

/// A colour from `0xRRGGBB`.
pub(crate) const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Every value a shadcn preset is made of. All fields are public: start from
/// [`Tokens::light`] or [`Tokens::dark`] and override what the application
/// does differently.
///
/// Colour names are shadcn's CSS variables. Lengths are points.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    /// `--background`.
    pub background: Color32,
    /// `--foreground`.
    pub foreground: Color32,
    /// `--card`.
    pub card: Color32,
    /// `--card-foreground`.
    pub card_foreground: Color32,
    /// `--popover`.
    pub popover: Color32,
    /// `--popover-foreground`.
    pub popover_foreground: Color32,
    /// `--primary`.
    pub primary: Color32,
    /// `--primary-foreground`.
    pub primary_foreground: Color32,
    /// `--secondary`.
    pub secondary: Color32,
    /// `--secondary-foreground`.
    pub secondary_foreground: Color32,
    /// `--muted`.
    pub muted: Color32,
    /// `--muted-foreground`.
    pub muted_foreground: Color32,
    /// `--accent`.
    pub accent: Color32,
    /// `--accent-foreground`.
    pub accent_foreground: Color32,
    /// `--destructive`.
    pub destructive: Color32,
    /// The text on `destructive` (`text-white`).
    pub destructive_foreground: Color32,
    /// A positive outcome. Not a shadcn variable: the Tailwind green
    /// that `destructive` is the red of.
    pub success: Color32,
    /// The text on `success`.
    pub success_foreground: Color32,
    /// A caution: the Tailwind amber.
    pub warning: Color32,
    /// The text on `warning`.
    pub warning_foreground: Color32,
    /// Neutral news: the Tailwind sky.
    pub info: Color32,
    /// The text on `info`.
    pub info_foreground: Color32,
    /// `--border`.
    pub border: Color32,
    /// `--input`.
    pub input: Color32,
    /// `--ring`.
    pub ring: Color32,
    /// The shadcn style these sizes belong to.
    pub style: ShadcnStyle,
    /// `--radius`, in whole points. `rounded-lg` is this, `rounded-md` two
    /// less and `rounded-sm` four less.
    pub radius: u8,
    /// The size of every control's text (`text-sm`, or `text-xs`).
    pub text_size: f32,
    /// The line height that goes with `text_size`.
    pub text_line_height: f32,
    /// The height of buttons, inputs and selects (`h-9`).
    pub control_height: f32,
    /// A button's padding (`px-4 py-2`).
    pub button_padding: Vec2,
    /// An input's and a select's padding (`px-3 py-1`).
    pub input_padding: Vec2,
    /// The gap between a control's children (`gap-2`).
    pub control_gap: f32,
    /// The side of an icon in a button or a segmented cell (`size-4`).
    pub icon_size: f32,
    /// The left and right padding of a toggle.
    pub toggle_padding_x: f32,
    /// The width of the focus ring (`ring-[3px]`).
    pub ring_width: f32,
    /// The corner radius of a checkbox.
    pub checkbox_radius: u8,
    /// The thickness of the slider's track.
    pub slider_track: f32,
    /// The corner radius of the slider's track and thumb; `u8::MAX` is round.
    pub slider_radius: u8,
    /// The padding of a tab list cell.
    pub cell_padding: Vec2,
    /// The padding between a popup and its items.
    pub popup_padding: f32,
    /// The padding of an item in a popup.
    pub item_padding: Vec2,
    /// How much of `primary` a primary button keeps under the pointer.
    pub primary_hover: Opacity,
    /// A secondary button under the pointer.
    pub secondary_hover: SecondaryHover,
    /// The surface of a ghost or outline button under the pointer.
    pub hover_surface: HoverSurface,
    /// How the destructive button is filled.
    pub destructive_look: DestructiveLook,
    /// Whether controls are shadowed.
    pub elevation: Elevation,
    /// A switch's track (`h-[1.15rem] w-8`).
    pub switch_track: Vec2,
    /// A switch's thumb diameter at the default track (`size-4`).
    pub switch_thumb: f32,
    /// The slider thumb's diameter: shadcn's `size-4`.
    pub slider_thumb: f32,
    /// The track of a checked switch.
    pub switch_track_on: Color32,
    /// The track of an unchecked switch.
    pub switch_track_off: Color32,
    /// The thumb of a checked switch.
    pub switch_thumb_on: Color32,
    /// The thumb of an unchecked switch.
    pub switch_thumb_off: Color32,
    /// The padding of a tab list or toggle group around its cells (`p-[3px]`).
    pub list_padding: f32,
    /// `shadow-xs`.
    pub shadow_xs: Shadow,
    /// `shadow-sm`.
    pub shadow_sm: Shadow,
    /// `shadow-md`, without its negative spread.
    pub shadow_md: Shadow,
}

impl Tokens {
    /// The neutral light palette.
    pub fn light() -> Self {
        let input = rgb(0xe5e5e5);
        let background = rgb(0xffffff);
        let primary = rgb(0x171717);
        Self {
            background,
            foreground: rgb(0x0a0a0a),
            card: background,
            card_foreground: rgb(0x0a0a0a),
            popover: background,
            popover_foreground: rgb(0x0a0a0a),
            primary,
            primary_foreground: rgb(0xfafafa),
            secondary: rgb(0xf5f5f5),
            secondary_foreground: primary,
            muted: rgb(0xf5f5f5),
            muted_foreground: rgb(0x737373),
            accent: rgb(0xf5f5f5),
            accent_foreground: primary,
            destructive: rgb(0xe7000b),
            destructive_foreground: Color32::WHITE,
            success: rgb(0x00a63e),
            success_foreground: rgb(0xfafafa),
            warning: rgb(0xfe9a00),
            warning_foreground: rgb(0x461901),
            info: rgb(0x0084d1),
            info_foreground: rgb(0xfafafa),
            border: input,
            input,
            ring: rgb(0xa1a1a1),
            switch_track_on: primary,
            switch_track_off: input,
            switch_thumb_on: background,
            switch_thumb_off: background,
            ..Self::sizes()
        }
    }

    /// The neutral dark palette.
    pub fn dark() -> Self {
        let primary = rgb(0xe5e5e5);
        let primary_foreground = rgb(0x171717);
        let foreground = rgb(0xfafafa);
        let input = Color32::from_white_alpha(38);
        Self {
            background: rgb(0x0a0a0a),
            foreground,
            card: primary_foreground,
            card_foreground: foreground,
            popover: primary_foreground,
            popover_foreground: foreground,
            primary,
            primary_foreground,
            secondary: rgb(0x262626),
            secondary_foreground: foreground,
            muted: rgb(0x262626),
            muted_foreground: rgb(0xa1a1a1),
            accent: rgb(0x262626),
            accent_foreground: foreground,
            destructive: rgb(0xff6467),
            destructive_foreground: Color32::WHITE,
            success: rgb(0x05df72),
            success_foreground: rgb(0x032e15),
            warning: rgb(0xffb900),
            warning_foreground: rgb(0x461901),
            info: rgb(0x00bcff),
            info_foreground: rgb(0x052f4a),
            border: Color32::from_white_alpha(26),
            input,
            ring: rgb(0x737373),
            switch_track_on: primary,
            switch_track_off: input.gamma_multiply(0.8),
            switch_thumb_on: primary_foreground,
            switch_thumb_off: foreground,
            ..Self::sizes()
        }
    }

    /// The tokens for `theme`.
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::light(),
            Theme::Dark => Self::dark(),
        }
    }

    /// The values the two themes share; its colours are placeholders. The
    /// sizes are the default style's.
    fn sizes() -> Self {
        let none = Color32::TRANSPARENT;
        let mut tokens = Self {
            background: none,
            foreground: none,
            card: none,
            card_foreground: none,
            popover: none,
            popover_foreground: none,
            primary: none,
            primary_foreground: none,
            secondary: none,
            secondary_foreground: none,
            muted: none,
            muted_foreground: none,
            accent: none,
            accent_foreground: none,
            destructive: none,
            destructive_foreground: none,
            success: none,
            success_foreground: none,
            warning: none,
            warning_foreground: none,
            info: none,
            info_foreground: none,
            border: none,
            input: none,
            ring: none,
            style: ShadcnStyle::Vega,
            radius: 0,
            text_size: 0.0,
            text_line_height: 0.0,
            control_height: 0.0,
            button_padding: Vec2::ZERO,
            input_padding: Vec2::ZERO,
            control_gap: 0.0,
            icon_size: 0.0,
            toggle_padding_x: 0.0,
            ring_width: 0.0,
            checkbox_radius: 0,
            slider_track: 0.0,
            slider_radius: 0,
            cell_padding: Vec2::ZERO,
            popup_padding: 0.0,
            item_padding: Vec2::ZERO,
            primary_hover: Opacity::OPAQUE,
            secondary_hover: SecondaryHover::Fade(Opacity::OPAQUE),
            hover_surface: HoverSurface::Accent,
            destructive_look: DestructiveLook::Solid,
            elevation: Elevation::Raised,
            switch_track: vec2(32.0, 18.4),
            switch_thumb: 16.0,
            slider_thumb: 0.0,
            switch_track_on: none,
            switch_track_off: none,
            switch_thumb_on: none,
            switch_thumb_off: none,
            list_padding: 0.0,
            shadow_xs: Shadow {
                offset: [0, 1],
                blur: 2,
                spread: 0,
                color: Color32::from_black_alpha(13),
            },
            shadow_sm: Shadow {
                offset: [0, 1],
                blur: 3,
                spread: 0,
                color: Color32::from_black_alpha(26),
            },
            shadow_md: Shadow {
                offset: [0, 4],
                blur: 6,
                spread: 0,
                color: Color32::from_black_alpha(26),
            },
        };
        ShadcnStyle::Vega.shape(&mut tokens);
        tokens
    }

    /// These tokens with the sizes of `style`; the colours are kept.
    #[must_use]
    pub fn in_style(mut self, style: ShadcnStyle) -> Self {
        style.shape(&mut self);
        self
    }

    /// `rounded-md`: the radius of buttons, inputs and selects.
    pub fn radius_md(&self) -> u8 {
        self.radius.saturating_sub(2)
    }

    /// `rounded-sm`.
    pub fn radius_sm(&self) -> u8 {
        self.radius.saturating_sub(4)
    }

    /// `accent/50`: the hover fill of a row on the page background, a step
    /// under [`accent`](Self::accent), which marks the selected row.
    pub fn accent_50(&self) -> Color32 {
        self.accent.gamma_multiply(0.5)
    }

    /// `muted/50`: the hover fill of a table row; selected is `muted`.
    pub fn muted_50(&self) -> Color32 {
        self.muted.gamma_multiply(0.5)
    }

    /// `secondary/80`: a secondary button under the pointer.
    pub fn secondary_80(&self) -> Color32 {
        self.secondary.gamma_multiply(0.8)
    }

    /// `primary/90`: a primary button under the pointer.
    pub fn primary_90(&self) -> Color32 {
        self.primary.gamma_multiply(0.9)
    }

    /// `destructive/90`: a destructive button under the pointer.
    pub fn destructive_90(&self) -> Color32 {
        self.destructive.gamma_multiply(0.9)
    }

    /// `destructive/60`: a destructive button at rest in the dark theme.
    pub fn destructive_60(&self) -> Color32 {
        self.destructive.gamma_multiply(0.6)
    }

    /// `input/30`: the fill of inputs, outline buttons and unchecked boxes in
    /// the dark theme.
    pub fn input_30(&self) -> Color32 {
        self.input.gamma_multiply(0.3)
    }

    /// `input/50`: an outline button under the pointer in the dark theme.
    pub fn input_50(&self) -> Color32 {
        self.input.gamma_multiply(0.5)
    }

    /// These tokens re-tinted with `theme_color`'s `primary`, `primary-foreground`
    /// and `ring` for `theme`, which must be the theme they were made for.
    #[must_use]
    pub fn with_theme_color(self, theme_color: ThemeColor, theme: Theme) -> Self {
        let Hues {
            primary,
            primary_foreground,
            ring,
        } = theme_color.hues(theme);
        let (switch_thumb_on, switch_track_on) = match theme {
            Theme::Light => (self.switch_thumb_on, primary),
            Theme::Dark => (primary_foreground, primary),
        };
        Self {
            primary,
            primary_foreground,
            ring,
            switch_track_on,
            switch_thumb_on,
            ..self
        }
    }

    /// `shadow-xs` on a control, or none in a flat style.
    pub fn control_shadow(&self) -> Shadow {
        self.elevation
            .shadow(self.shadow_xs)
            .unwrap_or(Shadow::NONE)
    }

    /// The text role of controls: the token's size on its line height.
    pub fn text_role(&self) -> TextSpec {
        TextSpec {
            size: self.text_size,
            line_height: self.text_line_height,
            family: egui::FontFamily::Proportional,
        }
    }

    /// The thumb of a switch whose track is `track`: the default thumb's
    /// proportion of the track's height.
    pub fn switch_thumb_for(&self, track: Vec2) -> f32 {
        track.y * self.switch_thumb / self.switch_track.y
    }
}
