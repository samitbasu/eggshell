//! The stacking of overlays: which one is above which.
//!
//! egui stacks layers of one [`Order`](egui::Order) in the order it first saw
//! them, and never forgets a layer, so neither the order a frame declares its
//! overlays in nor a reopened overlay's place in it is honoured. The library
//! keeps its own stack, the open overlays in the order they were opened, and
//! each frame repairs egui's order where it differs: see [`repair`].

use std::collections::HashSet;

use egui::{Context, Id, LayerId, Pos2};

use crate::dismiss::{FocusAt, Opening};
use crate::overlay::Band;

/// An open overlay.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Open {
    pub(crate) id: Id,
    pub(crate) band: Band,
    /// Where the pointer was when the overlay opened, for
    /// [`Anchor::Pointer`](crate::overlay::Anchor::Pointer).
    pub(crate) pointer: Option<Pos2>,
    /// The widget that had focus when the overlay opened, which gets it back
    /// when the overlay closes.
    pub(crate) restore: Option<Id>,
    /// Whether focus has been moved into the overlay yet.
    pub(crate) entered: Entered,
    /// Where focus was at the end of the last frame the overlay was drawn.
    pub(crate) focus: FocusAt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entered {
    Not,
    Yet,
}

/// The open overlays in the order they were opened, bottom first. Overlays
/// opened in one frame are in declaration order, and an overlay declared in
/// another's body is after it.
///
/// One per [`LayoutState`](crate::LayoutState): a context that lays out
/// several frames each frame (a header and a body) keeps each one's overlays
/// apart, so that one frame never closes the overlays of another.
#[derive(Clone, Debug)]
pub(crate) struct Stack {
    scope: Id,
    opens: Vec<Open>,
}

impl Stack {
    pub(crate) fn load(ctx: &Context, scope: Id) -> Self {
        ctx.data(|data| data.get_temp(scope)).unwrap_or(Self {
            scope,
            opens: Vec::new(),
        })
    }

    pub(crate) fn store(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(self.scope, self));
    }

    /// The entry of the overlay `id`, appended at the top if it was not open.
    pub(crate) fn enter(&mut self, id: Id, band: Band) -> (&mut Open, Opening) {
        let found = self.opens.iter().position(|open| open.id == id);
        let opening = if found.is_some() {
            Opening::Open
        } else {
            Opening::Opened
        };
        let at = found.unwrap_or_else(|| {
            self.opens.push(Open {
                id,
                band,
                pointer: None,
                restore: None,
                entered: Entered::Not,
                focus: FocusAt::Nowhere,
            });
            self.opens.len() - 1
        });
        (&mut self.opens[at], opening)
    }

    /// Forgets the overlays that were not declared this frame, and returns
    /// them.
    pub(crate) fn keep_only(&mut self, present: &HashSet<Id>) -> Vec<Open> {
        let (kept, closed) = std::mem::take(&mut self.opens)
            .into_iter()
            .partition(|open| present.contains(&open.id));
        self.opens = kept;
        closed
    }

    pub(crate) fn get_mut(&mut self, id: Id) -> Option<&mut Open> {
        self.opens.iter_mut().find(|open| open.id == id)
    }

    pub(crate) fn index_of(&self, id: Id) -> Option<usize> {
        self.opens.iter().position(|open| open.id == id)
    }
}

/// How many leading entries of `ranks` rise: the length of the longest prefix
/// whose ranks are in increasing order.
fn intact_prefix(ranks: &[usize]) -> usize {
    ranks
        .windows(2)
        .position(|pair| pair[1] < pair[0])
        .map_or(ranks.len(), |at| at + 1)
}

/// An overlay shown this frame.
pub(crate) struct Shown {
    pub(crate) id: Id,
    pub(crate) band: Band,
}

/// Flags the layers of `shown` (the open overlays that were drawn
/// this frame, in opening order, bottom first) that egui has in the wrong
/// place.
///
/// Per band, let `k` be the length of the longest prefix of the stack that
/// egui already has in the same relative order. egui's end-of-frame sort is
/// stable on `(order, flagged)`, so flagging the layers from `k` on puts them
/// above everything else, in the order egui has them in. The layer at `k` is
/// then above all of the prefix, so the correct prefix grows by at least one
/// layer a frame: the repair converges and never oscillates. A new overlay
/// needs no flag and one reopened after another, one.
///
/// `pressed` is the overlay a pointer press landed on this frame: egui raises
/// that layer by itself, so it and the layers above it are flagged too, which
/// keeps their order (a press on a menu must not bring it over its submenu).
pub(crate) fn repair(ctx: &Context, shown: &[Shown], pressed: Option<Id>) {
    let order: Vec<LayerId> = ctx.memory(|memory| memory.layer_ids().collect());
    for band in [Band::Floating, Band::Popup, Band::Tooltip] {
        let layers: Vec<(Id, LayerId)> = shown
            .iter()
            .filter(|shown| shown.band == band)
            .map(|shown| (shown.id, band.layer(shown.id)))
            .collect();
        let ranks: Vec<usize> = layers
            .iter()
            .map(|(_, layer)| {
                order
                    .iter()
                    .position(|candidate| candidate == layer)
                    .unwrap_or(usize::MAX)
            })
            .collect();
        let mut intact = intact_prefix(&ranks);
        if let Some(pressed) = layers.iter().position(|(id, _)| Some(*id) == pressed) {
            intact = intact.min(pressed);
        }
        for (_, layer) in &layers[intact..] {
            ctx.move_to_top(*layer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_intact_prefix_is_the_leading_run_of_rising_ranks() {
        assert_eq!(intact_prefix(&[]), 0);
        assert_eq!(intact_prefix(&[7]), 1);
        assert_eq!(intact_prefix(&[1, 4, 9]), 3);
        assert_eq!(intact_prefix(&[1, 4, 3, 9]), 2);
        assert_eq!(intact_prefix(&[5, 1]), 1);
    }

    /// egui's end-of-frame sort, on a list of layer names.
    fn sorted(order: &mut [u32], flagged: &[u32]) {
        order.sort_by_key(|layer| flagged.contains(layer));
    }

    /// One frame of the repair on a model of egui's order: `stack` is the
    /// library's stack, `order` egui's, `raised` the layer egui flags itself
    /// for a press, `pressed` the library's judgement of the same press.
    fn frame(stack: &[u32], order: &mut [u32], raised: Option<u32>, pressed: Option<usize>) {
        let ranks: Vec<usize> = stack
            .iter()
            .map(|layer| order.iter().position(|l| l == layer).expect("shown"))
            .collect();
        let mut intact = intact_prefix(&ranks);
        if let Some(pressed) = pressed {
            intact = intact.min(pressed);
        }
        let mut flagged: Vec<u32> = stack[intact..].to_vec();
        flagged.extend(raised);
        sorted(order, &flagged);
    }

    fn follows_the_stack(stack: &[u32], order: &[u32]) -> bool {
        let seen: Vec<u32> = order
            .iter()
            .copied()
            .filter(|layer| stack.contains(layer))
            .collect();
        seen == stack
    }

    #[test]
    fn three_stale_layers_reopened_in_reverse_take_two_frames() {
        let stack = [3, 2, 1];
        let mut order = vec![1, 2, 3];
        frame(&stack, &mut order, None, None);
        assert!(!follows_the_stack(&stack, &order));
        frame(&stack, &mut order, None, None);
        assert!(follows_the_stack(&stack, &order));
    }

    #[test]
    fn a_press_on_a_parent_keeps_it_under_its_submenu_only_when_judged() {
        let stack = [1, 2];
        let mut unjudged = vec![1, 2];
        frame(&stack, &mut unjudged, Some(1), None);
        assert!(
            !follows_the_stack(&stack, &unjudged),
            "egui alone raises the parent over its submenu"
        );
        let mut judged = vec![1, 2];
        frame(&stack, &mut judged, Some(1), Some(0));
        assert!(follows_the_stack(&stack, &judged));
    }
}
