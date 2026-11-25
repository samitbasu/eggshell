//! What a notice looks like: its card, its stack and its sentence, from the
//! [`NotifyStyle`].

use std::sync::Arc;

use crate::{Tw, tw};

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

    /// An action's button: filled, in the card's colours swapped.
    pub(super) fn action(&self) -> Tw {
        let action = &self.style.action;
        tw().flex()
            .items_center()
            .shrink_0()
            .h_px(action.height)
            .px_px(action.padding_x)
            .rounded_px(f32::from(action.radius))
            .bg(action.fill)
            .text(action.text)
            .role(action.role.clone())
            .sense_click()
    }

    /// The × circle, half out over the card's top-left corner.
    pub(super) fn close(&self) -> Tw {
        let close = &self.style.close;
        tw().absolute()
            .top_px(-close.outset)
            .left_px(-close.outset)
            .flex()
            .items_center()
            .justify_center()
            .size_px(close.size)
            .rounded_full()
            .border(1.0)
            .border_color(close.border)
            .bg(close.fill)
            .text(close.text)
            .sense_click()
            .hover(|paint| paint.bg(close.hover_fill).border_color(close.hover_border))
    }

    pub(super) fn close_glyph(&self) -> Tw {
        tw().size_px(self.style.close.glyph).shrink_0()
    }

    pub(super) fn sentence(&self) -> Tw {
        tw().flex_1().min_w_0()
    }
}
