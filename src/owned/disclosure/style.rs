//! How section headers and disclosure buttons look.

use crate::{Angle, Icon, icons};

/// TODO(prose): the look of [`section`](super::SectionExt::section) headers
/// and [`disclosure`](super::Ext::disclosure) buttons, one
/// family: the button is a section header's chevron on its own. Lengths are
/// in points.
///
/// ```
/// use eggshell_ui::owned::disclosure::Style;
///
/// let mut style = eggshell_ui::Style::default();
/// let mut look = style.disclosure.clone();
/// look.header.gap = 8.0;
/// style.disclosure = look;
/// assert_eq!(style.disclosure.header.gap, 8.0);
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Style {
    /// A section header's chevron.
    pub section: Glyph,
    /// A disclosure button's chevron.
    pub tree: Glyph,
    /// How far a section's chevron sits below the top of its header's
    /// first line.
    pub nudge: f32,
    /// A section's header.
    pub header: HeaderStyle,
}

/// TODO(prose): a chevron, which turns by `turn` times how far open it is.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Glyph {
    /// The icon while closed.
    pub icon: Icon,
    /// How far it turns once open.
    pub turn: Angle,
}

impl Glyph {
    /// TODO(prose): a chevron drawing `icon`, turned by `turn` once open.
    pub fn new(icon: Icon, turn: Angle) -> Self {
        Self { icon, turn }
    }
}

/// TODO(prose): a section header.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HeaderStyle {
    /// Gap between the title and the chevron.
    pub gap: f32,
}

impl Style {
    /// TODO(prose): headers in the style's text, medium, with a muted
    /// `chevron-down` that turns over trailing a section's title and a
    /// `chevron-right` that turns down in a disclosure button.
    pub(crate) fn shadcn() -> Self {
        Self {
            section: Glyph::new(Icon::new(icons::CHEVRON_DOWN), Angle::degrees(180.0)),
            tree: Glyph::new(Icon::new(icons::CHEVRON_RIGHT), Angle::degrees(90.0)),
            nudge: 2.0,
            header: HeaderStyle { gap: 16.0 },
        }
    }
}
