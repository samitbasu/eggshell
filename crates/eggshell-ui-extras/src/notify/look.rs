//! What a notice looks like: its card, its stack and its sentence, from the
//! [`NotifyStyle`].

use std::sync::Arc;

use eggshell_ui::{Tw, tw};

use super::NotifyStyle;

pub(super) struct Look {
    pub(super) style: Arc<NotifyStyle>,
}

impl Look {
    pub(super) fn new(style: Arc<NotifyStyle>) -> Self {
        Self { style }
    }

    pub(super) fn stack(&self) -> Tw {
        tw().flex()
            .flex_col()
            .gap(self.style.stack_gap / 4.0)
            .sense_hover()
    }

    pub(super) fn card(&self) -> Tw {
        let style = &self.style;
        let card = tw()
            .flex()
            .items_center()
            .gap(style.gap / 4.0)
            .w_px(style.width)
            .p(style.padding / 4.0)
            .border(style.border_width)
            .border_color(style.border)
            .bg(style.fill)
            .text(style.text)
            .rounded_px(f32::from(style.radius))
            .role(style.role.clone());
        match style.shadow {
            Some(shadow) => card.shadow(shadow),
            None => card,
        }
    }

    pub(super) fn sentence(&self) -> Tw {
        tw().flex_1().min_w_0()
    }
}
