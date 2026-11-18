//! Arrow-key navigation of an overlay's items, and of a group's on the
//! page ([`roving`](crate::roving)): the keyboard patterns of [`Nav`].
//!
//! A menu moves real focus between its items, in paint order. A grid moves
//! it too, to the nearest item in the arrow's direction as laid out. A listbox leaves focus on its
//! trigger and moves a virtual highlight, which the draw shows with the
//! item's `hover` look, and which Enter and Space activate: the item's
//! `on_click` runs in the next frame, as if it had been clicked.

use std::collections::HashMap;

use egui::{Align, Context, EventFilter, FocusDirection, Id, Key, Modifiers, Rect, Vec2};

use crate::kept::Handle;
use crate::overlay::{Nav, Wrap};
use crate::plane::Axis;
use crate::shell::Shell;

/// An open overlay with a keyboard pattern.
pub(crate) struct Menu {
    pub(crate) id: Id,
    pub(crate) nav: Nav,
    pub(crate) wrap: Wrap,
    /// The items that take input, in paint order, where they were laid out.
    pub(crate) items: Vec<(Id, Rect)>,
    /// The focusable widgets the overlay registered.
    pub(crate) own: Vec<Id>,
    /// The node the overlay is anchored to.
    pub(crate) anchor: Option<Id>,
}

/// Where a key moves among the items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Next,
    Previous,
    First,
    Last,
}

/// A direction on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// What a navigation key asks for: a step along the items in paint order,
/// or the nearest item toward a direction on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    Line(Step),
    Toward(Direction),
}

impl Move {
    /// The move of the frame's key for an overlay that navigates as `nav`,
    /// taking the key. An arrow is taken from egui too: egui holds the arrows
    /// of a widget only from the frame after it gained focus, and in that
    /// frame it would also move focus itself, after the overlay has.
    fn of_key(ctx: &Context, nav: Nav, home_end: HomeEnd) -> Option<Self> {
        let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
        let arrows: &[(Key, Self)] = match nav {
            Nav::Grid => &[
                (Key::ArrowDown, Self::Toward(Direction::Down)),
                (Key::ArrowUp, Self::Toward(Direction::Up)),
                (Key::ArrowLeft, Self::Toward(Direction::Left)),
                (Key::ArrowRight, Self::Toward(Direction::Right)),
            ],
            Nav::Row(_) => &[
                (Key::ArrowRight, Self::Line(Step::Next)),
                (Key::ArrowLeft, Self::Line(Step::Previous)),
            ],
            Nav::Radio(_) => &[
                (Key::ArrowDown, Self::Line(Step::Next)),
                (Key::ArrowRight, Self::Line(Step::Next)),
                (Key::ArrowUp, Self::Line(Step::Previous)),
                (Key::ArrowLeft, Self::Line(Step::Previous)),
            ],
            Nav::None | Nav::Menu(_) | Nav::Submenu(_) | Nav::Listbox(_) => &[
                (Key::ArrowDown, Self::Line(Step::Next)),
                (Key::ArrowUp, Self::Line(Step::Previous)),
            ],
        };
        let ends: &[(Key, Self)] = match (home_end, nav) {
            (HomeEnd::Ours, Nav::Radio(_)) | (HomeEnd::Theirs, _) => &[],
            (HomeEnd::Ours, _) => &[
                (Key::Home, Self::Line(Step::First)),
                (Key::End, Self::Line(Step::Last)),
            ],
        };
        if let Some((_, to)) = arrows.iter().find(|(key, _)| pressed(*key)) {
            ctx.memory_mut(|memory| memory.move_focus(FocusDirection::None));
            return Some(*to);
        }
        ends.iter()
            .find(|(key, _)| pressed(*key))
            .map(|(_, to)| *to)
    }

    /// The item the move goes to from `current`.
    fn target(self, items: &[(Id, Rect)], current: Option<Id>, wrap: Wrap) -> Option<Id> {
        let ids: Vec<Id> = items.iter().map(|(id, _)| *id).collect();
        match self {
            Self::Line(step) => target(&ids, current, step, wrap),
            Self::Toward(direction) => {
                let from = current.and_then(|current| items.iter().find(|(id, _)| *id == current));
                match from {
                    Some((id, rect)) => Some(nearest(*rect, items, direction).unwrap_or(*id)),
                    None => {
                        let step = match direction {
                            Direction::Right | Direction::Down => Step::Next,
                            Direction::Left | Direction::Up => Step::Previous,
                        };
                        target(&ids, None, step, wrap)
                    }
                }
            }
        }
    }
}

/// How far a candidate may reach back over the item it is reached from and
/// still lie in the direction: sub-pixel layout.
const SLACK: f32 = 0.5;

/// How much more a step across the direction counts than one along it.
const CROSS_WEIGHT: f32 = 2.0;

/// The item nearest to `from` in `direction`, among `items` in paint order.
///
/// A candidate counts only if it lies wholly past `from` in the direction
/// (within [`SLACK`]). The lowest of this triple wins:
///
/// 1. the gap from `from` to the candidate along the direction, plus
///    [`CROSS_WEIGHT`] times the offset between their centres across it,
///    counted as zero when their spans across it overlap: the same row or
///    column beats a diagonal, even one nearer along;
/// 2. the offset between their centres across the direction;
/// 3. the candidate's paint order.
///
/// Off the row or column, the nearest by that score is still reached: Left
/// from any row of a grid reaches a lone cell beside its first row.
///
/// None when nothing lies in the direction: the grid's edge.
fn nearest(from: Rect, items: &[(Id, Rect)], direction: Direction) -> Option<Id> {
    let along = match direction {
        Direction::Left | Direction::Right => Axis::X,
        Direction::Up | Direction::Down => Axis::Y,
    };
    let across = along.other();
    let forward = matches!(direction, Direction::Right | Direction::Down);
    let gap_along = |to: Rect| {
        let (to, from) = (along.span(to), along.span(from));
        if forward {
            to.0 - from.1
        } else {
            from.0 - to.1
        }
    };
    let overlaps = |to: Rect| {
        let (to, from) = (across.span(to), across.span(from));
        to.0 < from.1 && from.0 < to.1
    };
    let score = |to: Rect| {
        let offset = (across.at(to.center()) - across.at(from.center())).abs();
        let off_line = if overlaps(to) { 0.0 } else { offset };
        (gap_along(to).max(0.0) + CROSS_WEIGHT * off_line, offset)
    };
    items
        .iter()
        .enumerate()
        .filter(|(_, (_, to))| *to != from && gap_along(*to) >= -SLACK)
        .min_by(|(a_at, (_, a)), (b_at, (_, b))| {
            let (a_score, b_score) = (score(*a), score(*b));
            a_score
                .0
                .total_cmp(&b_score.0)
                .then(a_score.1.total_cmp(&b_score.1))
                .then(a_at.cmp(b_at))
        })
        .map(|(_, (id, _))| *id)
}

/// A key's move through the rows of a virtual list, which has no wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListMove {
    Next,
    Previous,
    First,
    Last,
    PageDown,
    PageUp,
}

impl ListMove {
    /// The move of the frame's key, taking it. An arrow is taken from egui
    /// too, as for an overlay's listbox.
    pub(crate) fn of_key(ctx: &Context) -> Option<Self> {
        let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
        const KEYS: [(Key, ListMove); 6] = [
            (Key::ArrowDown, ListMove::Next),
            (Key::ArrowUp, ListMove::Previous),
            (Key::Home, ListMove::First),
            (Key::End, ListMove::Last),
            (Key::PageDown, ListMove::PageDown),
            (Key::PageUp, ListMove::PageUp),
        ];
        let to = KEYS
            .iter()
            .find(|(key, _)| pressed(*key))
            .map(|(_, to)| *to)?;
        ctx.memory_mut(|memory| memory.move_focus(FocusDirection::None));
        Some(to)
    }

    /// The row this move goes to among `len` rows, from `current`. With no
    /// current row, a step and a page start from the rows shown, `visible`.
    /// A page is `page` rows.
    pub(crate) fn target(
        self,
        current: Option<usize>,
        len: usize,
        page: usize,
        visible: std::ops::Range<usize>,
    ) -> Option<usize> {
        let last = len.checked_sub(1)?;
        let from = |forward: bool| {
            current.unwrap_or(if forward {
                visible.start
            } else {
                visible.end.saturating_sub(1)
            })
        };
        let to = match self {
            Self::First => 0,
            Self::Last => last,
            Self::Next => current.map_or(visible.start, |at| at + 1),
            Self::Previous => current.map_or(from(false), |at| at.saturating_sub(1)),
            Self::PageDown => current.map_or(from(true), |at| at.saturating_add(page)),
            Self::PageUp => current.map_or(from(false), |at| at.saturating_sub(page)),
        };
        Some(to.min(last))
    }
}

/// Whether Home and End are the overlay's keys, or a text field's.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HomeEnd {
    Ours,
    Theirs,
}

/// The item `step` goes to from `current`: past an end it goes round or
/// stays as `wrap` says, and from something that is not an item it goes to
/// the first or last.
fn target(items: &[Id], current: Option<Id>, step: Step, wrap: Wrap) -> Option<Id> {
    let at = current.and_then(|current| items.iter().position(|&item| item == current));
    let last = items.len().checked_sub(1)?;
    let to = match (step, at, wrap) {
        (Step::First, ..) | (Step::Next, None, _) => 0,
        (Step::Last, ..) | (Step::Previous, None, _) => last,
        (Step::Next, Some(at), Wrap::Around) => (at + 1) % items.len(),
        (Step::Next, Some(at), Wrap::Stop) => (at + 1).min(last),
        (Step::Previous, Some(at), Wrap::Around) => at.checked_sub(1).unwrap_or(last),
        (Step::Previous, Some(at), Wrap::Stop) => at.saturating_sub(1),
    };
    items.get(to).copied()
}

/// The overlay the keys act on: the topmost that holds the focus in its own
/// widgets, else the topmost anchored to the focused widget, else, with
/// nothing focused, the topmost.
fn keyed<'a>(shell: &Shell, menus: &'a [Menu], focused: Option<Id>) -> Option<&'a Menu> {
    let Some(focused) = focused else {
        return menus.last();
    };
    menus
        .iter()
        .rfind(|menu| menu.own.contains(&focused))
        .or_else(|| {
            menus
                .iter()
                .rfind(|menu| menu.anchor.is_some_and(|anchor| shell.focus_within(anchor)))
        })
}

/// Applies the frame's navigation keys. Returns the overlays to dismiss.
/// `menus` are the open overlays with a pattern, the one on the bottom first.
pub(crate) fn navigate(ctx: &Context, kept: &Handle, menus: &[Menu]) -> Vec<Id> {
    let focused = ctx.memory(|memory| memory.focused());
    let Some(menu) = keyed(&Shell::new(ctx, kept), menus, focused) else {
        return Vec::new();
    };
    let home_end = if ctx.text_edit_focused() {
        HomeEnd::Theirs
    } else {
        HomeEnd::Ours
    };
    if let Some(focused) = focused {
        hold_arrows(ctx, focused);
    }
    let mut dismiss = Vec::new();
    match menu.nav {
        Nav::None => {}
        Nav::Menu(_) | Nav::Submenu(_) | Nav::Grid | Nav::Row(_) | Nav::Radio(_) => {
            if matches!(menu.nav, Nav::Submenu(_))
                && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowLeft))
            {
                dismiss.push(menu.id);
                if let Some(anchor) = menu.anchor {
                    focus_item(ctx, kept, anchor, focused);
                }
            } else if let Some(to) = Move::of_key(ctx, menu.nav, home_end)
                .and_then(|to| to.target(&menu.items, focused, menu.wrap))
            {
                focus_item(ctx, kept, to, focused);
            }
        }
        Nav::Listbox(_) => listbox(ctx, kept, menu, home_end),
    }
    dismiss
}

/// The item of a group on the page that the frame's key moves the focus to
/// from `focused`, one of `items`, taking the key. A text field in a group
/// keeps Home and End.
pub(crate) fn rove(ctx: &Context, nav: Nav, items: &[(Id, Rect)], focused: Id) -> Option<Id> {
    hold_arrows(ctx, focused);
    let home_end = if ctx.text_edit_focused() {
        HomeEnd::Theirs
    } else {
        HomeEnd::Ours
    };
    Move::of_key(ctx, nav, home_end)?
        .target(items, Some(focused), nav.wrap())
        .filter(|&to| to != focused)
}

/// Gives `to` the focus. With nothing focused before, egui has already read
/// the arrow as a move of focus this pass and would move it again from `to`,
/// so the focus is given at the start of the next frame instead.
fn focus_item(ctx: &Context, kept: &Handle, to: Id, before: Option<Id>) {
    if before.is_some() {
        ctx.memory_mut(|memory| memory.request_focus(to));
    } else {
        kept.viewport(ctx.viewport_id(), |here| {
            here.navigation.deferred = Some(to)
        });
        ctx.request_repaint();
    }
}

/// What a layout remembers of keyboard navigation.
#[derive(Default)]
pub(crate) struct Navigation {
    /// The item a navigation key chose with nothing focused.
    deferred: Option<Id>,
    activation: Option<Pending>,
}

/// Gives focus to the item a navigation key chose last frame, if nothing
/// has taken it since. Run before the frame's widgets.
pub(crate) fn apply_deferred(ctx: &Context, kept: &Handle) {
    let Some(item) = kept.viewport(ctx.viewport_id(), |here| here.navigation.deferred.take())
    else {
        return;
    };
    if ctx.memory(|memory| memory.focused()).is_none() {
        ctx.memory_mut(|memory| memory.request_focus(item));
    }
}

/// Keeps the focused widget's arrow keys from moving the focus: the overlay
/// moves it, or the highlight, itself.
pub(crate) fn hold_arrows(ctx: &Context, focused: Id) {
    let filter = EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        ..EventFilter::default()
    };
    ctx.memory_mut(|memory| memory.set_focus_lock_filter(focused, filter));
}

fn listbox(ctx: &Context, kept: &Handle, menu: &Menu, home_end: HomeEnd) {
    let mut active = Active::load(kept);
    let current = active
        .of(menu.id)
        .filter(|item| menu.items.iter().any(|(id, _)| id == item));
    if let Some(to) = Move::of_key(ctx, menu.nav, home_end)
        .and_then(|to| to.target(&menu.items, current, menu.wrap))
    {
        active.set(menu.id, to, menu.anchor);
        Shell::new(ctx, kept).scroll_into_view(to, None);
        ctx.request_repaint();
    } else if let Some(to) = pointed_at(ctx, &menu.items).filter(|&to| Some(to) != current) {
        active.set(menu.id, to, menu.anchor);
        ctx.request_repaint();
    }
    active.store(kept);
}

/// The item under a pointer that moved: the highlight follows it, so that
/// the pointer and the keys never leave two items looking highlighted.
fn pointed_at(ctx: &Context, items: &[(Id, Rect)]) -> Option<Id> {
    if ctx.input(|input| input.pointer.delta()) == Vec2::ZERO {
        return None;
    }
    items
        .iter()
        .map(|(id, _)| *id)
        .find(|&item| ctx.read_response(item).is_some_and(|r| r.hovered()))
}

/// The highlighted item of each listbox, and the node it is anchored to.
#[derive(Clone, Default)]
pub(crate) struct Active(HashMap<Id, Highlight>);

#[derive(Clone, Copy)]
struct Highlight {
    item: Id,
    anchor: Option<Id>,
    /// Whether the listbox is a virtual list, which is no overlay.
    list: bool,
}

impl Active {
    fn load(kept: &Handle) -> Self {
        kept.with(|kept| kept.listbox.clone())
    }

    fn store(self, kept: &Handle) {
        kept.with(|kept| kept.listbox = self);
    }

    fn of(&self, listbox: Id) -> Option<Id> {
        Some(self.0.get(&listbox)?.item)
    }

    fn set(&mut self, listbox: Id, item: Id, anchor: Option<Id>) {
        self.0.insert(
            listbox,
            Highlight {
                item,
                anchor,
                list: false,
            },
        );
    }
}

/// Highlights the row `item` of the virtual list `list`.
pub(crate) fn highlight_row(kept: &Handle, list: Id, item: Id) {
    kept.with(|kept| {
        kept.listbox.0.insert(
            list,
            Highlight {
                item,
                anchor: Some(list),
                list: true,
            },
        );
    });
}

/// Whether `item` is the highlighted item of a listbox.
pub(crate) fn is_highlighted(kept: &Handle, item: Id) -> bool {
    kept.with(|kept| kept.listbox.0.values().any(|h| h.item == item))
}

/// Forgets the highlight of the virtual lists `gone`.
pub(crate) fn forget_lists(kept: &mut crate::kept::Kept, gone: &[Id]) {
    for list in gone {
        kept.listbox.0.remove(list);
    }
}

/// Highlights `item` in the listbox `listbox` as it opens, scrolled to the
/// middle of its container, unless the listbox already has a highlight.
pub(crate) fn highlight_initially(ctx: &Context, kept: &Handle, listbox: Id, item: Id, anchor: Id) {
    let mut active = Active::load(kept);
    if active.of(listbox).is_some() {
        return;
    }
    active.set(listbox, item, Some(anchor));
    active.store(kept);
    Shell::new(ctx, kept).scroll_into_view(item, Some(Align::Center));
}

/// The items listboxes highlight now.
pub(crate) fn highlighted(kept: &Handle) -> Vec<Id> {
    Active::load(kept)
        .0
        .into_values()
        .map(|highlight| highlight.item)
        .collect()
}

/// Forgets the highlight of the listboxes that are not in `open`.
pub(crate) fn forget_closed(kept: &Handle, open: &[Id]) {
    let mut active = Active::load(kept);
    let before = active.0.len();
    active
        .0
        .retain(|listbox, highlight| highlight.list || open.contains(listbox));
    if active.0.len() != before {
        active.store(kept);
    }
}

/// Turns Enter, and Space unless a text field is being typed in, into the
/// activation of the highlighted item of the listbox the focus is on. Run
/// before the frame's widgets, so that the trigger or field that has the
/// focus does not take the key itself.
pub(crate) fn activate_highlighted(ctx: &Context, kept: &Handle) {
    let active = Active::load(kept);
    if active.0.is_empty() {
        return;
    }
    let shell = Shell::new(ctx, kept);
    let nothing_focused = ctx.memory(|memory| memory.focused()).is_none();
    let keyed = active
        .0
        .values()
        .find(|highlight| {
            highlight.anchor.is_some_and(|anchor| {
                if highlight.list {
                    crate::focus::holds_focus(ctx, anchor)
                } else {
                    shell.focus_within(anchor)
                }
            })
        })
        .or_else(|| {
            active
                .0
                .values()
                .find(|highlight| !highlight.list)
                .filter(|_| nothing_focused)
        });
    let Some(highlight) = keyed else {
        return;
    };
    let space = !ctx.text_edit_focused();
    let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    if pressed(Key::Enter) || (space && pressed(Key::Space)) {
        Pending::set(ctx, kept, highlight.item);
    }
}

#[derive(Clone, Copy)]
struct Pending {
    item: Id,
    pass: u64,
}

impl Pending {
    fn set(ctx: &Context, kept: &Handle, item: Id) {
        let pass = ctx.cumulative_pass_nr();
        kept.viewport(ctx.viewport_id(), |here| {
            here.navigation.activation = Some(Self { item, pass })
        });
    }
}

/// Activates `item` as a click on the next frame: a default or cancel button
/// answering its key.
pub(crate) fn request_activation(ctx: &Context, kept: &Handle, item: Id) {
    Pending::set(ctx, kept, item);
    ctx.request_repaint();
}

/// Whether Enter or Space activated the highlighted item `item` since it was
/// last asked: a click it did not get from the pointer. Asking takes it.
pub(crate) fn take_activation(ui: &egui::Ui, item: Id) -> bool {
    let Some(shell) = crate::shell::shell_or_report(ui, "keyboard activation (Enter/Space)") else {
        return false;
    };
    let (ctx, kept) = (ui.ctx(), shell.kept);
    let Some(pending) = kept.viewport(ctx.viewport_id(), |here| here.navigation.activation) else {
        return false;
    };
    let fresh = ctx.cumulative_pass_nr().saturating_sub(pending.pass) <= 1;
    if pending.item == item {
        kept.viewport(ctx.viewport_id(), |here| here.navigation.activation = None);
        return fresh;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(to: ListMove, current: Option<usize>) -> Option<usize> {
        to.target(current, 100, 10, 40..52)
    }

    #[test]
    fn steps_stop_at_the_ends_of_the_rows() {
        assert_eq!(row(ListMove::Next, Some(5)), Some(6));
        assert_eq!(row(ListMove::Next, Some(99)), Some(99));
        assert_eq!(row(ListMove::Previous, Some(5)), Some(4));
        assert_eq!(row(ListMove::Previous, Some(0)), Some(0));
    }

    #[test]
    fn home_and_end_go_to_the_first_and_last_row() {
        assert_eq!(row(ListMove::First, Some(50)), Some(0));
        assert_eq!(row(ListMove::Last, Some(50)), Some(99));
        assert_eq!(row(ListMove::First, None), Some(0));
        assert_eq!(row(ListMove::Last, None), Some(99));
    }

    #[test]
    fn a_page_moves_by_its_rows_and_stops_at_the_ends() {
        assert_eq!(row(ListMove::PageDown, Some(50)), Some(60));
        assert_eq!(row(ListMove::PageDown, Some(95)), Some(99));
        assert_eq!(row(ListMove::PageUp, Some(50)), Some(40));
        assert_eq!(row(ListMove::PageUp, Some(3)), Some(0));
    }

    #[test]
    fn with_no_current_row_a_step_starts_from_the_rows_shown() {
        assert_eq!(row(ListMove::Next, None), Some(40));
        assert_eq!(row(ListMove::PageDown, None), Some(40));
        assert_eq!(row(ListMove::Previous, None), Some(51));
        assert_eq!(row(ListMove::PageUp, None), Some(51));
    }

    #[test]
    fn no_rows_has_no_target() {
        assert_eq!(ListMove::Next.target(None, 0, 10, 0..0), None);
        assert_eq!(ListMove::Last.target(Some(0), 0, 10, 0..0), None);
    }

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    fn items() -> Vec<Id> {
        ["a", "b", "c"].map(id).to_vec()
    }

    fn go(current: Option<&str>, step: Step) -> Option<Id> {
        target(&items(), current.map(id), step, Wrap::Around)
    }

    #[test]
    fn down_and_up_wrap() {
        assert_eq!(go(Some("a"), Step::Next), Some(id("b")));
        assert_eq!(go(Some("c"), Step::Next), Some(id("a")));
        assert_eq!(go(Some("b"), Step::Previous), Some(id("a")));
        assert_eq!(go(Some("a"), Step::Previous), Some(id("c")));
    }

    #[test]
    fn stopping_stays_on_the_last_and_first_item() {
        let go = |current, step| target(&items(), Some(id(current)), step, Wrap::Stop);
        assert_eq!(go("c", Step::Next), Some(id("c")));
        assert_eq!(go("a", Step::Previous), Some(id("a")));
        assert_eq!(go("a", Step::Next), Some(id("b")));
        assert_eq!(go("c", Step::Previous), Some(id("b")));
    }

    #[test]
    fn home_and_end_go_to_the_ends() {
        assert_eq!(go(Some("b"), Step::First), Some(id("a")));
        assert_eq!(go(Some("b"), Step::Last), Some(id("c")));
    }

    #[test]
    fn from_outside_the_items_down_is_the_first_and_up_the_last() {
        assert_eq!(go(None, Step::Next), Some(id("a")));
        assert_eq!(go(Some("x"), Step::Next), Some(id("a")));
        assert_eq!(go(None, Step::Previous), Some(id("c")));
    }

    /// A cell 20 points square at column `col` and row `row` of a grid
    /// starting at `left`, with 4 points between cells.
    fn cell(left: f32, col: u32, row: u32) -> Rect {
        let min = egui::pos2(left + col as f32 * 24.0, row as f32 * 24.0);
        Rect::from_min_size(min, egui::vec2(20.0, 20.0))
    }

    /// The issue's picker: a lone cell, a rule, then 4 columns of 3 rows.
    fn picker() -> Vec<(Id, Rect)> {
        let mut items = vec![(id("lone"), cell(0.0, 0, 0))];
        for row in 0..3 {
            for col in 0..4 {
                items.push((id(&format!("{col},{row}")), cell(40.0, col, row)));
            }
        }
        items
    }

    fn rect_of(items: &[(Id, Rect)], name: &str) -> Rect {
        items
            .iter()
            .find(|(at, _)| *at == id(name))
            .expect("an item")
            .1
    }

    fn from(items: &[(Id, Rect)], name: &str, direction: Direction) -> Option<Id> {
        nearest(rect_of(items, name), items, direction)
    }

    #[test]
    fn arrows_move_along_rows_and_columns() {
        let items = picker();
        assert_eq!(from(&items, "1,1", Direction::Right), Some(id("2,1")));
        assert_eq!(from(&items, "1,1", Direction::Left), Some(id("0,1")));
        assert_eq!(from(&items, "1,1", Direction::Down), Some(id("1,2")));
        assert_eq!(from(&items, "1,1", Direction::Up), Some(id("1,0")));
    }

    #[test]
    fn left_from_the_first_column_crosses_to_the_lone_cell() {
        let items = picker();
        for row in 0..3 {
            assert_eq!(
                from(&items, &format!("0,{row}"), Direction::Left),
                Some(id("lone"))
            );
        }
        assert_eq!(from(&items, "lone", Direction::Right), Some(id("0,0")));
    }

    #[test]
    fn the_edges_stop() {
        let items = picker();
        assert_eq!(from(&items, "2,0", Direction::Up), None);
        assert_eq!(from(&items, "3,1", Direction::Right), None);
        assert_eq!(from(&items, "1,2", Direction::Down), None);
        assert_eq!(from(&items, "lone", Direction::Left), None);
        assert_eq!(from(&items, "lone", Direction::Up), None);
    }

    #[test]
    fn off_the_line_the_nearest_is_still_reached() {
        let items = picker();
        assert_eq!(from(&items, "lone", Direction::Down), Some(id("0,1")));
    }

    #[test]
    fn a_ragged_last_row_is_reached_at_its_nearest_cell() {
        let mut items: Vec<(Id, Rect)> = (0..2)
            .flat_map(|row| (0..4).map(move |col| (col, row)))
            .map(|(col, row)| (id(&format!("{col},{row}")), cell(0.0, col, row)))
            .collect();
        items.extend((0..2).map(|col| (id(&format!("{col},2")), cell(0.0, col, 2))));
        assert_eq!(from(&items, "3,1", Direction::Down), Some(id("1,2")));
        assert_eq!(from(&items, "1,2", Direction::Up), Some(id("1,1")));
        assert_eq!(
            from(&items, "1,2", Direction::Right),
            Some(id("2,1")),
            "off the row"
        );
    }

    #[test]
    fn a_lone_cell_goes_nowhere() {
        let items = vec![(id("only"), cell(0.0, 0, 0))];
        for direction in [
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ] {
            assert_eq!(from(&items, "only", direction), None);
        }
    }

    #[test]
    fn the_same_column_beats_a_nearer_diagonal() {
        let items = vec![
            (id("from"), cell(0.0, 0, 0)),
            (id("diagonal"), cell(0.0, 1, 1)),
            (id("in line"), cell(0.0, 0, 2)),
        ];
        assert_eq!(from(&items, "from", Direction::Down), Some(id("in line")));
    }

    #[test]
    fn ties_go_to_the_nearer_centre_then_to_paint_order() {
        let wide = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(44.0, 20.0));
        let left = cell(0.0, 0, 1);
        let right = cell(0.0, 1, 1);
        let items = vec![(id("wide"), wide), (id("left"), left), (id("right"), right)];
        assert_eq!(
            from(&items, "wide", Direction::Down),
            Some(id("left")),
            "equal: paint order"
        );
        let off_centre = Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(34.0, 20.0));
        let items = vec![
            (id("wide"), off_centre),
            (id("left"), left),
            (id("right"), right),
        ];
        assert_eq!(
            from(&items, "wide", Direction::Down),
            Some(id("right")),
            "nearer centre"
        );
    }

    #[test]
    fn cells_of_other_sizes_and_sub_pixel_overlap_still_count() {
        let from_rect = cell(0.0, 0, 0);
        let tall = Rect::from_min_max(egui::pos2(19.7, -10.0), egui::pos2(40.0, 40.0));
        let items = vec![(id("from"), from_rect), (id("tall"), tall)];
        assert_eq!(from(&items, "from", Direction::Right), Some(id("tall")));
        assert_eq!(from(&items, "tall", Direction::Left), Some(id("from")));
        let overlapping = Rect::from_min_size(egui::pos2(15.0, 0.0), egui::vec2(20.0, 20.0));
        let items = vec![(id("from"), from_rect), (id("overlapping"), overlapping)];
        assert_eq!(from(&items, "from", Direction::Right), None);
    }

    #[test]
    fn a_grid_move_from_no_item_goes_to_an_end() {
        let items = picker();
        let toward = |direction| Move::Toward(direction).target(&items, None, Wrap::Stop);
        assert_eq!(toward(Direction::Right), Some(id("lone")));
        assert_eq!(toward(Direction::Down), Some(id("lone")));
        assert_eq!(toward(Direction::Left), Some(id("3,2")));
        assert_eq!(toward(Direction::Up), Some(id("3,2")));
        let at_edge = Move::Toward(Direction::Up).target(&items, Some(id("2,0")), Wrap::Stop);
        assert_eq!(at_edge, Some(id("2,0")), "the edge keeps the item");
    }

    #[test]
    fn no_items_go_nowhere() {
        for step in [Step::Next, Step::Previous, Step::First, Step::Last] {
            assert_eq!(target(&[], None, step, Wrap::Around), None);
        }
    }
}
