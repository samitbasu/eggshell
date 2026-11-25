//! What a [`LayoutState`](crate::LayoutState) remembers between frames.
//!
//! Eggshell's own state lives in the `LayoutState` that the app holds, so it
//! is per layout by construction: an app that lays out a bar and a page in
//! one egui context cannot have one overwrite the other's. egui keeps egui's
//! own (focus, text-edit memory, the clock) in its context, and diagnostics
//! (the debug overlay, the continuity recorder) are context-wide by design.

use std::sync::{Arc, Mutex};

use std::collections::HashMap;

use egui::{Rect, Vec2, ViewportId};

use crate::nav::Active;
use crate::overlay_input::Typing;
use crate::owned::menu::Menus;
use crate::owned::tooltip::Tooltips;
use crate::presence::Presences;
use crate::stack::Stack;
use crate::transition::Tracking;

pub(crate) struct Kept {
    pub(crate) listbox: Active,
    pub(crate) menus: Menus,
    pub(crate) animation: Tracking,
    pub(crate) presences: Presences,
    pub(crate) lists: crate::vlist::Lists,
    pub(crate) scrolls: crate::scroll::Scrolls,
    /// The pop-outs' windows, by the pop-out's id.
    pub(crate) windows: HashMap<egui::Id, crate::window::Seen>,
    /// The size each pop-out's subtree was last laid out at inline: the size
    /// its window opens at when the application gives no frame.
    pub(crate) inline: HashMap<egui::Id, Vec2>,
    /// Where each pop-out's subtree was last drawn, and its focus there.
    pub(crate) carried: HashMap<egui::Id, crate::window::Carried>,
    /// What an embedded pop-out window draws around its content, once one
    /// has been drawn.
    pub(crate) chrome: Option<crate::window::Chrome>,
    pub(crate) mounts: crate::mount::Selection,
    viewports: HashMap<ViewportId, Viewport>,
    /// Where clipboard reads and writes go; the system's when unset.
    pub(crate) clipboard: Option<std::sync::Arc<dyn crate::ClipboardSource>>,
    /// The layout's own style, given to its state.
    pub(crate) style: crate::contract::Themed<std::sync::Arc<crate::Style>>,
    /// The layout's style as it was last given to egui.
    pub(crate) egui_applied: Option<crate::contract::Themed<std::sync::Arc<crate::Style>>>,
    /// The egui styles, light and dark, the layout last gave egui.
    #[cfg(debug_assertions)]
    pub(crate) egui_look: Option<std::sync::Arc<str>>,
}

impl Kept {
    fn new(style: crate::contract::Themed<std::sync::Arc<crate::Style>>) -> Self {
        Self {
            listbox: Default::default(),
            menus: Default::default(),
            animation: Default::default(),
            presences: Default::default(),
            lists: Default::default(),
            scrolls: Default::default(),
            windows: Default::default(),
            inline: Default::default(),
            carried: Default::default(),
            chrome: Default::default(),
            mounts: Default::default(),
            viewports: Default::default(),
            clipboard: Default::default(),
            style,
            egui_applied: Default::default(),
            #[cfg(debug_assertions)]
            egui_look: None,
        }
    }

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
    pub(crate) hover_cards: crate::hover::Cards,
    pub(crate) drags: crate::dnd::Drags,
    pub(crate) focus: crate::focus::Held,
    pub(crate) navigation: crate::nav::Navigation,
    pub(crate) scroll_keys: crate::scroll_keys::Keys,
    /// The buttons that dismiss their overlay pressed this pass, until the
    /// pass's dismissal takes them.
    pub(crate) closers: Vec<egui::Id>,
    /// The area the last layout placed things in.
    pub(crate) area: Option<Rect>,
}

/// Shared by a `LayoutState` and the frames it builds, which outlive a borrow
/// of it.
#[derive(Clone)]
pub(crate) struct Handle(Arc<Mutex<Kept>>);

impl Handle {
    /// What a layout of `style` keeps, before its first frame.
    pub(crate) fn new(style: crate::contract::Themed<std::sync::Arc<crate::Style>>) -> Self {
        Self(Arc::new(Mutex::new(Kept::new(style))))
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut Kept) -> R) -> R {
        f(&mut crate::sync::lock(&self.0))
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
