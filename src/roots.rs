//! Roots: the trees a frame is made of.
//!
//! A frame has one base root and, besides the content of its scroll
//! containers (which hang off their leaves), one root per open overlay. Every
//! root is a root of the same taffy tree and is collected into the same
//! [`Placed`] node list, so ids, hit order, Tab order and drag and drop see one
//! frame. [`RootKind`] is the one place a new kind of root (a pop-out window)
//! has to be added. The content of a region is not a root of its own here: it
//! hangs off the region's leaf, as a scroll container's does.

use std::collections::HashMap;

use egui::{Id, LayerId, Pos2, Rect, Vec2};
use taffy::{AlignContent, AlignItems, FlexDirection, NodeId, Size, Style, style_helpers::length};

use crate::dismiss::Opening;
use crate::key::{Key, child_id};
use crate::overlay::{
    Anchor, AnchorLookup, Anchored, Band, Behavior, Focus, MatchWidth, Modal, Outside, Placement,
    Side,
};
use crate::pipeline::{Placed, PlacedNode};
use crate::presence::Staged;
use crate::scroll::Description;

/// What a root is, and what only that kind of root declares.
pub(crate) enum RootKind {
    /// The root the application's view builds.
    Base,
    /// A floating layer placed against an anchor.
    Overlay(OverlayDecl),
    /// A popped-out subtree, in a window of its own.
    Window(crate::window::WindowDecl),
}

/// How an overlay is put on screen: next to an anchor, or as a modal.
pub(crate) enum Presentation {
    Floating {
        anchor: Anchor,
        placement: Placement,
        match_width: MatchWidth,
    },
    Modal(Modal),
}

/// What an overlay's chain declared.
pub(crate) struct OverlayDecl {
    pub(crate) id: Id,
    pub(crate) presentation: Presentation,
    pub(crate) band: Band,
    pub(crate) description: Description,
    pub(crate) behavior: Behavior,
    pub(crate) declared_at: &'static std::panic::Location<'static>,
    pub(crate) staged: Staged,
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
    pub(crate) fn is_modal(&self) -> bool {
        matches!(&self.kind, RootKind::Overlay(overlay) if matches!(overlay.presentation, Presentation::Modal(_)))
    }

    pub(crate) fn base() -> Self {
        Self {
            kind: RootKind::Base,
            parent: None,
            node: None,
            scrolls: Vec::new(),
        }
    }

    pub(crate) fn window(decl: crate::window::WindowDecl, parent: usize) -> Self {
        Self {
            kind: RootKind::Window(decl),
            parent: Some(parent),
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

/// The style of a modal's root: the screen, with its one child (the box)
/// placed by `modal`.
pub(crate) fn modal_style(modal: Modal, area: Rect) -> Style {
    let (direction, justify, align) = match modal {
        Modal::Center => (FlexDirection::Row, AlignContent::CENTER, AlignItems::CENTER),
        Modal::Edge(Side::Left) => (
            FlexDirection::Row,
            AlignContent::FLEX_START,
            AlignItems::STRETCH,
        ),
        Modal::Edge(Side::Right) => (
            FlexDirection::Row,
            AlignContent::FLEX_END,
            AlignItems::STRETCH,
        ),
        Modal::Edge(Side::Top) => (
            FlexDirection::Column,
            AlignContent::FLEX_START,
            AlignItems::STRETCH,
        ),
        Modal::Edge(Side::Bottom) => (
            FlexDirection::Column,
            AlignContent::FLEX_END,
            AlignItems::STRETCH,
        ),
    };
    Style {
        display: taffy::Display::Flex,
        flex_direction: direction,
        justify_content: justify,
        align_items: align,
        size: Size {
            width: length(area.width()),
            height: length(area.height()),
        },
        ..Style::default()
    }
}

/// How an overlay ended up on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// Next to its anchor, on this side of it.
    Floating(Side),
    Modal(Modal),
}

/// An overlay after layout.
#[derive(Clone, Debug)]
pub(crate) struct PlacedOverlay {
    pub(crate) id: Id,
    pub(crate) band: Band,
    pub(crate) form: Form,
    pub(crate) anchored: Anchored,
    pub(crate) anchor: Rect,
    /// The part of the window the overlay may be in.
    pub(crate) bounds: Rect,
    /// The size the overlay measured at before the room clamped it.
    pub(crate) natural: Vec2,
    pub(crate) anchor_decl: Anchor,
    pub(crate) opening: Opening,
    pub(crate) lookup: AnchorLookup,
    pub(crate) behavior: Behavior,
    /// Position among the open overlays, bottom first: the order they were
    /// opened in.
    pub(crate) stack_index: usize,
    pub(crate) description: Description,
    pub(crate) staged: Staged,
}

/// What a laid-out root is.
#[derive(Clone, Debug)]
pub(crate) enum PlacedRootKind {
    Base,
    Overlay(PlacedOverlay),
    Window(crate::window::PlacedWindow),
}

/// Where a root is drawn: in the viewport the frame is drawn in, or in the
/// native window of the pop-out at this root index. A pop-out's window
/// embedded in the viewport (egui's `embed_viewports`) is on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Home {
    Page,
    Window(usize),
}

impl Home {
    /// The home of the root at `root`; the page, if there is no such root.
    pub(crate) fn of(roots: &[PlacedRoot], root: usize) -> Self {
        roots.get(root).map_or(Self::Page, |root| root.home)
    }

    /// The pop-out window this home is the native window of; none for the
    /// page, or for an index that names no window's root.
    pub(crate) fn window(self, roots: &[PlacedRoot]) -> Option<&crate::window::PlacedWindow> {
        match self {
            Self::Page => None,
            Self::Window(root) => roots.get(root)?.window(),
        }
    }
}

/// The root at `index` and its root node.
pub(crate) fn root_and_node<'a>(
    roots: &'a [PlacedRoot],
    nodes: &'a [PlacedNode],
    index: usize,
) -> Option<(&'a PlacedRoot, &'a PlacedNode)> {
    let root = roots.get(index)?;
    Some((root, nodes.get(root.first)?))
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
    pub(crate) home: Home,
    pub(crate) kind: PlacedRootKind,
}

impl PlacedRoot {
    pub(crate) fn is_modal(&self) -> bool {
        self.overlay()
            .is_some_and(|overlay| matches!(overlay.form, Form::Modal(_)))
    }

    pub(crate) fn overlay(&self) -> Option<&PlacedOverlay> {
        match &self.kind {
            PlacedRootKind::Overlay(overlay) => Some(overlay),
            PlacedRootKind::Base | PlacedRootKind::Window(_) => None,
        }
    }

    pub(crate) fn window(&self) -> Option<&crate::window::PlacedWindow> {
        match &self.kind {
            PlacedRootKind::Window(window) => Some(window),
            PlacedRootKind::Base | PlacedRootKind::Overlay(_) => None,
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

/// The roots one pass draws, with one viewport's input: those at `home`,
/// over `root`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Here {
    pub(crate) home: Home,
    pub(crate) root: usize,
}

impl Here {
    /// The page: the base root and what is drawn over it.
    pub(crate) const PAGE: Self = Self {
        home: Home::Page,
        root: 0,
    };

    /// The pass of the root at `root`.
    pub(crate) fn of(roots: &[PlacedRoot], root: usize) -> Self {
        Self {
            home: Home::of(roots, root),
            root,
        }
    }
}

/// The root that takes the pointer at `p` in the pass `here`: the topmost
/// attached overlay whose box blocks it, else the last declared embedded
/// pop-out window it is over, else `here`'s root. An overlay that swallows
/// presses blocks the whole screen.
///
/// This is the one answer to "what is on top here": drop targets and the
/// press that raises a layer ask it, so they cannot disagree with what the
/// pointer reaches. Whether a press dismisses an overlay is another question,
/// which `dismissals` answers.
pub(crate) fn root_at(roots: &[PlacedRoot], nodes: &[PlacedNode], p: Pos2, here: Here) -> usize {
    let overlay = stacked_top_down(roots, here).into_iter().find(|&index| {
        root_and_node(roots, nodes, index).is_some_and(|(root, node)| {
            root.overlay()
                .is_some_and(|overlay| overlay.anchored == Anchored::Attached)
                && node.blocks_pointer()
                && (node.rect().contains(p) || covers_screen(roots, nodes, index))
        })
    });
    let window = || {
        roots.iter().rposition(|root| {
            root.home == here.home
                && root.window().is_some_and(|window| {
                    window.presentation == crate::window::Presentation::Embedded
                        && window.content.contains(p)
                })
        })
    };
    overlay.or_else(window).unwrap_or(here.root)
}

/// The indices of the overlay roots of the pass `here`, topmost first:
/// higher bands over lower, and inside a band the later opened over the
/// earlier.
pub(crate) fn stacked_top_down(roots: &[PlacedRoot], here: Here) -> Vec<usize> {
    let mut found: Vec<(egui::Order, usize, usize)> = roots
        .iter()
        .enumerate()
        .filter(|(_, root)| root.home == here.home)
        .filter_map(|(index, root)| {
            let overlay = root.overlay()?;
            Some((overlay.band.into(), overlay.stack_index, index))
        })
        .collect();
    found.sort_unstable_by(|a, b| b.cmp(a));
    found.into_iter().map(|(_, _, index)| index).collect()
}

impl<A> Placed<A> {
    /// The open overlays, the one opened first at the bottom.
    pub fn overlays(&self) -> Vec<OverlayRef<'_>> {
        let mut found: Vec<OverlayRef<'_>> = self
            .roots
            .iter()
            .filter_map(|root| {
                Some(OverlayRef {
                    node: self.nodes.get(root.first)?,
                    roots: &self.roots,
                    root,
                    overlay: root.overlay()?,
                })
            })
            .collect();
        found.sort_by_key(|overlay| overlay.stack_index());
        found
    }

    /// TODO(prose): the overlay declared with `key` at the top of the view, or `None` when it is not open.
    ///
    /// ```
    /// use eggshell_ui::testing::Headless;
    /// use eggshell_ui::overlay::Anchor;
    /// use eggshell_ui::{Builder, tw};
    /// use egui::{pos2, vec2};
    ///
    /// let placed = Headless::new(vec2(200.0, 200.0)).layout(&(), |b: &mut Builder<'_, '_, ()>, _: &()| {
    ///     b.overlay("menu").anchor(Anchor::Point(pos2(10.0, 10.0))).children(|s| {
    ///         s.div_id("item", tw().w_px(40.0).h_px(20.0), |_| {});
    ///     });
    /// });
    /// assert!(placed.overlay("menu").is_some());
    /// assert!(placed.overlay("other").is_none());
    /// ```
    pub fn overlay<K: Key>(&self, key: K) -> Option<OverlayRef<'_>> {
        self.overlay_with(child_id(self.root, &key))
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
    node: &'p PlacedNode,
    roots: &'p [PlacedRoot],
    root: &'p PlacedRoot,
    overlay: &'p PlacedOverlay,
}

impl<'p> OverlayRef<'p> {
    /// The overlay's box: where it was placed, after any flip, shift and
    /// clamp.
    pub fn rect(&self) -> Rect {
        self.node.rect()
    }

    /// The side of its anchor a floating overlay ended on; a flip changes it.
    /// A modal has none.
    pub fn side(&self) -> Option<Side> {
        match self.overlay.form {
            Form::Floating(side) => Some(side),
            Form::Modal(_) => None,
        }
    }

    /// How the overlay presents, if it is a modal.
    pub fn modal(&self) -> Option<Modal> {
        match self.overlay.form {
            Form::Modal(modal) => Some(modal),
            Form::Floating(_) => None,
        }
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

    /// The part of the window the overlay may be in: the window, cut to its
    /// [`Placement::boundary`](crate::overlay::Placement::boundary). A modal
    /// has the whole window.
    pub fn bounds(&self) -> Rect {
        self.overlay.bounds
    }

    /// The size the overlay measured at before the room clamped it.
    pub fn natural_size(&self) -> Vec2 {
        self.overlay.natural
    }

    /// Whether the overlay is leaving: it is kept on screen, inert, until its
    /// exit has run, and is not one of the overlays that are open.
    pub fn is_exiting(&self) -> bool {
        self.overlay.staged.is_exiting()
    }

    /// Where focus goes when the overlay opens.
    pub fn focus(&self) -> Focus {
        self.overlay.behavior.focus
    }

    /// What becomes of the press that dismisses the overlay.
    pub fn outside(&self) -> Outside {
        self.overlay.behavior.outside()
    }

    /// The band the overlay is in.
    pub fn band(&self) -> Band {
        self.overlay.band
    }

    /// The overlay's readable key path.
    pub fn path(&self) -> &'p str {
        self.node.path()
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
        Some(self.roots.get(self.root.parent?)?.overlay()?.id)
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
        let id = placed.expect_overlay(key).id();
        placed
            .roots
            .iter()
            .position(|root| root.overlay().is_some_and(|overlay| overlay.id == id))
            .expect("the overlay is a root")
    }

    #[test]
    fn an_overlay_that_is_not_open_is_none() {
        let placed = placed(&[Band::Popup]);
        assert!(placed.overlay(0_usize).is_some());
        assert!(placed.overlay(1_usize).is_none());
    }

    #[test]
    #[should_panic(expected = "no overlay 1; the open overlays are: /0")]
    fn expecting_an_overlay_that_is_not_open_names_the_open_ones() {
        placed(&[Band::Popup]).expect_overlay(1_usize);
    }

    #[test]
    fn the_page_is_where_no_overlay_is() {
        let placed = placed(&[Band::Popup]);
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, pos2(50.0, 50.0), Here::PAGE),
            root_of_overlay(&placed, 0_usize)
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, pos2(300.0, 250.0), Here::PAGE),
            0
        );
    }

    #[test]
    fn the_later_opened_overlay_is_on_top_where_two_overlap() {
        let placed = placed(&[Band::Popup, Band::Popup]);
        let overlap = pos2(75.0, 75.0);
        assert!(
            placed.expect_overlay(0_usize).rect().contains(overlap)
                && placed.expect_overlay(1_usize).rect().contains(overlap),
            "precondition: both contain the point"
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, overlap, Here::PAGE),
            root_of_overlay(&placed, 1_usize)
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, pos2(25.0, 25.0), Here::PAGE),
            root_of_overlay(&placed, 0_usize)
        );
    }

    #[test]
    fn a_higher_band_is_on_top_whatever_the_opening_order() {
        let placed = placed(&[Band::Popup, Band::Floating]);
        assert!(
            placed.expect_overlay(1_usize).stack_index()
                > placed.expect_overlay(0_usize).stack_index()
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, pos2(75.0, 75.0), Here::PAGE),
            root_of_overlay(&placed, 0_usize)
        );
    }

    #[test]
    fn a_tooltip_never_takes_the_pointer() {
        let placed = placed(&[Band::Popup, Band::Tooltip]);
        let both = pos2(75.0, 75.0);
        assert!(
            placed.expect_overlay(0_usize).rect().contains(both)
                && placed.expect_overlay(1_usize).rect().contains(both),
            "precondition: both contain the point"
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, both, Here::PAGE),
            root_of_overlay(&placed, 0_usize)
        );
        assert_eq!(
            root_at(&placed.roots, &placed.nodes, pos2(140.0, 140.0), Here::PAGE),
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
