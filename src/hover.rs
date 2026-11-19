//! Hover cards: overlays that hover intent opens and closes
//! ([`hover_anchored`](crate::overlay::OverlayChain::hover_anchored)).
//!
//! The chain declares the card at build and reads whether it is open from
//! [`Cards`]. At the end of the draw, once dismissal is judged, [`end`]
//! resolves the declared cards' triggers against the pointer and the focus
//! and moves the intent. The open card's box, and the overlays nested in it,
//! count as its trigger, so the pointer may cross into the card.

use std::collections::HashMap;
use std::ops::Range;

use egui::{Context, Id, Pos2};

use crate::builder::Builder;
use crate::intent::{Cause, FocusOpens, Intent, Interruption, Skip, Timing, Trigger, wanted};
use crate::kept::Handle;
use crate::nodes::Nodes;
use crate::overlay::HoverIntent;
use crate::pipeline::PlacedNode;
use crate::presence::Shown;
use crate::roots::{Here, Home, PlacedRoot, ancestors, root_at, root_of};

/// A card declared this frame.
struct Card {
    trigger: Id,
    overlay: Id,
    intent: HoverIntent,
}

/// The hover cards of a viewport: their intent, and what this frame declared.
#[derive(Default)]
pub(crate) struct Cards {
    pub(crate) intent: Intent<Timing>,
    declared: Vec<Card>,
    /// Dismissal closed a card this frame.
    dismissed: bool,
}

impl Cards {
    /// The rules of `dismiss.rs` closed a card: Esc, a press outside, focus
    /// leaving or the window losing focus.
    pub(crate) fn dismissed(&mut self) {
        self.dismissed = true;
    }
}

fn timing(intent: &HoverIntent) -> Timing {
    Timing {
        open: intent.open,
        close: intent.close,
        skip: Skip::Never,
        focus: FocusOpens::AfterDelay,
    }
}

/// The timing a tick takes when no trigger wants a card: nothing opens by
/// it, and an open card lingers for the close delay it opened with.
const NOTHING_WANTED: Timing = Timing {
    open: core::time::Duration::ZERO,
    close: core::time::Duration::ZERO,
    skip: Skip::Never,
    focus: FocusOpens::AfterDelay,
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
        cards.declared.retain(|card| card.trigger != trigger);
        cards.declared.push(Card {
            trigger,
            overlay,
            intent,
        });
        cards.intent.open_triggers().contains(&trigger)
    });
    Some(if open { Shown::Open } else { Shown::Closed })
}

/// The ids of the nodes in hover cards' boxes: Tab never stops in a card.
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
                .is_some_and(|overlay| overlay.behavior.hover.is_some())
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

/// Moves the cards' intent to this frame, after dismissal is judged, and
/// describes each trigger by its card. Returns what the tooltips yield to.
pub(crate) fn end(frame: Frame<'_>) -> Quiet<'_> {
    let Frame {
        ctx,
        kept,
        nodes,
        by_id,
        roots,
        here,
        subtrees,
    } = frame;
    let viewport = ctx.viewport_id();
    let (declared, dismissed, open) = kept.viewport(viewport, |here| {
        let cards = &mut here.hover_cards;
        (
            std::mem::take(&mut cards.declared),
            std::mem::take(&mut cards.dismissed),
            cards.intent.open_triggers(),
        )
    });
    let root_under = |at: Pos2| root_at(roots, nodes, at, here);
    let pointer = mouse_at(ctx);
    let under = pointer.map(root_under);
    let held = held_from(ctx).map(root_under);
    let pressed = crate::overlay_input::press_origin(ctx).map(root_under);
    let keyboard = crate::overlay_input::keyboard_was_last(ctx);
    let shell = crate::Shell::new(ctx, kept);
    let open_roots: Vec<usize> = declared
        .iter()
        .filter(|card| open.contains(&card.trigger))
        .filter_map(|card| {
            roots.iter().position(|root| {
                root.overlay()
                    .is_some_and(|overlay| overlay.id == card.overlay)
            })
        })
        .collect();
    let in_open_card = |root: usize| {
        open_roots
            .iter()
            .any(|&card| root == card || ancestors(roots, root).any(|at| at == card))
    };
    let mut requests = Vec::new();
    let mut silenced = Vec::new();
    let mut drawn = Vec::new();
    for card in &declared {
        let Some((index, node)) = by_id
            .get(&card.trigger)
            .and_then(|&index| Some((index, nodes.node(index)?)))
        else {
            continue;
        };
        let Some(trigger_root) =
            root_of(roots, index).filter(|&root| Home::of(roots, root) == here.home)
        else {
            continue;
        };
        drawn.push(card.trigger);
        let over_trigger =
            pointer.is_some_and(|at| node.rect().contains(at)) && under == Some(trigger_root);
        let is_open = open.contains(&card.trigger);
        let over_card =
            is_open && (under.is_some_and(in_open_card) || held.is_some_and(in_open_card));
        let focused = shell.focus_within(card.trigger);
        let causes = [
            (keyboard && focused).then_some(Cause::Focus),
            (over_trigger || over_card).then_some(Cause::Pointer),
        ];
        for cause in causes.into_iter().flatten() {
            requests.push(Trigger {
                id: card.trigger,
                payload: timing(&card.intent),
                cause,
                rect: node.rect(),
            });
        }
        silenced.push(Silenced {
            span: nodes.span(index),
            wants: [
                focused.then_some(Cause::Focus),
                over_trigger.then_some(Cause::Pointer),
            ],
        });
        if let Some(description) = &card.intent.description {
            describe(ctx, subtrees, card.trigger, description);
        }
    }
    let blurred = ctx.input(|input| crate::window::blurred(&input.events));
    let interruption = (dismissed || blurred || pressed.is_some_and(|root| !in_open_card(root)))
        .then_some(Interruption::Press);
    let wanted = wanted(requests);
    let timing = wanted
        .as_ref()
        .map_or(NOTHING_WANTED, |trigger| trigger.payload);
    let now = crate::intent::now(ctx);
    let focused = ctx.memory(|memory| memory.focused());
    let (wait, changed) = kept.viewport(viewport, |here| {
        let intent = &mut here.hover_cards.intent;
        for gone in intent
            .open_triggers()
            .into_iter()
            .filter(|trigger| !drawn.contains(trigger))
        {
            intent.forget(gone);
        }
        let before = intent.open_triggers();
        intent.follow_focus(focused);
        let wait = intent.tick(now, timing, wanted.as_ref(), interruption);
        (wait, intent.open_triggers() != before)
    });
    crate::intent::repaint(ctx, wait, changed);
    Quiet {
        nodes,
        by_id,
        silenced,
    }
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
