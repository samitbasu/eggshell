//! The chain `Builder::overlay` starts.

use egui::Role;

use crate::builder::{Builder, Scope, Slot};
use crate::overlay::{
    Anchor, Band, Behavior, DismissOn, Focus, MatchWidth, Modal, Nav, Outside, Placement, Wrap,
};
use crate::pipeline::DismissFn;
use crate::roots::{OverlayDecl, Presentation};
use crate::scroll::Description;

/// An overlay chain that has no anchor yet.
#[derive(Clone, Copy, Debug)]
pub struct Unplaced;

/// An overlay chain that floats next to its anchor.
#[derive(Clone, Copy, Debug)]
pub struct Floating {
    anchor: Anchor,
    placement: Placement,
    match_width: MatchWidth,
}

/// An overlay chain that covers the screen with a scrim and holds a dialog
/// box or a sheet, which the content's classes size.
#[derive(Clone, Copy, Debug)]
pub struct Modally {
    modal: Modal,
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
/// [`Unplaced`] chain into a [`Floating`] one and `modal` into a [`Modally`]
/// one. Those are the only kinds that can build their content, so an overlay
/// without a place is a compile error, and one that is both modal and
/// anchored cannot be written.
#[must_use = "an overlay is declared by `children`"]
pub struct OverlayChain<'b, 'f, A, State> {
    b: Builder<'b, 'f, A>,
    slot: Slot,
    band: Band,
    description: Description,
    behavior: Behavior,
    on_dismiss: Option<DismissFn<A>>,
    declared_at: &'static std::panic::Location<'static>,
    state: State,
}

impl<'b, 'f, A> OverlayChain<'b, 'f, A, Unplaced> {
    #[track_caller]
    pub(crate) fn new(b: Builder<'b, 'f, A>, slot: Slot) -> Self {
        Self {
            declared_at: std::panic::Location::caller(),
            b,
            slot,
            band: Band::default(),
            description: Description::default(),
            behavior: Behavior::default(),
            on_dismiss: None,
            state: Unplaced,
        }
    }

    /// What the overlay is placed against.
    pub fn anchor(self, anchor: Anchor) -> OverlayChain<'b, 'f, A, Floating> {
        self.with_state(Floating {
            anchor,
            placement: Placement::default(),
            match_width: MatchWidth::default(),
        })
    }

    /// Makes the overlay a modal: its root covers the screen with the
    /// style's scrim, which takes every press and the wheel, and the
    /// content's classes size the box inside it. [`Modal::Center`] centres
    /// the box, [`Modal::Edge`] docks it to a side of the screen. Focus is
    /// trapped in the modal (and in what is above it) while it is open, and
    /// a press on the scrim or Esc dismisses it, as [`DismissOn`] says.
    pub fn modal(self, modal: Modal) -> OverlayChain<'b, 'f, A, Modally> {
        self.with_state(Modally { modal })
    }

    fn with_state<State>(self, state: State) -> OverlayChain<'b, 'f, A, State> {
        let Self {
            b,
            slot,
            band,
            description,
            behavior,
            on_dismiss,
            declared_at,
            state: Unplaced,
        } = self;
        OverlayChain {
            b,
            slot,
            band,
            description,
            behavior,
            on_dismiss,
            declared_at,
            state,
        }
    }
}

impl<A, State> OverlayChain<'_, '_, A, State> {
    /// The layer family: how the overlay stacks against other layers.
    /// [`Band::Popup`] unless set.
    pub fn band(mut self, band: Band) -> Self {
        self.band = band;
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

    /// What the arrow keys do at the first and last item of the overlay's
    /// [`Nav`]. [`Wrap::Around`] unless set.
    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.behavior.wrap = wrap;
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

impl<A> OverlayChain<'_, '_, A, Floating> {
    /// Which side of the anchor the overlay goes on, and how it flips and
    /// shifts to stay on screen. [`Placement::below`] unless set.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.state.placement = placement;
        self
    }

    /// How the overlay's width relates to its anchor's.
    /// [`MatchWidth::Content`] unless set.
    pub fn match_width(mut self, match_width: MatchWidth) -> Self {
        self.state.match_width = match_width;
        self
    }
}

impl<'f, A, State> OverlayChain<'_, 'f, A, State> {
    fn declare(self, presentation: Presentation, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Self {
            b,
            slot,
            band,
            description,
            behavior,
            on_dismiss,
            declared_at,
            state: _,
        } = self;
        let decl = OverlayDecl {
            id: slot.id,
            presentation,
            band,
            description,
            behavior,
            declared_at,
        };
        b.overlay_with(&slot, decl, on_dismiss, body);
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
        let Floating {
            anchor,
            placement,
            match_width,
        } = self.state;
        self.declare(
            Presentation::Floating {
                anchor,
                placement,
                match_width,
            },
            body,
        );
    }
}

impl<'f, A> OverlayChain<'_, 'f, A, Modally> {
    /// Builds the modal's content: the dialog box or the sheet, a child of
    /// the screen-sized root. Size it with the box's own classes; nothing
    /// places it but the [`Modal`] the chain was given.
    ///
    /// Declare the overlay every frame it is open, as for a floating one. A
    /// modal opened from inside another is above it, and the lower one takes
    /// no input until it closes.
    pub fn children(self, body: impl FnOnce(&mut Scope<'_, 'f, A>)) {
        let Modally { modal } = self.state;
        self.declare(Presentation::Modal(modal), body);
    }
}
