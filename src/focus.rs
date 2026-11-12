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

use crate::dismiss::FocusAt;
use crate::stack::Open;

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

/// Whether a field takes focus when it appears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Autofocus {
    #[default]
    Never,
    OnAppear,
}

impl Autofocus {
    /// Requests focus for `response`'s widget if this is the first frame it
    /// is drawn after a frame in which it was not, or ever.
    pub(crate) fn apply(self, ctx: &Context, response: &egui::Response) {
        if self == Self::Never {
            return;
        }
        let seen = response.id.with("eggshell-seen");
        let now = ctx.cumulative_frame_nr();
        let last = ctx.data_mut(|data| data.get_temp::<u64>(seen));
        ctx.data_mut(|data| data.insert_temp(seen, now));
        if last.is_none_or(|last| last + 1 < now) {
            response.request_focus();
        }
    }
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

/// The widgets to give focus back to once the pointer is idle.
#[derive(Clone, Default)]
struct Returning(Vec<Id>);

fn returning_id() -> Id {
    crate::key::root_id().with("focus-returning")
}

/// Gives focus back to the widgets the `closed` overlays took it from, if
/// focus was in one of them or nowhere: Esc has already taken it away
/// (egui drops focus on Esc before any other code runs), and a click
/// somewhere that cannot take focus did the same. If focus is on another
/// widget, the user put it there. A widget that is no longer on screen cannot
/// take it.
///
/// `closed` is bottom first. The outermost overlay's widget is the one that
/// ends up with focus.
///
/// A widget under a modal waits: egui, which learns of a closed modal a frame
/// late, would drop the focus it was given.
///
/// While a pointer button is down, or was released this frame, focus waits:
/// egui drops the focus of a widget when a click lands elsewhere, which would
/// undo it. It is then given back if it is nowhere.
pub(crate) fn restore(ctx: &Context, closed: &[Open]) {
    let mut returning = ctx
        .data(|data| data.get_temp::<Returning>(returning_id()))
        .unwrap_or_default();
    let stranded = !closed.is_empty()
        && (ctx.memory(|memory| memory.focused()).is_none()
            || closed.iter().any(|open| focus_within(ctx, open.id)));
    if stranded {
        returning.0 = closed
            .iter()
            .rev()
            .filter_map(|open| open.restore)
            .collect();
    }
    let pointer_idle =
        ctx.input(|input| !input.pointer.any_down() && !input.pointer.any_released());
    let nowhere = ctx.memory(|memory| memory.focused()).is_none();
    if pointer_idle && (stranded || nowhere) {
        for target in std::mem::take(&mut returning.0) {
            let Some(response) = ctx.read_response(target) else {
                continue;
            };
            if ctx.memory(|memory| memory.is_above_modal_layer(response.layer_id)) {
                ctx.memory_mut(|memory| memory.request_focus(target));
            } else {
                returning.0.push(target);
            }
        }
    }
    ctx.data_mut(|data| data.insert_temp(returning_id(), returning));
}

/// Where focus is relative to the overlay `id`: in it, in the overlays
/// nested in it (`nested`), or on its `anchor`. Focus on a widget that was
/// not drawn this pass, such as one of an overlay that just closed, is
/// nowhere: egui lets go of it a pass later.
pub(crate) fn at(ctx: &Context, id: Id, nested: &[Id], anchor: Option<Id>) -> FocusAt {
    let Some(focused) = ctx.memory(|memory| memory.focused()) else {
        return FocusAt::Nowhere;
    };
    let drawn = ctx.viewport(|viewport| viewport.this_pass.widgets.get(focused).is_some());
    if !drawn || !ctx.input(|input| input.focused) {
        return FocusAt::Nowhere;
    }
    let inside = std::iter::once(id)
        .chain(nested.iter().copied())
        .chain(anchor)
        .any(|node| focus_within(ctx, node));
    if inside {
        FocusAt::Inside
    } else {
        FocusAt::Elsewhere
    }
}
