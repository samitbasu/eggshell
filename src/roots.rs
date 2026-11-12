//! Roots: the trees a frame is made of.
//!
//! A frame has one base root and, besides the content of its scroll
//! containers (which hang off their leaves), one root per open overlay. Every
//! root is a root of the same taffy tree and is collected into the same
//! [`Placed`] node list, so ids, hit order, Tab order and drag and drop see one
//! frame. [`RootKind`] is the one place a new kind of root (a pop-out window)
//! has to be added.

use std::collections::HashMap;

use egui::{Id, LayerId, Pos2, Rect, Vec2};
use taffy::{FlexDirection, NodeId, Style};

use crate::dismiss::Opening;
use crate::key::{Key, child_id};
use crate::overlay::{
    Anchor, AnchorLookup, Anchored, Band, Behavior, MatchWidth, Outside, Placement, Side,
};
use crate::pipeline::{Placed, PlacedNode};
use crate::scroll::Description;

/// What a root is, and what only that kind of root declares.
pub(crate) enum RootKind {
    /// The root the application's view builds.
    Base,
    /// A floating layer placed against an anchor.
    Overlay(OverlayDecl),
}

/// What an overlay's chain declared.
pub(crate) struct OverlayDecl {
    pub(crate) id: Id,
    pub(crate) anchor: Anchor,
    pub(crate) placement: Placement,
    pub(crate) band: Band,
    pub(crate) match_width: MatchWidth,
    pub(crate) description: Description,
    pub(crate) behavior: Behavior,
    pub(crate) declared_at: &'static std::panic::Location<'static>,
}

/// A root, from the moment its body starts to be built.
pub(crate) struct RootDecl {
    pub(crate) kind: RootKind,
    /// The root the declaration was made in.
    pub(crate) parent: Option<usize>,
    /// Set when the body is built; the slot is reserved before, so roots
    /// declared in the body come after it.
    pub(crate) node: Option<NodeId>,
    /// The scroll containers' leaves in this root and in their contents, an
    /// enclosing container after those it encloses.
    pub(crate) scrolls: Vec<NodeId>,
}

impl RootDecl {
    pub(crate) fn base() -> Self {
        Self {
            kind: RootKind::Base,
            parent: None,
            node: None,
            scrolls: Vec::new(),
        }
    }

    pub(crate) fn overlay(decl: OverlayDecl, parent: usize) -> Self {
        Self {
            kind: RootKind::Overlay(decl),
            parent: Some(parent),
            node: None,
            scrolls: Vec::new(),
        }
    }
}

/// The style of an overlay's root: its content stacked, as large as it wants
/// until layout clamps it to the room.
pub(crate) fn overlay_style() -> Style {
    Style {
        display: taffy::Display::Flex,
        flex_direction: FlexDirection::Column,
        ..Style::default()
    }
}

/// An overlay after layout.
#[derive(Clone, Debug)]
pub(crate) struct PlacedOverlay {
    pub(crate) id: Id,
    pub(crate) band: Band,
    pub(crate) side: Side,
    pub(crate) anchored: Anchored,
    pub(crate) anchor: Rect,
    pub(crate) anchor_decl: Anchor,
    pub(crate) opening: Opening,
    pub(crate) lookup: AnchorLookup,
    pub(crate) behavior: Behavior,
    /// Position among the open overlays, bottom first: the order they were
    /// opened in.
    pub(crate) stack_index: usize,
    pub(crate) description: Description,
}

/// What a laid-out root is.
#[derive(Clone, Debug)]
pub(crate) enum PlacedRootKind {
    Base,
    Overlay(PlacedOverlay),
}

impl PlacedOverlay {
    /// Whether the overlay takes the press that dismisses it. `root` is the
    /// overlay's root node.
    pub(crate) fn swallows(&self, root: &PlacedNode) -> bool {
        self.behavior.outside() == Outside::Swallow && root.blocks_pointer()
    }
}

/// A root of a laid-out frame.
#[derive(Clone, Debug)]
pub(crate) struct PlacedRoot {
    /// The index of the root's node; its subtree is the nodes up to its `end`.
    pub(crate) first: usize,
    pub(crate) parent: Option<usize>,
    pub(crate) kind: PlacedRootKind,
}

impl PlacedRoot {
    pub(crate) fn overlay(&self) -> Option<&PlacedOverlay> {
        match &self.kind {
            PlacedRootKind::Overlay(overlay) => Some(overlay),
            PlacedRootKind::Base => None,
        }
    }
}

/// Whether the overlay at `root` has a blocker that covers the whole screen:
/// it swallows the press that dismisses it, and no overlay it is nested in
/// does already. A submenu leaves the page to its menu's blocker, so that a
/// press on the menu reaches the menu's items.
pub(crate) fn covers_screen(roots: &[PlacedRoot], nodes: &[PlacedNode], root: usize) -> bool {
    let swallows = |at: usize| {
        roots[at]
            .overlay()
            .is_some_and(|overlay| overlay.swallows(&nodes[roots[at].first]))
    };
    swallows(root)
        && !std::iter::successors(roots[root].parent, |&at| roots[at].parent).any(swallows)
}

/// The roots of a frame.
pub(crate) fn root_of(roots: &[PlacedRoot], index: usize) -> usize {
    roots
        .iter()
        .rposition(|root| root.first <= index)
        .expect("the base root comes first")
}

/// The root that takes the pointer at `p`: the topmost attached overlay
/// whose box blocks it, or the base (root 0) when none does. An overlay that
/// swallows presses blocks the whole screen.
pub(crate) fn root_at(roots: &[PlacedRoot], nodes: &[PlacedNode], p: Pos2) -> usize {
    stacked_top_down(roots)
        .into_iter()
        .find(|&index| {
            let root = &roots[index];
            let node = &nodes[root.first];
            root.overlay()
                .is_some_and(|overlay| overlay.anchored == Anchored::Attached)
                && node.blocks_pointer()
                && (node.rect().contains(p) || covers_screen(roots, nodes, index))
        })
        .unwrap_or(0)
}

/// The indices of the overlay roots, topmost first: higher bands over
/// lower, and inside a band the later opened over the earlier.
pub(crate) fn stacked_top_down(roots: &[PlacedRoot]) -> Vec<usize> {
    let mut found: Vec<(egui::Order, usize, usize)> = roots
        .iter()
        .enumerate()
        .filter_map(|(index, root)| {
            let overlay = root.overlay()?;
            Some((overlay.band.into(), overlay.stack_index, index))
        })
        .collect();
    found.sort_unstable_by(|a, b| b.cmp(a));
    found.into_iter().map(|(_, _, index)| index).collect()
}

impl<A> Placed<A> {
    /// The root that takes the pointer at `p`: the index in the frame's
    /// roots of the topmost attached overlay that blocks it, or 0, the base.
    ///
    /// This is the one answer to "what is on top here": drop targets and the
    /// press that raises a layer ask it, so they cannot disagree with what
    /// the pointer reaches. Whether a press dismisses an overlay is another
    /// question, which `dismissals` answers.
    pub(crate) fn root_at(&self, p: Pos2) -> usize {
        root_at(&self.roots, &self.nodes, p)
    }

    /// The open overlays, the one opened first at the bottom.
    pub fn overlays(&self) -> Vec<OverlayRef<'_>> {
        let mut found: Vec<OverlayRef<'_>> = self
            .roots
            .iter()
            .filter_map(|root| {
                Some(OverlayRef {
                    nodes: &self.nodes,
                    roots: &self.roots,
                    root,
                    overlay: root.overlay()?,
                })
            })
            .collect();
        found.sort_by_key(|overlay| overlay.stack_index());
        found
    }

    /// The overlay declared with `key` at the top of the view.
    ///
    /// # Panics
    ///
    /// If there is none. The message lists the overlays that are open.
    #[track_caller]
    pub fn overlay<K: Key>(&self, key: K) -> OverlayRef<'_> {
        self.try_overlay(&key).unwrap_or_else(|| {
            panic!(
                "no overlay {key:?}; the open overlays are: {}",
                self.overlays()
                    .iter()
                    .map(|overlay| overlay.path())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })
    }

    /// [`overlay`](Self::overlay), or `None` when nothing is there.
    pub fn try_overlay<K: Key>(&self, key: &K) -> Option<OverlayRef<'_>> {
        self.overlay_with(child_id(crate::key::root_id(), key))
    }

    /// The overlay whose root has this id: for a nested overlay, the id of
    /// the node a query found.
    pub fn overlay_with(&self, id: Id) -> Option<OverlayRef<'_>> {
        self.overlays()
            .into_iter()
            .find(|overlay| overlay.overlay.id == id)
    }
}

/// An open overlay of a laid-out frame, for tests.
#[derive(Clone, Copy)]
pub struct OverlayRef<'p> {
    nodes: &'p [PlacedNode],
    roots: &'p [PlacedRoot],
    root: &'p PlacedRoot,
    overlay: &'p PlacedOverlay,
}

impl<'p> OverlayRef<'p> {
    fn node(&self) -> &'p PlacedNode {
        &self.nodes[self.root.first]
    }

    /// The overlay's box: where it was placed, after any flip, shift and
    /// clamp.
    pub fn rect(&self) -> Rect {
        self.node().rect()
    }

    /// The side of its anchor the overlay ended on; a flip changes it.
    pub fn side(&self) -> Side {
        self.overlay.side
    }

    /// Whether the anchor can be seen. A detached overlay is not drawn and
    /// takes no input, but stays open.
    pub fn anchored(&self) -> Anchored {
        self.overlay.anchored
    }

    /// The place among the open overlays, counting from the one opened first.
    pub fn stack_index(&self) -> usize {
        self.overlay.stack_index
    }

    /// The anchor this frame, as a rect.
    pub fn anchor(&self) -> Rect {
        self.overlay.anchor
    }

    /// The band the overlay is in.
    pub fn band(&self) -> Band {
        self.overlay.band
    }

    /// The overlay's readable key path.
    pub fn path(&self) -> &'p str {
        self.node().path()
    }

    /// The overlay's id.
    pub fn id(&self) -> Id {
        self.overlay.id
    }

    /// The egui layer the overlay is drawn on.
    pub fn layer(&self) -> LayerId {
        self.overlay.band.layer(self.overlay.id)
    }

    /// The id of the overlay this one is declared in, if it is nested.
    pub fn nested_in(&self) -> Option<Id> {
        Some(self.roots[self.root.parent?].overlay()?.id)
    }

    /// The size the overlay occupies.
    pub fn size(&self) -> Vec2 {
        self.rect().size()
    }
}

/// The index of every identified node, for looking anchors up while roots
/// are still being collected.
pub(crate) type ById = HashMap<Id, usize>;

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2};

    use super::*;
    use crate::testing::Headless;
    use crate::tw;

    /// Overlays of 100 by 100 at (0, 0), (50, 50) and so on, in the given
    /// bands, opened in the order given.
    fn placed(bands: &[Band]) -> Placed<()> {
        let bands = bands.to_vec();
        Headless::new(vec2(400.0, 300.0)).layout(&(), move |b, _| {
            for (i, band) in bands.into_iter().enumerate() {
                let at = 50.0 * i as f32;
                b.overlay(i)
                    .anchor(Anchor::Point(pos2(at, at)))
                    .band(band)
                    .children(|s| {
                        s.div_id("box", tw().w_px(100.0).h_px(100.0), |_| {});
                    });
            }
        })
    }

    fn root_of_overlay(placed: &Placed<()>, key: usize) -> usize {
        let id = placed.overlay(key).id();
        placed
            .roots
            .iter()
            .position(|root| root.overlay().is_some_and(|overlay| overlay.id == id))
            .expect("the overlay is a root")
    }

    #[test]
    fn the_page_is_where_no_overlay_is() {
        let placed = placed(&[Band::Popup]);
        assert_eq!(
            placed.root_at(pos2(50.0, 50.0)),
            root_of_overlay(&placed, 0_usize)
        );
        assert_eq!(placed.root_at(pos2(300.0, 250.0)), 0);
    }

    #[test]
    fn the_later_opened_overlay_is_on_top_where_two_overlap() {
        let placed = placed(&[Band::Popup, Band::Popup]);
        let overlap = pos2(75.0, 75.0);
        assert!(
            placed.overlay(0_usize).rect().contains(overlap)
                && placed.overlay(1_usize).rect().contains(overlap),
            "precondition: both contain the point"
        );
        assert_eq!(placed.root_at(overlap), root_of_overlay(&placed, 1_usize));
        assert_eq!(
            placed.root_at(pos2(25.0, 25.0)),
            root_of_overlay(&placed, 0_usize)
        );
    }

    #[test]
    fn a_higher_band_is_on_top_whatever_the_opening_order() {
        let placed = placed(&[Band::Popup, Band::Floating]);
        assert!(placed.overlay(1_usize).stack_index() > placed.overlay(0_usize).stack_index());
        assert_eq!(
            placed.root_at(pos2(75.0, 75.0)),
            root_of_overlay(&placed, 0_usize)
        );
    }

    #[test]
    fn a_tooltip_never_takes_the_pointer() {
        let placed = placed(&[Band::Popup, Band::Tooltip]);
        let both = pos2(75.0, 75.0);
        assert!(
            placed.overlay(0_usize).rect().contains(both)
                && placed.overlay(1_usize).rect().contains(both),
            "precondition: both contain the point"
        );
        assert_eq!(placed.root_at(both), root_of_overlay(&placed, 0_usize));
        assert_eq!(
            placed.root_at(pos2(140.0, 140.0)),
            0,
            "only the tooltip is there"
        );
    }

    #[test]
    fn a_node_blocks_the_pointer_by_what_it_paints() {
        use egui::Color32;
        let placed = Headless::new(vec2(100.0, 100.0)).layout(&(), |b, _| {
            let size = |classes: crate::Tw| classes.w_px(10.0).h_px(10.0);
            b.div_id("bare", size(tw()), |_| {});
            b.div_id("filled", size(tw().bg(Color32::RED)), |_| {});
            b.div_id(
                "framed",
                size(tw().border(1.0).border_color(Color32::RED)),
                |_| {},
            );
            b.div_id("colour_only", size(tw().border_color(Color32::RED)), |_| {});
            b.div_id("width_only", size(tw().border(1.0)), |_| {});
            b.div_id(
                "through",
                size(tw().bg(Color32::RED).pointer_events_none()),
                |_| {},
            );
        });
        let blocks = |key: &'static str| placed.at(key).node().expect("a node").blocks_pointer();
        assert!(!blocks("bare"));
        assert!(blocks("filled"));
        assert!(blocks("framed"));
        assert!(
            !blocks("colour_only"),
            "a colour with no width paints nothing"
        );
        assert!(
            !blocks("width_only"),
            "a width with no colour paints nothing"
        );
        assert!(!blocks("through"));
    }
}
