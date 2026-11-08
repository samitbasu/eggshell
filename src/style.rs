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
//! ring is the 1 pt `ring`-coloured border egui draws around a focused text
//! edit (no 3 pt halo at half opacity); `shadow-xs` on inputs is not drawn on
//! stock widgets; font weights (`font-medium`) need a registered family; and
//! selected text is drawn in the ring colour, because egui takes both from
//! `selection.stroke`.

use egui::{
    Color32, Context, CornerRadius, Id, Margin, Shadow, Stroke, Style, TextStyle, Theme, Vec2,
    style::WidgetVisuals, vec2,
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

    /// Applies the light preset to egui's light theme and the dark one to its
    /// dark theme, so that egui's own theme switching picks the right one.
    pub fn apply_both(ctx: &Context) {
        Self::light().apply(ctx);
        Self::dark().apply(ctx);
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
    /// cells.
    ///
    /// [`segmented`]: crate::prim::segmented::segmented
    pub fn segmented(&self) -> SegmentedClasses {
        SegmentedClasses {
            theme: self.theme,
            tokens: self.tokens.clone(),
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

/// The classes of a shadcn tab list or toggle group.
///
/// Pass [`container`](Self::container) and [`cell`](Self::cell) to
/// [`segmented`](crate::prim::segmented::segmented): a muted track with
/// padding and `rounded-lg`, in which the pressed cell is raised with the
/// background fill and `shadow-sm`.
#[derive(Clone, Debug)]
pub struct SegmentedClasses {
    theme: Theme,
    tokens: Tokens,
}

impl SegmentedClasses {
    /// The track.
    pub fn container(&self) -> Tw {
        let t = &self.tokens;
        tw().flex()
            .items_center()
            .h_px(t.control_height)
            .p(t.list_padding / 4.0)
            .rounded_px(f32::from(t.radius))
            .bg(t.muted)
    }

    /// One cell.
    pub fn cell(&self, cell: Cell) -> Tw {
        let t = &self.tokens;
        let base = tw()
            .flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .px(2.0)
            .py(1.0)
            .rounded_px(f32::from(t.radius_md()))
            .role(t.text_role());
        match (cell, self.theme) {
            (Cell::Released, _) => base.text(t.muted_foreground),
            (Cell::Pressed, Theme::Light) => {
                base.text(t.foreground).bg(t.background).shadow(t.shadow_sm)
            }
            (Cell::Pressed, Theme::Dark) => base
                .text(t.foreground)
                .bg(t.input.gamma_multiply(0.3))
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
        v.selection.bg_fill = t.primary;
        v.selection.stroke = Stroke::new(1.0, t.ring);
        v.hyperlink_color = t.primary;
        v.faint_bg_color = t.muted;
        v.extreme_bg_color = t.background;
        v.text_edit_bg_color = Some(match theme {
            Theme::Light => t.background,
            Theme::Dark => t.input.gamma_multiply(0.3),
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
        s.combo_height = 200.0;
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
