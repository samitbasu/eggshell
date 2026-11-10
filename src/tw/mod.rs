//! The `Tw` style builder, in Tailwind vocabulary.
//!
//! A [`Tw`] holds fully resolved values: points, [`egui::Color32`] and
//! [`TextSpec`](crate::TextSpec). Theming is the application's concern.
//!
//! Layout classes write a [`taffy::Style`]. One spacing unit is 4 points, so
//! `.p(1.5)` is `p-1.5` is 6 points. Methods suffixed `_px` take points
//! directly, `_frac(n, d)` is the fraction `n/d`, `_full` is 100%, and `_auto`
//! is the `auto` keyword. Layout methods take bare floats because the method
//! name carries the unit, exactly as in Tailwind.

mod easing;
mod grid;
mod layout;
mod merge;
mod paint;
mod text;
mod units;
mod variants;

use taffy::{Display, Style};

use crate::text::TextClasses;

pub use easing::Easing;
pub use grid::Track;
use merge::{Layout, Touched};
pub use paint::{BoxPaint, Divide, Ring, Sides};
pub use units::{Angle, Flip, Opacity, Progress};
pub use variants::Paint;
pub(crate) use variants::{Focusability, Grouping, Look, Presence, Sensing, State, Variants};

/// One Tailwind space unit, in points.
const SPACE_UNIT: f32 = 4.0;

/// A node's classes: layout, box paint, inherited text and behaviour.
#[derive(Clone, Debug)]
pub struct Tw {
    pub(crate) style: Style,
    pub(crate) paint: BoxPaint,
    pub(crate) text: TextClasses,
    pub(crate) z: Option<i16>,
    pub(crate) inert: bool,
    pub(crate) sensing: Sensing,
    pub(crate) focusability: Focusability,
    pub(crate) grouping: Grouping,
    pub(crate) variants: Variants,
    touched: Touched,
}

/// A fresh class list. Like CSS, it starts as `display: block`.
pub fn tw() -> Tw {
    Tw {
        style: Style {
            display: Display::Block,
            ..Style::default()
        },
        paint: BoxPaint::default(),
        text: TextClasses::default(),
        z: None,
        inert: false,
        sensing: Sensing::None,
        focusability: Focusability::Tab,
        grouping: Grouping::Ungrouped,
        variants: Variants::default(),
        touched: Touched::default(),
    }
}

impl Tw {
    /// The layout style the classes have produced so far.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// The box paint the classes have produced so far.
    pub fn paint_classes(&self) -> &BoxPaint {
        &self.paint
    }

    /// The text classes set so far.
    pub fn text_classes(&self) -> &TextClasses {
        &self.text
    }

    /// Sibling paint and hit order; never affects layout.
    pub fn z(mut self, z: i16) -> Self {
        self.z = Some(z);
        self
    }

    /// Makes the subtree non-interactive, like HTML's `inert` attribute: its
    /// widgets are disabled and take no input or focus.
    ///
    /// As on the web, inert content looks unchanged. To make it look disabled,
    /// add opacity, which fades boxes, text and widgets together (Tailwind's
    /// `inert:opacity-50`):
    ///
    /// ```
    /// use eggshell_ui::{Opacity, tw};
    /// let dimmed = tw().inert().opacity(Opacity::new(0.5));
    /// ```
    pub fn inert(mut self) -> Self {
        self.inert = true;
        self
    }

    fn styled(mut self, set: impl FnOnce(&mut Layout<'_>)) -> Self {
        self.write_layout(set);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_as_a_block_with_default_taffy_style() {
        let tw = tw();
        assert_eq!(
            *tw.style(),
            Style {
                display: Display::Block,
                ..Style::default()
            }
        );
        assert_eq!(
            taffy::Style::<String>::default().display,
            Display::Flex,
            "precondition: taffy defaults to flex, so tw() must set block itself"
        );
    }

    #[test]
    fn z_and_inert_do_not_touch_layout() {
        let styled = tw().z(30).inert();
        assert_eq!(styled.z, Some(30));
        assert!(styled.inert);
        assert_eq!(*styled.style(), *tw().style());
    }
}
