use std::time::Duration;

use eggshell_ui::contract::{
    self, Colors, Entry, Focus, Keyed, KindMotion, Look as BoxLook, Metrics, Paint, Size, States,
    SwitchMetrics, Variant,
};
use eggshell_ui::overlay::Distance;
use eggshell_ui::{Easing, Opacity, TextSpec, Tracking};
use egui::{Color32, Context, Id, Shadow, Theme};

use crate::styles::{DestructiveLook, HoverSurface, SecondaryHover, ShadcnStyle};
use crate::{ThemeColor, Tokens, size, variant};

/// The shadcn/ui preset: [`Tokens`] for one egui theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadcn {
    /// The egui theme whose style [`apply`](Self::apply) sets.
    pub theme: Theme,
    /// The values; override any of them before applying. `tokens.style` is
    /// the [`ShadcnStyle`] they belong to.
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

    /// The preset that goes with `theme`, in the default style.
    pub fn for_theme(theme: Theme) -> Self {
        Self {
            theme,
            tokens: Tokens::for_theme(theme),
        }
    }

    /// This preset with the sizes of `style`.
    #[must_use]
    pub fn with_style(mut self, style: ShadcnStyle) -> Self {
        self.tokens = self.tokens.in_style(style);
        self
    }

    /// This preset with `theme_color`'s colours for `primary`,
    /// `primary-foreground` and `ring`.
    #[must_use]
    pub fn theme_color(mut self, theme_color: ThemeColor) -> Self {
        self.tokens = self.tokens.with_theme_color(theme_color, self.theme);
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

    /// Applies both themes in `style` with `theme_color`, unless the
    /// context's current theme already has exactly that preset, so that an
    /// application can call it every frame and switch either at run time.
    pub fn apply_both_with(ctx: &Context, style: ShadcnStyle, theme_color: ThemeColor) {
        let of = |preset: Self| preset.with_style(style).theme_color(theme_color);
        let current = of(Self::for_theme(ctx.theme()));
        if Self::applied(ctx).as_ref() != Some(&current) {
            of(Self::light()).apply(ctx);
            of(Self::dark()).apply(ctx);
        }
    }

    /// The fill of an input-like box: `bg-background`, or `dark:bg-input/30`.
    fn input_fill(&self) -> Color32 {
        match self.theme {
            Theme::Light => self.tokens.background,
            Theme::Dark => self.tokens.input_30(),
        }
    }

    /// This preset as the [`eggshell_ui::Style`] contract.
    pub fn style(&self) -> contract::Style {
        let t = &self.tokens;
        let dark = self.theme == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let ring = Focus {
            width: t.ring_width,
            color: t.ring.gamma_multiply(0.5),
        };
        let focused = |rest: Colors| Colors {
            border: t.ring,
            ..rest
        };
        let flat = |fill, text| Colors {
            fill,
            border: clear,
            text,
        };
        let states = |rest: Colors, hover: Colors| States {
            focused: focused(rest),
            ..States::new(rest, hover, t.background)
        };
        let filled = |rest: Colors, hover: Colors| BoxLook::new(states(rest, hover));
        let raised = |paint: Paint, shadow: Shadow| Paint {
            shadow: t.elevation.shadow(shadow),
            ..paint
        };
        let shadowed = |look: BoxLook, shadow: Shadow| BoxLook {
            paint: raised(look.paint, shadow),
            ..look
        };

        let (hover_fill, hover_text, hover_fill_dark) = match t.hover_surface {
            HoverSurface::Accent => (t.accent, t.accent_foreground, t.accent_50()),
            HoverSurface::Muted => (t.muted, t.foreground, t.muted_50()),
        };
        let outline_rest = Colors {
            fill: self.input_fill(),
            border: t.input,
            text: t.foreground,
        };
        let outline_hover = Colors {
            fill: if dark { t.input_50() } else { hover_fill },
            text: if dark { t.foreground } else { hover_text },
            ..outline_rest
        };
        let destructive_ring = Focus {
            color: t.destructive.gamma_multiply(if dark { 0.4 } else { 0.2 }),
            ..ring
        };
        let destructive = match t.destructive_look {
            DestructiveLook::Solid => filled(
                flat(
                    if dark {
                        t.destructive_60()
                    } else {
                        t.destructive
                    },
                    t.destructive_foreground,
                ),
                flat(t.destructive_90(), t.destructive_foreground),
            ),
            DestructiveLook::Tinted => {
                let tint = |light: f32, deep: f32| {
                    t.destructive
                        .gamma_multiply(if dark { deep } else { light })
                };
                filled(
                    flat(tint(0.1, 0.2), t.destructive),
                    flat(tint(0.2, 0.3), t.destructive),
                )
            }
        };
        let destructive = BoxLook {
            paint: Paint {
                focus: Some(destructive_ring),
                ..destructive.paint
            },
            ..destructive
        };
        let outline_button = shadowed(
            BoxLook::new(states(outline_rest, outline_hover)).bordered(1.0),
            t.shadow_xs,
        );
        let ghost_button = filled(
            flat(clear, t.foreground),
            flat(if dark { hover_fill_dark } else { hover_fill }, hover_text),
        );
        let secondary_hover = match t.secondary_hover {
            SecondaryHover::Fade(opacity) => t.secondary.gamma_multiply(opacity.into()),
            SecondaryHover::Mix(share) => t.secondary.lerp_to_gamma(t.foreground, share.into()),
        };
        let variants = Keyed::new(
            variant::SECONDARY,
            filled(
                flat(t.secondary, t.secondary_foreground),
                flat(secondary_hover, t.secondary_foreground),
            ),
        )
        .with(
            variant::PRIMARY,
            filled(
                flat(t.primary, t.primary_foreground),
                flat(
                    t.primary.gamma_multiply(t.primary_hover.into()),
                    t.primary_foreground,
                ),
            ),
        )
        .with(variant::DESTRUCTIVE, destructive)
        .with(variant::OUTLINE, outline_button)
        .with(variant::GHOST, ghost_button);
        let control = Metrics {
            height: t.control_height,
            min_width: 0.0,
            padding_x: t.button_padding.x,
            gap: t.control_gap,
            radius: t.radius_md(),
            glyph: t.icon_size,
        };
        let toggle = Metrics {
            min_width: t.control_height,
            padding_x: t.toggle_padding_x,
            ..control
        };
        let square = |metrics: Metrics| Metrics {
            min_width: t.control_height,
            padding_x: 0.0,
            ..metrics
        };
        let accented = flat(t.accent, t.accent_foreground);
        let transition = Duration::from_millis(150);

        let box_rest = Colors {
            fill: self.input_fill(),
            border: t.input,
            text: t.primary_foreground,
        };
        let box_on = Colors {
            fill: t.primary,
            border: t.primary,
            text: t.primary_foreground,
        };
        let boxed = |colors| raised(Paint::new(states(colors, colors)), t.shadow_xs);
        let radio_colors = Colors {
            text: t.primary,
            ..box_rest
        };

        let switch_paint = |track, thumb| {
            let colors = Colors {
                fill: track,
                border: clear,
                text: thumb,
            };
            Paint::new(states(colors, colors))
        };
        let thumb = t.switch_thumb;
        let (selected_cell, released_cell) = (
            if dark {
                Colors {
                    fill: t.input_30(),
                    border: t.input,
                    text: t.foreground,
                }
            } else {
                Colors {
                    fill: t.background,
                    border: clear,
                    text: t.foreground,
                }
            },
            flat(clear, t.muted_foreground),
        );
        let input_rest = Colors {
            fill: self.input_fill(),
            border: t.input,
            text: t.foreground,
        };
        let input_box = shadowed(
            BoxLook::new(States {
                focused: focused(input_rest),
                ..States::new(
                    input_rest,
                    Colors {
                        fill: if dark { t.input_50() } else { t.background },
                        ..input_rest
                    },
                    t.background,
                )
            })
            .bordered(1.0),
            t.shadow_xs,
        );
        let field_rest = Colors {
            text: t.secondary_foreground,
            ..input_rest
        };
        let field_look = BoxLook::new(States {
            focused: focused(field_rest),
            ..States::new(field_rest, field_rest, t.background)
        })
        .bordered(1.0);
        let thumb_colors = Colors {
            fill: t.background,
            border: t.primary,
            text: t.foreground,
        };
        let popup_colors = Colors {
            fill: t.popover,
            border: t.border,
            text: t.popover_foreground,
        };
        let mut style = contract::Style::default();
        style.text = t.text_role();
        style.focus = ring;
        style.label = States::still(flat(clear, t.foreground), t.background);
        style.button = {
            let mut button = style.button;
            button.variants = variants;
            button.sizes = Keyed::new(Size::DEFAULT, control).with(size::ICON, square(control));
            button.toggle_sizes =
                Keyed::new(Size::DEFAULT, toggle).with(size::ICON, square(toggle));
            button.toggle_border_width = 0.0;
            button.toggle_off = Keyed::new(
                Variant::DEFAULT,
                Paint::new(states(
                    flat(clear, t.foreground),
                    flat(t.muted, t.muted_foreground),
                )),
            );
            button.toggle_on = Keyed::new(Variant::DEFAULT, Paint::new(states(accented, accented)));
            button.press_shift = 1.0;
            button.transition = transition;
            button
        };
        style.checkbox = {
            let mut checkbox = style.checkbox;
            checkbox.size = 16.0;
            checkbox.radius = t.checkbox_radius;
            checkbox.glyph = 14.0;
            checkbox.gap = 8.0;
            checkbox.border_width = 1.0;
            checkbox.off = boxed(box_rest);
            checkbox.on = boxed(box_on);
            checkbox.transition = transition;
            checkbox
        };
        style.radio = {
            let mut radio = style.radio;
            radio.size = 16.0;
            radio.dot = 8.0;
            radio.gap = 8.0;
            radio.border_width = 1.0;
            radio.off = boxed(radio_colors);
            radio.on = boxed(radio_colors);
            radio.transition = transition;
            radio
        };
        style.switch = {
            let mut switch = style.switch;
            switch.sizes = Keyed::new(
                Size::DEFAULT,
                SwitchMetrics {
                    track: t.switch_track,
                    thumb_off: thumb,
                    thumb_on: thumb,
                },
            );
            switch.gap = 8.0;
            switch.border_width = 0.0;
            switch.off = switch_paint(t.switch_track_off, t.switch_thumb_off);
            switch.on = switch_paint(t.switch_track_on, t.switch_thumb_on);
            switch.transition = transition;
            switch
        };
        style.segmented = {
            let mut segmented = style.segmented;
            segmented.track_fill = t.muted;
            segmented.track_radius = t.radius;
            segmented.track_padding = t.list_padding;
            segmented.height = t.control_height;
            segmented.cell_radius = t.radius_md();
            segmented.cell_padding = t.cell_padding;
            segmented.column_gap = 0.0;
            segmented.row_gap = 2.0;
            segmented.icon_size = t.icon_size;
            segmented.icon_gap = 6.0;
            segmented.cell_border_width = 1.0;
            segmented.selected = raised(
                Paint::new(States::still(selected_cell, t.muted)),
                t.shadow_sm,
            );
            segmented.unselected = Paint::new(States::still(released_cell, t.muted));
            segmented.transition = transition;
            segmented
        };
        style.slider = {
            let mut slider = style.slider;
            slider.track_height = t.slider_track;
            slider.thumb_size = t.slider_thumb;
            slider.radius = t.slider_radius;
            slider.track = States::still(flat(t.muted, t.foreground), t.background);
            slider.range = States::still(flat(t.primary, t.primary_foreground), t.background);
            slider.thumb = shadowed(
                BoxLook::new(States::still(thumb_colors, t.background)).bordered(1.0),
                t.shadow_sm,
            );
            slider.transition = transition;
            slider
        };
        style.progress = {
            let mut progress = style.progress;
            progress.height = 8.0;
            progress.radius = u8::MAX;
            progress.track = flat(t.primary.gamma_multiply(0.2), t.foreground);
            progress.range = flat(t.primary, t.primary_foreground);
            progress
        };
        style.select = {
            let mut select = style.select;
            select.trigger = input_box;
            select.metrics = control;
            select.chevron = 16.0;
            select.chevron_opacity = Opacity::new(0.5);
            select.transition = transition;
            select
        };
        // shadcn's DropdownMenu: `rounded-md border p-1 shadow-md min-w-[8rem]`,
        // items `rounded-sm px-2 py-1.5 gap-2`, `focus:bg-accent`.
        style.menu = {
            let mut menu = style.menu;
            menu.popup = BoxLook::new(States::still(popup_colors, t.popover))
                .bordered(1.0)
                .shadowed(t.shadow_md);
            menu.radius = t.radius_md();
            menu.padding = t.popup_padding;
            menu.min_width = 128.0;
            menu.offset = Distance::new(4.0);
            menu.item = BoxLook::new(States::new(
                flat(clear, t.popover_foreground),
                accented,
                t.popover,
            ));
            menu.item_radius = t.radius_sm();
            menu.item_padding = t.item_padding;
            menu.item_gap = t.item_padding.x;
            let tint = if dark { 0.2 } else { 0.1 };
            menu.destructive = States::new(
                flat(clear, t.destructive),
                flat(t.destructive.gamma_multiply(tint), t.destructive),
                t.popover,
            );
            menu.separator = t.border;
            menu.separator_margin = 4.0;
            menu.heading = t.popover_foreground;
            menu.shortcut = t.muted_foreground;
            menu.shortcut_text = TextSpec {
                size: 12.0,
                line_height: 16.0,
                family: egui::FontFamily::Proportional,
            };
            menu.shortcut_tracking = Tracking::points(1.2);
            menu.indicator = 16.0;
            menu.dot = 8.0;
            menu
        };
        style.stepper = {
            let mut stepper = style.stepper;
            stepper.group = input_box;
            stepper.height = t.control_height;
            stepper.padding_x = t.input_padding.x;
            stepper.radius = t.radius_md();
            stepper.button = ghost_button;
            stepper.button_width = 24.0;
            stepper.divider = t.input;
            stepper.glyph = 12.0;
            stepper.transition = transition;
            stepper
        };
        style.input = {
            let mut input = style.input;
            input.look = field_look;
            input.radius = t.radius_md();
            input.padding = t.input_padding;
            input.height = t.control_height;
            input.hint = None;
            input
        };
        style.selection = {
            let mut selection = style.selection;
            selection.fill = t.primary.gamma_multiply(0.3);
            selection.text = Some(t.foreground);
            selection
        };
        // shadcn's ScrollArea: `w-2.5` bar, `bg-border` `rounded-full` thumb.
        style.scroll.width = 10.0;
        style.scroll.thumb = t.border;
        style.scroll.thumb_hover = t.border;
        style.scroll.thumb_active = t.border;
        style.scroll.track = clear;
        style.scroll.radius = egui::CornerRadius::same(u8::MAX);
        // shadcn's overlays are `bg-black/50`; with no backdrop blur (#40) the
        // scrim alone separates a dialog from the page, so it is darker.
        style.scrim = Color32::from_black_alpha(204);
        // shadcn's `animate-in` (150 ms) for popovers, dropdown menus and
        // selects: `zoom-in-95 slide-in-from-*-2`. DialogContent:
        // `zoom-in-95 duration-200`. SheetContent:
        // `slide-in-from-*`, `ease-in-out`, 500 ms open and 300 ms close.
        let animate = KindMotion {
            entry: Entry::Pop,
            enter: Duration::from_millis(150),
            exit: Duration::from_millis(150),
            easing: Easing::Default,
        };
        style.motion.popover = animate;
        style.motion.menu = animate;
        style.motion.dialog = KindMotion {
            entry: Entry::Zoom,
            enter: Duration::from_millis(200),
            exit: Duration::from_millis(200),
            easing: Easing::Default,
        };
        style.motion.sheet = KindMotion {
            entry: Entry::Slide,
            enter: Duration::from_millis(500),
            exit: Duration::from_millis(300),
            easing: Easing::InOut,
        };
        style
    }

    /// Sets the egui style of this preset's theme from the tokens and
    /// remembers them for [`tokens`]. Safe to call every frame
    /// but only needed again when the tokens change.
    pub fn apply(&self, ctx: &Context) {
        let tokens = self.tokens.clone();
        ctx.style_mut_of(self.theme, |style| tokens.set_style(self.theme, style));
        ctx.data_mut(|data| data.insert_temp(tokens_id(self.theme), self.tokens.clone()));
        self.style().install_for(ctx, self.theme);
    }
}

fn tokens_id(theme: Theme) -> Id {
    Id::NULL.with(("eggshell-ui-shadcn-tokens", theme))
}

/// The tokens a preset was applied with for the context's current theme, if
/// one was.
pub fn tokens(ctx: &Context) -> Option<Tokens> {
    let id = tokens_id(ctx.theme());
    ctx.data(|data| data.get_temp::<Tokens>(id))
}
