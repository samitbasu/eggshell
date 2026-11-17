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

use std::sync::Arc;

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
            Self::Unavailable => write!(f, "the clipboard is {}", self.detail()),
            Self::Failed(why) => write!(f, "the clipboard failed: {why}"),
        }
    }
}

impl ClipboardError {
    /// TODO(prose): the failure without the clipboard named, for a caller's own sentence.
    ///
    /// ```
    /// use eggshell_ui::ClipboardError;
    /// assert_eq!(ClipboardError::Failed("no owner".into()).detail(), "no owner");
    /// ```
    pub fn detail(&self) -> &str {
        match self {
            Self::Unavailable => "not available",
            Self::Failed(why) => why,
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

/// What delivered a [`Paste`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// The paste event of the chord, which carried what the backend read.
    Event,
    /// A read of the clipboard: a menu item, a button, or a chord that is
    /// not a paste event.
    Read,
}

impl Paste {
    /// TODO(prose): what a paste event of the chord carries, else `None`.
    ///
    /// ```
    /// use eggshell_ui::Paste;
    /// let event = egui::Event::Paste("text".into());
    /// assert_eq!(Paste::from_event(&event), Some(Paste::Text("text".into())));
    /// ```
    pub fn from_event(event: &egui::Event) -> Option<Self> {
        match event {
            egui::Event::Paste(text) => Some(Self::Text(text.clone())),
            egui::Event::PasteImage(image) => Some(Self::Image((**image).clone())),
            _ => None,
        }
    }
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
        self.source().read()
    }

    /// Puts `image` on the clipboard. Unlike `Context::copy_image`, it
    /// reports a failure.
    pub fn write_clipboard_image(&self, image: &ColorImage) -> Result<(), ClipboardError> {
        self.source().write_image(image)
    }

    fn source(&self) -> Arc<dyn ClipboardSource> {
        self.kept
            .with(|kept| kept.clipboard.clone())
            .unwrap_or_else(|| Arc::new(System))
    }
}

/// The system's clipboard.
struct System;

#[cfg(all(
    feature = "clipboard",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]
mod system {
    use std::sync::{Mutex, OnceLock};

    use egui::{Color32, ColorImage};

    use super::{Clipboard, ClipboardError, ClipboardSource, System};

    /// One handle for the process, as the clipboard is one for the system:
    /// on X11 the writer must outlive its write to serve the data.
    fn handle() -> &'static Mutex<Option<arboard::Clipboard>> {
        static HANDLE: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
        HANDLE.get_or_init(|| Mutex::new(arboard::Clipboard::new().ok()))
    }

    fn with<R>(
        f: impl FnOnce(&mut arboard::Clipboard) -> Result<R, arboard::Error>,
    ) -> Result<R, ClipboardError> {
        let mut guard = crate::sync::lock(handle());
        let clipboard = guard.as_mut().ok_or(ClipboardError::Unavailable)?;
        f(clipboard).map_err(|error| match error {
            arboard::Error::ClipboardNotSupported => ClipboardError::Unavailable,
            other => ClipboardError::Failed(other.to_string()),
        })
    }

    /// What nothing there to read is, as against a clipboard that refused.
    fn absent<T>(read: Result<T, arboard::Error>) -> Result<Option<T>, arboard::Error> {
        match read {
            Ok(content) => Ok(Some(content)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(other) => Err(other),
        }
    }

    impl ClipboardSource for System {
        fn read(&self) -> Result<Clipboard, ClipboardError> {
            with(|clipboard| {
                let text = absent(clipboard.get_text())?;
                let image = absent(clipboard.get_image())?.map(|image| {
                    ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.bytes)
                });
                Ok(Clipboard { text, image })
            })
        }

        fn write_image(&self, image: &ColorImage) -> Result<(), ClipboardError> {
            let bytes: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|pixel: &Color32| pixel.to_srgba_unmultiplied())
                .collect();
            with(|clipboard| {
                clipboard.set_image(arboard::ImageData {
                    width: image.width(),
                    height: image.height(),
                    bytes: bytes.into(),
                })
            })
        }
    }
}

#[cfg(not(all(
    feature = "clipboard",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
)))]
impl ClipboardSource for System {
    fn read(&self) -> Result<Clipboard, ClipboardError> {
        Err(ClipboardError::Unavailable)
    }

    fn write_image(&self, _image: &ColorImage) -> Result<(), ClipboardError> {
        Err(ClipboardError::Unavailable)
    }
}
