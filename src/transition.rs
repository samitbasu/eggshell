//! Time-based transitions with CSS reversal semantics.
//!
//! Egui's own `animate_*_with_time` advances by a per-frame step, which runs
//! slow whenever frames are slower than 60 fps. Here progress is a function
//! of `input.time`, so a transition lasts its duration at any frame rate.

use core::time::Duration;

use egui::{Context, Id};

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
