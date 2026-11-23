//! The tooltip's part, and the look a style without a preset draws it with.

use crate::TextSpec;
use crate::contract::Look;
use crate::sheet::{Part, Sheet};
use egui::Vec2;

/// TODO(prose): the anatomy of a tooltip: shadcn's `tooltip`, of which eggshell draws the content. The arrow takes its colour from the content's background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TooltipPart {
    /// TODO(prose): the box that holds what the tooltip says.
    Content,
}

impl Part for TooltipPart {
    const SOURCE: &'static str = "tooltip.tsx";

    fn slot(self) -> &'static str {
        "tooltip-content"
    }
}

/// What a style without a preset draws a tooltip's box with.
pub(crate) struct TooltipDefaults {
    pub popup: Look,
    pub radius: u8,
    pub padding: Vec2,
    pub text: TextSpec,
    pub max_width: f32,
}

/// Writes `tooltip` as the tooltip's rule in `sheet`.
pub(crate) fn tooltip_defaults(sheet: &mut Sheet, tooltip: TooltipDefaults) {
    sheet.replace(TooltipPart::Content, move |t, _| {
        let colors = tooltip.popup.paint.states.rest;
        let classes = t
            .flex()
            .flex_col()
            .max_w_px(tooltip.max_width)
            .px_px(tooltip.padding.x)
            .py_px(tooltip.padding.y)
            .border(tooltip.popup.border_width)
            .border_color(colors.border)
            .bg(colors.fill)
            .text(colors.text)
            .rounded_px(f32::from(tooltip.radius))
            .role(tooltip.text.clone());
        match tooltip.popup.paint.shadow {
            Some(shadow) => classes.shadow(shadow),
            None => classes,
        }
    });
}
