//! How badges and chips look.

use std::sync::Arc;
use std::time::Duration;

use eggshell_ui::contract::{Keyed, Look, Variant};
use eggshell_ui::{Opacity, Style, TextSpec, Weight};
use egui::{Context, Theme};

/// TODO(prose): the look of [`badge`](super::BadgeExt::badge)s and
/// [`chip`](super::BadgeExt::chip)s, one visual family: a chip is a badge
/// that takes input. Lengths are in points.
///
/// ```
/// use eggshell_ui_extras::badge::BadgeStyle;
///
/// let ctx = egui::Context::default();
/// let mut style = (*BadgeStyle::of(&ctx)).clone();
/// style.height = 24.0;
/// style.install_for(&ctx, ctx.theme());
/// assert_eq!(BadgeStyle::of(&ctx).height, 24.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BadgeStyle {
    /// The variants the style defines; the default entry is the variant of a
    /// badge that names none. A badge draws only the `rest` colours; a chip
    /// draws every state and the focus ring.
    pub variants: Keyed<Variant, Look>,
    /// Height.
    pub height: f32,
    /// Left and right padding.
    pub padding_x: f32,
    /// Left padding beside a leading icon, in place of `padding_x`.
    pub icon_padding: f32,
    /// Gap between the icon, the label and a chip's ×.
    pub gap: f32,
    /// Corner radius; `u8::MAX` makes a pill.
    pub radius: u8,
    /// Side of the leading icon.
    pub glyph: f32,
    /// The label's text role.
    pub text: TextSpec,
    /// How long a chip's state changes animate.
    pub transition: Duration,
    /// A removable chip's ×.
    pub remove: RemoveStyle,
}

/// TODO(prose): the × that removes a chip.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RemoveStyle {
    /// Side of the × glyph.
    pub glyph: f32,
    /// Right padding of a removable chip, in place of `padding_x`.
    pub padding_end: f32,
    /// How much of the chip's text colour the × keeps at rest.
    pub rest: Opacity,
    /// How much it keeps while the pointer is over it.
    pub hover: Opacity,
}

impl BadgeStyle {
    /// TODO(prose): a badge in the style's default button look, in 12 pt
    /// medium text.
    pub fn from_style(style: &Style) -> Self {
        let buttons = &style.button.variants;
        Self {
            variants: Keyed::new(Variant::DEFAULT, *buttons.get(buttons.default_key())),
            height: 20.0,
            padding_x: 8.0,
            icon_padding: 6.0,
            gap: 4.0,
            radius: u8::MAX,
            glyph: 12.0,
            text: TextSpec {
                size: 12.0,
                line_height: 16.0,
                weight: Weight::Medium,
                ..style.text.clone()
            },
            transition: style.button.transition,
            remove: RemoveStyle {
                glyph: 12.0,
                padding_end: 6.0,
                rest: Opacity::new(1.0),
                hover: Opacity::new(1.0),
            },
        }
    }

    /// TODO(prose): the badge style installed for the context's theme, else
    /// [`from_style`](Self::from_style) of its [`Style`].
    pub fn of(ctx: &Context) -> Arc<Self> {
        crate::styles::of(ctx, Self::from_style)
    }

    /// TODO(prose): makes `self` the badge style of `ctx` while its theme is
    /// `theme`.
    pub fn install_for(self, ctx: &Context, theme: Theme) {
        crate::styles::install_for(self, ctx, theme);
    }
}
