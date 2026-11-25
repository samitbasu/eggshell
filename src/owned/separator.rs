//! TODO(prose): a line between groups of content: shadcn's `separator`.

use crate::sheet::Key as SheetKey;
use crate::{Builder, tw};

/// TODO(prose): a separator's anatomy for the style's sheet.
///
/// TODO(prose): the keys its rules see: `data-[orientation=…]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the line.
    Root,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "separator.tsx";

    fn slot(self) -> &'static str {
        "separator"
    }
}

/// TODO(prose): which way a separator runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// TODO(prose): across, between blocks stacked down.
    Horizontal,
    /// TODO(prose): down, between items side by side.
    Vertical,
}

impl Orientation {
    fn key(self) -> SheetKey {
        match self {
            Self::Horizontal => SheetKey::Attr("orientation", "horizontal"),
            Self::Vertical => SheetKey::Attr("orientation", "vertical"),
        }
    }
}

impl<'b, 'f, A: 'static> Builder<'b, 'f, A> {
    /// TODO(prose): a separator running as `orientation` says. Like shadcn's,
    /// it is decorative: assistive technology does not see it.
    pub fn separator(&mut self, orientation: Orientation) {
        let classes = self.part(Part::Root, &[orientation.key()], tw());
        self.div(classes, |_| {});
    }
}
