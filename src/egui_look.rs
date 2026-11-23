//! egui's own look from a [`Style`]'s tokens: the visuals, spacing and text
//! sizes egui's widgets are drawn with outside eggshell's.

use egui::{
    Color32, CornerRadius, Margin, Shadow, Stroke, TextStyle, Theme,
    style::{HandleShape, WidgetVisuals},
    vec2,
};

use crate::Style;
use crate::contract::{Tokens, corner};

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
            corner_radius: CornerRadius::same(corner(self.radius.md())),
            fg_stroke: Stroke::new(1.0, fg),
            expansion: 0.0,
        }
    }
}

impl Style {
    /// TODO(prose): writes this style's look into egui's `style` for `theme`: its visuals, spacing and text sizes, from the tokens. egui's own widgets get an approximation of the design system; eggshell's controls draw it exactly.
    ///
    /// ```
    /// use eggshell_ui::Style;
    ///
    /// let mut egui = egui::Style::default();
    /// let style = Style::default();
    /// style.egui_style(egui::Theme::Light, &mut egui);
    /// assert_eq!(egui.visuals.panel_fill, style.tokens.background);
    /// ```
    pub fn egui_style(&self, theme: Theme, style: &mut egui::Style) {
        let t = &self.tokens;
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
        v.handle_shape = HandleShape::Circle;
        v.slider_trailing_fill = true;
        v.selection.bg_fill = t.primary.gamma_multiply(0.3);
        v.selection.stroke = Stroke::new(1.0, t.foreground);
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
        v.window_corner_radius = CornerRadius::same(corner(t.radius.lg()));
        v.menu_corner_radius = CornerRadius::same(corner(t.radius.md()));
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
