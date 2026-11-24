use std::time::Duration;

use eggshell_ui::contract::{
    self, Arrow, CheckboxMetrics, Colors, Em, Entry, Focus, GroupEnd, InputMetrics, Keyed,
    KindMotion, Look as BoxLook, Metrics, Paint, RadioMetrics, SegmentedMetrics, Size,
    SliderMetrics, SplitterColors, States, StepperMetrics, SwitchMetrics, TextSize, Variant,
};
use eggshell_ui::overlay::Distance;
use eggshell_ui::owned::{FieldPart, InputGroupPart, InputPart, TablePart};
use eggshell_ui::sheet::{Sheet, When};
use eggshell_ui::{Aria, Easing, Match, Opacity, TextSpec, Weight};
use egui::{Color32, Context, Id, Shadow, Theme, vec2};

#[cfg(feature = "extras")]
use crate::variant;
use crate::{ContrastPair, FontSet, Palette, Tokens, size};
use crate::{HoverSurface, ShadcnStyle};

/// TODO(prose): eggshell's smaller fields, which shadcn does not draw: a `sm`
/// field a unit shorter than the preset's controls and a `xs` one three, each
/// with its own inline padding, over the generated look.
fn write_field_sizes(sheet: &mut Sheet, t: &Tokens) {
    let own = t.own_input_padding_x;
    for (size, height, padding_x) in [
        (size::SM, t.control_units - 1.0, own),
        (size::XS, t.control_units - 3.0, (own - 1.0).max(0.0)),
    ] {
        let when = When::always().size(size);
        sheet.rule_when(InputPart::Root, when, move |c, _| c.h(height).px(padding_x));
        sheet.rule_when(InputGroupPart::Root, when, move |c, _| c.h(height));
        sheet.rule_when(InputGroupPart::Input, when, move |c, _| c.px(padding_x));
        sheet.rule_when(
            InputGroupPart::Addon,
            when.attr("align", "inline-start"),
            move |c, _| c.pl(padding_x),
        );
        sheet.rule_when(
            InputGroupPart::Addon,
            when.attr("align", "inline-end"),
            move |c, _| c.pr(padding_x),
        );
    }
    sheet.rule_when(InputPart::Root, When::always().size(size::XS), |c, _| {
        c.py(0.5)
    });
}

/// The shadcn/ui preset: [`Tokens`] for one egui theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadcn {
    /// The values; override any of them before applying. `tokens.style` is
    /// the [`ShadcnStyle`] they belong to.
    pub tokens: Tokens,
}

impl Shadcn {
    /// TODO(prose): the egui theme whose style [`apply`](Self::apply) sets: the tokens'.
    pub fn theme(&self) -> Theme {
        self.tokens.theme
    }

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
            tokens: Tokens::for_theme(theme),
        }
    }

    /// This preset with the sizes of `style`.
    #[must_use]
    pub fn with_style(mut self, style: ShadcnStyle) -> Self {
        self.tokens = self.tokens.in_style(style);
        self
    }

    /// TODO(prose): this preset in `palette`'s colours.
    #[must_use]
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.tokens = self.tokens.with_palette(palette);
        self
    }

    /// TODO(prose): this preset accepting `pair` under AA contrast, so that
    /// [`apply`](Self::apply) does not warn about it.
    #[must_use]
    pub fn accept_contrast(mut self, pair: ContrastPair) -> Self {
        if !self.tokens.accepted_contrast.contains(&pair) {
            self.tokens.accepted_contrast.push(pair);
        }
        self
    }

    /// The preset applied to `ctx` for its current theme, if one was.
    pub fn applied(ctx: &Context) -> Option<Self> {
        Some(Self {
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

    /// Applies both themes in `style` with `palette`, unless the
    /// context's current theme already has exactly that preset, so that an
    /// application can call it every frame and switch either at run time.
    pub fn apply_both_with(ctx: &Context, style: ShadcnStyle, palette: Palette) {
        let of = |preset: Self| preset.with_style(style).with_palette(palette);
        let current = of(Self::for_theme(ctx.theme()));
        if Self::applied(ctx).as_ref() != Some(&current) {
            of(Self::light()).apply(ctx);
            of(Self::dark()).apply(ctx);
        }
    }

    /// TODO(prose): this preset in `style` with `palette`, light and dark, as a layout's style.
    ///
    /// ```
    /// use eggshell_ui::LayoutState;
    /// use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle};
    ///
    /// let style = Shadcn::themed(ShadcnStyle::Vega, Palette::new());
    /// let _layout = LayoutState::new().with_style(style);
    /// ```
    pub fn themed(
        style: ShadcnStyle,
        palette: Palette,
    ) -> eggshell_ui::contract::Themed<std::sync::Arc<contract::Style>> {
        let half = |preset: Self| {
            std::sync::Arc::new(
                preset
                    .with_style(style)
                    .with_palette(palette)
                    .layout_style(),
            )
        };
        eggshell_ui::contract::Themed {
            light: half(Self::light()),
            dark: half(Self::dark()),
        }
    }

    /// TODO(prose): this preset's style with its fonts and the extras' looks, as one half of a layout's style.
    ///
    /// ```
    /// use eggshell_ui::LayoutState;
    /// use eggshell_ui::contract::Themed;
    /// use eggshell_ui_shadcn::Shadcn;
    /// use std::sync::Arc;
    ///
    /// let style = Themed {
    ///     light: Arc::new(Shadcn::light().with_fonts(None).layout_style()),
    ///     dark: Arc::new(Shadcn::dark().with_fonts(None).layout_style()),
    /// };
    /// let _layout = LayoutState::new().with_style(style);
    /// ```
    pub fn layout_style(&self) -> contract::Style {
        let mut core = self.style();
        if let Some(font) = &self.tokens.font {
            core.set_extension(contract::Fonts(font.inserts()));
        }
        #[cfg(feature = "extras")]
        {
            // keep sorted
            let badge = self.badge(&core);
            let command = self.command(&core);
            let dialog = self.dialog(&core);
            let disclosure = self.disclosure(&core);
            let form = self.form(&core);
            let notify = self.notify(&core);
            let skeleton = self.skeleton(&core);
            core.set_extension(badge);
            core.set_extension(command);
            core.set_extension(dialog);
            core.set_extension(disclosure);
            core.set_extension(form);
            core.set_extension(self.hover_card());
            core.set_extension(self.markdown());
            core.set_extension(notify);
            core.set_extension(skeleton);
        }
        core
    }

    /// The fill of an input-like box: `bg-background`, or `dark:bg-input/30`.
    fn input_fill(&self) -> Color32 {
        match self.theme() {
            Theme::Light => self.tokens.background,
            Theme::Dark => self.tokens.input_30(),
        }
    }

    /// `rest` with the focused border, `focus-visible:border-ring`.
    fn focused(&self, rest: Colors) -> Colors {
        Colors {
            border: self.tokens.ring,
            ..rest
        }
    }

    /// A box's colours at rest and under the pointer, with the focused
    /// border, over the page.
    fn states(&self, rest: Colors, hover: Colors) -> States {
        States {
            focused: self.focused(rest),
            ..States::new(rest, hover, self.tokens.background)
        }
    }

    /// This preset as the [`eggshell_ui::Style`] contract.
    pub fn style(&self) -> contract::Style {
        let t = &self.tokens;
        let dark = self.theme() == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let focused = |rest: Colors| self.focused(rest);
        let flat = |fill, text| Colors {
            fill,
            border: clear,
            text,
        };
        let states = |rest: Colors, hover: Colors| self.states(rest, hover);
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
        let ghost_button = filled(
            flat(clear, t.foreground),
            flat(if dark { hover_fill_dark } else { hover_fill }, hover_text),
        );
        let control = Metrics {
            height: t.control_height(),
            min_width: 0.0,
            padding_x: t.points(t.button_padding.x),
            gap: t.points(t.control_gap),
            radius: t.radius_md(),
            glyph: t.points(t.icon_size),
            text: None,
        };
        let xs_text = Some(t.text.xs);
        let u = |units: f32| t.points(units);
        let small = |m: Metrics| Metrics {
            height: m.height - u(1.0),
            min_width: if m.min_width == 0.0 {
                0.0
            } else {
                m.height - u(1.0)
            },
            padding_x: (m.padding_x - u(1.0)).max(0.0),
            gap: m.gap.min(u(1.5)),
            ..m
        };
        let tiny = |m: Metrics| Metrics {
            height: m.height - u(3.0),
            min_width: if m.min_width == 0.0 {
                0.0
            } else {
                m.height - u(3.0)
            },
            padding_x: (m.padding_x - u(2.0)).max(0.0),
            gap: m.gap.min(u(1.0)),
            glyph: u(3.0),
            text: xs_text,
            ..m
        };
        let toggle_metrics = Metrics {
            min_width: t.control_height(),
            padding_x: t.points(t.toggle_padding_x),
            ..control
        };
        let square = |metrics: Metrics| Metrics {
            min_width: t.control_height(),
            padding_x: 0.0,
            ..metrics
        };
        let sized = |metrics: Metrics| {
            Keyed::new(Size::DEFAULT, metrics)
                .with(size::ICON, square(metrics))
                .with(size::SM, small(metrics))
                .with(size::XS, tiny(metrics))
                .with(size::ICON_SM, square(small(metrics)))
                .with(size::ICON_XS, square(tiny(metrics)))
        };
        let accented = flat(t.accent, t.accent_foreground);
        let transition = Duration::from_millis(150);

        let thumb = t.points(t.switch_thumb);
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
        let thumb_colors = Colors {
            fill: Color32::WHITE,
            border: t.primary,
            text: t.foreground,
        };
        let mut style = contract::Style::default();
        style.text = t.text_role();
        style.faces = t.font.as_ref().map(FontSet::declared).unwrap_or_default();
        style.set_label(States::still(flat(clear, t.foreground), t.background));
        style.toggle = {
            let mut toggle = style.toggle;
            toggle.text = Some(t.text_role_at(t.control_weight));
            toggle.sizes = sized(toggle_metrics);
            toggle.border_width = 0.0;
            toggle.off = Keyed::new(
                Variant::DEFAULT,
                Paint::new(states(
                    flat(clear, t.foreground),
                    flat(t.muted, t.muted_foreground),
                )),
            );
            toggle.on = Keyed::new(Variant::DEFAULT, Paint::new(states(accented, accented)));
            toggle.press_shift = 1.0;
            toggle.transition = transition;
            toggle
        };
        let checkbox = {
            let mut checkbox = style.checkbox().clone();
            checkbox.text = Some(t.text_role_at(t.label_weight));
            let box_at = |size: f32, glyph: f32, gap: f32, text| CheckboxMetrics {
                size,
                radius: t.checkbox_radius,
                glyph,
                gap,
                text,
            };
            checkbox.sizes = Keyed::new(Size::DEFAULT, box_at(16.0, 14.0, 8.0, None))
                .with(size::SM, box_at(u(3.5), u(3.0), u(1.5), None))
                .with(size::XS, box_at(u(3.0), u(2.5), u(1.5), xs_text));
            checkbox
        };
        style.set_checkbox(checkbox);
        let radio = {
            let mut radio = style.radio().clone();
            radio.text = Some(t.text_role_at(t.label_weight));
            let circle = |size: f32, dot: f32, gap: f32, text| RadioMetrics {
                size,
                dot,
                gap,
                text,
            };
            radio.sizes = Keyed::new(Size::DEFAULT, circle(16.0, 8.0, 8.0, None))
                .with(size::SM, circle(u(3.5), u(1.75), u(1.5), None))
                .with(size::XS, circle(u(3.0), u(1.5), u(1.5), xs_text));
            radio
        };
        style.set_radio(radio);
        let switch = {
            let mut switch = style.switch().clone();
            switch.text = Some(t.text_role_at(t.label_weight));
            let switch_at = |scale: f32, text| SwitchMetrics {
                track: t.switch_track * scale,
                thumb_off: thumb * scale,
                thumb_on: thumb * scale,
                text,
            };
            switch.sizes = Keyed::new(Size::DEFAULT, switch_at(1.0, None))
                .with(size::SM, switch_at(0.75, None))
                .with(size::XS, switch_at(0.625, xs_text));
            switch.gap = 8.0;
            switch
        };
        style.set_switch(switch);
        let segmented = {
            const CELL_TRIM: f32 = 1.0;
            let mut segmented = style.segmented().clone();
            // The one exception to tabs.tsx, whose list is `bg-muted`: on a
            // `muted` panel that fill is the panel's own colour and the track
            // disappears, so the list keeps `input` (the user's pick, #346).
            // Revisit once a cascade can give a track on a muted surface its
            // own fill (#443).
            segmented.track_fill = t.input;
            let track_at = |height: f32, padding_x: f32, icon: f32, text| SegmentedMetrics {
                track_radius: t.radius_lg(),
                track_padding: t.list_padding,
                height,
                cell_radius: t.radius_md(),
                // tabs.tsx, trigger: `px-2 py-1` in a box 1 px shorter than
                // the list's inside, which overflows its `text-sm` by 1 px;
                // the trimmed point comes out of the padding instead.
                cell_padding: vec2(padding_x, t.points(t.cell_padding.y) - CELL_TRIM / 2.0),
                column_gap: 0.0,
                row_gap: 2.0,
                icon_size: icon,
                icon_gap: u(1.5),
                text,
                // tabs.tsx, trigger: `h-[calc(100%-1px)]`.
                cell_trim: CELL_TRIM,
            };
            segmented.sizes = Keyed::new(
                Size::DEFAULT,
                track_at(
                    t.control_height(),
                    t.points(t.cell_padding.x),
                    t.points(t.icon_size),
                    None,
                ),
            )
            .with(
                size::SM,
                track_at(
                    t.control_height() - u(1.0),
                    t.points(t.cell_padding.x),
                    t.points(t.icon_size),
                    None,
                ),
            )
            .with(
                size::XS,
                track_at(
                    t.control_height() - u(3.0),
                    (t.points(t.cell_padding.x) - u(1.0)).max(0.0),
                    u(3.0),
                    xs_text,
                ),
            );
            segmented
        };
        style.set_segmented(segmented);
        style.slider = {
            let mut slider = style.slider;
            let slider_at = |track_height: f32,
                             thumb_size: f32,
                             text: Option<TextSize>,
                             header_gap: f32| SliderMetrics {
                track_height,
                thumb_size,
                radius: t.slider_radius,
                text,
                header_gap,
            };
            slider.sizes = Keyed::new(
                Size::DEFAULT,
                slider_at(
                    t.points(t.slider_track),
                    t.points(t.slider_thumb),
                    None,
                    8.0,
                ),
            )
            .with(
                size::SM,
                slider_at(
                    t.points(t.slider_track).min(u(1.0)),
                    t.points(t.slider_thumb) - u(0.5),
                    None,
                    u(1.5),
                ),
            )
            .with(
                size::XS,
                slider_at(
                    t.points(t.slider_track).min(u(0.75)),
                    t.points(t.slider_thumb) - u(1.0),
                    xs_text,
                    u(1.0),
                ),
            );
            slider.label = States::still(flat(clear, t.foreground), t.background);
            slider.readout = States::still(flat(clear, t.muted_foreground), t.background);
            slider.track = States::still(flat(t.input, t.foreground), t.background);
            slider.range = States::still(flat(t.primary, t.primary_foreground), t.background);
            slider.thumb = shadowed(
                BoxLook::new(States::still(thumb_colors, t.background)).bordered(1.0),
                t.shadow_sm,
            );
            slider.thumb.paint = slider.thumb.paint.haloed(Focus {
                width: t.slider_halo,
                color: t.ring.gamma_multiply(0.5),
            });
            slider.transition = transition;
            slider
        };
        style.progress = {
            let mut progress = style.progress;
            progress.height = 6.0;
            progress.radius = u8::MAX;
            progress.track = flat(t.input, t.foreground);
            progress.range = flat(t.primary, t.primary_foreground);
            progress
        };
        style.set_select({
            let mut select = style.select().clone();
            select.sizes = Keyed::new(Size::DEFAULT, control)
                .with(size::SM, small(control))
                .with(size::XS, tiny(control));
            select.chevron = 16.0;
            select.chevron_opacity = Opacity::new(0.5);
            select
        });
        // shadcn's DropdownMenu: `p-1`, items `px-2 py-1.5 gap-2`, a 16 pt
        // check and chevron, and a `text-xs` shortcut. The look is the
        // generated sheet's.
        style.set_menu({
            let mut menu = style.menu().clone();
            menu.padding = t.points(t.popup_padding);
            menu.offset = Distance::new(4.0);
            menu.item_padding = t.points_vec(t.item_padding);
            menu.item_gap = t.points(t.item_padding.x);
            menu.shortcut_text = TextSpec {
                size: 12.0,
                line_height: 16.0,
                family: egui::FontFamily::Proportional,
                weight: Weight::Normal,
            };
            menu.indicator = 16.0;
            menu
        });
        // shadcn's TooltipContent: a 10 pt arrow, flush with the trigger.
        style.set_tooltip({
            let mut tooltip = style.tooltip().clone();
            tooltip.arrow = Some(Arrow { size: 10.0 });
            tooltip.offset = Distance::new(0.0);
            // Radix's default delay, deliberately not shadcn's 0 ms, which
            // assumes tooltips are rare (desktop apps use them densely); the
            // grace is Radix's `skipDelayDuration`.
            tooltip.delay = Duration::from_millis(700);
            tooltip.grace = Duration::from_millis(300);
            tooltip
        });
        // shadcn's Kbd: `bg-muted text-muted-foreground h-5 min-w-5 rounded-sm
        // px-1 text-xs font-medium`, and inside a tooltip
        // `bg-background/20 text-background` (`/10` in the dark theme).
        // A captioned mount: `text-muted-foreground` above `border rounded-md
        // p-4`, as the docs page's examples were rendered for the pick.
        style.mount = {
            let mut mount = style.mount;
            mount.caption = t.muted_foreground;
            mount.caption_text = None;
            mount.gap = 6.0;
            mount.border = t.border;
            mount.border_width = 1.0;
            mount.radius = t.radius_md();
            mount.padding = 16.0;
            mount.bare_padding = 8.0;
            mount.selected_ring = t.primary;
            mount
        };
        style.kbd = {
            let mut kbd = style.kbd;
            kbd.page = flat(t.muted, t.muted_foreground);
            kbd.tooltip = flat(
                if dark {
                    t.background_10()
                } else {
                    t.background_20()
                },
                t.background,
            );
            kbd.border_width = 0.0;
            kbd.radius = t.radius_sm();
            kbd.padding_x = 4.0;
            kbd.height = 20.0;
            kbd.text = Some(TextSpec {
                size: 12.0,
                line_height: 16.0,
                family: egui::FontFamily::Proportional,
                weight: t.control_weight,
            });
            kbd
        };
        // shadcn's typography: a link is `font-medium text-primary underline`,
        // inline code `rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm`.
        style.rich = {
            let mut rich = style.rich;
            rich.link = t.primary;
            rich.link_weight = Weight::Medium;
            rich.code.fill = t.muted;
            rich.code.radius = t.radius_sm();
            rich.code.padding_x = 4.8;
            rich.code.padding_y = 3.2;
            rich.code.size = Em::new(0.875);
            rich
        };
        style.stepper = {
            let mut stepper = style.stepper;
            stepper.group = input_box;
            let group_at =
                |height: f32, (padding_x, button_width, glyph): (f32, f32, f32), text| {
                    StepperMetrics {
                        height,
                        padding_x,
                        radius: t.radius_md(),
                        button_width,
                        glyph,
                        text,
                    }
                };
            let own = u(t.own_input_padding_x);
            stepper.sizes = Keyed::new(
                Size::DEFAULT,
                group_at(
                    t.control_height(),
                    (t.points(t.input_padding.x), u(6.0), u(3.0)),
                    None,
                ),
            )
            .with(
                size::SM,
                group_at(t.control_height() - u(1.0), (own, u(5.0), u(3.0)), None),
            )
            .with(
                size::XS,
                group_at(t.control_height() - u(3.0), (own, u(4.0), u(2.5)), xs_text),
            );
            stepper.button = ghost_button;
            stepper.divider = t.input;
            stepper.transition = transition;
            stepper
        };
        let input = {
            let mut input = style.input().clone();
            let field_at = |text, (icon_size, icon_gap): (f32, f32)| InputMetrics {
                text,
                icon_size,
                icon_gap,
            };
            input.sizes = Keyed::new(Size::DEFAULT, field_at(None, (u(4.0), u(2.0))))
                .with(size::SM, field_at(None, (u(3.5), u(1.5))))
                .with(size::XS, field_at(xs_text, (u(3.0), u(1.5))));
            // input.tsx: `placeholder:text-muted-foreground`.
            input.hint = Some(t.muted_foreground);
            input.icon = t.muted_foreground;
            input
        };
        style.set_input(input);
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
        // Radix's ScrollArea keeps its thumb at least 18 px long.
        style.scroll.min_thumb_length = 18.0;
        // shadcn's ResizableHandle: a `w-px bg-border` line in a wider hit area,
        // which a faint band fills while hot.
        style.splitter.thickness = 6.0;
        style.splitter.line_width = 1.0;
        style.splitter.rest = SplitterColors {
            band: clear,
            line: t.border,
            grip: clear,
        };
        style.splitter.hot = SplitterColors {
            band: t.primary.gamma_multiply(0.15),
            line: t.primary,
            grip: clear,
        };
        style.splitter.grip = None;
        style.splitter.transition = Duration::from_millis(150);
        // shadcn's Table: `border-b` rows, none under the last; the stripes
        // and groups are eggshell's own.
        let mut table = style.table().clone();
        table.rule = t.border;
        table.rule_width = 1.0;
        table.stripe = t.muted.gamma_multiply(0.3);
        table.group_text_size = Some(TextSize {
            size: 12.0,
            line_height: 16.0,
        });
        table.group_weight = Weight::Medium;
        table.group_end = GroupEnd::Gap(12.0);
        style.set_table(table);
        // shadcn's overlays are `bg-black/50`; with no backdrop blur (#40) the
        // scrim alone separates a dialog from the page, so it is darker.
        style.scrim = Color32::from_black_alpha(204);
        style.tones = t.tones();
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
        style.motion.tooltip = KindMotion {
            entry: Entry::Zoom,
            ..animate
        };
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
        style.tokens = self.tokens.clone();
        style.sheet.as_preset();
        style.rewrite_rules();
        if dark {
            // field.tsx, a choice card's label: `dark:has-data-checked:bg-primary/10
            // dark:has-data-checked:border-primary/20`.
            style.sheet.rule(FieldPart::Label, |t, k| {
                t.has(Match::aria(Aria::Checked), |b| {
                    b.bg(k.primary.gamma_multiply(0.1))
                        .border_color(k.primary.gamma_multiply(0.2))
                })
            });
        }
        // A table's group rows are eggshell's own: shadcn's footer band.
        style
            .sheet
            .rule(TablePart::Group, |t, k| t.bg(k.muted.gamma_multiply(0.5)))
            .rule(TablePart::GroupCell, |t, k| t.text(k.muted_foreground));
        crate::sheets::write(&mut style.sheet, t.style, self.theme());
        write_field_sizes(&mut style.sheet, t);
        style.write_exceptions();
        #[cfg(feature = "extras")]
        {
            let disclosure = style.extension::<eggshell_ui_extras::disclosure::DisclosureStyle>();
            eggshell_ui_extras::disclosure::write_exceptions(&mut style.sheet, &disclosure);
        }
        style
    }

    /// This preset drawing its text in `font`, or with `None` in egui's
    /// fonts as the application has set them.
    #[must_use]
    pub fn with_fonts(mut self, font: Option<FontSet>) -> Self {
        self.tokens.font = font;
        self
    }

    /// Sets the egui style of this preset's theme from the tokens and
    /// remembers them for [`tokens`]. Safe to call every frame
    /// but only needed again when the tokens change.
    ///
    /// It also adds the tokens' [`font`](Tokens::font) to the context's
    /// fonts, in front of the fonts egui has, once: egui keeps the faces it
    /// already has by name. egui draws with added fonts from the next pass,
    /// or from this one if no text has been laid out in it yet; until then
    /// a weight is drawn in the nearest lighter registered face.
    ///
    /// TODO(prose): in a debug build it reports each [`ContrastPair`] under
    /// AA contrast as [`Problem::LowContrast`](eggshell_ui::Problem::LowContrast),
    /// unless the tokens accept it or it is shadcn's own colour; it never
    /// changes a colour.
    pub fn apply(&self, ctx: &Context) {
        eggshell_ui::theme::warn_low_contrast(ctx, &self.tokens);
        if let Some(font) = &self.tokens.font {
            font.install(ctx);
        }
        let style = self.style();
        ctx.style_mut_of(self.theme(), |egui| style.egui_style(self.theme(), egui));
        ctx.data_mut(|data| data.insert_temp(tokens_id(self.theme()), self.tokens.clone()));
    }

    /// TODO(prose): the look of a notification, shadcn's Sonner over sonner's
    /// own sizes. From `ref/shadcn` (f50887a) and `ref/sonner` (v2.0.7,
    /// `src/styles.css`):
    /// - the card is `--normal-bg: popover`, `--normal-text:
    ///   popover-foreground` and `--normal-border: border` (`bases/radix/ui/
    ///   sonner.tsx` 69-71), 1 pt border, 16 pt padding, 6 pt between its
    ///   parts, `font-size: 13px` (styles.css 96-106), 356 pt wide and 14 pt
    ///   between cards, 3 shown, 24 pt from the window (`src/index.tsx`
    ///   22-37);
    /// - the shadow is `0 4px 12px rgba(0, 0, 0, 0.1)` (styles.css 101);
    /// - the sentence is the toast's title: `font-weight: 500`,
    ///   `line-height: 1.5` (styles.css 141-144);
    /// - the corners are `cn-toast`'s `rounded-2xl` in Vega, 1.8 times
    ///   `--radius`, and `rounded-none` in Lyra (`style-vega.css` 1246,
    ///   `style-lyra.css` 1221);
    /// - an action's button is sonner's `[data-button]`: filled with
    ///   `--normal-text` under `--normal-bg`, 24 pt tall, `padding 0 8px`,
    ///   4 pt corners, 12 px at weight 500 (styles.css 181-202);
    /// - the × is sonner's `[data-close-button]`: a 20 pt circle with a 1 pt
    ///   border on the card's top-left corner, moved out by 35 % of itself
    ///   (styles.css 1-12, 219-246), in `gray12` over `gray4` and `gray2` /
    ///   `gray5` under the pointer in the light theme (the gray scale, 3-25),
    ///   and in `--normal-text` over `--normal-border`, with `hsl(0 0% 12%)`
    ///   and `hsl(0 0% 25%)` under the pointer, in the dark (styles.css 520-
    ///   536).
    #[cfg(feature = "extras")]
    pub fn notify(&self, style: &contract::Style) -> eggshell_ui_extras::notify::NotifyStyle {
        let t = &self.tokens;
        let mut notify = eggshell_ui_extras::notify::NotifyStyle::from_style(style);
        notify.fill = t.popover;
        notify.border = t.border;
        notify.text = t.popover_foreground;
        notify.border_width = 1.0;
        notify.radius = match t.style {
            ShadcnStyle::Vega => contract::corner(t.radius.lg() * 1.8),
            ShadcnStyle::Lyra => 0,
        };
        notify.shadow = Some(Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(26),
        });
        notify.role = TextSpec {
            size: 13.0,
            line_height: 19.5,
            ..t.text_role_at(Weight::Medium)
        };
        let dark = self.theme() == Theme::Dark;
        let gray = Color32::from_gray;
        notify.action.fill = t.popover_foreground;
        notify.action.text = t.popover;
        notify.action.radius = 4;
        notify.action.height = 24.0;
        notify.action.padding_x = 8.0;
        notify.action.role = TextSpec {
            size: 12.0,
            line_height: 16.0,
            ..t.text_role_at(Weight::Medium)
        };
        notify.close.size = 20.0;
        notify.close.outset = 7.0;
        notify.close.glyph = 12.0;
        notify.close.fill = t.popover;
        notify.close.border = if dark { t.border } else { gray(237) };
        notify.close.text = if dark { t.popover_foreground } else { gray(23) };
        notify.close.hover_fill = if dark { gray(31) } else { gray(248) };
        notify.close.hover_border = if dark { gray(64) } else { gray(232) };
        notify
    }

    /// TODO(prose): the skeleton's look: shadcn's `Skeleton`, `bg-muted
    /// rounded-md animate-pulse`: the muted fill, the radius of its style
    /// (square in Lyra) and Tailwind's two-second pulse down to half.
    #[cfg(feature = "extras")]
    pub fn skeleton(&self, style: &contract::Style) -> eggshell_ui_extras::skeleton::SkeletonStyle {
        let mut skeleton = eggshell_ui_extras::skeleton::SkeletonStyle::from_style(style);
        skeleton.fill = self.tokens.muted;
        skeleton.radius = self.tokens.radius_md();
        skeleton
    }

    /// The badge and chip look, after shadcn/ui's Badge in the preset's
    /// style (`cn-badge` in `style-*.css`): `h-5 gap-1 border
    /// border-transparent px-2 py-0.5 text-xs font-medium`, `pl-1.5` beside a
    /// leading icon and `pr-1.5` before a trailing one, `[&>svg]:size-3`,
    /// `rounded-4xl` in Vega and `rounded-none` in Lyra, and the base's
    /// `focus-visible:border-ring focus-visible:ring-[3px] ring-ring/50`. A
    /// chip's hover is the variant's `[a]:hover:` look.
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use eggshell_ui_shadcn::{Shadcn, variant};
    ///
    /// let shadcn = Shadcn::light();
    /// let badge = shadcn.badge(&shadcn.style());
    /// assert_eq!(badge.variants.default_key(), variant::PRIMARY);
    /// assert_eq!(badge.height, 20.0);
    /// ```
    #[cfg(feature = "extras")]
    pub fn badge(&self, style: &contract::Style) -> eggshell_ui_extras::badge::BadgeStyle {
        let t = &self.tokens;
        let dark = self.theme() == Theme::Dark;
        let clear = Color32::TRANSPARENT;
        let flat = |fill, text| Colors {
            fill,
            border: clear,
            text,
        };
        let ring = Focus {
            width: 3.0,
            color: t.ring.gamma_multiply(0.5),
        };
        let look = |rest: Colors, hover: Colors, focus: Focus| {
            let mut look = BoxLook::new(self.states(rest, hover)).bordered(1.0);
            look.paint.focus = Some(focus);
            look
        };
        let tint = |light: f32, deep: f32| {
            t.destructive
                .gamma_multiply(if dark { deep } else { light })
        };
        let outline = Colors {
            fill: clear,
            border: t.border,
            text: t.foreground,
        };
        let mut badge = eggshell_ui_extras::badge::BadgeStyle::from_style(style);
        badge.variants = Keyed::new(
            variant::PRIMARY,
            look(
                flat(t.primary, t.primary_foreground),
                flat(t.primary.gamma_multiply(0.8), t.primary_foreground),
                ring,
            ),
        )
        .with(
            variant::SECONDARY,
            look(
                flat(t.secondary, t.secondary_foreground),
                flat(t.secondary.gamma_multiply(0.8), t.secondary_foreground),
                ring,
            ),
        )
        .with(
            variant::OUTLINE,
            look(
                outline,
                Colors {
                    fill: t.muted,
                    text: t.muted_foreground,
                    ..outline
                },
                ring,
            ),
        )
        .with(
            variant::DESTRUCTIVE,
            look(
                flat(tint(0.1, 0.2), t.destructive),
                flat(tint(0.2, 0.2), t.destructive),
                Focus {
                    color: tint(0.2, 0.4),
                    ..ring
                },
            ),
        );
        badge.height = 20.0;
        badge.padding_x = 8.0;
        badge.icon_padding = self.tokens.points(1.5);
        badge.gap = 4.0;
        badge.radius = match t.style {
            ShadcnStyle::Vega => u8::MAX,
            ShadcnStyle::Lyra => 0,
        };
        badge.glyph = 12.0;
        badge.text = TextSpec {
            size: 12.0,
            line_height: 16.0,
            ..t.text_role_at(Weight::Medium)
        };
        badge.transition = Duration::from_millis(150);
        badge.remove.glyph = 12.0;
        badge.remove.padding_end = 6.0;
        badge
    }

    /// TODO(prose): the look of a command list's rows: shadcn's
    /// `CommandItem`, `data-selected:bg-muted data-selected:text-foreground`,
    /// and its list `max-h-72`.
    #[cfg(feature = "extras")]
    pub fn command(&self, style: &contract::Style) -> eggshell_ui_extras::command::CommandStyle {
        let mut command = eggshell_ui_extras::command::CommandStyle::from_style(style);
        command.selected_fill = self.tokens.muted;
        command.selected_text = self.tokens.foreground;
        command
    }

    /// The section header and disclosure button look, after shadcn/ui's
    /// Accordion in the preset's style, from `ref/shadcn` (f50887a):
    /// `style-vega.css` 3-17 (`cn-accordion-*`; Lyra's are
    /// `style-lyra.css` 3-17) and `bases/radix/ui/accordion.tsx` 41-69:
    /// - the trigger is `rounded-md py-4 text-sm font-medium hover:underline`
    ///   in Vega and `rounded-none py-2.5 text-xs` in Lyra, with the base's
    ///   `border border-transparent`, `focus-visible:border-ring` and the
    ///   style's ring (`ring-3`, `ring-1` in Lyra), and
    ///   `disabled:opacity-50`;
    /// - the content's inner block is `pt-0 pb-4` in Vega and `pb-2.5` in
    ///   Lyra;
    /// - the icon is `size-4 ml-auto text-muted-foreground`.
    ///
    /// Three values here are the user's picks from the previous registry's
    /// accordion, which the current source does not have: the icon turns
    /// with the region's progress (the source swaps `chevron-down` for
    /// `chevron-up` at once), it sits nudged `translate-y-0.5` (the source
    /// has no nudge), and there are 16 pt between the title and the icon
    /// (`gap-4`; the source's icon is `ml-auto`). A tree's disclosure button
    /// is not shadcn's: a 24 pt square with a `bg-accent` tint under the
    /// pointer, the user's pick (look 10c).
    ///
    /// TODO(prose)
    ///
    /// ```
    /// use eggshell_ui_shadcn::Shadcn;
    ///
    /// let shadcn = Shadcn::light();
    /// let disclosure = shadcn.disclosure(&shadcn.style());
    /// assert_eq!(disclosure.header.padding_y, 16.0);
    /// ```
    #[cfg(feature = "extras")]
    pub fn disclosure(
        &self,
        style: &contract::Style,
    ) -> eggshell_ui_extras::disclosure::DisclosureStyle {
        let t = &self.tokens;
        let lyra = t.style == ShadcnStyle::Lyra;
        let mut disclosure = eggshell_ui_extras::disclosure::DisclosureStyle::from_style(style);
        disclosure.section.size = 16.0;
        disclosure.tree.size = 16.0;
        disclosure.tone = Some(t.muted_foreground);
        disclosure.tone_opacity = Opacity::new(1.0);
        disclosure.nudge = 2.0;
        disclosure.header.ring = style.tokens.focus();
        disclosure.header.focus_border = Some(t.ring);
        disclosure.header.radius = if lyra { 0 } else { t.radius_md() };
        disclosure.header.padding_y = if lyra { 10.0 } else { 16.0 };
        disclosure.header.gap = 16.0;
        disclosure.header.text = t.text_role_at(Weight::Medium);
        disclosure.header.disabled = Opacity::new(0.5);
        disclosure.content_padding_bottom = if lyra { 10.0 } else { 16.0 };
        disclosure.button = 24.0;
        disclosure.button_hover = Some(t.accent);
        disclosure.separator = Some(t.border);
        disclosure
    }

    /// TODO(prose): the form section and labelled row look. shadcn has no
    /// grouped-form box, so the section is `FieldSet`'s semantics in `Card`'s
    /// frame, with the looks the user chose on #370. From `ref/shadcn`
    /// (f50887a), `registry/styles/style-{vega,lyra}.css`:
    /// - the box is `cn-card`'s `bg-card`, `rounded-xl` in Vega and
    ///   `rounded-none` in Lyra (vega 241, lyra 233). Its frame is a 1 pt
    ///   `border`, not the card's `ring-1 ring-foreground/10`, and has no
    ///   `shadow-xs`, as chosen;
    /// - the title is `cn-field-legend`'s label variant: `font-medium`,
    ///   `text-sm` and `mb-3` in Vega, `text-xs` and `mb-2.5` in Lyra
    ///   (vega 624, lyra 599);
    /// - a row is `cn-field`'s horizontal field: `gap-3` in Vega, `gap-2` in
    ///   Lyra (vega 632, lyra 607), `items-center` (`bases/radix/ui/field.tsx`
    ///   54);
    /// - sections stack `cn-field-set`'s `gap-6` in Vega, `gap-4` in Lyra
    ///   (vega 620, lyra 595);
    /// - the rule is `divide-y` in `border`, edge to edge; the box is `py-2`
    ///   and a row `px-4 py-2`, as chosen.
    ///
    /// ```
    /// use eggshell_ui_shadcn::Shadcn;
    ///
    /// let shadcn = Shadcn::light();
    /// let form = shadcn.form(&shadcn.style());
    /// assert_eq!(form.fill, shadcn.tokens.card);
    /// assert_eq!(form.radius, 14);
    /// ```
    #[cfg(feature = "extras")]
    pub fn form(&self, style: &contract::Style) -> eggshell_ui_extras::form::FormStyle {
        let t = &self.tokens;
        let vega = t.style == ShadcnStyle::Vega;
        let mut form = eggshell_ui_extras::form::FormStyle::from_style(style);
        form.fill = t.card;
        form.border = t.border;
        form.border_width = 1.0;
        form.radius = if vega {
            contract::corner(t.radius.xl())
        } else {
            0
        };
        form.padding_y = 8.0;
        form.row_padding_x = 16.0;
        form.row_padding_y = 8.0;
        form.rule = t.border;
        form.rule_width = 1.0;
        form.title = TextSpec {
            size: if vega { 14.0 } else { 12.0 },
            line_height: if vega { 20.0 } else { 16.0 },
            ..t.text_role_at(Weight::Medium)
        };
        form.title_color = t.foreground;
        form.title_gap = if vega { 12.0 } else { 10.0 };
        form.row_gap = if vega { 12.0 } else { 8.0 };
        form.row_min_height = t.control_height();
        form.label_color = t.foreground;
        form.value_color = t.muted_foreground;
        form.section_gap = if vega { 24.0 } else { 16.0 };
        form
    }

    /// TODO(prose): the hover card's look: shadcn's `HoverCardContent`
    /// (`w-64 rounded-lg bg-popover p-4 shadow-md duration-100`, 4 pt below,
    /// centred) in this preset's colours, with Radix's 700/300 ms delays.
    #[cfg(feature = "extras")]
    pub fn hover_card(&self) -> eggshell_ui_extras::hover_card::HoverCardStyle {
        let mut card = eggshell_ui_extras::hover_card::HoverCardStyle::from_style(&self.style());
        card.radius = self.tokens.radius_lg();
        card.transition = Duration::from_millis(100);
        card
    }

    /// TODO(prose): the Markdown widget's look: shadcn/typeset at its
    /// defaults, in this preset's colours.
    #[cfg(feature = "extras")]
    pub fn markdown(&self) -> eggshell_ui_extras::markdown::MarkdownStyle {
        eggshell_ui_extras::markdown::Typeset::default().markdown_style(&self.typeset_ink())
    }

    /// TODO(prose): the look of the dialog surface, the sheet and the
    /// popover: shadcn's `DialogContent`, `SheetContent` and
    /// `PopoverContent`. Placeholder values until the render round.
    #[cfg(feature = "extras")]
    pub fn dialog(&self, style: &contract::Style) -> eggshell_ui_extras::dialog::DialogStyle {
        use eggshell_ui_extras::dialog::{DialogStyle, Surface, shadow_lg};
        let t = &self.tokens;
        let mut dialog = DialogStyle::from_style(style);
        dialog.surface = Surface::new(t.background, t.foreground, t.border)
            .rounded(t.radius_lg())
            .shadow(shadow_lg());
        dialog.description_color = t.muted_foreground;
        dialog.cancel = variant::OUTLINE;
        dialog.default = variant::PRIMARY;
        dialog.close = variant::GHOST;
        dialog.close_size = size::ICON_SM;
        dialog.close_inset = 2.0;
        dialog
    }

    /// TODO(prose): the theme colours a [`Typeset`] takes, from this preset's
    /// tokens, for a typeset of other parameters.
    ///
    /// [`Typeset`]: eggshell_ui_extras::markdown::Typeset
    #[cfg(feature = "extras")]
    pub fn typeset_ink(&self) -> eggshell_ui_extras::markdown::TypesetInk {
        let t = &self.tokens;
        eggshell_ui_extras::markdown::TypesetInk {
            foreground: t.foreground,
            muted: t.muted_foreground,
            rule: t.border,
            code_fill: t.muted,
            radius: t.radius.lg(),
        }
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
