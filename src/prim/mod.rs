//! The library's primitives: display content it draws itself, and the
//! interactive `text_field` that applications use as a leaf.
//!
//! `text_field` pairs `measure` with `show`; both take their size from one
//! helper (design decision D21), so what is measured is what is used. Widget
//! ids are derived from the leaf's `Ui`, so focus and cursor follow a node
//! through reordering.

use egui::{Color32, ImageSource, Painter, Rect, WidgetText};

use crate::{Angle, Mirror, Progress};

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

/// An icon that turns with how far open what it marks is: a chevron, which
/// a section header turns over and a navigation menu's trigger turns up.
///
/// ```
/// use eggshell_ui::{Angle, Icon, Progress, TurningIcon, icons};
///
/// let chevron = TurningIcon::new(Icon::new(icons::CHEVRON_DOWN), Angle::degrees(180.0));
/// assert_eq!(chevron.at(Progress::START).rotate, Angle::ZERO);
/// assert_eq!(chevron.at(Progress::END).rotate, Angle::degrees(180.0));
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TurningIcon {
    /// The icon while closed.
    pub icon: Icon,
    /// How far it turns once open.
    pub turn: Angle,
}

impl TurningIcon {
    /// `icon`, turned by `turn` once open.
    pub fn new(icon: Icon, turn: Angle) -> Self {
        Self { icon, turn }
    }

    /// `icon`, which does not turn.
    pub fn still(icon: Icon) -> Self {
        Self::new(icon, Angle::ZERO)
    }

    /// The icon turned by the turn times `open`: unturned closed, all the
    /// way open.
    pub fn at(&self, open: Progress) -> Icon {
        self.icon
            .clone()
            .rotated(Angle::radians(f32::from(self.turn) * f32::from(open)))
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
