use eggshell_ui::contract::{Radius, Spacing, TextScale, TextSize, TextStep, corner};
use eggshell_ui::{TextSpec, Weight};
use egui::{Color32, Shadow, Theme, Vec2, vec2};

use crate::palette::{Palette, Scheme};
use crate::styles::{Elevation, HoverSurface, ShadcnStyle};
use crate::{ContrastPair, FontSet};

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
    /// `--chart-1`.
    pub chart_1: Color32,
    /// `--chart-2`.
    pub chart_2: Color32,
    /// `--chart-3`.
    pub chart_3: Color32,
    /// `--chart-4`.
    pub chart_4: Color32,
    /// `--chart-5`.
    pub chart_5: Color32,
    /// The shadcn style these sizes belong to.
    pub style: ShadcnStyle,
    /// TODO(prose): `--spacing`, the points of one spacing unit.
    pub spacing: Spacing,
    /// `--radius` and the steps derived from it: `rounded-lg` is the base,
    /// `rounded-md` two less and `rounded-sm` four less.
    pub radius: Radius,
    /// TODO(prose): Tailwind's text sizes, `text-xs` to `text-lg`.
    pub text: TextScale,
    /// TODO(prose): the step of `text` every control's text is: `text-sm`, or `text-xs`.
    pub control_text: TextStep,
    /// TODO(prose): the height of buttons, inputs and selects in spacing units (`h-9`).
    pub control_units: f32,
    /// A button's padding (`px-4 py-2`).
    pub button_padding: Vec2,
    /// An input's and a select's padding (`px-3 py-1`).
    pub input_padding: Vec2,
    /// TODO(prose): the inline padding of eggshell's own smaller fields, which
    /// shadcn does not name, in spacing units: a `sm` field's and a stepper's
    /// value's, and one unit more than an `xs` field's.
    pub own_input_padding_x: f32,
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
    /// The width of the ring around the slider thumb under the pointer and
    /// with focus (`hover:ring-4`).
    pub slider_halo: f32,
    /// The padding of a tab list cell.
    pub cell_padding: Vec2,
    /// The padding between a popup and its items.
    pub popup_padding: f32,
    /// The padding of an item in a popup.
    pub item_padding: Vec2,
    /// The surface of a ghost or outline button under the pointer.
    pub hover_surface: HoverSurface,
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
    /// `shadow-lg`, without its negative spread.
    pub shadow_lg: Shadow,
    /// `--font-sans`: the font [`Shadcn::apply`](crate::Shadcn::apply)
    /// registers, or `None` to leave egui's fonts alone.
    pub font: Option<FontSet>,
    /// The weight of buttons, toggles, tabs and key chips (`font-medium`).
    pub control_weight: Weight,
    /// TODO(prose): the pairs under AA contrast the application accepts:
    /// [`Shadcn::apply`](crate::Shadcn::apply) warns about the others in a
    /// debug build.
    pub accepted_contrast: Vec<ContrastPair>,
    /// The weight of a label (`.cn-label`): beside a checkbox, radio button
    /// or switch, and over a group of menu items.
    pub label_weight: Weight,
}

impl Tokens {
    /// The neutral light palette.
    pub fn light() -> Self {
        Self {
            destructive: rgb(0xe7000b),
            destructive_foreground: Color32::WHITE,
            success: rgb(0x008236),
            success_foreground: rgb(0xfafafa),
            warning: rgb(0xbb4d00),
            warning_foreground: rgb(0xfafafa),
            info: rgb(0x0069a8),
            info_foreground: rgb(0xfafafa),
            ..Self::sizes()
        }
        .with_palette(Palette::new(), Theme::Light)
    }

    /// The neutral dark palette.
    pub fn dark() -> Self {
        Self {
            destructive: rgb(0xff6467),
            destructive_foreground: Color32::WHITE,
            success: rgb(0x05df72),
            success_foreground: rgb(0x032e15),
            warning: rgb(0xffb900),
            warning_foreground: rgb(0x461901),
            info: rgb(0x00bcff),
            info_foreground: rgb(0x052f4a),
            ..Self::sizes()
        }
        .with_palette(Palette::new(), Theme::Dark)
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
            chart_1: none,
            chart_2: none,
            chart_3: none,
            chart_4: none,
            chart_5: none,
            style: ShadcnStyle::Vega,
            spacing: Spacing::DEFAULT,
            radius: Radius::new(0.0),
            text: TextScale::default(),
            control_text: TextStep::Sm,
            control_units: 0.0,
            button_padding: Vec2::ZERO,
            input_padding: Vec2::ZERO,
            own_input_padding_x: 0.0,
            control_gap: 0.0,
            icon_size: 0.0,
            toggle_padding_x: 0.0,
            ring_width: 0.0,
            checkbox_radius: 0,
            slider_track: 0.0,
            slider_radius: 0,
            slider_halo: 0.0,
            cell_padding: Vec2::ZERO,
            popup_padding: 0.0,
            item_padding: Vec2::ZERO,
            hover_surface: HoverSurface::Accent,
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
            shadow_lg: Shadow {
                offset: [0, 10],
                blur: 15,
                spread: 0,
                color: Color32::from_black_alpha(26),
            },
            font: FontSet::bundled(),
            control_weight: Weight::Normal,
            label_weight: Weight::Normal,
            accepted_contrast: Vec::new(),
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

    /// TODO(prose): the height of buttons, inputs and selects in points at the tokens' spacing.
    pub fn control_height(&self) -> f32 {
        self.spacing.points(self.control_units)
    }

    /// `rounded-md`: the radius of buttons, inputs and selects.
    pub fn radius_md(&self) -> u8 {
        corner(self.radius.md())
    }

    /// `rounded-sm`.
    pub fn radius_sm(&self) -> u8 {
        corner(self.radius.sm())
    }

    /// `rounded-lg`: the base radius.
    pub fn radius_lg(&self) -> u8 {
        corner(self.radius.lg())
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

    /// `background/20`: a key chip on a tooltip in the light theme.
    pub fn background_20(&self) -> Color32 {
        self.background.gamma_multiply(0.2)
    }

    /// `background/10`: a key chip on a tooltip in the dark theme.
    pub fn background_10(&self) -> Color32 {
        self.background.gamma_multiply(0.1)
    }

    /// `input/30`: the fill of inputs, outline buttons and unchecked boxes in
    /// the dark theme.
    pub fn input_30(&self) -> Color32 {
        self.input.gamma_multiply(0.3)
    }

    /// TODO(prose): the colours of each tone: the status tokens.
    pub fn tones(&self) -> eggshell_ui::contract::Tones {
        let on = |tone, foreground| eggshell_ui::contract::ToneColors { tone, foreground };
        eggshell_ui::contract::Tones {
            info: on(self.info, self.info_foreground),
            success: on(self.success, self.success_foreground),
            warning: on(self.warning, self.warning_foreground),
        }
    }

    /// `input/50`: an outline button under the pointer in the dark theme.
    pub fn input_50(&self) -> Color32 {
        self.input.gamma_multiply(0.5)
    }

    /// TODO(prose): these tokens with `palette`'s colours for `theme`, which
    /// must be the theme they were made for: every colour shadcn's base and
    /// theme colours set, and the switch's, which follow them. The semantic
    /// colours (`destructive`, `success`, `warning`, `info`) are kept.
    #[must_use]
    pub fn with_palette(self, palette: Palette, theme: Theme) -> Self {
        let Scheme {
            background,
            foreground,
            card,
            card_foreground,
            popover,
            popover_foreground,
            primary,
            primary_foreground,
            secondary,
            secondary_foreground,
            muted,
            muted_foreground,
            accent,
            accent_foreground,
            border,
            input,
            ring,
            charts: [chart_1, chart_2, chart_3, chart_4, chart_5],
        } = palette.scheme(theme);
        let (switch_track_off, switch_thumb_on, switch_thumb_off) = match theme {
            Theme::Light => (input, background, background),
            Theme::Dark => (input.gamma_multiply(0.8), primary_foreground, foreground),
        };
        Self {
            background,
            foreground,
            card,
            card_foreground,
            popover,
            popover_foreground,
            primary,
            primary_foreground,
            secondary,
            secondary_foreground,
            muted,
            muted_foreground,
            accent,
            accent_foreground,
            border,
            input,
            ring,
            chart_1,
            chart_2,
            chart_3,
            chart_4,
            chart_5,
            switch_track_on: primary,
            switch_track_off,
            switch_thumb_on,
            switch_thumb_off,
            ..self
        }
    }

    /// `shadow-xs` on a control, or none in a flat style.
    pub fn control_shadow(&self) -> Shadow {
        self.elevation
            .shadow(self.shadow_xs)
            .unwrap_or(Shadow::NONE)
    }

    /// The text role of controls: the token's size on its line height, at
    /// `weight`.
    pub fn text_role_at(&self, weight: Weight) -> TextSpec {
        TextSpec {
            weight,
            ..self.text_role()
        }
    }

    /// TODO(prose): the size and line height of every control's text.
    pub fn control_text_size(&self) -> TextSize {
        self.text.step(self.control_text)
    }

    /// The text role of controls: the token's size on its line height.
    pub fn text_role(&self) -> TextSpec {
        let TextSize { size, line_height } = self.control_text_size();
        TextSpec {
            size,
            line_height,
            family: egui::FontFamily::Proportional,
            weight: Weight::Normal,
        }
    }

    /// The thumb of a switch whose track is `track`: the default thumb's
    /// proportion of the track's height.
    pub fn switch_thumb_for(&self, track: Vec2) -> f32 {
        track.y * self.switch_thumb / self.switch_track.y
    }
}
