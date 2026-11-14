//! Time-based transitions with CSS reversal semantics.
//!
//! Egui's own `animate_*_with_time` advances by a per-frame step, which runs
//! slow whenever frames are slower than 60 fps. Here progress is a function
//! of `input.time`, so a transition lasts its duration at any frame rate.

use core::time::Duration;

use std::collections::HashMap;

use crate::kept::Handle;
use egui::{Context, Id, Pos2, Rect, Vec2};

use crate::continuity::{Interrupted, Slide, SlideId};
use crate::pipeline::PlacedNode;
use crate::tw::{Easing, Progress};

/// One transition in flight, from where the previous one left the value to
/// `target`.
#[derive(Clone, Copy)]
pub(crate) struct Transition {
    from: Progress,
    target: Progress,
    start: f64,
    duration: Duration,
}

impl Transition {
    pub(crate) fn settled(at: Progress, now: f64) -> Self {
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

    pub(crate) fn elapsed(&self, now: f64) -> Progress {
        let elapsed = (now - self.start).max(0.0) as f32;
        let total = self.duration.as_secs_f32();
        if total > 0.0 {
            Progress::new(elapsed / total)
        } else {
            Progress::END
        }
    }

    pub(crate) fn value(&self, now: f64, easing: Easing) -> Progress {
        Progress::new(
            easing
                .ease(self.elapsed(now))
                .lerp(self.from.into(), self.target.into()),
        )
    }

    /// This transition if it already heads for `target`, else retargeted.
    pub(crate) fn toward(self, target: Progress, now: f64, easing: Easing, full: Duration) -> Self {
        if self.target == target {
            self
        } else {
            self.retarget(target, now, easing, full)
        }
    }

    pub(crate) fn in_flight(&self, now: f64) -> bool {
        self.elapsed(now) < Progress::END
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

/// `duration`, or zero where the style asks for no motion: egui's
/// `animation_time` of zero is "reduced motion", the one switch every eggshell
/// animation (state transitions, slides, presence, size) goes through.
pub(crate) fn allowed(ctx: &Context, duration: Duration) -> Duration {
    if ctx.global_style().animation_time == 0.0 {
        Duration::ZERO
    } else {
        duration
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
    let duration = allowed(ctx, duration);
    if duration.is_zero() {
        return target;
    }
    let now = ctx.input(|input| input.time);
    let previous = ctx
        .data(|data| data.get_temp::<Transition>(id))
        .unwrap_or(Transition::settled(target, now));
    let transition = previous.toward(target, now, easing, duration);
    ctx.data_mut(|data| data.insert_temp(id, transition));
    if transition.in_flight(now) {
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
pub(crate) struct Tracking {
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

/// Moves the painting of nodes with an `animate_layout` from where the
/// previous frame laid them out to where this one does.
///
/// A node's position is taken within its parent's rect, so a parent that
/// moves carries its children along and they animate only for their own
/// moves. Nodes not seen this frame are forgotten when it ends.
pub(crate) struct LayoutAnimation {
    ctx: Context,
    kept: Handle,
    now: f64,
    previous: HashMap<Id, Tracked>,
    current: HashMap<Id, Tracked>,
    area: Rect,
    reach: f32,
    /// Whether a layout move starts a slide.
    starts_slides: StartsSlides,
}

/// Whether a node whose layout moved slides from where it was. A frame with a
/// size animation in flight moves nodes by layout itself, smoothly, so they
/// follow it: a slide on top would lag the motion and start anew every frame.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartsSlides {
    Yes,
    No,
}

impl StartsSlides {
    pub(crate) fn unless_size_in_flight(in_flight: bool) -> Self {
        if in_flight { Self::No } else { Self::Yes }
    }
}

impl LayoutAnimation {
    /// Starts a frame laid out in `area`. When `area` is not the previous
    /// frame's size, the window was resized (an area that only moves, as a
    /// scrolled one does, is not a resize): no move starts a slide, but the slides
    /// in flight in the previous frame go on, retargeted from where they are painted like any
    /// interrupted move.
    pub(crate) fn begin(ctx: &Context, kept: &Handle, area: Rect) -> Self {
        let before = kept.with(|kept| kept.animation.clone());
        let now = ctx.input(|input| input.time);
        let mut previous = before.nodes;
        if before.area.size() != area.size() {
            previous.retain(|_, tracked| tracked.progress.elapsed(before.time) < Progress::END);
        }
        Self {
            ctx: ctx.clone(),
            kept: kept.clone(),
            now,
            previous,
            current: HashMap::new(),
            area,
            reach: 0.0,
            starts_slides: StartsSlides::Yes,
        }
    }

    /// Sets whether this frame's layout moves start slides.
    pub(crate) fn starting(mut self, starts_slides: StartsSlides) -> Self {
        self.starts_slides = starts_slides;
        self
    }

    /// Notes how far from its layout this frame leaves each animated node of
    /// `nodes` painted: from where the previous frame painted it. A node
    /// whose layout is outside a clip by more than the farthest of them
    /// cannot be painted inside it.
    pub(crate) fn anticipate(&mut self, nodes: &[PlacedNode]) {
        let mut parents: Vec<(usize, Pos2)> = Vec::new();
        let mut reach: f32 = 0.0;
        for (index, node) in nodes.iter().enumerate() {
            while parents.last().is_some_and(|&(end, _)| end <= index) {
                parents.pop();
            }
            let min = node.rect().min;
            let variants = &node.interaction.variants;
            if let (Some(_), Some(id)) = (variants.animate_layout, node.id())
                && let Some(previous) = self.previous.get(&id)
            {
                let relative = min - parents.last().map_or(min, |&(_, parent)| parent);
                let painted = previous.relative + previous.offset(self.now, variants.easing);
                reach = reach.max((painted - relative).length());
            }
            parents.push((node.end, min));
        }
        self.reach = reach;
    }

    /// The farthest [`anticipate`](Self::anticipate) found.
    pub(crate) fn slide_reach(&self) -> f32 {
        self.reach
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
        let duration = allowed(&self.ctx, duration);
        let tracked = match self.previous.get(&id) {
            Some(previous)
                if previous.relative == relative || self.starts_slides == StartsSlides::No =>
            {
                Tracked {
                    relative,
                    ..*previous
                }
            }
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
        self.kept.with(|kept| {
            kept.animation = Tracking {
                area,
                time: self.now,
                nodes,
            };
        });
    }
}

/// How many nodes the layout animation of `layout` tracks.
#[doc(hidden)]
pub fn tracked_layout_nodes(layout: &crate::LayoutState) -> usize {
    layout.kept().with(|kept| kept.animation.nodes.len())
}
