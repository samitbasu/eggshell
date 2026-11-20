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

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash as _, Hasher as _};
use std::ops::Range;

use egui::{Align, Context, Id, IdSalt, Rect, Role, Vec2, pos2, vec2};
use nohash_hasher::IntMap;
use taffy::NodeId;

use crate::builder::{Builder, NodeHandle, Scope, Slot};
use crate::collapse::CollapseAxis;
use crate::dnd::EdgeScroll;
use crate::draw::NodeUi as _;
use crate::finite::clamp;
use crate::kept::Handle;
use crate::key::{Key, child_id};
use crate::layout_tree::LayoutTree;
use crate::nav::{Keys, ListMove};
use crate::owned::{Enablement, Mark};
use crate::scroll::{self, Axis, EdgeFade, Focusable, ScrollChain, ScrollStyle, TabRule};
use crate::shell::Shell;
use crate::tw::{Tw, tw};

/// How a list sizes its rows, and how far past its viewport it builds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VList {
    pub(crate) rows: RowHeights,
    pub(crate) overscan: f32,
    pub(crate) start: Start,
    /// Whether the list navigates as a listbox, and from where.
    pub(crate) listbox: Option<Keys>,
}

/// Where a list opens the first time it is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Start {
    #[default]
    Top,
    /// At its end, which it then holds as its rows are measured.
    End,
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

/// How near its end a list's offset counts as at it: egui rounds the
/// viewport it clamps the offset to to pixels, and the layout does not.
const AT_END: f32 = 1.0;

/// TODO(prose): the keyboard pattern of a virtual list.
///
/// ```
/// use eggshell_ui::vlist::ListNav;
/// use eggshell_ui::{Builder, tw};
///
/// fn view<'f>(b: &mut Builder<'_, 'f, ()>, rows: &'f [u32]) {
///     b.virtual_list("list", tw(), rows)
///         .nav(ListNav::Listbox)
///         .items(|s, row| {
///             s.text(tw(), row.to_string());
///         });
/// }
/// ```
///
/// TODO(prose): a virtual list stops at its ends, so it takes no [`Wrap`](crate::overlay::Wrap):
///
/// ```compile_fail
/// use eggshell_ui::overlay::{Nav, Wrap};
/// use eggshell_ui::{Builder, tw};
///
/// fn view<'f>(b: &mut Builder<'_, 'f, ()>, rows: &'f [u32]) {
///     b.virtual_list("list", tw(), rows)
///         .nav(Nav::Listbox(Wrap::Around))
///         .items(|s, row| {
///             s.text(tw(), row.to_string());
///         });
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ListNav {
    /// TODO(prose): the rows handle their own keys.
    #[default]
    None,
    /// TODO(prose): one Tab stop, with a virtual highlight that stops at the ends.
    Listbox,
}

/// A virtual list, started with [`Builder::virtual_list`] and finished with
/// [`items`](Self::items).
///
/// Rows of differing heights are measured as they are built; until then a
/// row counts as [`estimate_px`](Self::estimate_px) tall, one line of the
/// list's text unless set. Rows that are all one height are faster as
/// [`fixed_px`](Self::fixed_px). The last of the two called wins.
///
/// The list is a scroll container: [`focusable`](Self::focusable),
/// [`role`](Self::role), [`named`](Self::named) and the other options are
/// [`ScrollChain`]'s, for the list's container. It scrolls vertically.
#[must_use = "a virtual list is not added until you call `.items(..)`"]
pub struct ListChain<'a, 'f, A, R: Clone> {
    scroll: ScrollChain<'a, 'f, A>,
    rows: Cow<'f, [R]>,
    heights: Option<RowHeights>,
    overscan: f32,
    start: Start,
    nav: ListNav,
    row_classes: Tw,
    on_activate: Option<Activate<A, R>>,
    row_name: Option<std::rc::Rc<dyn Fn(R) -> String>>,
    keys: Keys,
    first: FirstRow,
    enablement: Option<RowEnablement<R>>,
    mark: Option<RowMark<R>>,
}

impl<'a, 'f, A, R: Clone> ListChain<'a, 'f, A, R> {
    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        crate::owned::options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        crate::owned::options::when_some(self, value, then)
    }
}

type Activate<A, R> = std::rc::Rc<dyn Fn(&mut A, R)>;
type RowEnablement<R> = std::rc::Rc<dyn Fn(R) -> Enablement>;
type RowMark<R> = std::rc::Rc<dyn Fn(R) -> Option<Mark>>;

/// Whether a listbox highlights its first row as its rows change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FirstRow {
    Left,
    Highlighted,
}

impl<'a, 'f, A: 'static, R: Key + Copy> ListChain<'a, 'f, A, R> {
    pub(crate) fn new(b: Builder<'a, 'f, A>, slot: Slot, classes: Tw, rows: Cow<'f, [R]>) -> Self {
        Self {
            scroll: ScrollChain::with_slot(b, slot, classes),
            rows,
            heights: None,
            overscan: OVERSCAN,
            start: Start::Top,
            nav: ListNav::None,
            row_classes: tw(),
            on_activate: None,
            row_name: None,
            keys: Keys::Own,
            first: FirstRow::Left,
            enablement: None,
            mark: None,
        }
    }

    /// TODO(prose): a listbox whose keys come from the node `field` (from [`Builder::id_of`](crate::Builder::id_of)) while the focus is in it: a search field over its results. The arrows, Page Up and Down move the highlight and Enter activates it, the field keeps Home and End, and the field announces the list it controls and the highlighted row. The list is no Tab stop of its own.
    ///
    /// ```
    /// use eggshell_ui::vlist::ListNav;
    /// use eggshell_ui::{Builder, tw};
    ///
    /// struct App { query: String, picked: Option<u32> }
    ///
    /// fn view<'f>(b: &mut Builder<'_, 'f, App>, app: &'f App, rows: &'f [u32]) {
    ///     b.text_field("query", &app.query)
    ///         .on_change(|app, query| app.query = query);
    ///     let field = b.id_of("query");
    ///     b.virtual_list("results", tw().h_px(300.0), rows)
    ///         .keys_from(field)
    ///         .highlight_first()
    ///         .on_activate(|app: &mut App, row| app.picked = Some(row))
    ///         .items(|s, row| s.text(tw(), row.to_string()));
    /// }
    /// ```
    pub fn keys_from(mut self, field: Id) -> Self {
        self.nav = ListNav::Listbox;
        self.keys = Keys::From(field);
        self
    }

    /// TODO(prose): a listbox highlights its first enabled row when it is first laid out and each time its rows change, as a palette's first match is: Enter picks it at once.
    pub fn highlight_first(mut self) -> Self {
        self.first = FirstRow::Highlighted;
        self
    }

    /// TODO(prose): whether each row of a listbox is enabled: a disabled row is announced disabled, and the keys, the pointer and activation pass it by.
    pub fn row_enablement(mut self, enablement: impl Fn(R) -> Enablement + 'static) -> Self {
        self.enablement = Some(std::rc::Rc::new(enablement));
        self
    }

    /// TODO(prose): the check of each row of a listbox that has one, announced as checked or not: a chosen option, a setting that is on.
    pub fn row_mark(mut self, mark: impl Fn(R) -> Option<Mark> + 'static) -> Self {
        self.mark = Some(std::rc::Rc::new(mark));
        self
    }

    /// TODO(prose): [`ListNav::Listbox`] makes the list one Tab stop with a virtual highlight moved by the arrow, Home, End and Page keys; [`ListNav::None`] is the default.
    pub fn nav(mut self, nav: ListNav) -> Self {
        self.nav = nav;
        self
    }

    /// TODO(prose): runs `then` with the row Enter, Space or a click activates; only for a listbox.
    pub fn on_activate(mut self, then: impl Fn(&mut A, R) + 'static) -> Self {
        self.on_activate = Some(std::rc::Rc::new(then));
        self
    }

    /// TODO(prose): the accessible name `name` gives each row of a listbox.
    pub fn row_name(mut self, name: impl Fn(R) -> String + 'static) -> Self {
        self.row_name = Some(std::rc::Rc::new(name));
        self
    }

    /// TODO(prose): classes of each row's wrapper, such as a `hover` look the highlighted row takes.
    pub fn row_classes(mut self, classes: Tw) -> Self {
        self.row_classes = self.row_classes.merge(classes);
        self
    }

    /// Opens the list at its end the first time it is laid out, as a chat
    /// opens on its latest message. A list at its end stays there as its
    /// rows are measured and as rows are added, until it is scrolled away.
    pub fn start_at_end(mut self) -> Self {
        self.start = Start::End;
        self
    }

    fn container(
        self,
        option: impl FnOnce(ScrollChain<'a, 'f, A>) -> ScrollChain<'a, 'f, A>,
    ) -> Self {
        Self {
            scroll: option(self.scroll),
            ..self
        }
    }

    /// See [`ScrollChain::focusable`].
    pub fn focusable(self, focusable: Focusable) -> Self {
        self.container(|scroll| scroll.focusable(focusable))
    }

    /// See [`ScrollChain::role`].
    pub fn role(self, role: Role) -> Self {
        self.container(|scroll| scroll.role(role))
    }

    /// See [`ScrollChain::named`].
    pub fn named(self, name: impl Into<String>) -> Self {
        self.container(|scroll| scroll.named(name))
    }

    /// See [`ScrollChain::scrollbar_style`].
    pub fn scrollbar_style(self, bars: ScrollStyle) -> Self {
        self.container(|scroll| scroll.scrollbar_style(bars))
    }

    /// See [`ScrollChain::fade_edges`].
    pub fn fade_edges(self, fade: EdgeFade) -> Self {
        self.container(|scroll| scroll.fade_edges(fade))
    }

    /// See [`ScrollChain::edge_scroll`].
    pub fn edge_scroll(self, edge: EdgeScroll) -> Self {
        self.container(|scroll| scroll.edge_scroll(edge))
    }

    /// See [`ScrollChain::no_edge_scroll`].
    pub fn no_edge_scroll(self) -> Self {
        self.container(ScrollChain::no_edge_scroll)
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
            scroll,
            rows,
            heights,
            overscan,
            start,
            nav,
            row_classes,
            on_activate,
            row_name,
            keys,
            first,
            enablement,
            mark,
        } = self;
        let listbox = (nav == ListNav::Listbox).then_some(keys);
        let scroll = match listbox {
            Some(Keys::Own) => scroll.focusable(Focusable::Always).role(Role::ListBox),
            Some(Keys::From(_)) => scroll.role(Role::ListBox),
            None => scroll,
        };
        let list = scroll.id();
        let kept = scroll.builder().kept().clone();
        let pass = scroll.builder().ctx().cumulative_pass_nr();
        let (handle, (plan, leaf)) = scroll.children_with(|s| {
            let heights =
                heights.unwrap_or_else(|| RowHeights::measured(s.text_style().spec.line_height));
            let plan = Plan::new(
                &kept,
                pass,
                list,
                VList {
                    rows: heights,
                    overscan,
                    start,
                    listbox,
                },
                &rows,
            );
            let leaf = s.rows_leaf(plan.total());
            (plan, leaf)
        });
        let scroll = handle.node();
        let mut b = handle.into_builder();
        let behavior = listbox.map(|keys| Listbox {
            list,
            kept: kept.clone(),
            on_activate,
            row_name,
            keys,
            enablement: enablement.clone(),
            mark,
        });
        if first == FirstRow::Highlighted
            && let Some(keys) = listbox
        {
            highlight_first(&kept, list, &rows, keys, enablement.as_deref());
        }
        b.list_region(
            leaf,
            list,
            plan.run(scroll, Items::new(rows, item, row_classes, behavior)),
        );
        b.into_handle(scroll, list)
    }
}

impl Shell {
    /// Scrolls the virtual list `list` (the id of its key, from
    /// [`Builder::id_of`](crate::Builder::id_of)) to show the row `row`. With
    /// an `align`, the row is aligned that way in the viewport; without, the
    /// list moves the least that shows it.
    ///
    /// The next layout of the list builds the row and the rows around it
    /// where its running offsets put it, and corrects once the row is
    /// measured. A request for a row the list does not have is dropped.
    pub fn scroll_to_row(&self, list: Id, row: impl Key, align: Option<Align>) {
        self.kept.with(|kept| {
            kept.scrolls.rows.insert(
                list,
                ScrollTo {
                    row: row_key(&row),
                    align,
                },
            );
        });
        self.ctx.request_repaint();
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollTo {
    /// The row's key in the list's height cache.
    row: IdSalt,
    align: Option<Align>,
}

/// The key of `row` in its list's height cache: unique in the list, whose
/// rows are all of one type.
fn row_key(row: &impl Key) -> IdSalt {
    IdSalt::new(row)
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
    /// The key of the highlighted row of a listbox.
    active: Option<IdSalt>,
    /// Where a listbox takes its keys from.
    keyed_by: Option<Keys>,
    /// The widget last focused in the field a listbox takes its keys from,
    /// which a click on a row gives the focus back to.
    typing: Option<Id>,
    /// The rows a listbox that highlights its first row last had.
    rows_digest: Option<u64>,
    /// The part of the last frame's correction its content was too short to
    /// take: the content grows once the rows it measured count.
    owed: f32,
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
    /// Whether a size inside it is still moving.
    moving: bool,
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
    /// Forgets the lists not laid out in the pass `pass`, and returns them.
    pub(crate) fn sweep(&mut self, pass: u64) -> Vec<Id> {
        let gone: Vec<Id> = self
            .0
            .iter()
            .filter(|(_, list)| list.pass != pass)
            .map(|(&id, _)| id)
            .collect();
        for id in &gone {
            self.0.remove(id);
        }
        gone
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
            Self::Measured { offsets } => offsets.len().saturating_sub(1),
        }
    }

    fn top(&self, at: usize) -> f32 {
        match self {
            Self::Fixed { height, .. } => at as f32 * height,
            Self::Measured { offsets } => {
                offsets.get(at).or(offsets.last()).copied().unwrap_or(0.0)
            }
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
    fn key(&self, at: usize) -> Option<IdSalt>;
    /// The id row `at` has when built under `base`.
    fn id(&self, base: Id, at: usize) -> Option<Id>;
    /// Whether row `at` is there and enabled.
    fn enabled(&self, at: usize) -> bool;
    /// Builds row `at` under its own key with `classes`, returning its id
    /// and node.
    fn build(&mut self, b: &mut Builder<'_, 'f, A>, at: usize, classes: Tw)
    -> Option<(Id, NodeId)>;
}

/// What a listbox's row wrappers do.
pub(crate) struct Listbox<A, R> {
    list: Id,
    kept: Handle,
    on_activate: Option<Activate<A, R>>,
    row_name: Option<std::rc::Rc<dyn Fn(R) -> String>>,
    keys: Keys,
    enablement: Option<RowEnablement<R>>,
    mark: Option<RowMark<R>>,
}

impl<A, R: Copy> Listbox<A, R> {
    fn enablement(&self, row: R) -> Enablement {
        self.enablement
            .as_ref()
            .map_or(Enablement::Enabled, |enablement| enablement(row))
    }
}

pub(crate) struct Items<'f, A, R: Clone, F> {
    rows: Cow<'f, [R]>,
    item: F,
    classes: Tw,
    listbox: Option<Listbox<A, R>>,
}

impl<'f, A, R: Clone, F> Items<'f, A, R, F> {
    pub(crate) fn new(
        rows: Cow<'f, [R]>,
        item: F,
        classes: Tw,
        listbox: Option<Listbox<A, R>>,
    ) -> Self {
        Self {
            rows,
            item,
            classes,
            listbox,
        }
    }
}

impl<'f, A: 'static, R: Key + Copy, F: FnMut(&mut Scope<'_, 'f, A>, R)> Rows<'f, A>
    for Items<'f, A, R, F>
{
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> Option<IdSalt> {
        self.rows.get(at).map(row_key)
    }

    fn id(&self, base: Id, at: usize) -> Option<Id> {
        self.rows.get(at).map(|row| child_id(base, row))
    }

    fn enabled(&self, at: usize) -> bool {
        match (&self.listbox, self.rows.get(at)) {
            (Some(listbox), Some(&row)) => listbox.enablement(row) == Enablement::Enabled,
            (None, Some(_)) => true,
            (_, None) => false,
        }
    }

    fn build(
        &mut self,
        b: &mut Builder<'_, 'f, A>,
        at: usize,
        classes: Tw,
    ) -> Option<(Id, NodeId)> {
        let row = *self.rows.get(at)?;
        let item = &mut self.item;
        let classes = classes.merge(self.classes.clone());
        let Some(listbox) = &self.listbox else {
            let handle = b.div_id(row, classes, |s| item(s, row));
            return Some((handle.id(), handle.node()));
        };
        let (list, kept, keys) = (listbox.list, listbox.kept.clone(), listbox.keys);
        let on_activate = listbox.on_activate.clone();
        let enablement = listbox.enablement(row);
        let mark = listbox.mark.as_ref().and_then(|mark| mark(row));
        let handle = b
            .div_id(row, classes.sense_click().group().focusable_none(), |s| {
                item(s, row)
            })
            .role(Role::ListBoxOption);
        let handle = match &listbox.row_name {
            Some(name) => handle.named(name(row)),
            None => handle,
        };
        let handle = handle.draw(move |app, ui| {
            let response = ui.node_response();
            let id = response.id;
            let moved = ui.input(|input| input.pointer.delta()) != Vec2::ZERO;
            if let Some(mark) = mark {
                let toggled = match mark {
                    Mark::On => egui::accesskit::Toggled::True,
                    Mark::Off => egui::accesskit::Toggled::False,
                };
                ui.ctx()
                    .accesskit_node_builder(id, |node| node.set_toggled(toggled));
            }
            if enablement == Enablement::Disabled {
                ui.ctx()
                    .accesskit_node_builder(id, |node| node.set_disabled());
                return;
            }
            if response.hovered() && moved && !crate::nav::is_highlighted(&kept, id) {
                select(&kept, list, row_key(&row), id, keys);
                ui.ctx().request_repaint();
            }
            if response.clicked() {
                select(&kept, list, row_key(&row), id, keys);
                let focus = match keys {
                    Keys::Own => Some(list),
                    Keys::From(_) => kept.with(|kept| kept.lists.0.get(&list)?.typing),
                };
                if let Some(focus) = focus {
                    ui.ctx().memory_mut(|memory| memory.request_focus(focus));
                }
            }
            if (response.clicked() || crate::nav::take_activation(ui, id))
                && let Some(on_activate) = &on_activate
            {
                on_activate(app, row);
            }
        });
        Some((handle.id(), handle.node()))
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
    /// Whether the list navigates as a listbox, and from where.
    listbox: Option<Keys>,
}

/// Highlights the row `key`, built as `id`, of the listbox `list`.
fn select(kept: &Handle, list: Id, key: IdSalt, id: Id, keys: Keys) {
    crate::nav::highlight_row(kept, list, id, keys);
    kept.with(|kept| {
        if let Some(state) = kept.lists.0.get_mut(&list) {
            state.active = Some(key);
        }
    });
}

/// Highlights the first enabled row of the listbox `list` when its rows are
/// not the ones it last had, and scrolls to it; with no row to highlight,
/// forgets the highlight.
fn highlight_first<R: Key + Copy>(
    kept: &Handle,
    list: Id,
    rows: &[R],
    keys: Keys,
    enablement: Option<&dyn Fn(R) -> Enablement>,
) {
    let digest = {
        let mut hasher = std::hash::DefaultHasher::new();
        rows.hash(&mut hasher);
        hasher.finish()
    };
    let changed = kept.with(|kept| {
        let state = kept.lists.0.get_mut(&list)?;
        let changed = state.rows_digest != Some(digest);
        state.rows_digest = Some(digest);
        Some(changed)
    });
    if changed != Some(true) {
        return;
    }
    let enabled = |row: &&R| enablement.is_none_or(|of| of(**row) == Enablement::Enabled);
    match rows.iter().find(enabled) {
        Some(first) => {
            let key = row_key(first);
            select(kept, list, key, child_id(list, first), keys);
            kept.with(|kept| {
                kept.scrolls.rows.insert(
                    list,
                    ScrollTo {
                        row: key,
                        align: None,
                    },
                );
            });
        }
        None => kept.with(|kept| {
            crate::nav::forget_lists(kept, &[list]);
            if let Some(state) = kept.lists.0.get_mut(&list) {
                state.active = None;
            }
        }),
    }
}

/// That each listbox laid out in the pass `pass` that takes its keys from a
/// field is controlled by it: by the widget in it that has the focus.
pub(crate) fn relations(kept: &Handle, shell: &Shell, pass: u64) -> Vec<crate::relation::Relation> {
    let fields: Vec<(Id, Id)> = kept.with(|kept| {
        kept.lists
            .0
            .iter()
            .filter(|(_, state)| state.pass == pass)
            .filter_map(|(&list, state)| match state.keyed_by {
                Some(Keys::From(field)) => Some((list, field)),
                _ => None,
            })
            .collect()
    });
    fields
        .into_iter()
        .map(|(list, field)| crate::relation::Relation::Controls {
            node: crate::nav::speaker(shell, field),
            target: list,
        })
        .collect()
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
            start,
            listbox,
        } = settings;
        let (tops, keys) = kept.with(|kept| {
            let state = kept.lists.0.entry(list).or_insert_with(|| ListState {
                hold: match start {
                    Start::Top => None,
                    Start::End => Some(Hold::End { offset: 0.0 }),
                },
                ..ListState::default()
            });
            state.pass = pass;
            state.keyed_by = listbox;
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
            listbox,
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
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct View {
    /// The rows' top in the content.
    top: f32,
    /// How far the content is scrolled.
    offset: f32,
    viewport: Vec2,
    /// The content's height.
    content: f32,
    /// The row a [`Shell::scroll_to_row`] asked for this frame, which the list keeps in
    /// place while it measures the rows around it.
    target: Option<Anchor>,
}

/// The row a list keeps where it is on the screen while it measures the
/// rows it built.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    at: usize,
    /// How much of the row's own change of height to keep off the screen
    /// position of its top: none when its top was placed, all when its
    /// bottom was.
    own: f32,
}

impl View {
    /// The farthest the content can be scrolled.
    fn limit(&self) -> f32 {
        (self.content - self.viewport.y).max(0.0)
    }

    /// Whether the content is scrolled to its end, and can be scrolled.
    fn at_end(&self) -> bool {
        let limit = self.limit();
        limit > 0.0 && self.offset >= limit - AT_END
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

impl<'f, S: 'static> ListRun<'f, S> {
    /// The same list, declared from a mount's builder into the frame around
    /// it: its rows are built under the mount again.
    pub(crate) fn lift<R: 'static>(self, mount: crate::mount::Mount<'f, R, S>) -> ListRun<'f, R> {
        ListRun {
            plan: self.plan,
            scroll: self.scroll,
            rows: Box::new(Mounted {
                rows: self.rows,
                mount,
            }),
        }
    }
}

/// A mount's rows, built from a builder of the data around it.
struct Mounted<'f, R, S> {
    rows: Box<dyn Rows<'f, S> + 'f>,
    mount: crate::mount::Mount<'f, R, S>,
}

impl<'f, R: 'static, S: 'static> Rows<'f, R> for Mounted<'f, R, S> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> Option<IdSalt> {
        self.rows.key(at)
    }

    fn id(&self, base: Id, at: usize) -> Option<Id> {
        self.rows.id(base, at)
    }

    fn enabled(&self, at: usize) -> bool {
        self.rows.enabled(at)
    }

    fn build(
        &mut self,
        b: &mut Builder<'_, 'f, R>,
        at: usize,
        classes: Tw,
    ) -> Option<(Id, NodeId)> {
        let rows = &mut self.rows;
        b.within(&self.mount, |b| rows.build(b, at, classes))
    }
}

impl<'f, A> ListRun<'f, A> {
    /// Where the viewport of the laid-out container is over the rows, which
    /// `leaf` holds, `width` wide. Forgets the heights if the width changed,
    /// keeps the first visible row in place, and takes up a
    /// [`Shell::scroll_to_row`] request.
    pub(crate) fn view(
        &mut self,
        ctx: &Context,
        tree: &LayoutTree,
        leaf: NodeId,
        width: f32,
    ) -> View {
        let list = self.plan.list;
        let own = tree.layout(self.scroll);
        let Some(node) = tree.scroll(self.scroll) else {
            return View::default();
        };
        let viewport = vec2(
            own.size.width - own.border.left - own.border.right - node.bars.gutter.x,
            own.size.height - own.border.top - own.border.bottom - node.bars.gutter.y,
        )
        .max(Vec2::ZERO);
        let mut view = View {
            top: tree.layout(leaf).location.y,
            offset: scroll::stored_offset(ctx, list, Axis::Y, node.size, viewport).y,
            viewport,
            content: node.size.y,
            target: None,
        };
        let shell = Shell::new(ctx, &self.plan.kept);
        self.navigate(ctx, &view);
        let requested = shell.requested_by(list).y;
        let moved = match self.forget_if_resized(width) {
            Some(Hold::End { offset }) if view.offset >= offset - AT_END => {
                let gap = view.limit() - view.offset;
                if gap.abs() > AT_END { gap } else { 0.0 }
            }
            Some(Hold::Row { key, at, top }) if view.offset > 0.0 => self
                .find(key, at)
                .map_or(0.0, |at| self.plan.tops.top(at) - top),
            _ => 0.0,
        } + self
            .plan
            .with_state(|state| std::mem::take(&mut state.owed));
        if moved.abs() > SAME {
            view.offset += moved;
            shell.scroll_by(list, vec2(0.0, moved));
        }
        view.offset = clamp(view.offset + requested, 0.0, view.limit());
        let request = self.plan.kept.with(|kept| kept.scrolls.rows.remove(&list));
        if let Some(request) = request
            && let Some(at) = self.find(request.row, 0)
        {
            let tops = &self.plan.tops;
            let row = (view.top + tops.top(at), view.top + tops.top(at + 1));
            let shown = (
                view.offset + node.padding.top,
                view.offset + view.viewport.y - node.padding.bottom,
            );
            let reveal = scroll::reveal(shown, row, request.align);
            view.offset = clamp(view.offset + reveal, 0.0, view.limit());
            let rect = Rect::from_min_max(pos2(0.0, row.0), pos2(view.viewport.x, row.1));
            shell.scroll_to_rect(list, rect, request.align);
            let own = match request.align {
                Some(align) => align.to_factor(),
                None if reveal > 0.0 => 1.0,
                None => 0.0,
            };
            view.target = Some(Anchor { at, own });
        }
        view
    }

    /// Moves a listbox's highlight with the frame's keys and pointer, and
    /// asks for the row to be scrolled into view.
    fn navigate(&self, ctx: &Context, view: &View) {
        let list = self.plan.list;
        let Some(keys) = self.plan.listbox else {
            return;
        };
        let kept = &self.plan.kept;
        let len = self.rows.len();
        let current = self
            .plan
            .with_state(|state| state.active)
            .and_then(|key| self.find(key, 0));
        let visible = self.plan.tops.visible(view.span(0.0));
        let focused = match keys {
            Keys::Own => crate::focus::holds_focus(ctx, list).then_some(list),
            Keys::From(field) => {
                let focused = ctx.memory(|memory| memory.focused());
                let typing = focused.filter(|_| Shell::new(ctx, kept).focus_within(field));
                if typing.is_some() {
                    self.plan.with_state(|state| state.typing = typing);
                }
                typing
            }
        };
        let to = focused.and_then(|focused| {
            crate::nav::hold_arrows(ctx, focused);
            ListMove::of_key(ctx, keys)?.target_enabled(
                current,
                len,
                visible.len().max(1),
                visible,
                |at| self.rows.enabled(at),
            )
        });
        if let Some(at) = to
            && let (Some(key), Some(id)) = (self.rows.key(at), self.rows.id(list, at))
        {
            select(kept, list, key, id, keys);
            kept.with(|kept| {
                kept.scrolls.rows.insert(
                    list,
                    ScrollTo {
                        row: key,
                        align: None,
                    },
                );
            });
            ctx.request_repaint();
        }
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
            let rows = offsets.len().saturating_sub(1);
            running_offsets(std::iter::repeat_n(estimate, rows), offsets);
        }
        hold
    }

    /// The row whose key is `key`, looked for at `hint` first.
    fn find(&self, key: IdSalt, hint: usize) -> Option<usize> {
        let key_at = |at: usize| match self.plan.keys.get(at) {
            Some(&key) => Some(key),
            None => self.rows.key(at),
        };
        if hint < self.rows.len() && key_at(hint) == Some(key) {
            return Some(hint);
        }
        (0..self.rows.len()).find(|&at| key_at(at) == Some(key))
    }

    /// The rows built in the last frame that are kept built: one with the
    /// focus inside, an open overlay declared inside, or a node inside that an
    /// overlay is anchored to.
    fn pinned(&self, b: &Builder<'_, 'f, A>, range: &Range<usize>) -> Vec<usize> {
        let built = self
            .plan
            .with_state(|state| std::mem::take(&mut state.built));
        if built.is_empty() {
            return Vec::new();
        }
        let stack = crate::stack::Stack::load(b.kept(), b.viewport());
        let anchors: Vec<Id> = b.anchored_to().collect();
        let mut pinned: Vec<usize> = built
            .iter()
            .filter(|row| {
                row.moving
                    || b.shell().focus_within(row.id)
                    || row
                        .overlays
                        .iter()
                        .any(|&overlay| stack.index_of(overlay).is_some())
                    || row.inside.iter().any(|id| anchors.contains(id))
            })
            .filter_map(|row| self.find(row.key, row.at))
            .filter(|at| !range.contains(at))
            .collect();
        if self.plan.listbox.is_some()
            && let Some(at) = self
                .plan
                .with_state(|state| state.active)
                .and_then(|key| self.find(key, 0))
            && !range.contains(&at)
        {
            pinned.push(at);
        }
        pinned.sort_unstable();
        pinned.dedup();
        pinned
    }

    /// Builds the rows `view` shows, the overscan and the pinned rows, into
    /// `b`, and returns them for [`Laid::settle`].
    pub(crate) fn build(mut self, b: &mut Builder<'_, 'f, A>, view: View) -> Laid {
        let range = self.plan.tops.visible(view.span(self.plan.overscan));
        let anchor = view.target.unwrap_or_else(|| Anchor {
            at: self.plan.tops.visible(view.span(0.0)).start,
            own: 0.0,
        });
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
            let (Some((id, node)), Some(key)) = (rows.build(b, at, classes.clone()), rows.key(at))
            else {
                return;
            };
            built.push(Built {
                at,
                key,
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
            anchor,
            column_top,
            end: view.at_end().then_some(view.offset),
            offset: view.offset,
            limit: view.limit(),
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
    anchor: Anchor,
    /// Where the column of visible rows was placed, if any were built.
    column_top: Option<f32>,
    /// The offset, if the content was scrolled to its end.
    end: Option<f32>,
    /// The offset the rows were chosen for, and the farthest this frame's
    /// content lets it go.
    offset: f32,
    limit: f32,
}

impl Laid {
    /// Takes the heights the built rows were laid out at: keeps them, moves
    /// the scroll offset by as much as the first visible row moved from where
    /// its running offset put it, and asks for another frame if a height
    /// changed.
    pub(crate) fn settle(self, ctx: &Context, tree: &mut LayoutTree) {
        let Self {
            plan,
            scroll,
            built,
            anchor,
            column_top,
            end,
            offset,
            limit,
        } = self;
        let mut moved = 0.0;
        let mut changed = false;
        let mut first = None;
        let mut top = column_top.unwrap_or(0.0);
        let mut rows_kept = Vec::with_capacity(built.len());
        let mut heights = Vec::with_capacity(built.len());
        for row in &built {
            let height = tree.layout(row.node).size.height;
            let used = plan.tops.height(row.at);
            changed |= (height - used).abs() > SAME;
            if row.flow == Flow::Column {
                if row.at < anchor.at {
                    moved += height - used;
                }
                if row.at == anchor.at {
                    moved += (height - used) * anchor.own;
                    first = Some(Hold::Row {
                        key: row.key,
                        at: row.at,
                        top,
                    });
                }
                top += height;
            }
            // A row whose size is moving is kept at where it is going, so
            // that it places the rows after it right once it is not built.
            let flight = crate::collapse::flight_in(tree, row.node, CollapseAxis::Height);
            heights.push((row.key, height + flight));
            rows_kept.push(BuiltRow {
                key: row.key,
                at: row.at,
                id: row.id,
                inside: identified_in(tree, row.node),
                overlays: row.overlays.clone(),
                moving: flight.abs() > SAME,
            });
        }
        let nodes: Vec<NodeId> = built.iter().map(|row| row.node).collect();
        let holds_stop = scroll::holds_tab_stop(tree, &nodes);
        if let Some(container) = tree.scroll_mut(scroll)
            && container.tab == TabRule::WhenScrollable
            && holds_stop
        {
            container.tab = TabRule::Never;
        }
        if moved.abs() > SAME {
            Shell::new(ctx, &plan.kept).scroll_by(plan.list, vec2(0.0, moved));
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
            let intended = offset + moved;
            state.owed = intended - clamp(intended, 0.0, limit);
        });
    }
}

/// The identified nodes in the subtree of `node`.
fn identified_in(tree: &LayoutTree, node: NodeId) -> Vec<Id> {
    let mut found = Vec::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        if let Some(id) = tree.node(node).and_then(|node| node.identity.id()) {
            found.push(id);
        }
        stack.extend(tree.children(node));
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
    let bottoms = offsets.get(1..).unwrap_or_default();
    let tops = offsets.get(..rows).unwrap_or_default();
    let first = bottoms.partition_point(|&bottom| bottom <= span.start);
    let end = tops.partition_point(|&top| top < span.end);
    first..end.max(first)
}

/// The rows of a list of `rows` rows, each `height` tall, that reach into
/// `span`. `height` is positive ([`ListChain::fixed_px`] makes it so).
pub(crate) fn visible_fixed(rows: usize, height: f32, span: Span) -> Range<usize> {
    let row = |at: f32| clamp(at / height, 0.0, rows as f32);
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
