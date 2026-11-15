//! What a [`LayoutState`](crate::LayoutState) remembers between frames.
//!
//! Eggshell's own state lives in the `LayoutState` that the app holds, so it
//! is per layout by construction: an app that lays out a bar and a page in
//! one egui context cannot have one overwrite the other's. egui keeps egui's
//! own (focus, text-edit memory, the clock) in its context, and diagnostics
//! (the debug overlay, the continuity recorder) are context-wide by design.

use std::sync::{Arc, Mutex, PoisonError};

use std::collections::HashMap;

use egui::{Rect, ViewportId};

use crate::nav::Active;
use crate::overlay_input::Typing;
use crate::owned::Menus;
use crate::presence::Presences;
use crate::stack::Stack;
use crate::tooltip::Tooltips;
use crate::transition::Tracking;

#[derive(Default)]
pub(crate) struct Kept {
    pub(crate) listbox: Active,
    pub(crate) menus: Menus,
    pub(crate) animation: Tracking,
    pub(crate) presences: Presences,
    pub(crate) lists: crate::vlist::Lists,
    pub(crate) scrolls: crate::scroll::Scrolls,
    pub(crate) focus: crate::focus::Focus,
    viewports: HashMap<ViewportId, Viewport>,
    /// Where clipboard reads and writes go; the system's when unset.
    pub(crate) clipboard: Option<std::sync::Arc<dyn crate::ClipboardSource>>,
}

impl Kept {
    /// What the layout keeps of the input of `viewport`.
    pub(crate) fn viewport(&mut self, viewport: ViewportId) -> &mut Viewport {
        self.viewports.entry(viewport).or_default()
    }
}

/// What a layout keeps per egui viewport: each viewport has its own input,
/// focus and overlays. What belongs to a node is kept in [`Kept`] whichever
/// viewport the node is drawn in.
#[derive(Default)]
pub(crate) struct Viewport {
    pub(crate) stack: Stack,
    pub(crate) typing: Typing,
    pub(crate) tooltips: Tooltips,
    pub(crate) drags: crate::dnd::Drags,
    pub(crate) focus: crate::focus::Held,
    pub(crate) navigation: crate::nav::Navigation,
    pub(crate) scroll_keys: crate::scroll_keys::Keys,
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

    /// [`with`](Self::with) what is kept for `viewport`.
    pub(crate) fn viewport<R>(
        &self,
        viewport: ViewportId,
        f: impl FnOnce(&mut Viewport) -> R,
    ) -> R {
        self.with(|kept| f(kept.viewport(viewport)))
    }
}
