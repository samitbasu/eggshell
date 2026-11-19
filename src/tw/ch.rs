//! Widths measured in the node's text: in `ch`, the advance of "0" in the
//! node's resolved font, or as wide as the widest of a set of templates
//! ([`Reserve`]). Both are only known once the builder has resolved the
//! node's text style.

use std::borrow::Cow;
use std::sync::Arc;

use taffy::Style;
use taffy::style::{Dimension, LengthPercentage, LengthPercentageAuto};

/// TODO(prose): text a node reserves room for: the widest of a set of
/// templates, measured in the node's text style. A digit in a template
/// stands for any digit.
///
/// ```
/// use eggshell_ui::{Reserve, tw};
///
/// const LINK: [&str; 4] = ["Not connected", "Listening", "Live 00.0 kHz", "Lost 000 s"];
/// let word = tw().min_w_text(Reserve::widest_of(LINK));
/// let count = tw().min_w_text(Reserve::digits(6)).tabular_nums().text_right();
/// # let _ = (word, count);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reserve(Arc<[Cow<'static, str>]>);

impl Reserve {
    /// TODO(prose): as wide as the widest of `templates`. An empty set
    /// reserves nothing.
    pub fn widest_of<S: Into<Cow<'static, str>>>(templates: impl IntoIterator<Item = S>) -> Self {
        Self(templates.into_iter().map(Into::into).collect())
    }

    /// TODO(prose): as wide as `n` digits: `widest_of(["0".repeat(n)])`.
    /// Zero reserves nothing.
    pub fn digits(n: u8) -> Self {
        match n {
            0 => Self::widest_of(None::<&'static str>),
            n => Self::widest_of(["0".repeat(usize::from(n))]),
        }
    }

    pub(crate) fn templates(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|template| template.as_ref())
    }
}

/// A minimum width the node's text decides.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MinWidth {
    /// `n` times the advance of "0".
    Ch(f32),
    /// The widest template's content width, plus the node's padding and border.
    Text(Reserve),
}

/// The widths a class list wrote in the node's text, awaiting its font.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextWidths {
    pub width: Option<f32>,
    pub min_width: Option<MinWidth>,
    pub max_width: Option<f32>,
}

impl TextWidths {
    /// Writes each pending width into `style` in points, measuring one `ch`
    /// with `advance` only if a `ch` width is pending, and the widest
    /// template with `widest` only if a reservation is.
    pub(crate) fn resolve(
        &self,
        style: &mut Style,
        advance: impl FnOnce() -> f32,
        widest: impl FnOnce(&Reserve) -> f32,
    ) {
        if *self == Self::default() {
            return;
        }
        let ch_min = match self.min_width {
            Some(MinWidth::Ch(n)) => Some(n),
            _ => None,
        };
        let ch = match [self.width, ch_min, self.max_width]
            .iter()
            .any(Option::is_some)
        {
            true => advance(),
            false => 0.0,
        };
        if let Some(n) = self.width {
            style.size.width = Dimension::length(n * ch);
        }
        match &self.min_width {
            Some(MinWidth::Ch(n)) => {
                style.min_size.width = LengthPercentageAuto::length(n * ch);
            }
            Some(MinWidth::Text(reserve)) => {
                let around = horizontal_frame(style);
                style.min_size.width = LengthPercentageAuto::length(widest(reserve) + around);
            }
            None => {}
        }
        if let Some(n) = self.max_width {
            style.max_size.width = LengthPercentageAuto::length(n * ch);
        }
    }

    /// The numbers the widths hold, for the finiteness check.
    pub(crate) fn numbers(&self) -> impl Iterator<Item = f32> {
        let min = match self.min_width {
            Some(MinWidth::Ch(n)) => Some(n),
            _ => None,
        };
        [self.width, min, self.max_width].into_iter().flatten()
    }

    /// The reservation pending, if one is.
    #[cfg(debug_assertions)]
    pub(crate) fn reserve(&self) -> Option<&Reserve> {
        match &self.min_width {
            Some(MinWidth::Text(reserve)) => Some(reserve),
            _ => None,
        }
    }
}

/// The definite horizontal padding and border of `style`, which a content
/// width is placed inside.
fn horizontal_frame(style: &Style) -> f32 {
    let points = |side: LengthPercentage| {
        let raw = side.into_raw();
        match raw.tag() {
            taffy::CompactLength::LENGTH_TAG => raw.value(),
            _ => 0.0,
        }
    };
    points(style.padding.left)
        + points(style.padding.right)
        + points(style.border.left)
        + points(style.border.right)
}
