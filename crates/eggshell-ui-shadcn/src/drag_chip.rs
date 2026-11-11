//! The drag chip: shadcn's tooltip look as the image of a drag.

use eggshell_ui::{Builder, NodeHandle, TextSpec, drag_grab, tw};
use egui::{FontFamily, vec2};

use crate::{Shadcn, tokens};

/// How far the chip sits from the pointer, down and to the right, in points,
/// so that it does not cover the cursor.
const OFFSET: f32 = 12.0;

/// `text-xs`.
const TEXT_XS: TextSpec = TextSpec {
    size: 12.0,
    line_height: 16.0,
    family: FontFamily::Proportional,
};

/// Builds a chip reading `text` at the pointer, for a
/// [`drag_preview`](NodeHandle::drag_preview): shadcn's tooltip look, the
/// `primary` background with `primary-foreground` text, `rounded-md`,
/// `text-xs` and `px-3 py-1.5`, from the tokens of the context's current theme.
///
/// ```html
/// <div class="absolute whitespace-nowrap rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground">text</div>
/// ```
pub fn drag_chip<A>(b: &mut Builder<'_, '_, A>, text: impl Into<String>) {
    let t = tokens(b.ctx()).unwrap_or_else(|| Shadcn::of(b.ctx()).tokens);
    let at = drag_grab(b.ctx()) + vec2(OFFSET, OFFSET);
    b.div(
        tw().absolute()
            .left_px(at.x)
            .top_px(at.y)
            .px(3.0)
            .py(1.5)
            .rounded_px(f32::from(t.radius_md()))
            .bg(t.primary)
            .text(t.primary_foreground)
            .whitespace_nowrap(),
        |b| b.text(tw().role(TEXT_XS), text),
    );
}

/// [`drag_chip`] as a drag source's image.
pub trait DragChip {
    /// Makes a chip reading `text` the image of the drag of this source.
    fn with_drag_chip(self, text: impl Into<String>) -> Self;
}

impl<'f, A: 'static> DragChip for NodeHandle<'_, 'f, A> {
    fn with_drag_chip(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.drag_preview(move |b| drag_chip(b, text))
    }
}
