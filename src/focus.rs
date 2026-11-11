//! Keyboard focus. egui owns it; this module reads it and keeps the one
//! thing egui lacks: which widgets sit inside which node.
//!
//! egui ids have no hierarchy, so while a frame is drawn the library notes
//! the focusable widgets registered inside each identified node's subtree.
//! The note outlives the frame in the egui context, so the next frame's
//! build (and the next frame's draw, before a node's widgets exist again)
//! can ask [`focus_within`]. Only membership lags a frame, never focus: the
//! focused id is current, the widgets it is looked up among are last frame's.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Context, Id, LayerId};

type Subtrees = HashMap<Id, Box<[Id]>>;

#[derive(Clone, Default)]
struct Records {
    pass: u64,
    previous: Arc<Subtrees>,
    current: Subtrees,
}

fn records_id() -> Id {
    crate::key::root_id().with("focus-records")
}

/// Whether `id` has keyboard focus, as [`egui::Response::has_focus`] decides.
pub(crate) fn holds_focus(ctx: &Context, id: Id) -> bool {
    ctx.input(|input| input.focused) && ctx.memory(|memory| memory.has_focus(id))
}

/// Whether `node`, or a focusable widget registered inside it, has keyboard
/// focus.
///
/// Which widgets are inside `node` is what its subtree registered in the
/// previous frame. Works at build time, where the node does not exist yet,
/// and at draw time.
pub fn focus_within(ctx: &Context, node: Id) -> bool {
    if holds_focus(ctx, node) {
        return true;
    }
    let records: Records = ctx
        .data(|data| data.get_temp(records_id()))
        .unwrap_or_default();
    let known = if records.pass == ctx.cumulative_pass_nr() {
        &*records.previous
    } else {
        &records.current
    };
    known
        .get(&node)
        .is_some_and(|ids| ids.iter().any(|&id| holds_focus(ctx, id)))
}

/// How many widgets `layer` has registered so far this pass.
pub(crate) fn registered(ctx: &Context, layer: LayerId) -> usize {
    ctx.viewport(|viewport| {
        viewport
            .this_pass
            .widgets
            .layers()
            .find(|(candidate, _)| **candidate == layer)
            .map_or(0, |(_, widgets)| widgets.len())
    })
}

/// The focusable, enabled widgets `layer` registered after the first `from`.
pub(crate) fn focusable_since(ctx: &Context, layer: LayerId, from: usize) -> Box<[Id]> {
    ctx.viewport(|viewport| {
        viewport
            .this_pass
            .widgets
            .layers()
            .find(|(candidate, _)| **candidate == layer)
            .map(|(_, widgets)| widgets.get(from..).unwrap_or_default())
            .unwrap_or_default()
            .iter()
            .filter(|widget| widget.enabled && widget.sense.is_focusable())
            .map(|widget| widget.id)
            .collect()
    })
}

/// Adds this pass's subtrees to the context's record.
pub(crate) fn store(ctx: &Context, subtrees: impl IntoIterator<Item = (Id, Box<[Id]>)>) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let records = data.get_temp_mut_or_default::<Records>(records_id());
        if records.pass != pass {
            records.previous = Arc::new(std::mem::take(&mut records.current));
            records.pass = pass;
        }
        records
            .current
            .extend(subtrees.into_iter().filter(|(_, ids)| !ids.is_empty()));
    });
}
