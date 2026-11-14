//! The library's primitives: display content it draws itself, and the
//! interactive `text_field` that applications use as a leaf.
//!
//! `text_field` pairs `measure` with `show`; both take their size from one
//! helper (design decision D21), so what is measured is what is used. Widget
//! ids are derived from the leaf's `Ui`, so focus and cursor follow a node
//! through reordering.

pub mod text_field;

use egui::{Color32, ImageSource, Painter, Rect, WidgetText};

use crate::{Angle, Flip};

/// An icon: an image source tinted with the inherited text colour.
///
/// Applications define their icon set as functions returning `Icon`.
#[derive(Clone, Debug)]
pub struct Icon {
    /// The image to draw.
    pub src: ImageSource<'static>,
    /// Rotation about the centre.
    pub rotate: Angle,
    /// Mirroring, applied before the rotation.
    pub flip: Flip,
}

impl Icon {
    /// An unrotated, unmirrored icon.
    pub fn new(src: ImageSource<'static>) -> Self {
        Self {
            src,
            rotate: Angle::ZERO,
            flip: Flip::None,
        }
    }

    /// This icon rotated by `angle`.
    pub fn rotated(self, angle: Angle) -> Self {
        Self {
            rotate: angle,
            ..self
        }
    }

    /// This icon mirrored.
    pub fn flipped(self, flip: Flip) -> Self {
        Self { flip, ..self }
    }
}

pub(crate) type PaintFn = Box<dyn Fn(&Painter, Rect, Color32)>;

/// What a node displays, when it displays anything.
pub(crate) enum Prim {
    Text(String),
    /// Text with egui's formatting; never a [`WidgetText::Text`].
    Rich(WidgetText),
    Icon(Icon),
    Image(ImageSource<'static>),
    Paint(PaintFn),
}

impl Prim {
    /// The text this shows, if it is text.
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Rich(text) => Some(text.text()),
            Self::Icon(_) | Self::Image(_) | Self::Paint(_) => None,
        }
    }
}
