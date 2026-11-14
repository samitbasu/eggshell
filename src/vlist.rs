//! Virtual lists: [`Builder::virtual_list`](crate::Builder::virtual_list).
//!
//! A virtual list is a scroll container that builds only the rows in and
//! near its viewport. Its rows are built as a [region](crate::Builder::region)'s
//! content is: once the container has been laid out in the frame, so the
//! viewport and the offset they are chosen for are the frame's own.
//!
//! The list keeps, per [`LayoutState`](crate::LayoutState), the height each
//! row measured the last time it was built, and takes a row it has not
//! measured to be the estimate ([`ListChain::estimate_px`]). Running sums of those
//! heights place the rows: the built rows are stacked in one column at the
//! first one's running offset, so a wrong estimate never makes them overlap
//! or leave gaps, and the content is as tall as the sum. The heights are
//! forgotten when the list's width changes, and a row that leaves the list
//! loses its height.
//!
//! The first visible row stays where it is on the screen when the rows above
//! it change: when they are measured anew, and when rows are inserted above
//! it or removed, the scroll offset moves by as much as the row did (scroll
//! anchoring), except at the very top, where new rows show. A list scrolled
//! to its end stays at its end while the rows there are measured.
//!
//! A row built in the last frame is built again, wherever it is, while a
//! widget inside it has keyboard focus, while an overlay declared inside it
//! is open, or while an overlay is anchored to a node inside it: egui drops
//! the focus of a widget that is not drawn, and an overlay needs its anchor.
//!
//! Tab reaches the built rows only.

use std::collections::HashMap;
use std::ops::Range;

use egui::{Align, Context, Id, IdSalt, Rect, Vec2, pos2, vec2};
use nohash_hasher::IntMap;
use taffy::{NodeId, TaffyTree};

use crate::builder::{Builder, NodeHandle, Scope, Slot};
use crate::kept::Handle;
use crate::key::Key;
use crate::pipeline::Node;
use crate::scroll::{self, Axis, ScrollChain, TabRule};
use crate::tw::{Tw, tw};

/// How a list sizes its rows, and how far past its viewport it builds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VList {
    pub(crate) rows: RowHeights,
    pub(crate) overscan: f32,
}

/// What a list knows of its rows' heights before it lays them out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RowHeights {
    /// Measured once built; until then, the estimate.
    Measured { estimate: f32 },
    /// All the same height.
    Fixed(f32),
}

impl RowHeights {
    fn measured(estimate: f32) -> Self {
        Self::Measured {
            estimate: row_height(estimate),
        }
    }

    fn fixed(height: f32) -> Self {
        Self::Fixed(row_height(height))
    }
}

/// `points` as the height of a row not laid out: at least [`MIN_ROW`], which
/// a height that is not a number becomes too.
fn row_height(points: f32) -> f32 {
    if points.is_finite() {
        points.max(MIN_ROW)
    } else {
        MIN_ROW
    }
}

/// The overscan of a list unless set: rows this far beyond each edge of the
/// viewport are built too.
const OVERSCAN: f32 = 500.0;

/// The least height a row counts as before it is laid out: a row of no
/// height would bring every row after it into view.
const MIN_ROW: f32 = 1.0;

/// How much a height may change and still count as unchanged.
const SAME: f32 = 0.01;

/// A virtual list, started with [`Builder::virtual_list`] and finished with
/// [`items`](Self::items).
///
/// Rows of differing heights are measured as they are built; until then a
/// row counts as [`estimate_px`](Self::estimate_px) tall, one line of the
/// list's text unless set. Rows that are all one height are faster as
/// [`fixed_px`](Self::fixed_px). The last of the two called wins.
#[must_use = "a virtual list is not added until you call `.items(..)`"]
pub struct ListChain<'a, 'f, A, R> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    classes: Tw,
    rows: &'f [R],
    heights: Option<RowHeights>,
    overscan: f32,
}

impl<'a, 'f, A: 'static, R: Key + Copy> ListChain<'a, 'f, A, R> {
    pub(crate) fn new(b: Builder<'a, 'f, A>, slot: Slot, classes: Tw, rows: &'f [R]) -> Self {
        Self {
            b,
            slot,
            classes,
            rows,
            heights: None,
            overscan: OVERSCAN,
        }
    }

    /// Rows of differing heights, each measured the first time it is built:
    /// until then a row counts as `points` tall. Less than one point, or a
    /// height that is not a number, becomes one.
    pub fn estimate_px(mut self, points: f32) -> Self {
        self.heights = Some(RowHeights::measured(points));
        self
    }

    /// Rows that are all `points` tall: nothing is measured, and each row is
    /// given that height. Less than one point, or a height that is not a
    /// number, becomes one.
    pub fn fixed_px(mut self, points: f32) -> Self {
        self.heights = Some(RowHeights::fixed(points));
        self
    }

    /// Builds the rows within `points` of the viewport's edges as well, so
    /// that a fast scroll shows rows rather than blank space. 500 unless set.
    pub fn overscan_px(mut self, points: f32) -> Self {
        self.overscan = points.max(0.0);
        self
    }

    /// Adds the list, whose rows `item` builds, each under its own key. The
    /// returned handle attaches behaviour to the list's scroll container.
    #[track_caller]
    pub fn items(self, item: impl FnMut(&mut Scope<'_, 'f, A>, R) + 'f) -> NodeHandle<'a, 'f, A> {
        let Self {
            mut b,
            slot,
            classes,
            rows,
            heights,
            overscan,
        } = self;
        let list = slot.id;
        let kept = b.kept().clone();
        let pass = b.ctx().cumulative_pass_nr();
        let mut planned = None;
        let scroll = ScrollChain::with_slot(b.reborrow(), slot, classes)
            .children(|s| {
                let heights = heights
                    .unwrap_or_else(|| RowHeights::measured(s.text_style().spec.line_height));
                let plan = Plan::new(
                    &kept,
                    pass,
                    list,
                    VList {
                        rows: heights,
                        overscan,
                    },
                    rows,
                );
                let leaf = s.rows_leaf(plan.total());
                planned = Some((plan, leaf));
            })
            .node();
        let (plan, leaf) = planned.expect("the content was built");
        b.list_region(leaf, list, plan.run(scroll, Items::new(rows, item)));
        b.into_handle(scroll)
    }
}

/// Scrolls the virtual list `list` (the id of its key, from
/// [`Builder::id_of`](crate::Builder::id_of)) to show the row `row`. With an
/// `align`, the row is aligned that way in the viewport; without, the list
/// moves the least that shows it.
///
/// The next layout of the list builds the row and the rows around it where
/// its running offsets put it, and corrects once the row is measured. A
/// request for a row the list does not have is dropped.
// Kept in the context under the list's id, as `scroll::scroll_by` keeps its
// requests; #156 is to give these ctx-only scroll requests a route to the
// layout's own state, and this one moves with them.
pub fn scroll_to(ctx: &Context, list: Id, row: impl Key, align: Option<Align>) {
    ctx.data_mut(|data| {
        data.insert_temp(
            request_key(list),
            ScrollTo {
                row: row_key(&row),
                align,
            },
        );
    });
    ctx.request_repaint();
}

#[derive(Clone, Copy, Debug)]
struct ScrollTo {
    /// The row's key in the list's height cache.
    row: IdSalt,
    align: Option<Align>,
}

/// The key of `row` in its list's height cache: unique in the list, whose
/// rows are all of one type.
fn row_key(row: &impl Key) -> IdSalt {
    IdSalt::new(row)
}

fn request_key(list: Id) -> Id {
    list.with("eggshell-vlist-scroll-to")
}

/// The key of the leaf a list's rows are built into.
#[derive(Clone, Copy, Hash)]
pub(crate) struct RowsKey;

impl std::fmt::Debug for RowsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<rows>")
    }
}

/// What a list keeps from one frame to the next.
#[derive(Default)]
pub(crate) struct ListState {
    /// Each row's height as last measured, by the row's key.
    heights: IntMap<IdSalt, f32>,
    /// The width the heights were measured at.
    width: Option<f32>,
    /// The rows built in the last frame.
    built: Vec<BuiltRow>,
    /// What the last frame held still for the next.
    hold: Option<Hold>,
    /// Kept allocations.
    keys: Vec<IdSalt>,
    offsets: Vec<f32>,
    pass: u64,
}

/// A row built in the last frame: what may keep it built.
#[derive(Clone, Debug)]
struct BuiltRow {
    key: IdSalt,
    at: usize,
    /// The row's own id.
    id: Id,
    /// The identified nodes inside it, an overlay's anchor among them.
    inside: Vec<Id>,
    /// The overlays declared inside it.
    overlays: Vec<Id>,
}

/// What a list keeps in place on the screen while the rows above it change.
#[derive(Clone, Copy, Debug)]
enum Hold {
    /// The first visible row, by its key, where it was found, and its top
    /// as laid out.
    Row { key: IdSalt, at: usize, top: f32 },
    /// The end of the list, which the offset was at.
    End { offset: f32 },
}

/// The lists of a `LayoutState`, by the list's id.
#[derive(Default)]
pub(crate) struct Lists(HashMap<Id, ListState>);

impl Lists {
    /// Forgets the lists not laid out in the pass `pass`.
    pub(crate) fn sweep(&mut self, pass: u64) {
        self.0.retain(|_, list| list.pass == pass);
    }
}

/// Where the rows of a list go, by their heights.
#[derive(Debug)]
enum Tops {
    Fixed { rows: usize, height: f32 },
    Measured { offsets: Vec<f32> },
}

impl Tops {
    fn len(&self) -> usize {
        match self {
            Self::Fixed { rows, .. } => *rows,
            Self::Measured { offsets } => offsets.len() - 1,
        }
    }

    fn top(&self, at: usize) -> f32 {
        match self {
            Self::Fixed { height, .. } => at as f32 * height,
            Self::Measured { offsets } => offsets[at],
        }
    }

    fn height(&self, at: usize) -> f32 {
        self.top(at + 1) - self.top(at)
    }

    fn total(&self) -> f32 {
        self.top(self.len())
    }

    fn visible(&self, span: Span) -> Range<usize> {
        match self {
            Self::Fixed { rows, height } => visible_fixed(*rows, *height, span),
            Self::Measured { offsets } => visible(offsets, span),
        }
    }
}

/// A list's rows and how to build one, with the row type erased.
pub(crate) trait Rows<'f, A> {
    fn len(&self) -> usize;
    /// The key of row `at` in the height cache.
    fn key(&self, at: usize) -> IdSalt;
    /// Builds row `at` under its own key with `classes`, returning its id
    /// and node.
    fn build(&mut self, b: &mut Builder<'_, 'f, A>, at: usize, classes: Tw) -> (Id, NodeId);
}

pub(crate) struct Items<'f, R, F> {
    rows: &'f [R],
    item: F,
}

impl<'f, R, F> Items<'f, R, F> {
    pub(crate) fn new(rows: &'f [R], item: F) -> Self {
        Self { rows, item }
    }
}

impl<'f, A, R: Key + Copy, F: FnMut(&mut Scope<'_, 'f, A>, R)> Rows<'f, A> for Items<'f, R, F> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> IdSalt {
        row_key(&self.rows[at])
    }

    fn build(&mut self, b: &mut Builder<'_, 'f, A>, at: usize, classes: Tw) -> (Id, NodeId) {
        let row = self.rows[at];
        let item = &mut self.item;
        let handle = b.div_id(row, classes, |s| item(s, row));
        (handle.id(), handle.node())
    }
}

/// A list as its build leaves it: where its rows go, before the frame's
/// layout says which are seen.
pub(crate) struct Plan {
    kept: Handle,
    list: Id,
    rows: RowHeights,
    overscan: f32,
    tops: Tops,
    keys: Vec<IdSalt>,
}

impl Plan {
    /// Places `rows` of the list `list` by the heights kept for them.
    pub(crate) fn new<R: Key>(
        kept: &Handle,
        pass: u64,
        list: Id,
        settings: VList,
        rows: &[R],
    ) -> Self {
        let VList {
            rows: heights,
            overscan,
        } = settings;
        let (tops, keys) = kept.with(|kept| {
            let state = kept.lists.0.entry(list).or_default();
            state.pass = pass;
            match heights {
                RowHeights::Fixed(height) => (
                    Tops::Fixed {
                        rows: rows.len(),
                        height,
                    },
                    Vec::new(),
                ),
                RowHeights::Measured { estimate } => {
                    let mut keys = std::mem::take(&mut state.keys);
                    keys.clear();
                    keys.extend(rows.iter().map(row_key));
                    let mut offsets = std::mem::take(&mut state.offsets);
                    let mut known = 0;
                    running_offsets(
                        keys.iter().map(|key| match state.heights.get(key) {
                            Some(&height) => {
                                known += 1;
                                height
                            }
                            None => estimate,
                        }),
                        &mut offsets,
                    );
                    if known < state.heights.len() {
                        state.heights = keys
                            .iter()
                            .filter_map(|key| Some((*key, *state.heights.get(key)?)))
                            .collect();
                    }
                    (Tops::Measured { offsets }, keys)
                }
            }
        });
        Self {
            kept: kept.clone(),
            list,
            rows: heights,
            overscan,
            tops,
            keys,
        }
    }

    /// The height of the content: every row at its kept or estimated height.
    pub(crate) fn total(&self) -> f32 {
        self.tops.total()
    }

    /// The list, to be run once the scroll container `scroll` is laid out.
    pub(crate) fn run<'f, A>(self, scroll: NodeId, rows: impl Rows<'f, A> + 'f) -> ListRun<'f, A> {
        ListRun {
            plan: self,
            scroll,
            rows: Box::new(rows),
        }
    }
}

/// A list waiting for its container's layout.
pub(crate) struct ListRun<'f, A> {
    plan: Plan,
    scroll: NodeId,
    rows: Box<dyn Rows<'f, A> + 'f>,
}

/// Where a list's viewport is over its rows this frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct View {
    /// The rows' top in the content.
    top: f32,
    /// How far the content is scrolled.
    offset: f32,
    viewport: Vec2,
    /// The content's height.
    content: f32,
}

impl View {
    /// The farthest the content can be scrolled.
    fn limit(&self) -> f32 {
        (self.content - self.viewport.y).max(0.0)
    }

    /// Whether the content is scrolled to its end, and can be scrolled.
    fn at_end(&self) -> bool {
        let limit = self.limit();
        limit > 0.0 && self.offset >= limit - SAME
    }

    /// The part of the rows `margin` beyond the viewport's edges.
    fn span(&self, margin: f32) -> Span {
        let start = self.offset - self.top;
        Span {
            start: start - margin,
            end: start + self.viewport.y + margin,
        }
    }
}

impl Plan {
    fn with_state<T>(&self, f: impl FnOnce(&mut ListState) -> T) -> T {
        self.kept
            .with(|kept| f(kept.lists.0.entry(self.list).or_default()))
    }
}

impl<'f, A> ListRun<'f, A> {
    /// Where the viewport of the laid-out container is over the rows, which
    /// `leaf` holds, `width` wide. Forgets the heights if the width changed,
    /// keeps the first visible row in place, and takes up a
    /// [`scroll_to`] request.
    pub(crate) fn view(
        &mut self,
        ctx: &Context,
        tree: &TaffyTree<Node>,
        leaf: NodeId,
        width: f32,
    ) -> View {
        let list = self.plan.list;
        let own = tree.layout(self.scroll).expect("node exists");
        let node = tree
            .get_node_context(self.scroll)
            .and_then(|node| node.scroll.as_ref())
            .expect("a list's container is a scroll container");
        let viewport = vec2(
            own.size.width - own.border.left - own.border.right - node.bars.gutter.x,
            own.size.height - own.border.top - own.border.bottom - node.bars.gutter.y,
        )
        .max(Vec2::ZERO);
        let mut view = View {
            top: tree.layout(leaf).expect("node exists").location.y,
            offset: scroll::stored_offset(ctx, list, Axis::Y, node.size, viewport).y,
            viewport,
            content: node.size.y,
        };
        let requested = scroll::requested_by(ctx, list).y;
        let moved = match self.forget_if_resized(width) {
            Some(Hold::End { offset }) if view.offset >= offset - SAME => {
                view.limit() - view.offset
            }
            Some(Hold::Row { key, at, top }) if view.offset > 0.0 => self
                .find(key, at)
                .map_or(0.0, |at| self.plan.tops.top(at) - top),
            _ => 0.0,
        };
        if moved.abs() > SAME {
            view.offset += moved;
            scroll::scroll_by(ctx, list, vec2(0.0, moved));
        }
        view.offset = (view.offset + requested).clamp(0.0, view.limit());
        let request = ctx.data_mut(|data| {
            let request = data.get_temp::<ScrollTo>(request_key(list));
            data.remove::<ScrollTo>(request_key(list));
            request
        });
        if let Some(request) = request
            && let Some(at) = self.find(request.row, 0)
        {
            let tops = &self.plan.tops;
            let row = (view.top + tops.top(at), view.top + tops.top(at + 1));
            let shown = (view.offset, view.offset + view.viewport.y);
            view.offset =
                (view.offset + scroll::reveal(shown, row, request.align)).clamp(0.0, view.limit());
            let rect = Rect::from_min_max(pos2(0.0, row.0), pos2(view.viewport.x, row.1));
            scroll::scroll_to_rect(ctx, list, rect, request.align);
        }
        view
    }

    /// Forgets the kept heights if the list is no longer `width` wide, and
    /// returns what the last frame held still.
    fn forget_if_resized(&mut self, width: f32) -> Option<Hold> {
        let (resized, hold) = self.plan.with_state(|state| {
            let resized = state.width.is_some_and(|was| (was - width).abs() > SAME);
            state.width = Some(width);
            if resized {
                state.heights.clear();
            }
            (resized, state.hold)
        });
        if resized
            && let (RowHeights::Measured { estimate }, Tops::Measured { offsets }) =
                (self.plan.rows, &mut self.plan.tops)
        {
            let rows = offsets.len() - 1;
            running_offsets(std::iter::repeat_n(estimate, rows), offsets);
        }
        hold
    }

    /// The row whose key is `key`, looked for at `hint` first.
    fn find(&self, key: IdSalt, hint: usize) -> Option<usize> {
        let key_at = |at: usize| match self.plan.keys.get(at) {
            Some(&key) => key,
            None => self.rows.key(at),
        };
        if hint < self.rows.len() && key_at(hint) == key {
            return Some(hint);
        }
        (0..self.rows.len()).find(|&at| key_at(at) == key)
    }

    /// The rows built in the last frame that are kept built: one with the
    /// focus inside, an open overlay declared inside, or a node inside that an
    /// overlay is anchored to.
    fn pinned(&self, b: &Builder<'_, 'f, A>, range: &Range<usize>) -> Vec<usize> {
        let ctx = b.ctx();
        let built = self
            .plan
            .with_state(|state| std::mem::take(&mut state.built));
        if built.is_empty() {
            return Vec::new();
        }
        let stack = crate::stack::Stack::load(b.kept());
        let anchors: Vec<Id> = b.anchored_to().collect();
        let mut pinned: Vec<usize> = built
            .iter()
            .filter(|row| {
                crate::focus::focus_within(ctx, row.id)
                    || row
                        .overlays
                        .iter()
                        .any(|&overlay| stack.index_of(overlay).is_some())
                    || row.inside.iter().any(|id| anchors.contains(id))
            })
            .filter_map(|row| self.find(row.key, row.at))
            .filter(|at| !range.contains(at))
            .collect();
        pinned.sort_unstable();
        pinned.dedup();
        pinned
    }

    /// Builds the rows `view` shows, the overscan and the pinned rows, into
    /// `b`, and returns them for [`Laid::settle`].
    pub(crate) fn build(mut self, b: &mut Builder<'_, 'f, A>, view: View) -> Laid {
        let range = self.plan.tops.visible(view.span(self.plan.overscan));
        let first_visible = self.plan.tops.visible(view.span(0.0)).start;
        let pinned = self.pinned(b, &range);
        let classes = match self.plan.rows {
            RowHeights::Fixed(height) => tw().h_px(height).shrink_0(),
            RowHeights::Measured { .. } => tw().shrink_0(),
        };
        let mut built = Vec::new();
        let mut row = |b: &mut Builder<'_, 'f, A>,
                       rows: &mut Box<dyn Rows<'f, A> + 'f>,
                       at: usize,
                       flow: Flow| {
            let before = b.roots_declared();
            let (id, node) = rows.build(b, at, classes.clone());
            built.push(Built {
                at,
                key: rows.key(at),
                id,
                node,
                flow,
                overlays: b.overlays_from(before).collect(),
            });
        };
        let column = |top: f32| {
            tw().absolute()
                .top_px(top)
                .inset_x_px(0.0)
                .flex()
                .flex_col()
        };
        let tops = &self.plan.tops;
        let rows = &mut self.rows;
        for &at in pinned.iter().filter(|&&at| at < range.start) {
            b.div(column(tops.top(at)), |b| row(b, rows, at, Flow::Alone));
        }
        if !range.is_empty() {
            b.div(column(tops.top(range.start)), |b| {
                for at in range.clone() {
                    row(b, rows, at, Flow::Column);
                }
            });
        }
        for &at in pinned.iter().filter(|&&at| at >= range.end) {
            b.div(column(tops.top(at)), |b| row(b, rows, at, Flow::Alone));
        }
        let column_top = range.clone().next().map(|at| tops.top(at));
        Laid {
            plan: self.plan,
            scroll: self.scroll,
            built,
            first_visible,
            column_top,
            end: view.at_end().then_some(view.offset),
        }
    }
}

/// Whether a row was built in the column of the visible rows, or on its own
/// because it is pinned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    Column,
    Alone,
}

/// A row as built this frame.
struct Built {
    at: usize,
    key: IdSalt,
    id: Id,
    node: NodeId,
    flow: Flow,
    overlays: Vec<Id>,
}

/// A list's rows as built, waiting for their layout.
pub(crate) struct Laid {
    plan: Plan,
    scroll: NodeId,
    built: Vec<Built>,
    first_visible: usize,
    /// Where the column of visible rows was placed, if any were built.
    column_top: Option<f32>,
    /// The offset, if the content was scrolled to its end.
    end: Option<f32>,
}

impl Laid {
    /// Takes the heights the built rows were laid out at: keeps them, moves
    /// the scroll offset by as much as the first visible row moved from where
    /// its running offset put it, and asks for another frame if a height
    /// changed.
    pub(crate) fn settle(self, ctx: &Context, tree: &mut TaffyTree<Node>) {
        let Self {
            plan,
            scroll,
            built,
            first_visible,
            column_top,
            end,
        } = self;
        let mut moved = 0.0;
        let mut changed = false;
        let mut first = None;
        let mut top = column_top.unwrap_or(0.0);
        let mut rows_kept = Vec::with_capacity(built.len());
        let mut heights = Vec::with_capacity(built.len());
        for row in &built {
            let height = tree.layout(row.node).expect("node exists").size.height;
            let used = plan.tops.height(row.at);
            changed |= (height - used).abs() > SAME;
            if row.flow == Flow::Column {
                if row.at < first_visible {
                    moved += height - used;
                }
                if row.at == first_visible {
                    first = Some(Hold::Row {
                        key: row.key,
                        at: row.at,
                        top,
                    });
                }
                top += height;
            }
            heights.push((row.key, height));
            rows_kept.push(BuiltRow {
                key: row.key,
                at: row.at,
                id: row.id,
                inside: identified_in(tree, row.node),
                overlays: row.overlays.clone(),
            });
        }
        let nodes: Vec<NodeId> = built.iter().map(|row| row.node).collect();
        let holds_stop = scroll::holds_tab_stop(tree, &nodes);
        if let Some(container) = tree
            .get_node_context_mut(scroll)
            .and_then(|node| node.scroll.as_mut())
            && container.tab == TabRule::WhenScrollable
            && holds_stop
        {
            container.tab = TabRule::Never;
        }
        if moved.abs() > SAME {
            scroll::scroll_by(ctx, plan.list, vec2(0.0, moved));
        }
        if changed {
            ctx.request_repaint();
        }
        let Plan {
            kept,
            list,
            rows,
            tops,
            keys,
            ..
        } = plan;
        kept.with(|kept| {
            let state = kept.lists.0.entry(list).or_default();
            if let RowHeights::Measured { .. } = rows {
                state.heights.extend(heights);
            }
            if let Tops::Measured { offsets } = tops {
                state.offsets = offsets;
            }
            state.keys = keys;
            state.built = rows_kept;
            state.hold = end.map(|offset| Hold::End { offset }).or(first);
        });
    }
}

/// The identified nodes in the subtree of `node`.
fn identified_in(tree: &TaffyTree<Node>, node: NodeId) -> Vec<Id> {
    let mut found = Vec::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        if let Some(id) = tree.get_node_context(node).and_then(|node| node.id) {
            found.push(id);
        }
        stack.extend(tree.children(node).unwrap_or_default());
    }
    found
}

/// A stretch of a list along its axis, in the list's coordinates: from
/// `start` to `end`, in points from the top of the first row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Span {
    pub(crate) start: f32,
    pub(crate) end: f32,
}

/// Writes the running offsets of rows `heights` into `into`: the top of each
/// row, then the total, so a list of `n` rows has `n + 1` offsets.
pub(crate) fn running_offsets(heights: impl IntoIterator<Item = f32>, into: &mut Vec<f32>) {
    into.clear();
    into.push(0.0);
    let mut total = 0.0;
    into.extend(heights.into_iter().map(|height| {
        total += height;
        total
    }));
}

/// The rows that reach into `span`, given the list's running offsets.
pub(crate) fn visible(offsets: &[f32], span: Span) -> Range<usize> {
    let rows = offsets.len().saturating_sub(1);
    let first = offsets[1..].partition_point(|&bottom| bottom <= span.start);
    let end = offsets[..rows].partition_point(|&top| top < span.end);
    first..end.max(first)
}

/// The rows of a list of `rows` rows, each `height` tall, that reach into
/// `span`. `height` is positive ([`ListChain::fixed_px`] makes it so).
pub(crate) fn visible_fixed(rows: usize, height: f32, span: Span) -> Range<usize> {
    let row = |at: f32| (at / height).clamp(0.0, rows as f32);
    let first = row(span.start).floor() as usize;
    let end = row(span.end).ceil() as usize;
    first..end.max(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offsets(heights: &[f32]) -> Vec<f32> {
        let mut into = vec![99.0];
        running_offsets(heights.iter().copied(), &mut into);
        into
    }

    fn span(start: f32, end: f32) -> Span {
        Span { start, end }
    }

    #[test]
    fn running_offsets_are_each_rows_top_then_the_total() {
        assert_eq!(offsets(&[10.0, 20.0, 5.0]), vec![0.0, 10.0, 30.0, 35.0]);
    }

    #[test]
    fn an_empty_list_has_one_offset_and_no_visible_rows() {
        let empty = offsets(&[]);
        assert_eq!(empty, vec![0.0]);
        assert_eq!(visible(&empty, span(0.0, 100.0)), 0..0);
        assert_eq!(visible_fixed(0, 20.0, span(0.0, 100.0)), 0..0);
    }

    #[test]
    fn the_visible_rows_are_those_that_reach_into_the_span() {
        let at = offsets(&[10.0, 20.0, 5.0, 40.0]);
        assert_eq!(
            visible(&at, span(0.0, 10.0)),
            0..1,
            "a row ending at the span's end"
        );
        assert_eq!(
            visible(&at, span(10.0, 31.0)),
            1..3,
            "starts on a row's top"
        );
        assert_eq!(visible(&at, span(12.0, 34.0)), 1..3);
        assert_eq!(
            visible(&at, span(-50.0, 500.0)),
            0..4,
            "overscan past both ends"
        );
    }

    #[test]
    fn a_span_past_the_end_sees_no_rows() {
        let at = offsets(&[10.0, 20.0]);
        assert_eq!(visible(&at, span(30.0, 80.0)), 2..2);
        assert_eq!(visible(&at, span(400.0, 480.0)), 2..2);
        assert_eq!(visible_fixed(2, 15.0, span(30.0, 80.0)), 2..2);
    }

    #[test]
    fn a_span_before_the_start_sees_no_rows() {
        let at = offsets(&[10.0, 20.0]);
        assert_eq!(visible(&at, span(-40.0, 0.0)), 0..0);
        assert_eq!(visible_fixed(2, 15.0, span(-40.0, 0.0)), 0..0);
    }

    #[test]
    fn zero_height_rows_inside_the_span_are_visible() {
        let at = offsets(&[10.0, 0.0, 10.0]);
        assert_eq!(visible(&at, span(5.0, 15.0)), 0..3);
    }

    #[test]
    fn fixed_rows_see_what_the_running_offsets_would() {
        let height = 17.0;
        let at = offsets(&[height; 1000]);
        for (start, end) in [
            (0.0, 50.0),
            (33.0, 34.0),
            (500.0, 900.0),
            (16_990.0, 20_000.0),
        ] {
            assert_eq!(
                visible_fixed(1000, height, span(start, end)),
                visible(&at, span(start, end)),
                "{start}..{end}"
            );
        }
    }
}
