//! Widths in `ch`: the advance of "0" in the node's resolved font, which is
//! only known once the builder has resolved the node's text style.

use taffy::Style;
use taffy::style::{Dimension, LengthPercentageAuto};

/// The widths a class list wrote in `ch`, awaiting the font they are measured in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ChWidths {
    pub width: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
}

impl ChWidths {
    /// Writes each pending width into `style` in points, measuring one `ch`
    /// with `advance` only if any is pending.
    pub(crate) fn resolve(self, style: &mut Style, advance: impl FnOnce() -> f32) {
        if self == Self::default() {
            return;
        }
        let ch = advance();
        if let Some(n) = self.width {
            style.size.width = Dimension::length(n * ch);
        }
        if let Some(n) = self.min_width {
            style.min_size.width = LengthPercentageAuto::length(n * ch);
        }
        if let Some(n) = self.max_width {
            style.max_size.width = LengthPercentageAuto::length(n * ch);
        }
    }
}
