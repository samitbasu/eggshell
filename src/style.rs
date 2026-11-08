//! Opt-in presets that give egui's widgets and the library's primitives a
//! coherent look.
//!
//! [`Shadcn`] is the shadcn/ui "new-york" style with the neutral base colour,
//! light and dark. Nothing here runs unless the application calls
//! [`Shadcn::apply`]: the library never changes egui's style on its own.
//!
//! Applying a preset does two things. It maps the [`Tokens`] onto the egui
//! [`Style`] of the preset's theme (widget fills, strokes, radii, selection,
//! spacing), which stock widgets and the size estimates in
//! [`widget`](crate::widget) follow automatically. It also remembers the
//! tokens in the context, for the few things egui's style cannot express: the
//! switch's colours and the input's padding, which egui shares with buttons.
//!
//! What egui cannot express and the preset therefore approximates: the focus
//! ring is the 1 pt border egui draws around a focused text edit (no 3 pt
//! halo at half opacity); `shadow-xs` on inputs is not drawn on stock
//! widgets; font weights (`font-medium`) need a registered family; and
//! egui takes the focus border and the colour of selected text from the one
//! `selection.stroke`, which is `foreground`, so the selection fill is
//! `primary/30` to keep that text readable.
//!
//! # Hovered and selected
//!
//! shadcn's components pair the two as follows, and the helpers on
//! [`Tokens`] name the pairs:
//!
//! | Component | Hovered | Selected |
//! |---|---|---|
//! | List or menu row on the page | `accent/50` ([`Tokens::accent_50`]) | `accent` with `accent-foreground` |
//! | Table row | `muted/50` ([`Tokens::muted_50`]) | `muted` |
//! | Ghost and outline button | `accent` with `accent-foreground` (dark: `accent/50`, `input/50`) | |
//! | Secondary, primary, destructive button | `secondary/80`, `primary/90`, `destructive/90` | |
//!
//! In the neutral palette `muted`, `accent` and `secondary` are one colour,
//! so a hover on a `muted` surface needs the half-strength variant of the
//! selected colour, not a different token.

use egui::{
    Color32, Context, CornerRadius, Id, Margin, Shadow, Stroke, Style, TextStyle, Theme, Vec2,
    style::{Visuals, WidgetVisuals, Widgets},
    vec2,
};

use crate::prim::segmented::Cell;
use crate::{TextSpec, Tw, tw};

/// A colour from `0xRRGGBB`.
const fn rgb(hex: u32) -> Color32 {
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
    /// `--radius`, in whole points. `rounded-lg` is this, `rounded-md` two
    /// less and `rounded-sm` four less.
    pub radius: u8,
    /// The size of `text-sm`, the size of every control's text.
    pub text_size: f32,
    /// The height of buttons, inputs and selects (`h-9`).
    pub control_height: f32,
    /// A button's padding (`px-4 py-2`).
    pub button_padding: Vec2,
    /// An input's and a select's padding (`px-3 py-1`).
    pub input_padding: Vec2,
    /// A switch's track (`h-[1.15rem] w-8`).
    pub switch_track: Vec2,
    /// A switch's thumb diameter at the default track (`size-4`).
    pub switch_thumb: f32,
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

    /// The values the two themes share; its colours are placeholders.
    fn sizes() -> Self {
        let none = Color32::TRANSPARENT;
        Self {
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
            radius: 10,
            text_size: 14.0,
            control_height: 36.0,
            button_padding: vec2(16.0, 8.0),
            input_padding: vec2(12.0, 4.0),
            switch_track: vec2(32.0, 18.4),
            switch_thumb: 16.0,
            switch_track_on: none,
            switch_track_off: none,
            switch_thumb_on: none,
            switch_thumb_off: none,
            list_padding: 3.0,
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
        }
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

    /// These tokens re-tinted with `accent`'s `primary`, `primary-foreground`
    /// and `ring` for `theme`, which must be the theme they were made for.
    #[must_use]
    pub fn with_accent(self, accent: Accent, theme: Theme) -> Self {
        let Hues {
            primary,
            primary_foreground,
            ring,
        } = accent.hues(theme);
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

    /// The text role of `text-sm`: the token's size on a 20 point line.
    pub fn text_role(&self) -> TextSpec {
        TextSpec {
            size: self.text_size,
            line_height: 20.0,
            family: egui::FontFamily::Proportional,
        }
    }

    /// The thumb of a switch whose track is `track`: the default thumb's
    /// proportion of the track's height.
    pub fn switch_thumb_for(&self, track: Vec2) -> f32 {
        track.y * self.switch_thumb / self.switch_track.y
    }
}

/// One of shadcn's theme colours, which re-tints `primary`,
/// `primary-foreground` and `ring`. Neutral is shadcn's default.
///
/// In the dark theme an accent departs from shadcn's registry, which uses
/// its 700/800 shade for `primary`: that is dim on near-black, so `primary`
/// is the 500 shade (the `ring`) with a black `primary-foreground`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Accent {
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

impl Accent {
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
    fn hues(self, theme: Theme) -> Hues {
        let (primary, foreground, ring) = match (self, theme) {
            (Self::Neutral, Theme::Light) => (0x171717, 0xfafafa, 0xa1a1a1),
            (Self::Neutral, Theme::Dark) => (0xe5e5e5, 0x171717, 0x737373),
            (Self::Blue, Theme::Light) => (0x1447e6, 0xeff6ff, 0x2b7fff),
            (Self::Blue, Theme::Dark) => (0x2b7fff, BRIGHT_ON, 0x2b7fff),
            (Self::Violet, Theme::Light) => (0x7008e7, 0xf5f3ff, 0x8e51ff),
            (Self::Violet, Theme::Dark) => (0x8e51ff, BRIGHT_ON, 0x8e51ff),
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

struct Hues {
    primary: Color32,
    primary_foreground: Color32,
    ring: Color32,
}

/// The shadcn/ui preset: [`Tokens`] for one egui theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadcn {
    /// The egui theme whose style [`apply`](Self::apply) sets.
    pub theme: Theme,
    /// The values; override any of them before applying.
    pub tokens: Tokens,
}

impl Shadcn {
    /// The light preset, for egui's light theme.
    pub fn light() -> Self {
        Self::for_theme(Theme::Light)
    }

    /// The dark preset, for egui's dark theme.
    pub fn dark() -> Self {
        Self::for_theme(Theme::Dark)
    }

    /// The preset that goes with `theme`.
    pub fn for_theme(theme: Theme) -> Self {
        Self {
            theme,
            tokens: Tokens::for_theme(theme),
        }
    }

    /// This preset with `accent`'s colours for `primary`,
    /// `primary-foreground` and `ring`.
    #[must_use]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.tokens = self.tokens.with_accent(accent, self.theme);
        self
    }

    /// The preset applied to `ctx` for its current theme, if one was.
    pub fn applied(ctx: &Context) -> Option<Self> {
        Some(Self {
            theme: ctx.theme(),
            tokens: tokens(ctx)?,
        })
    }

    /// The preset applied to `ctx` for its current theme, or the default
    /// preset for that theme if none was applied.
    pub fn of(ctx: &Context) -> Self {
        Self::applied(ctx).unwrap_or_else(|| Self::for_theme(ctx.theme()))
    }

    /// Applies the light preset to egui's light theme and the dark one to its
    /// dark theme, so that egui's own theme switching picks the right one.
    pub fn apply_both(ctx: &Context) {
        Self::light().apply(ctx);
        Self::dark().apply(ctx);
    }

    /// Applies both themes with `accent`, unless the context's current theme
    /// already has exactly that preset, so that an application can call it
    /// every frame.
    pub fn apply_both_with(ctx: &Context, accent: Accent) {
        let current = Self::for_theme(ctx.theme()).accent(accent);
        if Self::applied(ctx).as_ref() != Some(&current) {
            Self::light().accent(accent).apply(ctx);
            Self::dark().accent(accent).apply(ctx);
        }
    }

    /// Sets the egui style of this preset's theme from the tokens and
    /// remembers them for the library's primitives. Safe to call every frame
    /// but only needed again when the tokens change.
    pub fn apply(&self, ctx: &Context) {
        let tokens = self.tokens.clone();
        ctx.style_mut_of(self.theme, |style| tokens.set_style(self.theme, style));
        ctx.data_mut(|data| data.insert_temp(tokens_id(self.theme), self.tokens.clone()));
    }

    /// The classes of a shadcn tab list or toggle group over [`segmented`]
    /// cells: one row as wide as its cells (`w-fit`).
    ///
    /// [`segmented`]: crate::prim::segmented::segmented
    pub fn segmented(&self) -> SegmentedClasses {
        SegmentedClasses {
            theme: self.theme,
            tokens: self.tokens.clone(),
            rows: Rows::One,
        }
    }
}

fn tokens_id(theme: Theme) -> Id {
    Id::NULL.with(("bullseye-shadcn-tokens", theme))
}

/// The tokens a preset was applied with for the context's current theme, if
/// one was.
pub fn tokens(ctx: &Context) -> Option<Tokens> {
    ctx.data(|data| data.get_temp::<Tokens>(tokens_id(ctx.theme())))
}

/// Whether a track keeps its cells on one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rows {
    One,
    Wrapping,
}

/// shadcn's `Button` variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ButtonVariant {
    /// `default`: the `primary` fill.
    Primary,
    /// `secondary`: the `secondary` fill. The look every stock button has.
    #[default]
    Secondary,
    /// `destructive`.
    Destructive,
    /// `outline`: bordered on the background.
    Outline,
    /// `ghost`: no fill until hovered.
    Ghost,
}

/// Whether a checkbox is ticked or a radio button chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Off,
    On,
}

impl From<bool> for Mark {
    fn from(on: bool) -> Self {
        if on { Self::On } else { Self::Off }
    }
}

/// What egui reads from a widget's visuals for one state of a button, a
/// checkbox or a radio button.
#[derive(Clone, Copy)]
struct Look {
    fill: Color32,
    stroke: Color32,
    text: Color32,
}

impl Look {
    fn apply(self, visuals: &mut WidgetVisuals) {
        visuals.weak_bg_fill = self.fill;
        visuals.bg_fill = self.fill;
        visuals.bg_stroke.color = self.stroke;
        visuals.fg_stroke.color = self.text;
    }
}

/// Gives the states a widget can be in `rest` and `hover` looks. A disabled
/// widget is drawn in egui's noninteractive state, at half opacity.
fn set_looks(widgets: &mut Widgets, rest: Look, hover: Look) {
    for visuals in [&mut widgets.noninteractive, &mut widgets.inactive] {
        rest.apply(visuals);
    }
    for visuals in [&mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        hover.apply(visuals);
    }
}

impl Shadcn {
    /// Restyles `visuals`, those of the `Ui` a button is added to, for a
    /// button of `variant`. Strokes keep their width and widgets their
    /// expansion, so the size does not depend on the variant. A variant
    /// other than secondary has its own text colours, so it drops the `Ui`'s
    /// override of the text colour.
    pub fn style_button(&self, variant: ButtonVariant, visuals: &mut Visuals) {
        let t = &self.tokens;
        let dark = self.theme == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let filled = |fill, hover, text| {
            (
                Look {
                    fill,
                    stroke: clear,
                    text,
                },
                Look {
                    fill: hover,
                    stroke: clear,
                    text,
                },
            )
        };
        let (rest, hover) = match variant {
            ButtonVariant::Secondary => return,
            ButtonVariant::Primary => filled(t.primary, t.primary_90(), t.primary_foreground),
            ButtonVariant::Destructive => filled(
                if dark {
                    t.destructive_60()
                } else {
                    t.destructive
                },
                t.destructive_90(),
                t.destructive_foreground,
            ),
            ButtonVariant::Outline => {
                let rest = Look {
                    fill: if dark { t.input_30() } else { t.background },
                    stroke: t.input,
                    text: t.foreground,
                };
                let hover = Look {
                    fill: if dark { t.input_50() } else { t.accent },
                    text: if dark {
                        t.foreground
                    } else {
                        t.accent_foreground
                    },
                    ..rest
                };
                (rest, hover)
            }
            ButtonVariant::Ghost => {
                let rest = Look {
                    fill: clear,
                    stroke: clear,
                    text: t.foreground,
                };
                let hover = Look {
                    fill: if dark { t.accent_50() } else { t.accent },
                    text: t.accent_foreground,
                    ..rest
                };
                (rest, hover)
            }
        };
        visuals.override_text_color = None;
        set_looks(&mut visuals.widgets, rest, hover);
    }

    /// Restyles `widgets` for a checkbox: the background in an `input`
    /// border, `primary` with a `primary-foreground` tick when ticked.
    pub(crate) fn style_checkbox(&self, mark: Mark, widgets: &mut Widgets) {
        let t = &self.tokens;
        let look = match mark {
            Mark::Off => Look {
                fill: self.input_fill(),
                stroke: t.input,
                text: t.foreground,
            },
            Mark::On => Look {
                fill: t.primary,
                stroke: t.primary,
                text: t.primary_foreground,
            },
        };
        set_looks(widgets, look, look);
    }

    /// Restyles `widgets` for a radio button: the background in an `input`
    /// border, with a `primary` dot when chosen.
    pub(crate) fn style_radio(&self, widgets: &mut Widgets) {
        let t = &self.tokens;
        let look = Look {
            fill: self.input_fill(),
            stroke: t.input,
            text: t.primary,
        };
        set_looks(widgets, look, look);
    }

    /// The fill of an input-like box: `bg-background`, or `dark:bg-input/30`.
    fn input_fill(&self) -> Color32 {
        match self.theme {
            Theme::Light => self.tokens.background,
            Theme::Dark => self.tokens.input_30(),
        }
    }

    /// The colour of a label beside a checkbox or a radio button.
    pub(crate) fn label_color(&self) -> Color32 {
        self.tokens.foreground
    }
}

/// The classes of a shadcn tab list or toggle group.
///
/// Pass [`container`](Self::container) and [`cell`](Self::cell) to
/// [`segmented`](crate::prim::segmented::segmented): a muted track with
/// padding and `rounded-lg`, in which the pressed cell is raised with the
/// background fill and `shadow-sm`. The track is as wide as its cells; give
/// it `w_full` to stretch it, or use [`wrapping`](Self::wrapping) for many
/// options.
///
/// ```html
/// <div class="flex items-center w-fit h-9 p-[3px] rounded-lg bg-muted">
///   <span class="grow flex items-center justify-center px-2 py-1 rounded-md text-muted-foreground">…</span>
/// </div>
/// ```
#[derive(Clone, Debug)]
pub struct SegmentedClasses {
    theme: Theme,
    tokens: Tokens,
    rows: Rows,
}

impl SegmentedClasses {
    /// These classes for a track that wraps its cells onto more rows when it
    /// is narrower than they are, and takes its height from the rows.
    #[must_use]
    pub fn wrapping(mut self) -> Self {
        self.rows = Rows::Wrapping;
        self
    }

    /// The track.
    pub fn container(&self) -> Tw {
        let t = &self.tokens;
        let track = tw()
            .flex()
            .items_center()
            .w_fit()
            .p(t.list_padding / 4.0)
            .rounded_px(f32::from(t.radius))
            .bg(t.muted);
        match self.rows {
            Rows::One => track.h_px(t.control_height),
            Rows::Wrapping => track.flex_wrap().h_auto().gap(0.5),
        }
    }

    /// One cell.
    pub fn cell(&self, cell: Cell) -> Tw {
        let t = &self.tokens;
        let base = tw()
            .flex()
            .grow()
            .items_center()
            .justify_center()
            .px(2.0)
            .py(1.0)
            .rounded_px(f32::from(t.radius_md()))
            .role(t.text_role());
        let base = match self.rows {
            Rows::One => base.h_full(),
            Rows::Wrapping => base.h_auto(),
        };
        match (cell, self.theme) {
            (Cell::Released, _) => base.text(t.muted_foreground),
            (Cell::Pressed, Theme::Light) => {
                base.text(t.foreground).bg(t.background).shadow(t.shadow_sm)
            }
            (Cell::Pressed, Theme::Dark) => base
                .text(t.foreground)
                .bg(t.input_30())
                .border(1.0)
                .border_color(t.input)
                .shadow(t.shadow_sm),
        }
    }
}

impl Tokens {
    fn widget(
        &self,
        weak_bg_fill: Color32,
        bg_fill: Color32,
        bg_stroke: Stroke,
        fg: Color32,
    ) -> WidgetVisuals {
        WidgetVisuals {
            weak_bg_fill,
            bg_fill,
            bg_stroke,
            corner_radius: CornerRadius::same(self.radius_md()),
            fg_stroke: Stroke::new(1.0, fg),
            expansion: 0.0,
        }
    }

    fn set_style(&self, theme: Theme, style: &mut Style) {
        let t = self;
        let v = &mut style.visuals;
        v.dark_mode = theme == Theme::Dark;
        v.override_text_color = None;
        v.widgets.noninteractive = t.widget(
            t.background,
            t.background,
            Stroke::new(1.0, t.border),
            t.foreground,
        );
        v.widgets.inactive = t.widget(
            t.secondary,
            t.input,
            Stroke::new(1.0, t.input),
            t.secondary_foreground,
        );
        v.widgets.hovered = t.widget(
            t.secondary.gamma_multiply(0.8),
            t.input,
            Stroke::new(1.0, t.input),
            t.secondary_foreground,
        );
        v.widgets.active = t.widget(
            t.accent,
            t.input,
            Stroke::new(1.0, t.ring),
            t.accent_foreground,
        );
        v.widgets.open = t.widget(
            t.accent,
            t.input,
            Stroke::new(1.0, t.input),
            t.accent_foreground,
        );
        v.selection.bg_fill = t.primary.gamma_multiply(0.3);
        v.selection.stroke = Stroke::new(1.0, t.foreground);
        v.hyperlink_color = t.ring;
        v.faint_bg_color = t.muted;
        v.extreme_bg_color = t.background;
        v.text_edit_bg_color = Some(match theme {
            Theme::Light => t.background,
            Theme::Dark => t.input_30(),
        });
        v.code_bg_color = t.muted;
        v.error_fg_color = t.destructive;
        v.panel_fill = t.background;
        v.window_fill = t.popover;
        v.window_stroke = Stroke::new(1.0, t.border);
        v.window_corner_radius = CornerRadius::same(t.radius);
        v.menu_corner_radius = CornerRadius::same(t.radius_md());
        v.text_cursor.stroke = Stroke::new(2.0, t.foreground);
        v.disabled_alpha = 0.5;
        let overlay = Shadow {
            offset: [0, 4],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(26),
        };
        v.window_shadow = overlay;
        v.popup_shadow = overlay;

        let s = &mut style.spacing;
        s.interact_size.y = t.control_height;
        s.button_padding = t.button_padding;
        s.item_spacing = vec2(8.0, 8.0);
        s.icon_width = t.switch_thumb;
        s.icon_width_inner = t.switch_thumb * 0.625;
        s.icon_spacing = 8.0;
        s.combo_height = 400.0;
        s.window_margin = Margin::same(16);
        s.menu_margin = Margin::same(4);

        for text_style in [TextStyle::Body, TextStyle::Button, TextStyle::Monospace] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                font.size = t.text_size;
            }
        }
    }
}

/// How the library draws inputs and selects under the preset.
pub(crate) fn input_padding(ctx: &Context) -> Option<Vec2> {
    tokens(ctx).map(|t| t.input_padding)
}
