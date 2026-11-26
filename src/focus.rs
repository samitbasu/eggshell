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

use egui::{Context, Id, LayerId, SurrenderFocusOn};

use crate::dismiss::FocusAt;
use crate::kept::Handle;
use crate::pipeline::OpenFocus;
use crate::shell::Shell;
use crate::stack::Open;

type Subtrees = HashMap<Id, Box<[Id]>>;

/// What a layout remembers of the focus of one viewport.
#[derive(Default)]
pub(crate) struct Held {
    /// The focusable widgets each identified node drawn in the viewport
    /// registered.
    records: Records,
    returning: Returning,
    /// The multi-line text field that has focus, whose Return is its own.
    pub(crate) multiline: Option<Id>,
    /// The overlays of this viewport that closed since its last pass, when
    /// it is not the one layout runs in: their focus goes back in its pass.
    pub(crate) closed: Vec<Open>,
}

#[derive(Default)]
struct Records {
    pass: u64,
    previous: Arc<Subtrees>,
    current: Subtrees,
}

/// Whether `id` has keyboard focus, as [`egui::Response::has_focus`] decides.
pub(crate) fn holds_focus(ctx: &Context, id: Id) -> bool {
    ctx.input(|input| input.focused) && ctx.memory(|memory| memory.has_focus(id))
}

/// When a control asks for focus, and what a field then selects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Autofocus {
    #[default]
    Never,
    OnAppear(Select),
    OnChange(Token, Select),
    /// As the overlay it is in opens: the node is marked, and the overlay
    /// gives it focus.
    OnOverlayOpen(Select),
    /// As the overlay it is in opens, unless another node is marked
    /// [`OnOverlayOpen`](Self::OnOverlayOpen).
    #[cfg_attr(not(feature = "dialog"), expect(dead_code))]
    OnOverlayOpenByDefault,
    /// On every frame it is set.
    Now(Select),
}

/// TODO(prose): what a field selects when the application focuses it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Select {
    /// TODO(prose): the whole text, so typing replaces it.
    All,
    /// TODO(prose): nothing; the caret stays where the field keeps it.
    #[default]
    None,
}

/// What an application hands a field to ask for its focus: hashed, so a field
/// holds one without holding the application's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token(u64);

impl Token {
    pub(crate) fn of(token: &impl std::hash::Hash) -> Self {
        use std::hash::Hasher as _;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl Autofocus {
    /// Whether the node holding the control's widget is the one its overlay
    /// opens on.
    pub(crate) fn open_focus(self) -> OpenFocus {
        match self {
            Self::OnOverlayOpen(_) => OpenFocus::Here,
            Self::OnOverlayOpenByDefault => OpenFocus::Fallback,
            _ => OpenFocus::Elsewhere,
        }
    }

    /// The same request, selecting as `select` says when it takes focus.
    pub(crate) fn selecting(self, select: Select) -> Self {
        match self {
            Self::OnAppear(_) => Self::OnAppear(select),
            Self::OnChange(token, _) => Self::OnChange(token, select),
            Self::OnOverlayOpen(_) => Self::OnOverlayOpen(select),
            Self::Now(_) => Self::Now(select),
            Self::Never | Self::OnOverlayOpenByDefault => self,
        }
    }

    /// Requests focus for `response`'s widget if this is the first frame it
    /// is drawn after a frame in which it was not, or ever, if its token
    /// changed, or if it asks now. Returns what to select when it did, or
    /// when its overlay opened on it.
    pub(crate) fn apply(self, ctx: &Context, response: &egui::Response) -> Option<Select> {
        let (token, select) = match self {
            Self::Never | Self::OnOverlayOpenByDefault => return None,
            Self::OnOverlayOpen(select) => {
                return took_opening_focus(ctx, response).then_some(select);
            }
            Self::Now(select) => {
                response.request_focus();
                return Some(select);
            }
            Self::OnAppear(select) => (None, select),
            Self::OnChange(token, select) => (Some(token), select),
        };
        let asks = match token {
            Some(token) => {
                let slot = response.id.with("eggshell-token");
                let last = ctx.data_mut(|data| data.get_temp::<Token>(slot));
                ctx.data_mut(|data| data.insert_temp(slot, token));
                last.is_some_and(|last| last != token)
            }
            None => {
                let seen = response.id.with("eggshell-seen");
                let now = ctx.cumulative_frame_nr();
                let last = ctx.data_mut(|data| data.get_temp::<u64>(seen));
                ctx.data_mut(|data| data.insert_temp(seen, now));
                last.is_none_or(|last| last.saturating_add(1) < now)
            }
        };
        asks.then(|| {
            response.request_focus();
            select
        })
    }
}

fn opening_focus_slot(widget: Id) -> Id {
    widget.with("eggshell-opening-focus")
}

/// Notes that an opening overlay gave focus to `widget`, which takes the
/// note the first frame it is drawn with focus.
pub(crate) fn note_opening_focus(ctx: &Context, widget: Id) {
    ctx.data_mut(|data| data.insert_temp(opening_focus_slot(widget), ()));
}

/// Whether `response`'s widget has focus that its overlay gave it as it
/// opened, and has not yet taken.
fn took_opening_focus(ctx: &Context, response: &egui::Response) -> bool {
    let slot = opening_focus_slot(response.id);
    let noted = ctx.data_mut(|data| data.remove_temp::<()>(slot)).is_some();
    noted && response.has_focus()
}

impl Shell {
    /// Whether `node`, or a focusable widget registered inside it, has
    /// keyboard focus.
    ///
    /// Which widgets are inside `node` is what this layout's subtree
    /// registered in the previous frame. Works at build time, where the node
    /// does not exist yet, and at draw time.
    pub fn focus_within(&self, node: Id) -> bool {
        let ctx = &self.ctx;
        if holds_focus(ctx, node) {
            return true;
        }
        let ids = self.kept.viewport(ctx.viewport_id(), |here| {
            let records = &here.focus.records;
            let known = if records.pass == ctx.cumulative_pass_nr() {
                &*records.previous
            } else {
                &records.current
            };
            known.get(&node).cloned()
        });
        ids.is_some_and(|ids| ids.iter().any(|&id| holds_focus(ctx, id)))
    }
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

/// Adds this pass's subtrees to the layout's record.
pub(crate) fn store(
    ctx: &Context,
    kept: &Handle,
    subtrees: impl IntoIterator<Item = (Id, Box<[Id]>)>,
) {
    let pass = ctx.cumulative_pass_nr();
    kept.viewport(ctx.viewport_id(), |here| {
        let records = &mut here.focus.records;
        if records.pass != pass {
            records.previous = Arc::new(std::mem::take(&mut records.current));
            records.pass = pass;
        }
        records
            .current
            .extend(subtrees.into_iter().filter(|(_, ids)| !ids.is_empty()));
    });
}

/// The widget that had focus as a pass began.
#[derive(Clone, Copy, Default)]
struct FocusedAtStart {
    pass: u64,
    focused: Option<Id>,
}

fn focused_at_start_id(ctx: &Context) -> Id {
    crate::key::viewport_id(ctx, "focused-at-start")
}

/// A widget held by the pointer that lost focus to another widget lets go of
/// the press: egui's text field takes focus back in every pass it is
/// dragged, which would undo a Tab, or pull focus out of the overlay that the
/// keyboard opened. Run once a pass, before the widgets are drawn.
pub(crate) fn release_press_on_blur(ctx: &Context) {
    let pass = ctx.cumulative_pass_nr();
    let focused = ctx.memory(|memory| memory.focused());
    let key = focused_at_start_id(ctx);
    let last = ctx.data_mut(|data| {
        let last = data.get_temp::<FocusedAtStart>(key).unwrap_or_default();
        if last.pass != pass {
            data.insert_temp(key, FocusedAtStart { pass, focused });
        }
        last
    });
    if last.pass == pass || !ctx.input(|input| input.pointer.any_down()) {
        return;
    }
    if let Some(left) = last.focused
        && focused.is_some_and(|now| now != left)
        && ctx
            .read_response(left)
            .is_some_and(|response| response.is_pointer_button_down_on())
    {
        ctx.stop_dragging();
    }
}

/// Where the pointer is, as far as the focus of the widget under an overlay
/// is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerOn {
    /// On an overlay that leaves focus where it is ([`Focus::Stay`]).
    StayingOverlay,
    Elsewhere,
}

/// Keeps egui from taking the focus of a text field or trigger away for a
/// press on its own suggestion list or options, as `preventDefault` on
/// `mousedown` does on the web. Held until dropped.
pub(crate) struct AnchorFocusHeld {
    ctx: Context,
    surrender: SurrenderFocusOn,
}

impl AnchorFocusHeld {
    /// Holds the focus for as long as the result lives, if the pointer is on
    /// a staying overlay. egui surrenders the focus of a widget when a click
    /// lands elsewhere, as the widget registers.
    pub(crate) fn new(ctx: &Context, pointer: PointerOn) -> Option<Self> {
        (pointer == PointerOn::StayingOverlay).then(|| Self {
            ctx: ctx.clone(),
            surrender: ctx.options_mut(|options| {
                std::mem::replace(
                    &mut options.input_options.surrender_focus_on,
                    SurrenderFocusOn::Never,
                )
            }),
        })
    }
}

impl Drop for AnchorFocusHeld {
    fn drop(&mut self) {
        self.ctx
            .options_mut(|options| options.input_options.surrender_focus_on = self.surrender);
    }
}

/// The widgets to give focus back to once the pointer is idle.
#[derive(Default)]
struct Returning(Vec<Id>);

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
pub(crate) fn restore(ctx: &Context, kept: &Handle, closed: &[Open]) {
    let shell = Shell::new(ctx, kept);
    let mut returning = kept.viewport(ctx.viewport_id(), |here| {
        std::mem::take(&mut here.focus.returning)
    });
    let stranded = !closed.is_empty()
        && (ctx.memory(|memory| memory.focused()).is_none()
            || closed.iter().any(|open| shell.focus_within(open.id)));
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
                kept.viewport(ctx.viewport_id(), |here| {
                    here.tooltips.focus_restored(target);
                    here.hover_cards.intent.focus_restored(target);
                });
            } else {
                returning.0.push(target);
            }
        }
    }
    kept.viewport(ctx.viewport_id(), |here| here.focus.returning = returning);
}

/// Where focus is relative to the overlay `id`: in it, in the overlays
/// nested in it (`nested`), or on its `anchor`. Focus on a widget that was
/// not drawn this pass, such as one of an overlay that just closed, is
/// nowhere: egui lets go of it a pass later.
pub(crate) fn at(shell: &Shell, id: Id, nested: &[Id], anchor: Option<Id>) -> FocusAt {
    let ctx = &shell.ctx;
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
        .any(|node| shell.focus_within(node));
    if inside {
        FocusAt::Inside
    } else {
        FocusAt::Elsewhere
    }
}
