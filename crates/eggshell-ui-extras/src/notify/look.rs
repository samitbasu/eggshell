//! What a notice looks like until the render round settles its style: the
//! page's popover surface, at sonner's sizes.

use eggshell_ui::{Style, TextSpec, Tw, tw};
use egui::epaint::Shadow;

pub(super) struct Look {
    fill: egui::Color32,
    border: egui::Color32,
    text: egui::Color32,
    border_width: f32,
    radius: u8,
    shadow: Option<Shadow>,
    role: TextSpec,
}

const WIDTH: f32 = 356.0;
/// Tailwind spacing units.
const PADDING: f32 = 4.0;
const INNER_GAP: f32 = 1.5;
const STACK_GAP: f32 = 3.5;

impl Look {
    pub(super) fn of(style: &Style) -> Self {
        let popup = &style.menu.popup;
        let colors = popup.paint.states.rest;
        Self {
            fill: colors.fill,
            border: colors.border,
            text: colors.text,
            border_width: popup.border_width,
            radius: style.menu.radius,
            shadow: popup.paint.shadow,
            role: style
                .menu
                .text
                .clone()
                .unwrap_or_else(|| style.text.clone()),
        }
    }

    pub(super) fn stack(&self) -> Tw {
        tw().flex().flex_col().gap(STACK_GAP).sense_hover()
    }

    pub(super) fn card(&self) -> Tw {
        let card = tw()
            .flex()
            .items_center()
            .gap(INNER_GAP)
            .w_px(WIDTH)
            .p(PADDING)
            .border(self.border_width)
            .border_color(self.border)
            .bg(self.fill)
            .text(self.text)
            .rounded_px(f32::from(self.radius))
            .role(self.role.clone());
        match self.shadow {
            Some(shadow) => card.shadow(shadow),
            None => card,
        }
    }

    pub(super) fn sentence(&self) -> Tw {
        tw().flex_1().min_w_0()
    }
}
