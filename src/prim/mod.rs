//! The library's primitives: display content it draws itself, and the
//! interactive ones (`text_field`, `select`) that
//! applications use as leaves.
//!
//! Each interactive primitive pairs `measure` with `show`; both take their
//! size from one helper (design decision D21), so what is measured is what is
//! used. Widget ids are derived from the leaf's `Ui`, so focus, cursor and
//! open popups follow a node through reordering.

pub mod select;
pub mod text_field;

use egui::{Color32, ImageSource, Painter, Rect};

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
    Icon(Icon),
    Image(ImageSource<'static>),
    Paint(PaintFn),
}
