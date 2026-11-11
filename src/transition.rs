//! Time-based transitions with CSS reversal semantics.
//!
//! Egui's own `animate_*_with_time` advances by a per-frame step, which runs
//! slow whenever frames are slower than 60 fps. Here progress is a function
//! of `input.time`, so a transition lasts its duration at any frame rate.

use core::time::Duration;

use std::collections::HashMap;

use egui::{Context, Id, Rect, Vec2};

use crate::continuity::{Interrupted, Slide, SlideId};
use crate::tw::{Easing, Progress};

/// One transition in flight, from where the previous one left the value to
/// `target`.
#[derive(Clone, Copy)]
struct Transition {
    from: Progress,
    target: Progress,
    start: f64,
    duration: Duration,
}

impl Transition {
    fn settled(at: Progress, now: f64) -> Self {
        Self {
            from: at,
            target: at,
            start: now,
            duration: Duration::ZERO,
        }
    }

    fn from_start(now: f64, duration: Duration) -> Self {
        Self {
            from: Progress::START,
            target: Progress::END,
            start: now,
            duration,
        }
    }

    fn elapsed(&self, now: f64) -> Progress {
        let elapsed = (now - self.start).max(0.0) as f32;
        let total = self.duration.as_secs_f32();
        if total > 0.0 {
            Progress::new(elapsed / total)
        } else {
            Progress::END
        }
    }

    fn value(&self, now: f64, easing: Easing) -> Progress {
        Progress::new(
            easing
                .ease(self.elapsed(now))
                .lerp(self.from.into(), self.target.into()),
        )
    }

    /// Retargets from the current value. As in CSS, a transition reversed
    /// mid-flight starts from where the value is and lasts only in proportion
    /// to the distance it has left to travel.
    fn retarget(&self, target: Progress, now: f64, easing: Easing, full: Duration) -> Self {
        let current = self.value(now, easing);
        let distance = (f32::from(target) - f32::from(current)).abs();
        Self {
            from: current,
            target,
            start: now,
            duration: full.mul_f32(distance),
        }
    }
}

/// The value of the transition keyed by `id` towards `target`
/// ([`Progress::START`] or [`Progress::END`]) over `duration`, eased.
///
/// A transition seen for the first time is already at its target. Repaints
/// are requested while it is in flight and not after it settles.
pub(crate) fn animate(
    ctx: &Context,
    id: Id,
    target: Progress,
    duration: Duration,
    easing: Easing,
) -> Progress {
    if duration.is_zero() {
        return target;
    }
    let now = ctx.input(|input| input.time);
    let previous = ctx
        .data(|data| data.get_temp::<Transition>(id))
        .unwrap_or(Transition::settled(target, now));
    let transition = if previous.target == target {
        previous
    } else {
        previous.retarget(target, now, easing, duration)
    };
    ctx.data_mut(|data| data.insert_temp(id, transition));
    if transition.elapsed(now) < Progress::END {
        ctx.request_repaint();
    }
    transition.value(now, easing)
}

/// How far a node whose layout position moved is still painted from it.
#[derive(Clone, Copy)]
struct Tracked {
    /// The node's position within its parent in the previous frame's layout.
    relative: Vec2,
    /// Where the node was painted, relative to its layout, when it last moved.
    start: Vec2,
    progress: Transition,
    /// The slide this one interrupted, where the retarget found it.
    interrupted: Option<Interrupted>,
}

impl Tracked {
    fn offset(&self, now: f64, easing: Easing) -> Vec2 {
        self.start * (1.0 - f32::from(self.progress.value(now, easing)))
    }
}

/// The animated nodes of the last frame drawn into one `Ui`, and the area
/// they were laid out in.
#[derive(Clone)]
struct Tracking {
    area: Rect,
    time: f64,
    nodes: HashMap<Id, Tracked>,
}

impl Default for Tracking {
    fn default() -> Self {
        Self {
            area: Rect::NOTHING,
            time: 0.0,
            nodes: HashMap::new(),
        }
    }
}

fn tracking_id(scope: Id) -> Id {
    crate::key::root_id().with(("layout-animation", scope))
}

/// Moves the painting of nodes with an `animate_layout` from where the
/// previous frame laid them out to where this one does.
///
/// A node's position is taken within its parent's rect, so a parent that
/// moves carries its children along and they animate only for their own
/// moves. Nodes not seen this frame are forgotten when it ends.
pub(crate) struct LayoutAnimation {
    ctx: Context,
    scope: Id,
    now: f64,
    previous: HashMap<Id, Tracked>,
    current: HashMap<Id, Tracked>,
    area: Rect,
}

impl LayoutAnimation {
    /// Starts a frame laid out in `area`. When `area` is not the previous
    /// frame's size, the window was resized (an area that only moves, as a
    /// scrolled one does, is not a resize): no move starts a slide, but the slides
    /// in flight in the previous frame go on, retargeted from where they are painted like any
    /// interrupted move.
    pub(crate) fn begin(ctx: &Context, scope: Id, area: Rect) -> Self {
        let before: Tracking = ctx
            .data(|data| data.get_temp(tracking_id(scope)))
            .unwrap_or_default();
        let now = ctx.input(|input| input.time);
        let mut previous = before.nodes;
        if before.area.size() != area.size() {
            previous.retain(|_, tracked| tracked.progress.elapsed(before.time) < Progress::END);
        }
        Self {
            ctx: ctx.clone(),
            scope,
            now,
            previous,
            current: HashMap::new(),
            area,
        }
    }

    /// How far to paint the node `id` from its layout, at `relative` to its
    /// parent's rect, for `duration` along `easing`.
    pub(crate) fn offset(
        &mut self,
        id: Id,
        relative: Vec2,
        duration: Duration,
        easing: Easing,
    ) -> Vec2 {
        let now = self.now;
        let tracked = match self.previous.get(&id) {
            Some(previous) if previous.relative == relative => Tracked {
                relative,
                ..*previous
            },
            Some(previous) => Tracked {
                relative,
                start: previous.relative + previous.offset(now, easing) - relative,
                progress: Transition::from_start(now, duration),
                interrupted: Some(Interrupted {
                    start: previous.start,
                    progress: previous.progress.value(now, easing),
                }),
            },
            None => Tracked {
                relative,
                start: Vec2::ZERO,
                progress: Transition::settled(Progress::END, now),
                interrupted: None,
            },
        };
        self.current.insert(id, tracked);
        if tracked.progress.elapsed(now) < Progress::END {
            self.ctx.request_repaint();
        }
        tracked.offset(now, easing)
    }

    /// The state of the slide of `id` drawn this frame, with the node's whole
    /// paint offset from its layout, `offset`.
    pub(crate) fn slide(&self, id: Id, easing: Easing, offset: Vec2) -> Option<Slide> {
        let tracked = self.current.get(&id)?;
        Some(Slide {
            id: SlideId::new(tracked.progress.start),
            start: tracked.start,
            progress: tracked.progress.value(self.now, easing),
            interrupted: tracked.interrupted,
            offset,
        })
    }

    pub(crate) fn finish(self) {
        let nodes = self.current;
        let area = self.area;
        self.ctx.data_mut(|data| {
            data.insert_temp(
                tracking_id(self.scope),
                Tracking {
                    area,
                    time: self.now,
                    nodes,
                },
            );
        });
    }
}

/// How many nodes the layout animation of the `Ui` with id `scope` tracks.
#[doc(hidden)]
pub fn tracked_layout_nodes(ctx: &Context, scope: Id) -> usize {
    ctx.data(|data| data.get_temp::<Tracking>(tracking_id(scope)))
        .map_or(0, |tracking| tracking.nodes.len())
}
