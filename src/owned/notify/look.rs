//! What a notice looks like: its card, its stack and its sentence, from the
//! [`Style`].

use std::sync::Arc;

use crate::{Tw, tw};

use super::Style;

pub(super) struct Look {
    pub(super) style: Arc<Style>,
}

impl Look {
    pub(super) fn new(style: Arc<Style>) -> Self {
        Self { style }
    }

    pub(super) fn stack(&self) -> Tw {
        tw().flex()
            .flex_col()
            .gap_px(self.style.stack_gap)
            .sense_hover()
    }

    pub(super) fn card(&self) -> Tw {
        let style = &self.style;
        let card = tw()
            .relative()
            .flex()
            .items_center()
            .gap_px(style.gap)
            .w_px(style.width)
            .p_px(style.padding)
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

    /// An action's button; its look is `Part::ActionButton`'s.
    pub(super) fn action(&self) -> Tw {
        tw().flex().items_center().shrink_0().sense_click()
    }

    /// The ×; its look is `Part::CloseButton`'s.
    pub(super) fn close(&self) -> Tw {
        tw().absolute()
            .flex()
            .items_center()
            .justify_center()
            .sense_click()
    }

    pub(super) fn sentence(&self) -> Tw {
        tw().flex_1().min_w_0()
    }
}
