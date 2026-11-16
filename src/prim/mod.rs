//! The library's primitives: display content it draws itself, and the
//! interactive `text_field` that applications use as a leaf.
//!
//! `text_field` pairs `measure` with `show`; both take their size from one
//! helper (design decision D21), so what is measured is what is used. Widget
//! ids are derived from the leaf's `Ui`, so focus and cursor follow a node
//! through reordering.

pub mod text_field;

use egui::{Color32, ImageSource, Painter, Rect, WidgetText};

use crate::{Angle, Mirror};

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
    pub mirror: Mirror,
}

impl Icon {
    /// An unrotated, unmirrored icon.
    pub fn new(src: ImageSource<'static>) -> Self {
        Self {
            src,
            rotate: Angle::ZERO,
            mirror: Mirror::None,
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
    pub fn mirrored(self, mirror: Mirror) -> Self {
        Self { mirror, ..self }
    }
}

pub(crate) type PaintFn = Box<dyn Fn(&Painter, Rect, Color32)>;

/// What a node displays, when it displays anything.
pub(crate) enum Prim {
    Text(String),
    /// Text with egui's formatting; never a [`WidgetText::Text`].
    Rich(WidgetText),
    Spans(crate::spans::Spans),
    Icon(Icon),
    Image(ImageSource<'static>),
    Paint(PaintFn),
}

impl Prim {
    /// The text this shows, if it is text.
    pub(crate) fn text(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Text(text) => Some(text.as_str().into()),
            Self::Rich(text) => Some(text.text().into()),
            Self::Spans(spans) => Some(spans.text().into()),
            Self::Icon(_) | Self::Image(_) | Self::Paint(_) => None,
        }
    }
}
