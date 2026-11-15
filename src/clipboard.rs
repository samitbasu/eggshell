//! The clipboard, read on demand.
//!
//! egui reads the clipboard only at the paste chord, and sends what it found
//! as an event. A Paste menu item or button is a click, with no event, so it
//! asks: [`Shell::read_clipboard`]. The source of the answer belongs to the
//! layout ([`LayoutState::with_clipboard`](crate::LayoutState::with_clipboard)),
//! so a test installs a [`FakeClipboard`](crate::testing::FakeClipboard) where
//! there is no display, and two layouts of a context can differ.
//!
//! A [`Command::paste`](crate::Command::paste) gets the same [`Paste`] from
//! the chord (the event's contents) and from a menu click (a read), collapsed
//! the same way.

use egui::ColorImage;

use crate::shell::Shell;

/// What the clipboard holds. Both are present when the clipboard holds both.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clipboard {
    /// The text, if the clipboard holds any.
    pub text: Option<String>,
    /// The image, if the clipboard holds one.
    pub image: Option<ColorImage>,
}

/// Why the clipboard could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    /// There is no clipboard to reach: no display, a build without the
    /// `clipboard` feature, or wasm.
    Unavailable,
    /// The system refused.
    Failed(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => f.write_str("the clipboard is not available"),
            Self::Failed(why) => write!(f, "the clipboard failed: {why}"),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// Where a layout's clipboard reads and writes go. The system's by default;
/// a test installs [`FakeClipboard`](crate::testing::FakeClipboard).
pub trait ClipboardSource: Send + Sync {
    /// What the clipboard holds.
    fn read(&self) -> Result<Clipboard, ClipboardError>;
    /// Puts `image` on the clipboard.
    fn write_image(&self, image: &ColorImage) -> Result<(), ClipboardError>;
}

/// What a paste carries: one thing, as egui's paste events do. Text wins over
/// an image, as in egui-winit, which sends the image only when there is no
/// text.
#[derive(Clone, Debug, PartialEq)]
pub enum Paste {
    /// The clipboard's text.
    Text(String),
    /// The clipboard's image, when it holds no text.
    Image(ColorImage),
    /// Nothing to paste.
    Nothing,
}

impl From<Clipboard> for Paste {
    fn from(clipboard: Clipboard) -> Self {
        match clipboard {
            Clipboard {
                text: Some(text), ..
            } => Self::Text(text),
            Clipboard {
                image: Some(image), ..
            } => Self::Image(image),
            Clipboard {
                text: None,
                image: None,
            } => Self::Nothing,
        }
    }
}

impl Shell {
    /// What the clipboard holds now, from this layout's source.
    pub fn read_clipboard(&self) -> Result<Clipboard, ClipboardError> {
        Err(ClipboardError::Unavailable)
    }

    /// Puts `image` on the clipboard. Unlike `Context::copy_image`, it
    /// reports a failure.
    pub fn write_clipboard_image(&self, image: &ColorImage) -> Result<(), ClipboardError> {
        let _ = image;
        Err(ClipboardError::Unavailable)
    }
}
