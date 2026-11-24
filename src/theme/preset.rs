use crate::contract;
use egui::{Context, Id, Theme};
#[cfg(any(
    feature = "badge",
    feature = "command",
    feature = "dialog",
    feature = "disclosure",
    feature = "form",
    feature = "hover-card",
    feature = "markdown",
    feature = "notify",
    feature = "skeleton"
))]
use {
    crate::contract::{Colors, Focus, Keyed, Look as BoxLook, States},
    crate::{Opacity, TextSpec, Weight},
    egui::Color32,
    std::time::Duration,
};

use crate::theme::ShadcnStyle;
use crate::theme::{ContrastPair, FontSet, Palette, Tokens};
#[cfg(any(
    feature = "badge",
    feature = "command",
    feature = "dialog",
    feature = "disclosure",
    feature = "form",
    feature = "hover-card",
    feature = "markdown",
    feature = "notify",
    feature = "skeleton"
))]
use crate::theme::{size, variant};

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
    /// use eggshell_ui::theme::{Palette, Shadcn, ShadcnStyle};
    ///
    /// let style = Shadcn::themed(ShadcnStyle::Vega, Palette::new());
    /// let _layout = LayoutState::new().with_style(style);
    /// ```
    pub fn themed(
        style: ShadcnStyle,
        palette: Palette,
    ) -> crate::contract::Themed<std::sync::Arc<contract::Style>> {
        let half = |preset: Self| {
            std::sync::Arc::new(
                preset
                    .with_style(style)
                    .with_palette(palette)
                    .layout_style(),
            )
        };
        crate::contract::Themed {
            light: half(Self::light()),
            dark: half(Self::dark()),
        }
    }

    /// TODO(prose): this preset's style with its fonts and the extras' looks, as one half of a layout's style.
    ///
    /// ```
    /// use eggshell_ui::LayoutState;
    /// use eggshell_ui::contract::Themed;
    /// use eggshell_ui::theme::Shadcn;
    /// use std::sync::Arc;
    ///
    /// let style = Themed {
    ///     light: Arc::new(Shadcn::light().with_fonts(None).layout_style()),
    ///     dark: Arc::new(Shadcn::dark().with_fonts(None).layout_style()),
    /// };
    /// let _layout = LayoutState::new().with_style(style);
    /// ```
    pub fn layout_style(&self) -> contract::Style {
        self.style()
    }

    /// This preset as the [`Style`](crate::Style) contract.
    pub fn style(&self) -> contract::Style {
        contract::Style::new(&self.tokens)
    }

    /// A box's colours at rest and under the pointer, with the focused
    /// border, `focus-visible:border-ring`, over the page.
    #[cfg(feature = "badge")]
    fn states(t: &Tokens, rest: Colors, hover: Colors) -> States {
        States {
            focused: Colors {
                border: t.ring,
                ..rest
            },
            ..States::new(rest, hover, t.background)
        }
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
    /// AA contrast as [`Problem::LowContrast`](crate::Problem::LowContrast),
    /// unless the tokens accept it or it is shadcn's own colour; it never
    /// changes a colour.
    pub fn apply(&self, ctx: &Context) {
        crate::theme::warn_low_contrast(ctx, &self.tokens);
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
    #[cfg(feature = "notify")]
    pub(crate) fn notify(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::notify::NotifyStyle {
        let mut notify = crate::owned::notify::NotifyStyle::from_base(style);
        notify.fill = t.popover;
        notify.border = t.border;
        notify.text = t.popover_foreground;
        notify.border_width = 1.0;
        notify.radius = match t.style {
            ShadcnStyle::Vega => contract::corner(t.radius.lg() * 1.8),
            ShadcnStyle::Lyra => 0,
        };
        notify.shadow = Some(egui::Shadow {
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
        let dark = t.theme == Theme::Dark;
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
    #[cfg(feature = "skeleton")]
    pub(crate) fn skeleton(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::skeleton::SkeletonStyle {
        let mut skeleton = crate::owned::skeleton::SkeletonStyle::from_base(style);
        skeleton.fill = t.muted;
        skeleton.radius = t.radius_md();
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
    /// use eggshell_ui::theme::{Shadcn, variant};
    ///
    /// let shadcn = Shadcn::light();
    /// let badge = shadcn
    ///     .style()
    ///     .badge;
    /// assert_eq!(badge.variants.default_key(), variant::PRIMARY);
    /// assert_eq!(badge.height, 20.0);
    /// ```
    #[cfg(feature = "badge")]
    pub(crate) fn badge(t: &Tokens, style: &contract::Base<'_>) -> crate::owned::badge::BadgeStyle {
        let dark = t.theme == Theme::Dark;
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
            let mut look = BoxLook::new(Self::states(t, rest, hover)).bordered(1.0);
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
        let mut badge = crate::owned::badge::BadgeStyle::from_base(style);
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
        badge.icon_padding = t.points(1.5);
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
    #[cfg(feature = "command")]
    pub(crate) fn command(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::command::CommandStyle {
        let mut command = crate::owned::command::CommandStyle::from_base(style);
        command.selected_fill = t.muted;
        command.selected_text = t.foreground;
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
    /// use eggshell_ui::theme::Shadcn;
    ///
    /// let shadcn = Shadcn::light();
    /// let disclosure = shadcn
    ///     .style()
    ///     .disclosure;
    /// assert_eq!(disclosure.header.padding_y, 16.0);
    /// ```
    #[cfg(feature = "disclosure")]
    pub(crate) fn disclosure(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::disclosure::DisclosureStyle {
        let lyra = t.style == ShadcnStyle::Lyra;
        let mut disclosure = crate::owned::disclosure::DisclosureStyle::from_base(style);
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
    /// use eggshell_ui::theme::Shadcn;
    ///
    /// let shadcn = Shadcn::light();
    /// let form = shadcn
    ///     .style()
    ///     .form;
    /// assert_eq!(form.fill, shadcn.tokens.card);
    /// assert_eq!(form.radius, 14);
    /// ```
    #[cfg(feature = "form")]
    pub(crate) fn form(t: &Tokens, style: &contract::Base<'_>) -> crate::owned::form::FormStyle {
        let vega = t.style == ShadcnStyle::Vega;
        let mut form = crate::owned::form::FormStyle::from_base(style);
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
    #[cfg(feature = "hover-card")]
    pub(crate) fn hover_card(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::hover_card::HoverCardStyle {
        let mut card = crate::owned::hover_card::HoverCardStyle::from_base(style);
        card.radius = t.radius_lg();
        card.transition = Duration::from_millis(100);
        card
    }

    /// TODO(prose): the Markdown widget's look: shadcn/typeset at its
    /// defaults, in this preset's colours.
    #[cfg(feature = "markdown")]
    pub(crate) fn markdown(t: &Tokens) -> crate::owned::markdown::MarkdownStyle {
        crate::owned::markdown::Typeset::default().markdown_style(&Self::typeset_ink(t))
    }

    /// TODO(prose): the look of the dialog surface, the sheet and the
    /// popover: shadcn's `DialogContent`, `SheetContent` and
    /// `PopoverContent`. Placeholder values until the render round.
    #[cfg(feature = "dialog")]
    pub(crate) fn dialog(
        t: &Tokens,
        style: &contract::Base<'_>,
    ) -> crate::owned::dialog::DialogStyle {
        use crate::owned::dialog::{DialogStyle, Surface, shadow_lg};
        let mut dialog = DialogStyle::from_base(style);
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
    /// [`Typeset`]: crate::owned::markdown::Typeset
    #[cfg(feature = "markdown")]
    pub(crate) fn typeset_ink(t: &Tokens) -> crate::owned::markdown::TypesetInk {
        crate::owned::markdown::TypesetInk {
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
