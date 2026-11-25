//! How dialogs, sheets and popovers look.

use crate::contract::{Size, Variant};
use crate::{TextSpec, Weight};
use egui::Color32;
use egui::epaint::Shadow;

/// TODO(prose): the look of the dialog surface, the sheet and the popover.
/// Placeholder values until the render round settles them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DialogStyle {
    /// The box of a dialog or a sheet.
    pub surface: Surface,
    /// The box of a popover.
    pub popover: Surface,
    /// The title's text role.
    pub title: TextSpec,
    /// The description's text role.
    pub description: TextSpec,
    /// The variant of the footer's cancel button.
    pub cancel: Variant,
    /// The variant of the footer's default button.
    pub default: Variant,
    /// The variant of the close button (×).
    pub close: Variant,
    /// The size of the close button (×).
    pub close_size: Size,
}

/// A box's paint.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Surface {
    /// The fill.
    pub fill: Color32,
    /// The text colour inside.
    pub text: Color32,
    /// The border's colour.
    pub border: Color32,
    /// The border's width, in points.
    pub border_width: f32,
    /// The corner radius, in points.
    pub radius: u8,
    /// The drop shadow.
    pub shadow: Option<Shadow>,
}

impl Surface {
    /// A surface of these colours, with a border of `border_width` points
    /// and corners of `radius`.
    pub fn new(fill: Color32, text: Color32, border: Color32) -> Self {
        Self {
            fill,
            text,
            border,
            border_width: 1.0,
            radius: 0,
            shadow: None,
        }
    }

    /// With corners of `radius` points.
    pub fn rounded(self, radius: u8) -> Self {
        Self { radius, ..self }
    }

    /// With `shadow`.
    pub fn shadow(self, shadow: Shadow) -> Self {
        Self {
            shadow: Some(shadow),
            ..self
        }
    }
}

/// Tailwind's `shadow-lg`, without its negative spread.
pub fn shadow_lg() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 15,
        spread: 0,
        color: Color32::from_black_alpha(26),
    }
}

impl DialogStyle {
    /// A look derived from the core `style` and egui's `visuals`: the panel
    /// fill and window stroke for the box, the menu's popup for the popover,
    /// and the style's default button variant everywhere.
    pub(crate) fn from_base(style: &crate::contract::Base<'_>) -> Self {
        let popup = crate::owned::styles::Popover::of_tokens(style.tokens);
        let colors = popup.colors;
        let k = &style.tokens;
        let text = |size: f32, line_height: f32, weight: Weight| TextSpec {
            size,
            line_height,
            weight,
            ..style.text.clone()
        };
        Self {
            surface: Surface::new(k.background, k.foreground, k.border)
                .rounded(crate::contract::corner(k.radius.lg()))
                .shadow(shadow_lg()),
            popover: Surface {
                fill: colors.fill,
                text: colors.text,
                border: colors.border,
                border_width: popup.border_width,
                radius: popup.radius,
                shadow: Some(popup.shadow),
            },
            title: text(18.0, 18.0, Weight::SemiBold),
            description: text(14.0, 20.0, Weight::Normal),
            cancel: Variant::DEFAULT,
            default: Variant::DEFAULT,
            close: Variant::DEFAULT,
            close_size: Size::DEFAULT,
        }
    }
}
