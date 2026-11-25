//! How badges and chips look.

use std::time::Duration;

use crate::contract::{Colors, Keyed, Look, States, Variant};
use crate::{TextSpec, Weight};
use egui::Color32;

/// TODO(prose): the look of [`badge`](super::Ext::badge)s and
/// [`chip`](super::Ext::chip)s, one visual family: a chip is a badge
/// that takes input. Lengths are in points.
///
/// ```
/// use eggshell_ui::owned::badge::Style;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = style.badge.clone();
/// look.height = 24.0;
/// style.badge = look;
/// assert_eq!(style.badge.height, 24.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// The variants the style defines; the default entry is the variant of a
    /// badge that names none. A badge draws only the `rest` colours; a chip
    /// draws every state and the focus ring.
    pub variants: Keyed<Variant, Look>,
    /// Height.
    pub height: f32,
    /// Left and right padding.
    pub padding_x: f32,
    /// Gap between the icon, the label and a chip's ×.
    pub gap: f32,
    /// Corner radius; `u8::MAX` makes a pill.
    pub radius: u8,
    /// The label's text role.
    pub text: TextSpec,
    /// How long a chip's state changes animate.
    pub transition: Duration,
}

impl Style {
    /// TODO(prose): a badge in the style's secondary colours, the accent's
    /// under the pointer, in 12 pt medium text.
    pub(crate) fn from_base(style: &crate::contract::Base<'_>) -> Self {
        let t = &style.tokens;
        let flat = |fill, text| Colors {
            fill,
            border: Color32::TRANSPARENT,
            text,
        };
        let look = Look::new(States::new(
            flat(t.secondary, t.secondary_foreground),
            flat(t.accent, t.accent_foreground),
            t.background,
        ));
        Self {
            variants: Keyed::new(Variant::DEFAULT, look),
            height: 20.0,
            padding_x: 8.0,
            gap: 4.0,
            radius: u8::MAX,
            text: TextSpec {
                size: 12.0,
                line_height: 16.0,
                weight: Weight::Medium,
                ..style.text.clone()
            },
            transition: style.toggle.transition,
        }
    }
}
