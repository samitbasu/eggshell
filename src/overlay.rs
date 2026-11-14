//! Overlays: where a floating layer goes, and which layer it is on.
//!
//! This module holds the declaration types ([`Anchor`], [`Placement`],
//! [`Band`], [`MatchWidth`]) and the pure placement function [`place`]. It
//! draws nothing and reads no egui state: the caller resolves the anchor to a
//! rect and measures the overlay, and `place` says where the overlay goes.
//!
//! The design is in `docs/overlays.md`.

use crate::tw::Sides;
use egui::{Id, Order, Pos2, Rect, Vec2, pos2, vec2};

pub use crate::overlay_chain::{Floating, Modally, OverlayChain, Unplaced};
pub use crate::presence::{Entrance, Shown};

/// A signed length in points: a gap, a shift or a margin.
///
/// Non-finite input becomes zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Distance(f32);

impl Distance {
    /// No distance.
    pub const ZERO: Self = Self(0.0);

    /// A length in points; negative values are allowed. Non-finite input
    /// becomes [`Distance::ZERO`].
    pub fn new(points: f32) -> Self {
        if points.is_finite() {
            Self(points)
        } else {
            Self::ZERO
        }
    }
}

impl From<Distance> for f32 {
    fn from(distance: Distance) -> f32 {
        distance.0
    }
}

/// A side of a rect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
}

impl Side {
    /// The side across from this one.
    pub fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// Where the overlay's edge sits along the edge it is placed against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    /// Starts where the edge starts (left or top).
    Start,
    /// Centred on the edge.
    Center,
    /// Ends where the edge ends (right or bottom).
    End,
}

/// What to do when the preferred side has no room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flip {
    /// Stay on the preferred side and be shifted into the bounds, over the
    /// anchor if need be: the overlay keeps its size, so
    /// [`Spot::available`] is the whole bounds along the main axis.
    Never,
    /// Move to the opposite side when it fits, or has more room.
    ToOpposite,
}

/// What an overlay is placed against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// An identified node's final rect this frame.
    Node(Id),
    /// A point in screen space (a context menu): a zero-size rect.
    Point(Pos2),
    /// A rect in screen space.
    Rect(Rect),
    /// The pointer position when the overlay opened; fixed afterwards.
    Pointer,
    /// The screen: sheets and other edge overlays.
    Screen,
}

/// How a modal overlay presents its box on the scrim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modal {
    /// Centred on the screen: a dialog.
    Center,
    /// Docked to a side of the screen, stretched along it: a sheet.
    Edge(Side),
}

/// The layer family an overlay is drawn in. Inside a band, overlays stack in
/// the order they were opened, and a nested overlay is above its parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Band {
    /// Non-modal panels that sit with the page: sheets, docked flyouts.
    Floating,
    /// Popovers, menus, selects, dialogs and their scrims.
    #[default]
    Popup,
    /// Tooltips and hover cards. Never sensing.
    Tooltip,
}

impl From<Band> for Order {
    fn from(band: Band) -> Order {
        match band {
            Band::Floating => Order::Middle,
            Band::Popup => Order::Foreground,
            Band::Tooltip => Order::Tooltip,
        }
    }
}

impl Band {
    /// Whether the overlay senses the pointer: a tooltip does not, and a
    /// press on its trigger is the page's.
    pub(crate) fn senses_input(self) -> bool {
        self != Self::Tooltip
    }

    /// The egui layer of the overlay `id` in this band: one layer per
    /// overlay, so that its bounding box is the overlay's box and the layers
    /// between overlays belong to the page.
    pub(crate) fn layer(self, id: Id) -> egui::LayerId {
        egui::LayerId::new(self.into(), id.with("overlay"))
    }
}

/// How the overlay's width relates to its anchor's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchWidth {
    /// Sized to its content.
    #[default]
    Content,
    /// At least as wide as the anchor (a select's list under its trigger).
    Anchor,
}

/// What dismisses an overlay: the library calls its
/// [`on_dismiss`](OverlayChain::on_dismiss) and the application closes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DismissOn {
    /// A press outside it, Esc, focus leaving it, and the window losing
    /// focus.
    #[default]
    Default,
    /// Esc only: a confirmation that must be answered, or a suggestion list
    /// whose outside click is the field's own concern.
    EscOnly,
    /// Nothing: a tooltip, which the library closes itself.
    Never,
}

impl DismissOn {
    pub(crate) fn outside(self) -> bool {
        self == Self::Default
    }

    pub(crate) fn esc(self) -> bool {
        self != Self::Never
    }

    pub(crate) fn focus_out(self) -> bool {
        self == Self::Default
    }
}

/// What becomes of the press that dismisses an overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outside {
    /// It goes on to what is under the overlay: a popover.
    PassThrough,
    /// The overlay takes it, and nothing under it sees it: a menu. While such
    /// an overlay is open its blocker covers the whole screen and draws
    /// nothing.
    Swallow,
}

impl Outside {
    /// What an overlay that navigates as `nav` does unless told otherwise:
    /// menus swallow. A listbox passes through, since an autosuggest's anchor
    /// is the field the press may be on; a select says
    /// [`Swallow`](Self::Swallow) itself.
    pub(crate) fn of(nav: Nav) -> Self {
        match nav {
            Nav::Menu | Nav::Submenu => Self::Swallow,
            Nav::Listbox | Nav::None => Self::PassThrough,
        }
    }
}

/// Where keyboard focus goes when an overlay opens, and where it comes back
/// to when it closes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Focus {
    /// Into the overlay: its first focusable. Menus, dialogs, popovers with
    /// fields.
    #[default]
    Enter,
    /// Stays where it is: a select's list or an autosuggest, where the
    /// trigger or field keeps focus and the highlight is virtual.
    Stay,
    /// Not involved: a tooltip.
    None,
}

/// The keyboard pattern of an overlay's items. Items are the nodes with an
/// item role ([`NodeHandle::role`](crate::NodeHandle::role) with
/// `MenuItem`, `MenuItemCheckbox`, `MenuItemRadio` or `ListBoxOption`) that
/// can take focus; a disabled item is skipped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Nav {
    /// The content handles its own keys.
    #[default]
    None,
    /// Up and Down move focus between items, wrapping; Home and End go to the
    /// first and last. The items are real focus targets.
    Menu,
    /// [`Menu`](Self::Menu), and Left dismisses the overlay and focuses the
    /// item it is anchored to.
    Submenu,
    /// The same keys, but focus stays where it is and the highlight is
    /// virtual: Enter and Space activate the highlighted item.
    Listbox,
}

/// What an arrow key does at the first or last item of a [`Nav`] overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Wrap {
    /// Goes on to the other end: menus, and a listbox unless told otherwise.
    #[default]
    Around,
    /// Stays on the item: a select's list, as Radix's.
    Stop,
}

/// What an overlay does with input, as its chain declared it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Behavior {
    pub(crate) dismiss_on: DismissOn,
    pub(crate) outside: Option<Outside>,
    pub(crate) focus: Focus,
    pub(crate) nav: Nav,
    pub(crate) wrap: Wrap,
}

impl Behavior {
    /// What a press outside the overlay does to the press.
    pub(crate) fn outside(self) -> Outside {
        self.outside.unwrap_or_else(|| Outside::of(self.nav))
    }
}

/// The region an overlay is kept inside, besides the window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Boundary {
    /// The window.
    #[default]
    Window,
    /// An identified node's rect this frame. A node that is not in the frame
    /// dismisses the overlay, as a missing [`Anchor::Node`] does.
    Node(Id),
    /// A rect in screen space.
    Rect(Rect),
}

impl Boundary {
    /// This boundary pulled in by a distance on each side: a small margin
    /// at the top and a large band at the bottom.
    #[must_use]
    pub fn inset(self, inset: Sides<Distance>) -> InsetBoundary {
        InsetBoundary {
            boundary: self,
            inset,
        }
    }
}

/// A [`Boundary`] pulled in by a distance on each side.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InsetBoundary {
    boundary: Boundary,
    inset: Sides<Distance>,
}

impl From<Boundary> for InsetBoundary {
    fn from(boundary: Boundary) -> Self {
        boundary.inset(Sides::default())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Relation {
    /// Outside the anchor, next to the chosen side.
    Beside,
    /// Inside the bounds, against the chosen side.
    Inside,
}

/// Where an overlay goes relative to its anchor.
///
/// Start from a preset and adjust it:
/// `Placement::below().align(Align::End).gap(Distance::new(4.0))`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    side: Side,
    align: Align,
    relation: Relation,
    gap: Distance,
    shift: Distance,
    flip: Flip,
    margin: Distance,
    boundary: InsetBoundary,
}

impl Placement {
    /// Next to the anchor on `side`, aligned to its start, flipping when
    /// there is no room.
    pub fn beside(side: Side) -> Self {
        Self {
            side,
            align: Align::Start,
            relation: Relation::Beside,
            gap: Distance::ZERO,
            shift: Distance::ZERO,
            flip: Flip::ToOpposite,
            margin: Distance::ZERO,
            boundary: InsetBoundary::default(),
        }
    }

    /// Under the anchor: a select or a popover.
    pub fn below() -> Self {
        Self::beside(Side::Bottom)
    }

    /// At a point: a context menu. Below and to the right of the point, or
    /// above and to the left at the screen's corner.
    pub fn menu() -> Self {
        Self::below()
    }

    /// Right of a menu item, overlapping its edge a little, flipping to its
    /// left.
    pub fn submenu() -> Self {
        Self::beside(Side::Right)
            .gap(Distance::new(-4.0))
            .shift(Distance::new(-4.0))
    }

    /// Above the anchor, centred, a little apart.
    pub fn tooltip() -> Self {
        Self::beside(Side::Top)
            .align(Align::Center)
            .gap(Distance::new(4.0))
    }

    /// Inside the bounds, against `side`: a sheet. The anchor is ignored,
    /// there is no flip, and [`gap`](Self::gap) has no effect.
    pub fn edge(side: Side) -> Self {
        Self {
            relation: Relation::Inside,
            flip: Flip::Never,
            ..Self::beside(side)
        }
    }

    /// Where the overlay's edge sits along the edge it is placed against.
    #[must_use]
    pub fn align(self, align: Align) -> Self {
        Self { align, ..self }
    }

    /// The space between the anchor and the overlay; negative overlaps.
    #[must_use]
    pub fn gap(self, gap: Distance) -> Self {
        Self { gap, ..self }
    }

    /// Moves the overlay along the edge, away from the alignment's side.
    #[must_use]
    pub fn shift(self, shift: Distance) -> Self {
        Self { shift, ..self }
    }

    /// What to do when the preferred side has no room.
    #[must_use]
    pub fn flip(self, flip: Flip) -> Self {
        Self { flip, ..self }
    }

    /// Keeps the overlay this far from the bounds' edges.
    #[must_use]
    pub fn margin(self, margin: Distance) -> Self {
        Self { margin, ..self }
    }

    /// Keeps the overlay inside a region as well as the window: it slides
    /// along both axes to stay in, and a flip counts the room inside it.
    /// `Boundary::Node(canvas).inset(..)` and a plain `Boundary::Rect(..)`
    /// both work.
    #[must_use]
    pub fn boundary(self, boundary: impl Into<InsetBoundary>) -> Self {
        Self {
            boundary: boundary.into(),
            ..self
        }
    }

    /// The node the boundary is, if it is one.
    pub(crate) fn boundary_node(&self) -> Option<Id> {
        match self.boundary.boundary {
            Boundary::Node(id) => Some(id),
            Boundary::Window | Boundary::Rect(_) => None,
        }
    }

    /// The part of the window the overlay may be in: the boundary, inset,
    /// intersected with the window. Empty, it collapses to a point at the
    /// window-side corner.
    pub(crate) fn region(&self, window: Rect) -> Rect {
        let region = match self.boundary.boundary {
            Boundary::Rect(rect) => rect,
            Boundary::Window | Boundary::Node(_) => window,
        };
        let inset = self.boundary.inset;
        let region = Rect::from_min_max(
            region.min + vec2(f32::from(inset.left), f32::from(inset.top)),
            region.max - vec2(f32::from(inset.right), f32::from(inset.bottom)),
        );
        let min = region.min.max(window.min);
        Rect::from_min_max(min, region.max.min(window.max).max(min))
    }

    /// This placement with a node boundary replaced by the node's `rect`.
    pub(crate) fn with_node_rect(mut self, rect: Rect) -> Self {
        if self.boundary_node().is_some() {
            self.boundary.boundary = Boundary::Rect(rect);
        }
        self
    }
}

impl Default for Placement {
    fn default() -> Self {
        Self::below()
    }
}

/// Whether the anchor can be seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchored {
    /// The anchor meets the bounds.
    Attached,
    /// The anchor lies wholly outside the bounds, or is clipped away by its
    /// containers (scrolled out of its viewport): the overlay is hidden, its
    /// open state untouched. An anchor that is not in the frame at all is not
    /// this: the overlay is dismissed.
    Detached,
}

/// Whether the node an overlay is anchored to is in the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnchorLookup {
    Found,
    /// An [`Anchor::Node`] naming no node of the frame.
    Missing,
}

impl Anchored {
    /// Attached only if both are.
    pub(crate) fn and(self, other: Self) -> Self {
        if self == Self::Attached && other == Self::Attached {
            Self::Attached
        } else {
            Self::Detached
        }
    }
}

/// The outcome of [`place`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Where the overlay goes. Inside the bounds whenever they can hold it.
    pub rect: Rect,
    /// The side it ended on; a flip changes it.
    pub side: Side,
    /// The most the overlay may measure here, so that content taller than
    /// that scrolls instead of leaving the screen: the room on `side` along
    /// its axis, the bounds across it.
    pub available: Vec2,
    /// Whether the overlay should be shown.
    pub anchored: Anchored,
}

impl Spot {
    /// Where the overlay goes once it has been solved again at `size`, the
    /// room [`available`](Self::available) having cut it short: on the same
    /// side as before, so that clamping cannot flip it.
    pub(crate) fn refit(
        self,
        anchor: Rect,
        size: Vec2,
        placement: &Placement,
        bounds: Rect,
    ) -> Spot {
        let placement = Placement {
            side: self.side,
            flip: Flip::Never,
            ..*placement
        };
        place(anchor, size, &placement, bounds)
    }
}

/// Where an overlay of `size` goes.
///
/// `size` is the overlay's natural, unclamped size: the choice of side
/// depends on it alone, so clamping the overlay to [`Spot::available`] cannot
/// change the side next frame. `anchor` is already resolved to a rect (a
/// point is a zero-size rect) and `bounds` is where the overlay must stay.
pub fn place(anchor: Rect, size: Vec2, placement: &Placement, bounds: Rect) -> Spot {
    let margin = f32::from(placement.margin);
    let gap = f32::from(placement.gap);
    let shift = f32::from(placement.shift);
    let area = placement.region(bounds).shrink(margin);
    let relation = placement.relation;

    let room = |side: Side| match (relation, side) {
        (Relation::Inside, side) if side.is_vertical() => area.height(),
        (Relation::Inside, _) => area.width(),
        (Relation::Beside, Side::Bottom) => area.bottom() - anchor.bottom() - gap,
        (Relation::Beside, Side::Top) => anchor.top() - area.top() - gap,
        (Relation::Beside, Side::Right) => area.right() - anchor.right() - gap,
        (Relation::Beside, Side::Left) => anchor.left() - area.left() - gap,
    };
    let need = |side: Side| if side.is_vertical() { size.y } else { size.x };
    let fits = |side: Side| need(side) <= room(side);
    let preferred = placement.side;
    let opposite = preferred.opposite();
    let side = match placement.flip {
        Flip::ToOpposite
            if !fits(preferred) && (fits(opposite) || room(opposite) > room(preferred)) =>
        {
            opposite
        }
        _ => preferred,
    };

    let reference = match relation {
        Relation::Beside => anchor,
        Relation::Inside => area,
    };
    let along = |lo: f32, hi: f32, len: f32| match placement.align {
        Align::Start => lo + shift,
        Align::Center => (lo + hi - len) / 2.0 + shift,
        Align::End => hi - len - shift,
    };
    let mut min = if side.is_vertical() {
        let y = match (relation, side) {
            (Relation::Beside, Side::Bottom) => reference.bottom() + gap,
            (Relation::Beside, _) => reference.top() - gap - size.y,
            (Relation::Inside, Side::Bottom) => reference.bottom() - size.y,
            (Relation::Inside, _) => reference.top(),
        };
        pos2(along(reference.left(), reference.right(), size.x), y)
    } else {
        let x = match (relation, side) {
            (Relation::Beside, Side::Right) => reference.right() + gap,
            (Relation::Beside, _) => reference.left() - gap - size.x,
            (Relation::Inside, Side::Right) => reference.right() - size.x,
            (Relation::Inside, _) => reference.left(),
        };
        pos2(x, along(reference.top(), reference.bottom(), size.y))
    };
    // An overlay larger than the area starts at its edge.
    min.x = min.x.min(area.right() - size.x).max(area.left());
    min.y = min.y.min(area.bottom() - size.y).max(area.top());

    let main = match placement.flip {
        Flip::Never => {
            if side.is_vertical() {
                area.height()
            } else {
                area.width()
            }
        }
        Flip::ToOpposite => room(side).max(0.0),
    };
    let available = if side.is_vertical() {
        vec2(area.width(), main)
    } else {
        vec2(main, area.height())
    };
    Spot {
        rect: Rect::from_min_size(min, size),
        side,
        available,
        anchored: if relation == Relation::Inside || anchor.intersects(bounds) {
            Anchored::Attached
        } else {
            Anchored::Detached
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    const SIZE: Vec2 = vec2(100.0, 60.0);
    const SIDES: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];
    const ALIGNS: [Align; 3] = [Align::Start, Align::Center, Align::End];

    fn d(points: f32) -> Distance {
        Distance::new(points)
    }

    fn anchor() -> Rect {
        Rect::from_min_size(pos2(150.0, 120.0), vec2(80.0, 30.0))
    }

    fn at(side: Side, align: Align) -> Placement {
        Placement::beside(side).align(align)
    }

    #[test]
    fn distance_ignores_non_finite_input() {
        assert_eq!(f32::from(d(-4.0)), -4.0);
        assert_eq!(d(f32::NAN), Distance::ZERO);
        assert_eq!(d(f32::INFINITY), Distance::ZERO);
    }

    #[test]
    fn bands_map_to_egui_orders_from_back_to_front() {
        assert_eq!(Order::from(Band::Floating), Order::Middle);
        assert_eq!(Order::from(Band::Popup), Order::Foreground);
        assert_eq!(Order::from(Band::Tooltip), Order::Tooltip);
        assert_eq!(Band::default(), Band::Popup);
        assert!(Order::from(Band::Floating) < Order::from(Band::Popup));
        assert!(Order::from(Band::Popup) < Order::from(Band::Tooltip));
    }

    #[test]
    fn every_side_and_alignment_with_room_to_spare() {
        let a = anchor();
        let cases = [
            (Side::Bottom, Align::Start, pos2(150.0, 150.0)),
            (Side::Bottom, Align::Center, pos2(140.0, 150.0)),
            (Side::Bottom, Align::End, pos2(130.0, 150.0)),
            (Side::Top, Align::Start, pos2(150.0, 60.0)),
            (Side::Top, Align::Center, pos2(140.0, 60.0)),
            (Side::Top, Align::End, pos2(130.0, 60.0)),
            (Side::Right, Align::Start, pos2(230.0, 120.0)),
            (Side::Right, Align::Center, pos2(230.0, 105.0)),
            (Side::Right, Align::End, pos2(230.0, 90.0)),
            (Side::Left, Align::Start, pos2(50.0, 120.0)),
            (Side::Left, Align::Center, pos2(50.0, 105.0)),
            (Side::Left, Align::End, pos2(50.0, 90.0)),
        ];
        for (side, align, min) in cases {
            let spot = place(a, SIZE, &at(side, align), BOUNDS);
            assert_eq!(
                spot.rect,
                Rect::from_min_size(min, SIZE),
                "{side:?} {align:?}"
            );
            assert_eq!(spot.side, side);
            assert_eq!(spot.anchored, Anchored::Attached);
        }
    }

    #[test]
    fn gap_and_shift_move_the_overlay_and_a_negative_gap_overlaps() {
        let a = anchor();
        let p = at(Side::Bottom, Align::Start).gap(d(4.0)).shift(d(2.0));
        assert_eq!(place(a, SIZE, &p, BOUNDS).rect.min, pos2(152.0, 154.0));
        let sub = at(Side::Right, Align::Start).gap(d(-4.0)).shift(d(-4.0));
        assert_eq!(place(a, SIZE, &sub, BOUNDS).rect.min, pos2(226.0, 116.0));
    }

    #[test]
    fn shift_runs_against_the_alignment_at_the_end() {
        let p = at(Side::Bottom, Align::End).shift(d(2.0));
        assert_eq!(place(anchor(), SIZE, &p, BOUNDS).rect.min.x, 128.0);
    }

    #[test]
    fn flips_to_the_opposite_side_only_when_the_preferred_does_not_fit() {
        let low = Rect::from_min_size(pos2(150.0, 250.0), vec2(80.0, 30.0));
        assert!(BOUNDS.bottom() - low.bottom() < SIZE.y, "no room below");
        assert!(low.top() >= SIZE.y, "room above");
        let flipped = place(low, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(flipped.side, Side::Top);
        assert_eq!(flipped.rect.bottom(), low.top());

        let stay = place(anchor(), SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(stay.side, Side::Bottom);

        let never = at(Side::Bottom, Align::Start).flip(Flip::Never);
        let clamped = place(low, SIZE, &never, BOUNDS);
        assert_eq!(clamped.side, Side::Bottom);
        assert_eq!(
            clamped.rect.bottom(),
            BOUNDS.bottom(),
            "kept on screen, covering the anchor"
        );
    }

    #[test]
    fn a_submenu_near_the_right_edge_flips_to_the_left_of_its_item() {
        let item = Rect::from_min_size(pos2(300.0, 100.0), vec2(90.0, 24.0));
        assert!(
            BOUNDS.right() - item.right() < SIZE.x,
            "no room on the right"
        );
        let spot = place(item, SIZE, &Placement::submenu(), BOUNDS);
        assert_eq!(spot.side, Side::Left);
        assert_eq!(spot.rect.right(), item.left() + 4.0, "overlaps by the gap");
        assert_eq!(spot.rect.top(), item.top() - 4.0);
    }

    #[test]
    fn a_tooltip_sits_above_and_centred_with_a_gap() {
        let spot = place(anchor(), SIZE, &Placement::tooltip(), BOUNDS);
        assert_eq!(spot.side, Side::Top);
        assert_eq!(spot.rect.bottom(), anchor().top() - 4.0);
        assert_eq!(spot.rect.center().x, anchor().center().x);
    }

    #[test]
    fn neither_side_fits_picks_the_roomier_and_reports_what_is_available() {
        let tall = vec2(100.0, 250.0);
        let a = Rect::from_min_size(pos2(150.0, 100.0), vec2(80.0, 30.0));
        let spot = place(a, tall, &at(Side::Bottom, Align::Start), BOUNDS);
        assert!(a.top() < tall.y && BOUNDS.bottom() - a.bottom() < tall.y);
        assert_eq!(spot.side, Side::Bottom, "170 below beats 100 above");
        assert_eq!(spot.available.y, 170.0);
        assert_eq!(spot.available.x, BOUNDS.width());

        let a = Rect::from_min_size(pos2(150.0, 180.0), vec2(80.0, 30.0));
        assert!(a.top() < tall.y && BOUNDS.bottom() - a.bottom() < tall.y);
        let spot = place(a, tall, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(spot.side, Side::Top, "180 above beats 90 below");
        assert_eq!(spot.available.y, 180.0);
    }

    #[test]
    fn available_on_a_horizontal_side_is_the_room_across_and_the_bounds_down() {
        let wide = vec2(500.0, 20.0);
        let spot = place(anchor(), wide, &at(Side::Right, Align::Start), BOUNDS);
        assert_eq!(spot.side, Side::Right, "170 right beats 150 left");
        assert_eq!(spot.available, vec2(170.0, BOUNDS.height()));
    }

    #[test]
    fn the_cross_axis_is_shifted_into_the_bounds() {
        let near_right = Rect::from_min_size(pos2(360.0, 120.0), vec2(30.0, 30.0));
        assert!(near_right.left() + SIZE.x > BOUNDS.right());
        let spot = place(near_right, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(spot.rect.right(), BOUNDS.right());
        let near_left = Rect::from_min_size(pos2(5.0, 120.0), vec2(30.0, 30.0));
        assert!(near_left.right() - SIZE.x < BOUNDS.left());
        let spot = place(near_left, SIZE, &at(Side::Bottom, Align::End), BOUNDS);
        assert_eq!(spot.rect.left(), BOUNDS.left());
    }

    #[test]
    fn the_margin_keeps_clear_of_the_edges_and_counts_against_the_room() {
        let p = at(Side::Bottom, Align::Start).margin(d(8.0));
        let near_right = Rect::from_min_size(pos2(380.0, 120.0), vec2(10.0, 30.0));
        assert_eq!(place(near_right, SIZE, &p, BOUNDS).rect.right(), 392.0);
        let low = Rect::from_min_size(pos2(150.0, 150.0), vec2(80.0, 30.0));
        let size = vec2(100.0, 115.0);
        let room = BOUNDS.bottom() - low.bottom();
        assert!(
            size.y <= room && size.y > room - 8.0,
            "fits only without the margin"
        );
        assert_eq!(
            place(low, size, &at(Side::Bottom, Align::Start), BOUNDS).side,
            Side::Bottom
        );
        assert_eq!(place(low, size, &p, BOUNDS).side, Side::Top);
    }

    #[test]
    fn an_overlay_larger_than_the_bounds_starts_at_the_edge() {
        let huge = vec2(500.0, 400.0);
        assert!(huge.x > BOUNDS.width() && huge.y > BOUNDS.height());
        let spot = place(anchor(), huge, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(spot.rect.min, BOUNDS.min);
    }

    #[test]
    fn a_point_anchor_is_a_context_menu_that_flips_at_the_corner() {
        let point = |x, y| Rect::from_min_size(pos2(x, y), Vec2::ZERO);
        let menu = Placement::menu();
        assert_eq!(
            place(point(50.0, 50.0), SIZE, &menu, BOUNDS).rect.min,
            pos2(50.0, 50.0)
        );
        let corner = place(point(390.0, 295.0), SIZE, &menu, BOUNDS);
        assert_eq!(corner.side, Side::Top);
        assert_eq!(corner.rect.right_bottom(), pos2(400.0, 295.0));
    }

    #[test]
    fn an_anchor_outside_the_bounds_is_detached_and_the_overlay_is_still_inside() {
        let gone = Rect::from_min_size(pos2(150.0, -100.0), vec2(80.0, 30.0));
        assert!(!gone.intersects(BOUNDS));
        let spot = place(gone, SIZE, &at(Side::Bottom, Align::Start), BOUNDS);
        assert_eq!(spot.anchored, Anchored::Detached);
        assert!(BOUNDS.contains_rect(spot.rect));
    }

    #[test]
    fn an_anchor_partly_inside_the_bounds_is_attached() {
        let partial = Rect::from_min_size(pos2(150.0, -10.0), vec2(80.0, 30.0));
        assert!(partial.intersects(BOUNDS) && !BOUNDS.contains_rect(partial));
        let spot = place(partial, SIZE, &Placement::below(), BOUNDS);
        assert_eq!(spot.anchored, Anchored::Attached);
    }

    #[test]
    fn placement_is_mirror_symmetric() {
        let flip_x = |r: Rect| {
            Rect::from_min_max(
                pos2(BOUNDS.right() - r.right(), r.top()),
                pos2(BOUNDS.right() - r.left(), r.bottom()),
            )
        };
        let flip_y = |r: Rect| {
            Rect::from_min_max(
                pos2(r.left(), BOUNDS.bottom() - r.bottom()),
                pos2(r.right(), BOUNDS.bottom() - r.top()),
            )
        };
        let swap = |align| match align {
            Align::Start => Align::End,
            Align::End => Align::Start,
            Align::Center => Align::Center,
        };
        let anchors = [
            anchor(),
            Rect::from_min_size(pos2(5.0, 5.0), vec2(30.0, 20.0)),
            Rect::from_min_size(pos2(350.0, 270.0), vec2(40.0, 25.0)),
            Rect::from_min_size(pos2(190.0, 140.0), Vec2::ZERO),
        ];
        for a in anchors {
            for size in [SIZE, vec2(300.0, 20.0), vec2(20.0, 290.0)] {
                for align in ALIGNS {
                    let below = place(a, size, &at(Side::Bottom, align), BOUNDS);
                    let above = place(flip_y(a), size, &at(Side::Top, align), BOUNDS);
                    assert_eq!(flip_y(below.rect), above.rect, "y {a:?} {size:?} {align:?}");

                    let mirrored = place(flip_x(a), size, &at(Side::Bottom, swap(align)), BOUNDS);
                    assert_eq!(
                        flip_x(below.rect),
                        mirrored.rect,
                        "x {a:?} {size:?} {align:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_result_stays_inside_the_bounds_that_can_hold_it() {
        for ax in [0.0, 5.0, 150.0, 330.0, 399.0] {
            for ay in [0.0, 5.0, 120.0, 270.0, 299.0] {
                let a = Rect::from_min_size(pos2(ax, ay), vec2(40.0, 25.0));
                for side in SIDES {
                    for align in ALIGNS {
                        let spot = place(a, SIZE, &at(side, align), BOUNDS);
                        assert!(
                            BOUNDS.contains_rect(spot.rect),
                            "{a:?} {side:?} {align:?} -> {:?}",
                            spot.rect
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_choice_depends_on_the_natural_size_only_so_clamping_cannot_make_it_flicker() {
        let a = Rect::from_min_size(pos2(150.0, 100.0), vec2(80.0, 30.0));
        let natural = vec2(100.0, 250.0);
        let p = at(Side::Bottom, Align::Start);
        let first = place(a, natural, &p, BOUNDS);
        assert!(
            natural.y > first.available.y,
            "the natural size does not fit"
        );
        assert_eq!(first, place(a, natural, &p, BOUNDS));
        let clamped = vec2(natural.x, first.available.y);
        assert_eq!(place(a, clamped, &p, BOUNDS).side, first.side);
    }

    #[test]
    fn an_edge_pins_the_overlay_inside_the_bounds_against_its_side() {
        let sheet = vec2(120.0, 300.0);
        let bar = vec2(400.0, 50.0);
        let cases = [
            (Side::Left, sheet, pos2(0.0, 0.0)),
            (Side::Right, sheet, pos2(280.0, 0.0)),
            (Side::Top, bar, pos2(0.0, 0.0)),
            (Side::Bottom, bar, pos2(0.0, 250.0)),
        ];
        for (side, size, min) in cases {
            let spot = place(BOUNDS, size, &Placement::edge(side), BOUNDS);
            assert_eq!(spot.rect, Rect::from_min_size(min, size), "{side:?}");
            assert_eq!(spot.side, side);
            assert_eq!(spot.anchored, Anchored::Attached);
        }
    }

    #[test]
    fn an_edge_with_a_margin_keeps_clear_of_the_bounds() {
        let margin = d(8.0);
        let size = vec2(100.0, 60.0);
        let cases = [
            (Side::Left, pos2(8.0, 8.0)),
            (Side::Right, pos2(292.0, 8.0)),
            (Side::Top, pos2(8.0, 8.0)),
            (Side::Bottom, pos2(8.0, 232.0)),
        ];
        for (side, min) in cases {
            let p = Placement::edge(side).margin(margin);
            let spot = place(BOUNDS, size, &p, BOUNDS);
            assert_eq!(spot.rect.min, min, "{side:?}");
            assert!(BOUNDS.shrink(8.0).contains_rect(spot.rect));
        }
    }

    #[test]
    fn an_edge_never_flips_and_its_room_is_the_whole_bounds() {
        let size = vec2(500.0, 20.0);
        assert!(size.x > BOUNDS.width());
        let spot = place(BOUNDS, size, &Placement::edge(Side::Left), BOUNDS);
        assert_eq!(spot.side, Side::Left);
        assert_eq!(spot.available, BOUNDS.size());
        assert_eq!(spot.rect.min, BOUNDS.min);
    }

    #[test]
    fn an_edge_aligns_along_the_bounds() {
        let size = vec2(100.0, 60.0);
        let centred = place(
            BOUNDS,
            size,
            &Placement::edge(Side::Top).align(Align::Center),
            BOUNDS,
        );
        assert_eq!(centred.rect.min, pos2(150.0, 0.0));
        let end = place(
            BOUNDS,
            size,
            &Placement::edge(Side::Left).align(Align::End),
            BOUNDS,
        );
        assert_eq!(end.rect.min, pos2(0.0, 240.0));
    }

    mod boundary {
        use super::*;

        const CANVAS: Rect = Rect::from_min_max(pos2(50.0, 40.0), pos2(350.0, 260.0));

        fn insets() -> Sides<Distance> {
            Sides {
                top: d(8.0),
                left: d(8.0),
                right: d(8.0),
                bottom: d(48.0),
            }
        }

        fn inside() -> Rect {
            Rect::from_min_max(pos2(58.0, 48.0), pos2(342.0, 212.0))
        }

        fn bounded(placement: Placement) -> Placement {
            placement.boundary(Boundary::Rect(CANVAS).inset(insets()))
        }

        #[test]
        fn the_default_boundary_is_the_window() {
            let a = anchor();
            for side in SIDES {
                let plain = at(side, Align::Center);
                assert_eq!(
                    place(a, SIZE, &plain.boundary(Boundary::Window), BOUNDS),
                    place(a, SIZE, &plain, BOUNDS)
                );
            }
            assert_eq!(Placement::default().boundary, InsetBoundary::default());
        }

        #[test]
        fn an_anchor_near_each_edge_slides_the_overlay_inside_the_inset_boundary() {
            let anchors = [
                Rect::from_min_size(pos2(CANVAS.left() - 20.0, 100.0), vec2(30.0, 20.0)),
                Rect::from_min_size(pos2(CANVAS.right() - 10.0, 100.0), vec2(30.0, 20.0)),
                Rect::from_min_size(pos2(150.0, CANVAS.top() - 10.0), vec2(30.0, 20.0)),
                Rect::from_min_size(pos2(150.0, CANVAS.bottom() - 10.0), vec2(30.0, 20.0)),
            ];
            for anchor in anchors {
                for side in SIDES {
                    let p = bounded(at(side, Align::Start).flip(Flip::Never));
                    let spot = place(anchor, SIZE, &p, BOUNDS);
                    assert_eq!(spot.side, side);
                    assert!(
                        inside().contains_rect(spot.rect),
                        "{anchor:?} {side:?}: {:?} not in {:?}",
                        spot.rect,
                        inside()
                    );
                }
            }
        }

        #[test]
        fn never_flipping_slides_instead() {
            let low = Rect::from_min_size(pos2(150.0, 190.0), vec2(40.0, 20.0));
            assert!(inside().bottom() - low.bottom() < SIZE.y, "no room below");
            assert!(BOUNDS.bottom() - low.bottom() >= SIZE.y, "the window has");
            let spot = place(
                low,
                SIZE,
                &bounded(Placement::below().flip(Flip::Never)),
                BOUNDS,
            );
            assert_eq!(spot.side, Side::Bottom);
            assert_eq!(spot.rect.bottom(), inside().bottom());
        }

        #[test]
        fn the_flip_decision_uses_the_room_inside_the_boundary() {
            let low = Rect::from_min_size(pos2(150.0, 190.0), vec2(40.0, 20.0));
            assert!(
                BOUNDS.bottom() - low.bottom() >= SIZE.y,
                "the window has room"
            );
            let window_only = place(low, SIZE, &Placement::below(), BOUNDS);
            assert_eq!(window_only.side, Side::Bottom);
            let spot = place(low, SIZE, &bounded(Placement::below()), BOUNDS);
            assert_eq!(spot.side, Side::Top);
            assert_eq!(spot.rect.bottom(), low.top());
        }

        #[test]
        fn available_is_measured_inside_the_boundary() {
            let spot = place(
                Rect::from_min_size(pos2(150.0, 100.0), vec2(40.0, 20.0)),
                SIZE,
                &bounded(Placement::below()),
                BOUNDS,
            );
            assert_eq!(
                spot.available,
                vec2(inside().width(), inside().bottom() - 120.0)
            );
        }

        #[test]
        fn never_flipping_offers_the_whole_boundary_so_the_overlay_keeps_its_size() {
            let at_the_bottom = Rect::from_min_size(pos2(150.0, 200.0), vec2(40.0, 20.0));
            let spot = place(
                at_the_bottom,
                SIZE,
                &bounded(Placement::below().flip(Flip::Never)),
                BOUNDS,
            );
            assert_eq!(spot.available, inside().size());
            assert_eq!(spot.rect.size(), SIZE);
        }

        #[test]
        fn the_window_still_clamps_a_boundary_that_extends_past_it() {
            let wide = Rect::from_min_max(pos2(-100.0, -100.0), pos2(900.0, 900.0));
            let p = Placement::below()
                .flip(Flip::Never)
                .boundary(Boundary::Rect(wide));
            let spot = place(
                Rect::from_min_size(pos2(380.0, 290.0), vec2(20.0, 10.0)),
                SIZE,
                &p,
                BOUNDS,
            );
            assert!(BOUNDS.contains_rect(spot.rect), "{:?}", spot.rect);
            assert_eq!(spot.rect.max, BOUNDS.max);
        }

        #[test]
        fn a_boundary_outside_the_window_leaves_an_empty_area_not_a_panic() {
            let away = Rect::from_min_size(pos2(1000.0, 1000.0), vec2(50.0, 50.0));
            let p = Placement::below().boundary(Boundary::Rect(away));
            let spot = place(anchor(), SIZE, &p, BOUNDS);
            assert_eq!(spot.rect.size(), SIZE);
        }

        #[test]
        fn a_node_boundary_is_a_rect_once_resolved() {
            let node = Id::NULL.with("canvas");
            let p = Placement::below().boundary(Boundary::Node(node).inset(insets()));
            assert_eq!(p.boundary_node(), Some(node));
            let resolved = p.with_node_rect(CANVAS);
            assert_eq!(resolved.boundary_node(), None);
            assert_eq!(resolved, bounded(Placement::below()));
        }
    }
}
