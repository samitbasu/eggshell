//! Display content drawn by the library.

use egui::{ImageSource, Painter, Rect};

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

pub(crate) type PaintFn = Box<dyn Fn(&Painter, Rect)>;

/// What a node displays, when it displays anything.
pub(crate) enum Prim {
    Text(String),
    Icon(Icon),
    Image(ImageSource<'static>),
    Paint(PaintFn),
}
