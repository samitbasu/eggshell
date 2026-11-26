//! Hover cards: overlays that hover intent opens and closes
//! ([`hover_anchored`](crate::overlay::OverlayChain::hover_anchored)).
//!
//! The chain declares the card at build and reads whether it is open from
//! [`Cards`]. At the end of the draw, once dismissal is judged, [`end`]
//! resolves the declared cards' triggers against the pointer and the focus
//! and moves the intent. The open card's box, and the overlays nested in it,
//! count as its trigger, so the pointer may cross into the card.
//!
//! The cards of a [`TriggerGroup`] share an intent of their own, a row's: a
//! press toggles them, they move along the row by
//! [`trigger_group::switch`](crate::trigger_group::switch), and [`route_keys`]
//! puts the open one's widgets in the Tab order after its trigger.

use std::collections::HashMap;
use std::ops::Range;

use egui::{Context, FocusDirection, Id, Key, Modifiers, Pos2, Rect};

use crate::builder::Builder;
use crate::intent::{
    Cause, FocusOpens, Intent, Interruption, Press, Skip, Timing, Trigger, wanted,
};
use crate::kept::Handle;
use crate::nodes::Nodes;
use crate::overlay::HoverIntent;
#[cfg(doc)]
use crate::overlay::TriggerGroup;
use crate::pipeline::PlacedNode;
use crate::presence::Shown;
use crate::roots::{Here, Home, PlacedRoot, ancestors, root_at, root_of};
use crate::shell::Shell;

/// A card declared this frame.
struct Card {
    trigger: Id,
    overlay: Id,
    intent: HoverIntent,
}

impl Card {
    fn group(&self) -> Option<Id> {
        self.intent.group.map(|group| group.id)
    }
}

/// The hover cards of a viewport: their intent, and what this frame declared.
#[derive(Default)]
pub(crate) struct Cards {
    /// The intent of the cards in no group.
    pub(crate) intent: Intent<Timing>,
    rows: HashMap<Id, Row>,
    declared: Vec<Card>,
    /// The cards dismissal closed this frame, by overlay.
    dismissed: Vec<Id>,
}

impl Cards {
    /// The rules of `dismiss.rs` closed the card `overlay`: Esc, a press
    /// outside, focus leaving or the window losing focus.
    pub(crate) fn dismissed(&mut self, overlay: Id) {
        self.dismissed.push(overlay);
    }

    fn intent_of(&mut self, group: Option<Id>) -> &mut Intent<Timing> {
        match group {
            Some(group) => &mut self.rows.entry(group).or_default().intent,
            None => &mut self.intent,
        }
    }
}

/// A [`TriggerGroup`]'s intent, and what its keys need of the last frame.
#[derive(Default)]
struct Row {
    intent: Intent<Timing>,
    triggers: Vec<Id>,
    open: Option<OpenCard>,
    /// The widget that had focus as the last frame ended.
    focused: Option<Id>,
    /// The trigger Down was pressed on: it opens, and focus goes to its
    /// card's first widget once it is drawn.
    entering: Option<Id>,
    /// A widget of the open card that a Tab from outside the row reached,
    /// and the way the next frame moves the focus on from it: past the
    /// card, which is in the Tab order only through its trigger.
    stray: Option<(Id, FocusDirection)>,
}

/// The open trigger of a row, as last drawn.
#[derive(Clone, Debug, PartialEq)]
struct OpenCard {
    trigger: Id,
    /// Its card's focusable widgets, in Tab order.
    widgets: Box<[Id]>,
    /// The item of the row after the trigger, from which Shift+Tab goes into
    /// the card.
    after: Option<Id>,
}

impl OpenCard {
    /// Whether the focus on `widget` is the row's own business around the
    /// card: on the trigger, in the card, or on the item after it.
    fn around(&self, widget: Id) -> bool {
        widget == self.trigger || self.widgets.contains(&widget) || self.after == Some(widget)
    }
}

/// What a key does to the focus of a row, before the frame's widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    /// Takes the key and focuses the widget.
    To(Id),
    /// Focuses the trigger, and lets the Tab go on from it.
    Past(Id),
    /// Takes the key, and opens the trigger with focus going in.
    Enter(Id),
}

/// A key that moves the focus of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowKey {
    Tab,
    ShiftTab,
    Down,
}

impl RowKey {
    const ALL: [(Self, Modifiers, Key); 3] = [
        (Self::Tab, Modifiers::NONE, Key::Tab),
        (Self::ShiftTab, Modifiers::SHIFT, Key::Tab),
        (Self::Down, Modifiers::NONE, Key::ArrowDown),
    ];
}

impl Row {
    /// Where `key` takes the focus from `focused`, if the row has a say.
    fn route(&self, focused: Id, key: RowKey) -> Option<Route> {
        if key == RowKey::Down {
            return self
                .triggers
                .contains(&focused)
                .then_some(Route::Enter(focused));
        }
        let open = self.open.as_ref()?;
        let first = open.widgets.first().copied();
        let last = open.widgets.last().copied();
        match key {
            RowKey::Tab if focused == open.trigger => first.map(Route::To),
            RowKey::Tab => (last == Some(focused)).then_some(Route::Past(open.trigger)),
            RowKey::ShiftTab if first == Some(focused) => Some(Route::To(open.trigger)),
            RowKey::ShiftTab => last.filter(|_| open.after == Some(focused)).map(Route::To),
            RowKey::Down => None,
        }
    }
}

/// Moves the focus on the frame's Tab or Down within a row of triggers and
/// its open card. Run before the frame's widgets: a Tab left to egui moves on
/// from where this puts the focus.
pub(crate) fn route_keys(ctx: &Context, kept: &Handle) {
    let strays: Vec<(Id, FocusDirection)> = kept.viewport(ctx.viewport_id(), |here| {
        here.hover_cards
            .rows
            .values_mut()
            .filter_map(|row| row.stray.take())
            .collect()
    });
    for (widget, onward) in strays {
        ctx.memory_mut(|memory| {
            memory.request_focus(widget);
            memory.move_focus(onward);
        });
    }
    let Some(focused) = ctx.memory(|memory| memory.focused()) else {
        return;
    };
    let Some((row, key, modifiers, route)) = kept.viewport(ctx.viewport_id(), |here| {
        here.hover_cards.rows.iter().find_map(|(&row, state)| {
            RowKey::ALL.iter().find_map(|&(which, modifiers, key)| {
                let pressed = ctx.input(|input| input.modifiers.matches_exact(modifiers))
                    && ctx.input(|input| input.key_pressed(key));
                pressed
                    .then(|| state.route(focused, which))
                    .flatten()
                    .map(|route| (row, key, modifiers, route))
            })
        })
    }) else {
        return;
    };
    let take = || {
        ctx.input_mut(|input| input.consume_key(modifiers, key));
        ctx.memory_mut(|memory| memory.move_focus(FocusDirection::None));
    };
    match route {
        Route::To(widget) => {
            take();
            ctx.memory_mut(|memory| memory.request_focus(widget));
        }
        Route::Past(trigger) => ctx.memory_mut(|memory| memory.request_focus(trigger)),
        Route::Enter(trigger) => {
            take();
            let first = kept.viewport(ctx.viewport_id(), |here| {
                let state = here.hover_cards.rows.get_mut(&row)?;
                let first = state
                    .open
                    .as_ref()
                    .filter(|open| open.trigger == trigger)
                    .and_then(|open| open.widgets.first().copied());
                if first.is_none() {
                    state.entering = Some(trigger);
                }
                first
            });
            if let Some(first) = first {
                ctx.memory_mut(|memory| memory.request_focus(first));
            } else {
                ctx.request_repaint();
            }
        }
    }
}

fn timing(intent: &HoverIntent) -> Timing {
    let (skip, press) = match intent.group {
        Some(group) => (Skip::Grace(group.skip), Press::Toggles),
        None => (Skip::Never, Press::Pins),
    };
    Timing {
        open: intent.open,
        close: intent.close,
        skip,
        focus: FocusOpens::AfterDelay,
        press,
    }
}

/// The timing a tick takes when no trigger wants a card: nothing opens by
/// it, and an open card lingers for the close delay it opened with.
const NOTHING_WANTED: Timing = Timing {
    open: core::time::Duration::ZERO,
    close: core::time::Duration::ZERO,
    skip: Skip::Never,
    focus: FocusOpens::AfterDelay,
    press: Press::Pins,
};

/// Declares the card `overlay`, which `trigger` opens. Returns whether it is
/// shown, or `None` for a card declared inside another card, which is
/// reported and never opens.
pub(crate) fn declare<A>(
    b: &Builder<'_, '_, A>,
    (overlay, trigger): (Id, Id),
    intent: HoverIntent,
    at: &'static std::panic::Location<'static>,
) -> Option<Shown> {
    if b.in_hover_card() {
        crate::report::report_at(
            b.ctx(),
            crate::Problem::NestedHoverCard,
            Some(overlay),
            at,
            || "a hover card declared inside another hover card never opens".to_owned(),
        );
        return None;
    }
    let open = b.kept().viewport(b.viewport(), |here| {
        let cards = &mut here.hover_cards;
        let group = intent.group.map(|group| group.id);
        cards.declared.retain(|card| card.trigger != trigger);
        cards.declared.push(Card {
            trigger,
            overlay,
            intent,
        });
        cards.intent_of(group).open_triggers().contains(&trigger)
    });
    Some(if open { Shown::Open } else { Shown::Closed })
}

/// Whether the card `trigger` of the row `group` opens is open, as
/// [`declare`] will say: for the trigger's own look, built before its card.
#[cfg(feature = "navigation-menu")]
pub(crate) fn is_open<A>(b: &Builder<'_, '_, A>, group: Id, trigger: Id) -> bool {
    b.kept().viewport(b.viewport(), |here| {
        here.hover_cards
            .intent_of(Some(group))
            .open_triggers()
            .contains(&trigger)
    })
}

/// The ids of the nodes in hover cards' boxes: Tab never stops in a card,
/// unless it is one of a row's.
pub(crate) fn off_tab<'a>(
    nodes: &'a Nodes,
    roots: &'a [PlacedRoot],
    here: Here,
) -> impl Iterator<Item = Id> + 'a {
    roots
        .iter()
        .filter(move |root| root.home == here.home)
        .filter(|root| {
            root.overlay()
                .and_then(|overlay| overlay.behavior.hover)
                .is_some_and(|hovered| hovered.group.is_none())
        })
        .flat_map(|root| nodes.subtree(root.first).iter().filter_map(PlacedNode::id))
}

/// What [`end`] reads of the frame.
pub(crate) struct Frame<'a> {
    pub(crate) ctx: &'a Context,
    pub(crate) kept: &'a Handle,
    pub(crate) nodes: &'a Nodes,
    pub(crate) by_id: &'a HashMap<Id, usize>,
    pub(crate) roots: &'a [PlacedRoot],
    pub(crate) here: Here,
    /// The focusable widgets each identified node registered this frame.
    pub(crate) subtrees: &'a [(Id, Box<[Id]>)],
}

/// Where the pointer is, unless the last pointer input was a touch: touch
/// never opens a card.
fn mouse_at(ctx: &Context) -> Option<Pos2> {
    if crate::overlay_input::touch_was_last(ctx) {
        return None;
    }
    ctx.input(|input| input.pointer.hover_pos())
}

/// Where the press still held down this frame started, if one is.
fn held_from(ctx: &Context) -> Option<Pos2> {
    ctx.input(|input| {
        input
            .pointer
            .any_down()
            .then(|| input.pointer.press_origin())
            .flatten()
    })
}

/// What the cards' triggers are judged against: the pointer, the press, the
/// keyboard and the focus of the frame.
struct Sight<'a> {
    frame: Frame<'a>,
    pointer: Option<Pos2>,
    under: Option<usize>,
    held: Option<usize>,
    pressed: Option<(Pos2, usize)>,
    keyboard: bool,
    tabbed: bool,
    blurred: bool,
    focused: Option<Id>,
    shell: Shell,
}

/// A card's trigger as drawn this frame.
struct Drawn {
    index: usize,
    rect: Rect,
    root: usize,
}

impl<'a> Sight<'a> {
    fn new(frame: Frame<'a>) -> Self {
        let ctx = frame.ctx;
        let root_under = |at: Pos2| root_at(frame.roots, frame.nodes, at, frame.here);
        let pointer = mouse_at(ctx);
        Self {
            pointer,
            under: pointer.map(root_under),
            held: held_from(ctx).map(root_under),
            pressed: crate::overlay_input::press_origin(ctx).map(|at| (at, root_under(at))),
            keyboard: crate::overlay_input::keyboard_was_last(ctx),
            tabbed: ctx.input(|input| input.key_pressed(Key::Tab)),
            blurred: ctx.input(|input| crate::window::blurred(&input.events)),
            focused: ctx.memory(|memory| memory.focused()),
            shell: Shell::new(ctx, frame.kept),
            frame,
        }
    }

    fn drawn(&self, card: &Card) -> Option<Drawn> {
        let Frame {
            nodes,
            by_id,
            roots,
            ..
        } = self.frame;
        let index = *by_id.get(&card.trigger)?;
        let node = nodes.node(index)?;
        let root =
            root_of(roots, index).filter(|&root| Home::of(roots, root) == self.frame.here.home)?;
        Some(Drawn {
            index,
            rect: node.rect(),
            root,
        })
    }

    /// The roots of the open cards among `cards`.
    fn open_roots(&self, cards: &[&Card], open: &[Id]) -> Vec<usize> {
        cards
            .iter()
            .filter(|card| open.contains(&card.trigger))
            .filter_map(|card| {
                self.frame.roots.iter().position(|root| {
                    root.overlay()
                        .is_some_and(|overlay| overlay.id == card.overlay)
                })
            })
            .collect()
    }

    fn in_any(&self, open_roots: &[usize], root: usize) -> bool {
        open_roots
            .iter()
            .any(|&card| root == card || ancestors(self.frame.roots, root).any(|at| at == card))
    }
}

/// What one row's (or the lone cards') triggers want this frame.
struct Wants {
    requests: Vec<Trigger<Timing>>,
    drawn: Vec<(Id, Rect)>,
}

/// Moves the cards' intent to this frame, after dismissal is judged, and
/// describes each trigger by its card. Returns what the tooltips yield to.
pub(crate) fn end(frame: Frame<'_>) -> Quiet<'_> {
    let (ctx, kept, nodes, by_id) = (frame.ctx, frame.kept, frame.nodes, frame.by_id);
    let (declared, dismissed) = kept.viewport(ctx.viewport_id(), |here| {
        let cards = &mut here.hover_cards;
        (
            std::mem::take(&mut cards.declared),
            std::mem::take(&mut cards.dismissed),
        )
    });
    let sight = Sight::new(frame);
    let mut groups: Vec<Option<Id>> = vec![None];
    for card in &declared {
        if !groups.contains(&card.group()) {
            groups.push(card.group());
        }
    }
    kept.viewport(ctx.viewport_id(), |here| {
        here.hover_cards
            .rows
            .retain(|row, _| groups.contains(&Some(*row)));
    });
    let mut silenced = Vec::new();
    for group in groups {
        let cards: Vec<&Card> = declared
            .iter()
            .filter(|card| card.group() == group)
            .collect();
        settle(&sight, group, &cards, &dismissed, &mut silenced);
    }
    Quiet {
        nodes,
        by_id,
        silenced,
    }
}

/// Moves the intent of the cards of `group`, or of the lone cards, to this
/// frame.
fn settle(
    sight: &Sight<'_>,
    group: Option<Id>,
    cards: &[&Card],
    dismissed: &[Id],
    silenced: &mut Vec<Silenced>,
) {
    let Frame {
        ctx,
        kept,
        nodes,
        subtrees,
        ..
    } = sight.frame;
    let viewport = ctx.viewport_id();
    let (open, pinned, last_focused, entering) = kept.viewport(viewport, |here| {
        let cards = &mut here.hover_cards;
        let intent = cards.intent_of(group);
        let (open, pinned) = (intent.open_triggers(), intent.pinned());
        let row = group.and_then(|group| cards.rows.get(&group));
        (
            open,
            pinned,
            row.and_then(|row| row.focused),
            row.and_then(|row| row.entering),
        )
    });
    let open_roots = sight.open_roots(cards, &open);
    let in_open_card = |root: usize| sight.in_any(&open_roots, root);
    let mut wants = Wants {
        requests: Vec::new(),
        drawn: Vec::new(),
    };
    for card in cards {
        let Some(drawn) = sight.drawn(card) else {
            continue;
        };
        wants.drawn.push((card.trigger, drawn.rect));
        let over_trigger = sight.pointer.is_some_and(|at| drawn.rect.contains(at))
            && sight.under == Some(drawn.root);
        let is_open = open.contains(&card.trigger);
        let over_card = is_open
            && (sight.under.is_some_and(in_open_card) || sight.held.is_some_and(in_open_card));
        let focused = sight.shell.focus_within(card.trigger);
        let causes = match group {
            None => [
                (sight.keyboard && focused).then_some(Cause::Focus),
                (over_trigger || over_card).then_some(Cause::Pointer),
                None,
            ],
            Some(_) => {
                let switched =
                    crate::trigger_group::switch(open.first().copied(), card.trigger).is_some();
                let arrived = sight.keyboard
                    && !sight.tabbed
                    && sight.focused == Some(card.trigger)
                    && last_focused != sight.focused;
                let pressed = ctx
                    .read_response(card.trigger)
                    .is_some_and(|response| response.clicked())
                    || (entering == Some(card.trigger) && !is_open)
                    || (switched && (arrived || (over_trigger && pinned.is_some())));
                [
                    (is_open && sight.shell.focus_within(card.overlay)).then_some(Cause::Focus),
                    (over_trigger || over_card).then_some(Cause::Pointer),
                    pressed.then_some(Cause::Pressed),
                ]
            }
        };
        for cause in causes.into_iter().flatten() {
            wants.requests.push(Trigger {
                id: card.trigger,
                payload: timing(&card.intent),
                cause,
                rect: drawn.rect,
            });
        }
        silenced.push(Silenced {
            span: nodes.span(drawn.index),
            wants: [
                focused.then_some(Cause::Focus),
                over_trigger.then_some(Cause::Pointer),
            ],
        });
        if let Some(description) = &card.intent.description {
            describe(ctx, subtrees, card.trigger, description);
        }
    }
    let on_a_trigger =
        |at: Pos2| group.is_some() && wants.drawn.iter().any(|(_, rect)| rect.contains(at));
    let interrupted = cards.iter().any(|card| dismissed.contains(&card.overlay))
        || sight.blurred
        || sight
            .pressed
            .is_some_and(|(at, root)| !in_open_card(root) && !on_a_trigger(at));
    let interruption = interrupted.then_some(Interruption::Press);
    let wanted = wanted(wants.requests);
    let timing = wanted
        .as_ref()
        .map_or(NOTHING_WANTED, |trigger| trigger.payload);
    let now = crate::intent::now(ctx);
    let drawn: Vec<Id> = wants.drawn.iter().map(|(id, _)| *id).collect();
    let (wait, before, after) = kept.viewport(viewport, |here| {
        let intent = here.hover_cards.intent_of(group);
        for gone in intent
            .open_triggers()
            .into_iter()
            .filter(|trigger| !drawn.contains(trigger))
        {
            intent.forget(gone);
        }
        let before = intent.open_triggers();
        intent.follow_focus(sight.focused);
        let wait = intent.tick(now, timing, wanted.as_ref(), interruption);
        (wait, before, intent.open_triggers())
    });
    crate::intent::repaint(ctx, wait, after != before);
    if let Some(group) = group {
        let row = RowFrame {
            cards,
            last_focused,
            before: &before,
            after: &after,
        };
        row.finish(sight, group, drawn);
    }
}

/// A row's frame, once its intent has moved.
struct RowFrame<'a> {
    cards: &'a [&'a Card],
    last_focused: Option<Id>,
    before: &'a [Id],
    after: &'a [Id],
}

impl RowFrame<'_> {
    fn widgets(&self, subtrees: &[(Id, Box<[Id]>)], trigger: Id) -> Box<[Id]> {
        let overlay = self
            .cards
            .iter()
            .find(|card| card.trigger == trigger)
            .map(|card| card.overlay);
        subtrees
            .iter()
            .find(|(node, _)| Some(*node) == overlay)
            .map(|(_, widgets)| widgets.clone())
            .unwrap_or_default()
    }

    /// Gives focus back to the trigger of a card that closed with focus in
    /// it, or on the trigger (Esc drops egui's focus first); sends it into
    /// the card Down opened; and notes what the next frame's keys need.
    fn finish(self, sight: &Sight<'_>, group: Id, triggers: Vec<Id>) {
        let Frame {
            ctx,
            kept,
            subtrees,
            ..
        } = sight.frame;
        let last_open = kept.viewport(ctx.viewport_id(), |here| {
            here.hover_cards
                .rows
                .get(&group)
                .and_then(|row| row.open.clone())
        });
        let closed = self.before.first().filter(|_| self.after.is_empty());
        if let Some(&trigger) = closed
            && sight.focused.is_none()
        {
            let was_in = self.last_focused.is_some_and(|last| {
                last == trigger
                    || last_open
                        .as_ref()
                        .is_some_and(|open| open.trigger == trigger && open.widgets.contains(&last))
            });
            if was_in {
                ctx.memory_mut(|memory| memory.request_focus(trigger));
            }
        }
        let open = self.after.first().map(|&trigger| OpenCard {
            trigger,
            widgets: self.widgets(subtrees, trigger),
            after: item_after(sight.frame.nodes, sight.frame.by_id, trigger),
        });
        let entered = kept.viewport(ctx.viewport_id(), |here| {
            here.hover_cards
                .rows
                .get(&group)
                .and_then(|row| row.entering)
        });
        let mut entering = entered.filter(|trigger| self.after.contains(trigger));
        if let Some(card) = &open
            && entering == Some(card.trigger)
            && let Some(&first) = card.widgets.first()
        {
            ctx.memory_mut(|memory| memory.request_focus(first));
            entering = None;
        }
        let focused = ctx.memory(|memory| memory.focused());
        let stray = open
            .as_ref()
            .filter(|_| sight.tabbed)
            .filter(|card| focused.is_some_and(|focused| card.widgets.contains(&focused)))
            .filter(|card| !self.last_focused.is_some_and(|last| card.around(last)))
            .and_then(|card| {
                if ctx.input(|input| input.modifiers.shift) {
                    card.widgets
                        .first()
                        .map(|&first| (first, FocusDirection::Previous))
                } else {
                    card.widgets
                        .last()
                        .map(|&last| (last, FocusDirection::Next))
                }
            });
        if stray.is_some() {
            ctx.request_repaint();
        }
        kept.viewport(ctx.viewport_id(), |here| {
            let row = here.hover_cards.rows.entry(group).or_default();
            row.stray = stray;
            row.open = open;
            row.triggers = triggers;
            row.focused = focused;
            row.entering = entering;
        });
    }
}

/// The item after `trigger` in the [`nav`](crate::NodeHandle::nav) group it
/// is an item of.
fn item_after(nodes: &Nodes, by_id: &HashMap<Id, usize>, trigger: Id) -> Option<Id> {
    let index = *by_id.get(&trigger)?;
    let group = nodes
        .ancestors(index)
        .find(|&at| nodes.get(at).is_some_and(|node| node.group().is_some()))?;
    let items = crate::roving::items_under(nodes, group);
    let at = items.iter().position(|&item| item == index)?;
    nodes.get(*items.get(at + 1)?)?.id()
}

/// Describes the widgets inside `trigger` by `description`, after what
/// their own tooltips say.
fn describe(ctx: &Context, subtrees: &[(Id, Box<[Id]>)], trigger: Id, description: &str) {
    let widgets = subtrees
        .iter()
        .find(|(node, _)| *node == trigger)
        .map(|(_, widgets)| &**widgets)
        .unwrap_or_default();
    for &widget in widgets {
        ctx.accesskit_node_builder(widget, |node| {
            let joined = match node.description() {
                Some(own) if !own.is_empty() => format!("{own} {description}"),
                _ => description.to_owned(),
            };
            node.set_description(joined);
        });
    }
}

/// A card trigger, as the tooltips see it.
struct Silenced {
    /// The trigger's node and its descendants.
    span: Range<usize>,
    /// What it is wanted for, touch or not.
    wants: [Option<Cause>; 2],
}

/// What the tooltips yield to: a card's trigger takes the tooltip slot of
/// its subtree.
pub(crate) struct Quiet<'a> {
    nodes: &'a Nodes,
    by_id: &'a HashMap<Id, usize>,
    silenced: Vec<Silenced>,
}

impl Quiet<'_> {
    /// Whether the tooltip trigger `id` is a card's trigger or inside one.
    pub(crate) fn holds(&self, id: Id) -> bool {
        self.by_id.get(&id).is_some_and(|index| {
            self.silenced
                .iter()
                .any(|silenced| silenced.span.contains(index))
        })
    }

    /// Whether the tooltip trigger `id`, wanted for `cause`, is around a card
    /// trigger wanted for the same cause: the card trigger is deeper, so it
    /// wins, and shows no tooltip.
    pub(crate) fn outranks(&self, id: Id, cause: Cause) -> bool {
        let Some(&index) = self.by_id.get(&id) else {
            return false;
        };
        let around = self.nodes.span(index);
        self.silenced.iter().any(|silenced| {
            around.contains(&silenced.span.start) && silenced.wants.contains(&Some(cause))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    /// A row of `a` and `b`, with `a`'s card open holding `one` and `two`.
    fn row() -> Row {
        Row {
            triggers: vec![id("a"), id("b")],
            open: Some(OpenCard {
                trigger: id("a"),
                widgets: Box::new([id("one"), id("two")]),
                after: Some(id("b")),
            }),
            ..Row::default()
        }
    }

    #[test]
    fn tab_from_the_open_trigger_goes_into_its_card_and_from_its_last_widget_past_the_trigger() {
        let row = row();
        assert_eq!(row.route(id("a"), RowKey::Tab), Some(Route::To(id("one"))));
        assert_eq!(
            row.route(id("one"), RowKey::Tab),
            None,
            "egui's within the card"
        );
        assert_eq!(
            row.route(id("two"), RowKey::Tab),
            Some(Route::Past(id("a")))
        );
        assert_eq!(
            row.route(id("b"), RowKey::Tab),
            None,
            "a closed trigger's is egui's"
        );
    }

    #[test]
    fn shift_tab_from_the_first_widget_goes_back_to_the_trigger() {
        let row = row();
        assert_eq!(
            row.route(id("one"), RowKey::ShiftTab),
            Some(Route::To(id("a")))
        );
        assert_eq!(row.route(id("two"), RowKey::ShiftTab), None);
    }

    #[test]
    fn shift_tab_from_the_item_after_the_trigger_goes_to_the_cards_last_widget() {
        let row = row();
        assert_eq!(
            row.route(id("b"), RowKey::ShiftTab),
            Some(Route::To(id("two")))
        );
        assert_eq!(row.route(id("elsewhere"), RowKey::ShiftTab), None);
    }

    #[test]
    fn down_on_any_trigger_enters_it() {
        let row = row();
        assert_eq!(
            row.route(id("b"), RowKey::Down),
            Some(Route::Enter(id("b")))
        );
        assert_eq!(row.route(id("one"), RowKey::Down), None, "the card's own");
    }

    #[test]
    fn with_nothing_open_tab_is_egui_s() {
        let row = Row {
            open: None,
            ..row()
        };
        assert_eq!(row.route(id("a"), RowKey::Tab), None);
    }
}
