//! Arrow-key navigation of an overlay's items: the keyboard patterns of
//! [`Nav`].
//!
//! A menu moves real focus between its items. A listbox leaves focus on its
//! trigger and moves a virtual highlight, which the draw shows with the
//! item's `hover` look, and which Enter and Space activate: the item's
//! `on_click` runs in the next frame, as if it had been clicked.

use std::collections::HashMap;

use egui::{Context, EventFilter, Id, Key, Modifiers};

use crate::focus::focus_within;
use crate::overlay::Nav;

/// An open overlay with a keyboard pattern.
pub(crate) struct Menu {
    pub(crate) id: Id,
    pub(crate) nav: Nav,
    /// The items that take input, in paint order.
    pub(crate) items: Vec<Id>,
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

impl Step {
    fn of_key(ctx: &Context, home_end: HomeEnd) -> Option<Self> {
        let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
        if pressed(Key::ArrowDown) {
            Some(Self::Next)
        } else if pressed(Key::ArrowUp) {
            Some(Self::Previous)
        } else if home_end == HomeEnd::Ours && pressed(Key::Home) {
            Some(Self::First)
        } else if home_end == HomeEnd::Ours && pressed(Key::End) {
            Some(Self::Last)
        } else {
            None
        }
    }
}

/// Whether Home and End are the overlay's keys, or a text field's.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HomeEnd {
    Ours,
    Theirs,
}

/// The item `step` goes to from `current`: wrapping, and the first or last
/// when `current` is not an item.
fn target(items: &[Id], current: Option<Id>, step: Step) -> Option<Id> {
    let at = current.and_then(|current| items.iter().position(|&item| item == current));
    let last = items.len().checked_sub(1)?;
    let to = match (step, at) {
        (Step::First, _) | (Step::Next, None) => 0,
        (Step::Last, _) | (Step::Previous, None) => last,
        (Step::Next, Some(at)) => (at + 1) % items.len(),
        (Step::Previous, Some(at)) => at.checked_sub(1).unwrap_or(last),
    };
    items.get(to).copied()
}

/// The overlay the keys act on: the topmost that holds the focus in its own
/// widgets, else the topmost anchored to the focused widget, else, with
/// nothing focused, the topmost.
fn keyed<'a>(ctx: &Context, menus: &'a [Menu], focused: Option<Id>) -> Option<&'a Menu> {
    let Some(focused) = focused else {
        return menus.last();
    };
    menus
        .iter()
        .rfind(|menu| menu.own.contains(&focused))
        .or_else(|| {
            menus
                .iter()
                .rfind(|menu| menu.anchor.is_some_and(|anchor| focus_within(ctx, anchor)))
        })
}

/// Applies the frame's navigation keys. Returns the overlays to dismiss.
/// `menus` are the open overlays with a pattern, the one on the bottom first.
pub(crate) fn navigate(ctx: &Context, menus: &[Menu]) -> Vec<Id> {
    let focused = ctx.memory(|memory| memory.focused());
    let Some(menu) = keyed(ctx, menus, focused) else {
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
        Nav::Menu | Nav::Submenu => {
            if menu.nav == Nav::Submenu
                && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowLeft))
            {
                dismiss.push(menu.id);
                if let Some(anchor) = menu.anchor {
                    ctx.memory_mut(|memory| memory.request_focus(anchor));
                }
            } else if let Some(step) = Step::of_key(ctx, home_end)
                && let Some(to) = target(&menu.items, focused, step)
            {
                focus_item(ctx, to, focused);
            }
        }
        Nav::Listbox => listbox(ctx, menu, home_end),
    }
    dismiss
}

/// Gives `to` the focus. With nothing focused before, egui has already read
/// the arrow as a move of focus this pass and would move it again from `to`,
/// so the focus is given at the start of the next frame instead.
fn focus_item(ctx: &Context, to: Id, before: Option<Id>) {
    if before.is_some() {
        ctx.memory_mut(|memory| memory.request_focus(to));
    } else {
        ctx.data_mut(|data| data.insert_temp(deferred_key(), to));
        ctx.request_repaint();
    }
}

fn deferred_key() -> Id {
    crate::key::root_id().with("deferred-item-focus")
}

/// Gives focus to the item a navigation key chose last frame, if nothing
/// has taken it since. Run before the frame's widgets.
pub(crate) fn apply_deferred(ctx: &Context) {
    let Some(item) = ctx.data(|data| data.get_temp::<Id>(deferred_key())) else {
        return;
    };
    ctx.data_mut(|data| data.remove::<Id>(deferred_key()));
    if ctx.memory(|memory| memory.focused()).is_none() {
        ctx.memory_mut(|memory| memory.request_focus(item));
    }
}

/// Keeps the focused widget's arrow keys from moving the focus: the overlay
/// moves it, or the highlight, itself.
fn hold_arrows(ctx: &Context, focused: Id) {
    let filter = EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        ..EventFilter::default()
    };
    ctx.memory_mut(|memory| memory.set_focus_lock_filter(focused, filter));
}

fn listbox(ctx: &Context, menu: &Menu, home_end: HomeEnd) {
    let mut active = Active::load(ctx);
    let current = active.of(menu.id).filter(|item| menu.items.contains(item));
    if let Some(step) = Step::of_key(ctx, home_end)
        && let Some(to) = target(&menu.items, current, step)
    {
        active.set(menu.id, to, menu.anchor);
    }
    active.store(ctx);
}

/// The highlighted item of each listbox, and the node it is anchored to.
#[derive(Clone, Default)]
struct Active(HashMap<Id, Highlight>);

#[derive(Clone, Copy)]
struct Highlight {
    item: Id,
    anchor: Option<Id>,
}

fn active_key() -> Id {
    crate::key::root_id().with("listbox-active")
}

impl Active {
    fn load(ctx: &Context) -> Self {
        ctx.data(|data| data.get_temp(active_key()))
            .unwrap_or_default()
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(active_key(), self));
    }

    fn of(&self, listbox: Id) -> Option<Id> {
        Some(self.0.get(&listbox)?.item)
    }

    fn set(&mut self, listbox: Id, item: Id, anchor: Option<Id>) {
        self.0.insert(listbox, Highlight { item, anchor });
    }
}

/// The items listboxes highlight now.
pub(crate) fn highlighted(ctx: &Context) -> Vec<Id> {
    Active::load(ctx)
        .0
        .into_values()
        .map(|highlight| highlight.item)
        .collect()
}

/// Forgets the highlight of the listboxes that are not in `open`.
pub(crate) fn forget_closed(ctx: &Context, open: &[Id]) {
    let mut active = Active::load(ctx);
    let before = active.0.len();
    active.0.retain(|listbox, _| open.contains(listbox));
    if active.0.len() != before {
        active.store(ctx);
    }
}

/// Turns Enter, and Space unless a text field is being typed in, into the
/// activation of the highlighted item of the listbox the focus is on. Run
/// before the frame's widgets, so that the trigger or field that has the
/// focus does not take the key itself.
pub(crate) fn activate_highlighted(ctx: &Context) {
    let active = Active::load(ctx);
    if active.0.is_empty() {
        return;
    }
    let keyed = active.0.values().find(|highlight| {
        highlight
            .anchor
            .is_some_and(|anchor| focus_within(ctx, anchor))
    });
    let Some(highlight) = keyed else {
        return;
    };
    let space = !ctx.text_edit_focused();
    let pressed = |key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    if pressed(Key::Enter) || (space && pressed(Key::Space)) {
        Pending::set(ctx, highlight.item);
    }
}

#[derive(Clone, Copy)]
struct Pending {
    item: Id,
    pass: u64,
}

fn pending_key() -> Id {
    crate::key::root_id().with("listbox-activation")
}

impl Pending {
    fn set(ctx: &Context, item: Id) {
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|data| data.insert_temp(pending_key(), Self { item, pass }));
    }
}

/// Activates `item` as a click on the next frame: a default or cancel button
/// answering its key.
pub(crate) fn request_activation(ctx: &Context, item: Id) {
    Pending::set(ctx, item);
    ctx.request_repaint();
}

/// Whether Enter or Space activated the highlighted item `item` since it was
/// last asked: a click it did not get from the pointer. Asking takes it.
pub(crate) fn take_activation(ctx: &Context, item: Id) -> bool {
    let Some(pending) = ctx.data(|data| data.get_temp::<Pending>(pending_key())) else {
        return false;
    };
    let fresh = ctx.cumulative_pass_nr().saturating_sub(pending.pass) <= 1;
    if pending.item == item {
        ctx.data_mut(|data| data.remove::<Pending>(pending_key()));
        return fresh;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    fn items() -> Vec<Id> {
        ["a", "b", "c"].map(id).to_vec()
    }

    fn go(current: Option<&str>, step: Step) -> Option<Id> {
        target(&items(), current.map(id), step)
    }

    #[test]
    fn down_and_up_wrap() {
        assert_eq!(go(Some("a"), Step::Next), Some(id("b")));
        assert_eq!(go(Some("c"), Step::Next), Some(id("a")));
        assert_eq!(go(Some("b"), Step::Previous), Some(id("a")));
        assert_eq!(go(Some("a"), Step::Previous), Some(id("c")));
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

    #[test]
    fn no_items_go_nowhere() {
        for step in [Step::Next, Step::Previous, Step::First, Step::Last] {
            assert_eq!(target(&[], None, step), None);
        }
    }
}
