//! How tabs look beyond their tab list, which is drawn as the segmented
//! control is.

use std::sync::Arc;

use eggshell_ui::Style;
use egui::{Context, Theme};

/// TODO(prose): the look of [`tabs`](super::TabsExt::tabs) around the tab
/// list; the list and its tabs are drawn in the style's segmented look.
/// Lengths are in points.
///
/// ```
/// use eggshell_ui_extras::tabs::TabsStyle;
///
/// let ctx = egui::Context::default();
/// let mut style = (*TabsStyle::of(&ctx)).clone();
/// style.gap = 12.0;
/// style.install_for(&ctx, ctx.theme());
/// assert_eq!(TabsStyle::of(&ctx).gap, 12.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TabsStyle {
    /// Gap between the tab list and the panel.
    pub gap: f32,
}

impl TabsStyle {
    /// TODO(prose): an 8 pt gap between the list and the panel.
    pub fn from_style(_: &Style) -> Self {
        Self { gap: 8.0 }
    }

    /// TODO(prose): the tabs style installed for the context's theme, else
    /// [`from_style`](Self::from_style) of its [`Style`].
    pub fn of(ctx: &Context) -> Arc<Self> {
        crate::styles::of(ctx, Self::from_style)
    }

    /// TODO(prose): makes `self` the tabs style of `ctx` while its theme is
    /// `theme`.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        crate::styles::install_for(self, ctx, theme);
    }
}
