//! What a [`LayoutState`](crate::LayoutState) remembers between frames.
//!
//! Eggshell's own state lives in the `LayoutState` that the app holds, so it
//! is per layout by construction: an app that lays out a bar and a page in
//! one egui context cannot have one overwrite the other's. egui keeps egui's
//! own (focus, text-edit memory, the clock) in its context, and diagnostics
//! (the debug overlay, the continuity recorder) are context-wide by design.

use std::sync::{Arc, Mutex, PoisonError};

use egui::Rect;

use crate::nav::Active;
use crate::overlay_input::Typing;
use crate::stack::Stack;
use crate::transition::Tracking;

#[derive(Default)]
pub(crate) struct Kept {
    pub(crate) stack: Stack,
    pub(crate) listbox: Active,
    pub(crate) typing: Typing,
    pub(crate) animation: Tracking,
    /// The area the last layout placed things in.
    pub(crate) area: Option<Rect>,
}

/// Shared by a `LayoutState` and the frames it builds, which outlive a borrow
/// of it.
#[derive(Clone, Default)]
pub(crate) struct Handle(Arc<Mutex<Kept>>);

impl Handle {
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut Kept) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}
