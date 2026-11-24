//! The tooltip's part, and the look a style without a preset draws it with.

use crate::sheet::Part;

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
