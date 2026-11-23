//! The tooltip's part, and the sheet rule that gives it today's
//! [`TooltipStyle`] look.
//!
//! TODO(#446): this rule is written from the values `TooltipStyle` holds, as
//! the step that moves the mechanism onto the sheet; the generated rules of
//! shadcn's `cn-tooltip-content` replace it.

use crate::TextSpec;
use crate::contract::TooltipStyle;
use crate::sheet::{Part, Sheet};

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

/// Writes the rule that draws `tooltip`'s box, in the text role `text`, at the
/// sheet's current layer.
pub(crate) fn write_rules(sheet: &mut Sheet, tooltip: &TooltipStyle, text: &TextSpec) {
    let (tooltip, text) = (tooltip.clone(), text.clone());
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
            .role(text.clone());
        match tooltip.popup.paint.shadow {
            Some(shadow) => classes.shadow(shadow),
            None => classes,
        }
    });
}
