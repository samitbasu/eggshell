//! The stacking of overlays: which one is above which.
//!
//! egui stacks layers of one [`Order`](egui::Order) in the order it first saw
//! them, and never forgets a layer, so neither the order a frame declares its
//! overlays in nor a reopened overlay's place in it is honoured. The library
//! keeps its own stack, the open overlays in the order they were opened, and
//! each frame repairs egui's order where it differs.

use std::collections::HashSet;

use egui::{Context, Id, Pos2};

use crate::overlay::Band;

/// An open overlay.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Open {
    pub(crate) id: Id,
    pub(crate) band: Band,
    /// Where the pointer was when the overlay opened, for
    /// [`Anchor::Pointer`](crate::overlay::Anchor::Pointer).
    pub(crate) pointer: Option<Pos2>,
}

/// The open overlays in the order they were opened, bottom first. Overlays
/// opened in one frame are in declaration order, and an overlay declared in
/// another's body is after it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stack(Vec<Open>);

fn key() -> Id {
    crate::key::root_id().with("overlay-stack")
}

impl Stack {
    pub(crate) fn load(ctx: &Context) -> Self {
        ctx.data(|data| data.get_temp(key())).unwrap_or_default()
    }

    pub(crate) fn store(self, ctx: &Context) {
        ctx.data_mut(|data| data.insert_temp(key(), self));
    }

    /// The entry of the overlay `id`, appended at the top if it was not open.
    pub(crate) fn enter(&mut self, id: Id, band: Band) -> &mut Open {
        let at = self
            .0
            .iter()
            .position(|open| open.id == id)
            .unwrap_or_else(|| {
                self.0.push(Open {
                    id,
                    band,
                    pointer: None,
                });
                self.0.len() - 1
            });
        &mut self.0[at]
    }

    /// Forgets the overlays that were not declared this frame.
    pub(crate) fn keep_only(&mut self, present: &HashSet<Id>) {
        self.0.retain(|open| present.contains(&open.id));
    }

    pub(crate) fn index_of(&self, id: Id) -> Option<usize> {
        self.0.iter().position(|open| open.id == id)
    }
}
