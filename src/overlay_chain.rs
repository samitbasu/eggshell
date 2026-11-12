//! The chain `Builder::overlay` starts.

use egui::Role;

use crate::builder::{Builder, Scope, Slot};
use crate::overlay::{
    Anchor, Band, Behavior, DismissOn, Focus, MatchWidth, Nav, Outside, Placement,
};
use crate::pipeline::DismissFn;
use crate::roots::OverlayDecl;
use crate::scroll::Description;

/// An overlay chain that has no anchor yet.
#[derive(Clone, Copy, Debug)]
pub struct Unplaced;

/// An overlay chain that floats next to its anchor.
#[derive(Clone, Copy, Debug)]
pub struct Floating {
    anchor: Anchor,
}

/// An overlay being declared: [`Builder::overlay`] starts it, and
/// [`children`](OverlayChain::children) builds its content and ends it.
///
/// ```
/// use eggshell_ui::overlay::{Anchor, Placement};
/// use eggshell_ui::{Builder, tw};
///
/// struct App { open: bool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.button("trigger", "Filters")
///         .on_click(|app| app.open = !app.open);
///     if app.open {
///         let trigger = b.id_of("trigger");
///         b.overlay("popover")
///             .anchor(Anchor::Node(trigger))
///             .placement(Placement::below())
///             .children(|s| s.text(tw().p(2.0), "Hello"));
///     }
/// }
/// ```
///
/// The state parameter says what is declared so far: `anchor` turns an
/// [`Unplaced`] chain into a [`Floating`] one, the only kind that can build its
/// content, so an overlay without a place is a compile error.
#[must_use = "an overlay is declared by `children`"]
pub struct OverlayChain<'b, 'f, A, State> {
    b: Builder<'b, 'f, A>,
    slot: Slot,
    placement: Placement,
    band: Band,
    match_width: MatchWidth,
    description: Description,
    behavior: Behavior,
    on_dismiss: Option<DismissFn<A>>,
    state: State,
}

impl<'b, 'f, A> OverlayChain<'b, 'f, A, Unplaced> {
    pub(crate) fn new(b: Builder<'b, 'f, A>, slot: Slot) -> Self {
        Self {
            b,
            slot,
            placement: Placement::default(),
            band: Band::default(),
            match_width: MatchWidth::default(),
            description: Description::default(),
            behavior: Behavior::default(),
            on_dismiss: None,
            state: Unplaced,
        }
    }

    /// What the overlay is placed against.
    pub fn anchor(self, anchor: Anchor) -> OverlayChain<'b, 'f, A, Floating> {
        let Self {
            b,
            slot,
            placement,
            band,
            match_width,
            description,
            behavior,
            on_dismiss,
            state: Unplaced,
        } = self;
        OverlayChain {
            b,
            slot,
            placement,
            band,
            match_width,
            description,
            behavior,
            on_dismiss,
            state: Floating { anchor },
        }
    }
}

impl<A, State> OverlayChain<'_, '_, A, State> {
    /// Which side of the anchor the overlay goes on, and how it flips and
    /// shifts to stay on screen. [`Placement::below`] unless set.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// The layer family: how the overlay stacks against other layers.
    /// [`Band::Popup`] unless set.
    pub fn band(mut self, band: Band) -> Self {
        self.band = band;
        self
    }

    /// How the overlay's width relates to its anchor's.
    /// [`MatchWidth::Content`] unless set.
    pub fn match_width(mut self, match_width: MatchWidth) -> Self {
        self.match_width = match_width;
        self
    }

    /// The overlay's role for assistive technology.
    pub fn role(mut self, role: Role) -> Self {
        self.description.role = Some(role);
        self
    }

    /// The overlay's accessible name. Name every overlay that has a role.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.description.name = Some(name.into());
        self
    }

    /// What dismisses the overlay. [`DismissOn::Default`] unless set.
    pub fn dismiss_on(mut self, dismiss_on: DismissOn) -> Self {
        self.behavior.dismiss_on = dismiss_on;
        self
    }

    /// What becomes of the press that dismisses the overlay. A menu (an
    /// overlay with [`Nav::Menu`] or [`Nav::Submenu`]) swallows it, anything
    /// else lets it through, unless set.
    pub fn outside(mut self, outside: Outside) -> Self {
        self.behavior.outside = Some(outside);
        self
    }

    /// Where focus goes when the overlay opens, and comes back from when it
    /// closes. [`Focus::Enter`] unless set.
    pub fn focus(mut self, focus: Focus) -> Self {
        self.behavior.focus = focus;
        self
    }

    /// The keyboard pattern of the overlay's items. [`Nav::None`] unless set.
    pub fn nav(mut self, nav: Nav) -> Self {
        self.behavior.nav = nav;
        self
    }

    /// Runs `then` when the library dismisses the overlay (see
    /// [`DismissOn`]): close it. The library never changes the application's
    /// state itself, and an overlay no handler closes stays open.
    pub fn on_dismiss(mut self, then: impl FnOnce(&mut A) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(then));
        self
    }
}

impl<'f, A> OverlayChain<'_, 'f, A, Floating> {
    /// Builds the overlay's content: an ordinary builder body that takes the
    /// text style in effect here, like the content of a scroll container.
    /// Overlays declared in the body are nested in this one.
    ///
    /// Declare the overlay every frame it is open. It closes when a frame
    /// does not declare it; the library holds no open state of its own.
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            b,
            slot,
            placement,
            band,
            match_width,
            description,
            behavior,
            on_dismiss,
            state: Floating { anchor },
        } = self;
        let decl = OverlayDecl {
            id: slot.id,
            anchor,
            placement,
            band,
            match_width,
            description,
            behavior,
        };
        b.overlay_with(&slot, decl, on_dismiss, body);
    }
}
