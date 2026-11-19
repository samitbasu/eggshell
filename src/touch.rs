//! The first touch of a session.
//!
//! egui hit-tests a press against the widgets of the pass before, and a
//! scroll area senses a drag on touch only once a touch event has arrived. A
//! first press therefore lands before the drag sense exists, and its swipe
//! scrolls nothing. The plugin holds the pointer events of that first batch
//! for one pass, so that the pass in which the press is judged has seen the
//! touch.

use crate::Shell;
use egui::{Context, Event, Id, Plugin, Pos2, RawInput, Ui, Vec2};

#[derive(Default)]
struct HoldFirstTouch {
    held: Vec<Event>,
}

/// An event that moves the pointer or presses it.
fn is_pointer(event: &Event) -> bool {
    matches!(
        event,
        Event::PointerButton { .. } | Event::PointerMoved(_) | Event::PointerGone
    )
}

impl Plugin for HoldFirstTouch {
    fn debug_name(&self) -> &'static str {
        "eggshell-ui hold first touch"
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        if !self.held.is_empty() {
            let mut events = std::mem::take(&mut self.held);
            events.append(&mut input.events);
            input.events = events;
            return;
        }
        let first_touch = !ctx.input(|input| input.has_touch_screen())
            && input
                .events
                .iter()
                .any(|event| matches!(event, Event::Touch { .. }));
        if first_touch {
            let (pointer, rest) = std::mem::take(&mut input.events)
                .into_iter()
                .partition(is_pointer);
            self.held = pointer;
            input.events = rest;
            ctx.request_repaint();
        }
    }
}

/// Holds the pointer events of the first touch of a session by one pass.
pub(crate) fn register(ctx: &Context) {
    if ctx.plugin_opt::<HoldFirstTouch>().is_none() {
        ctx.add_plugin(HoldFirstTouch::default());
    }
}

/// How far a touch must travel before its direction is read: egui's own
/// distance past which a press is no longer a click.
const SLOP: f32 = 6.0;

/// What a touch that starts on a control with one axis does with the drag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Reading {
    /// The control drags: a mouse, or a touch that went along its axis.
    Along,
    /// A touch that has not yet gone far enough to tell.
    Pending,
    /// A touch that went across the axis: the container scrolls by this much
    /// more, and the control does nothing.
    Across(Vec2),
    /// A touch that lifted where it began.
    Tapped(Pos2),
}

#[derive(Clone, Copy)]
enum Held {
    Mouse,
    Touch {
        origin: Pos2,
        last: Pos2,
        side: Option<Side>,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Along,
    Across,
}

/// Reads the drag on the horizontal control `id` each frame it is `down`.
/// Direction decides on a touch, as on a native slider; a mouse is
/// [`Along`](Reading::Along) from its press.
pub(crate) fn read_horizontal_drag(ui: &Ui, id: Id, down: bool) -> Reading {
    let key = id.with("eggshell-touch-drag");
    let held = ui.data(|data| data.get_temp::<Option<Held>>(key)).flatten();
    let at = ui.input(|input| input.pointer.interact_pos());
    let (held, reading) = match (held, down, at) {
        (
            Some(Held::Touch {
                origin, side: None, ..
            }),
            false,
            _,
        ) => (None, Reading::Tapped(origin)),
        (_, false, _) | (_, true, None) => (None, Reading::Along),
        (None, true, Some(at)) => {
            if ui.input(|input| input.any_touches()) {
                let held = Held::Touch {
                    origin: at,
                    last: at,
                    side: None,
                };
                (Some(held), Reading::Pending)
            } else {
                (Some(Held::Mouse), Reading::Along)
            }
        }
        (Some(Held::Mouse), true, _) => (Some(Held::Mouse), Reading::Along),
        (Some(Held::Touch { origin, last, side }), true, Some(at)) => {
            let side = side.or_else(|| {
                let moved = at - origin;
                (moved.length() >= SLOP).then(|| {
                    if moved.x.abs() >= moved.y.abs() {
                        Side::Along
                    } else {
                        Side::Across
                    }
                })
            });
            let reading = match side {
                None => Reading::Pending,
                Some(Side::Along) => Reading::Along,
                Some(Side::Across) => Reading::Across(at - last),
            };
            (
                Some(Held::Touch {
                    origin,
                    last: at,
                    side,
                }),
                reading,
            )
        }
    };
    ui.data_mut(|data| data.insert_temp(key, held));
    reading
}

/// Scrolls the container around `node` by what a touch that went across it
/// moved this frame.
pub(crate) fn scroll_across(shell: &Shell, node: Id, reading: Reading) {
    if let Reading::Across(by) = reading {
        shell.scroll_within(node, -by);
    }
}
