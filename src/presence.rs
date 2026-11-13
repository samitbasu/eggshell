//! Presence: an overlay that enters and leaves with motion.
//!
//! The application declares the overlay every frame and says whether it is
//! [`Shown`]. The library keeps the overlay mounted while it leaves, so the
//! application's own state can already say `Closed` while the box is still
//! on screen. The phase is `Closed | Entering | Open | Exiting`; progress is
//! the retargetable eased [`Progress`] of [`transition`](crate::transition),
//! so a reopen mid-exit reverses from where the box is.
//!
//! The motion is paint-only: layout and hit rects are final from the first
//! frame, and the draw moves, fades and scales what the overlay painted.

use core::time::Duration;
use std::collections::{HashMap, HashSet};

use egui::{Context, Id, Vec2, vec2};

use crate::kept::Handle;
use crate::overlay::Side;
use crate::pipeline::DismissFn;
use crate::roots::Form;
use crate::transition::Transition;
use crate::tw::{Easing, Opacity, Progress};

/// Whether an overlay is shown or being closed, as the application's state
/// says. The overlay is declared either way; the library keeps it on screen
/// until its exit has run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shown {
    /// The overlay is open: it enters, or has entered.
    Open,
    /// The overlay is closed: it exits, or has exited.
    #[default]
    Closed,
}

impl Shown {
    /// Whether the state says open.
    pub fn is_open(self) -> bool {
        self == Self::Open
    }
}

/// How an overlay arrives and leaves.
///
/// Only the scrim of a modal fades. The box and its contents are never faded:
/// without group opacity (#40) a per-shape fade lets the page show through the
/// box mid-motion. The entrances move the box instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Entrance {
    /// No motion of the box; the scrim of a modal fades.
    Fade,
    /// Scales the box up from 95% about its centre: shadcn's `zoom-in-95`.
    Zoom,
    /// [`Zoom`](Self::Zoom) with a short slide away from the anchor, for an
    /// overlay that floats next to one: shadcn's `zoom-in-95
    /// slide-in-from-top-2`. A modal has no anchor and only zooms.
    Pop,
    /// Slides in from this side of the screen by the box's own size.
    Slide(Side),
}

impl Entrance {
    const ZOOM_FROM: f32 = 0.95;
    /// shadcn's `slide-in-from-top-2`, 0.5rem.
    const POP_SLIDE: f32 = 8.0;

    fn scale(self, progress: Progress) -> f32 {
        match self {
            Self::Zoom | Self::Pop => progress.lerp(Self::ZOOM_FROM, 1.0),
            _ => 1.0,
        }
    }

    /// How far the box is painted from its layout.
    fn offset(self, progress: Progress, size: Vec2, form: Form) -> Vec2 {
        let left = 1.0 - f32::from(progress);
        match (self, form) {
            (Self::Slide(side), _) => direction(side) * size * left,
            (Self::Pop, Form::Floating(side)) => {
                direction(side.opposite()) * Self::POP_SLIDE * left
            }
            _ => Vec2::ZERO,
        }
    }
}

/// The unit vector pointing out of the screen's `side`.
fn direction(side: Side) -> Vec2 {
    match side {
        Side::Top => vec2(0.0, -1.0),
        Side::Bottom => vec2(0.0, 1.0),
        Side::Left => vec2(-1.0, 0.0),
        Side::Right => vec2(1.0, 0.0),
    }
}

/// An [`Entrance`] without a side, as the style names one; the side of a
/// sheet is its own edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Entry {
    /// [`Entrance::Fade`].
    #[default]
    Fade,
    /// [`Entrance::Zoom`].
    Zoom,
    /// [`Entrance::Pop`].
    Pop,
    /// [`Entrance::Slide`] from the edge a modal is docked to; an overlay
    /// without one has no box motion.
    Slide,
}

impl Entry {
    pub(crate) fn entrance(self, edge: Option<Side>) -> Entrance {
        match (self, edge) {
            (Self::Fade, _) => Entrance::Fade,
            (Self::Zoom, _) => Entrance::Zoom,
            (Self::Pop, _) => Entrance::Pop,
            (Self::Slide, Some(side)) => Entrance::Slide(side),
            (Self::Slide, None) => Entrance::Fade,
        }
    }
}

/// The motion of one kind of overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KindMotion {
    /// How the overlay arrives and leaves.
    pub entry: Entry,
    /// How long it takes to enter. Zero is no motion.
    pub enter: Duration,
    /// How long it takes to leave. Zero is no motion.
    pub exit: Duration,
    /// The timing function of both.
    pub easing: Easing,
}

impl KindMotion {
    /// No motion: the overlay appears and vanishes at once.
    pub const INSTANT: Self = Self {
        entry: Entry::Fade,
        enter: Duration::ZERO,
        exit: Duration::ZERO,
        easing: Easing::Default,
    };
}

/// Enter and exit motion per kind of overlay: what an overlay that was
/// given [`shown`](crate::overlay::OverlayChain::shown) and no motion of its
/// own does.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MotionStyle {
    /// Anchored overlays that are not menus.
    pub popover: KindMotion,
    /// Menus and the lists selects open.
    pub menu: KindMotion,
    /// Modals centred on the screen.
    pub dialog: KindMotion,
    /// Modals docked to a screen edge.
    pub sheet: KindMotion,
    /// Tooltips.
    pub tooltip: KindMotion,
}

impl MotionStyle {
    /// No overlay moves.
    pub const INSTANT: Self = Self {
        popover: KindMotion::INSTANT,
        menu: KindMotion::INSTANT,
        dialog: KindMotion::INSTANT,
        sheet: KindMotion::INSTANT,
        tooltip: KindMotion::INSTANT,
    };
}

impl Default for MotionStyle {
    fn default() -> Self {
        Self::INSTANT
    }
}

/// Where in its life an overlay is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Entering,
    Open,
    Exiting,
}

/// An overlay's presence in one frame, with the motion it is painted with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Staged {
    pub(crate) phase: Phase,
    pub(crate) progress: Progress,
    pub(crate) entrance: Entrance,
}

impl Staged {
    /// An overlay that is simply there: declared without presence.
    pub(crate) const STEADY: Self = Self {
        phase: Phase::Open,
        progress: Progress::END,
        entrance: Entrance::Fade,
    };

    pub(crate) fn is_steady(&self) -> bool {
        self.phase == Phase::Open
    }

    /// An exiting overlay takes no input but still blocks it.
    pub(crate) fn is_exiting(&self) -> bool {
        self.phase == Phase::Exiting
    }

    pub(crate) fn scrim(&self) -> Opacity {
        Opacity::new(self.progress.into())
    }

    pub(crate) fn scale(&self) -> f32 {
        self.entrance.scale(self.progress)
    }

    pub(crate) fn offset(&self, size: Vec2, form: Form) -> Vec2 {
        self.entrance.offset(self.progress, size, form)
    }
}

/// What the chain declared, with the style's defaults filled in.
pub(crate) struct Motion {
    pub(crate) shown: Shown,
    pub(crate) enter: Duration,
    pub(crate) exit: Duration,
    pub(crate) easing: Easing,
    pub(crate) entrance: Entrance,
}

/// What stepping an overlay's presence found.
pub(crate) enum Step {
    /// Not on screen: build nothing. `exited` is set in the frame it got
    /// there.
    Closed {
        exited: bool,
    },
    Present(Staged),
}

/// The overlays that have a presence, by id.
#[derive(Default)]
pub(crate) struct Presences {
    entries: HashMap<Id, Transition>,
    seen: HashSet<Id>,
}

impl Presences {
    /// Moves the presence of `id` to this frame.
    pub(crate) fn step(&mut self, ctx: &Context, id: Id, motion: &Motion) -> Step {
        let now = ctx.input(|input| input.time);
        self.seen.insert(id);
        let (target, full) = match motion.shown {
            Shown::Open => (Progress::END, motion.enter),
            Shown::Closed => (Progress::START, motion.exit),
        };
        let known = self.entries.get(&id).copied();
        let transition = known
            .unwrap_or_else(|| Transition::settled(Progress::START, now))
            .toward(target, now, motion.easing, full);
        let in_flight = transition.in_flight(now);
        let progress = transition.value(now, motion.easing);
        let phase = match (motion.shown, in_flight) {
            (Shown::Open, true) => Phase::Entering,
            (Shown::Open, false) => Phase::Open,
            (Shown::Closed, true) => Phase::Exiting,
            (Shown::Closed, false) => {
                self.entries.remove(&id);
                return Step::Closed {
                    exited: known.is_some(),
                };
            }
        };
        self.entries.insert(id, transition);
        if in_flight {
            ctx.request_repaint();
        }
        Step::Present(Staged {
            phase,
            progress,
            entrance: motion.entrance,
        })
    }

    pub(crate) fn contains(&self, id: Id) -> bool {
        self.entries.contains_key(&id)
    }

    /// Forgets the overlays a frame did not declare, and starts the next
    /// frame's accounting.
    pub(crate) fn sweep(&mut self) {
        let seen = std::mem::take(&mut self.seen);
        self.entries.retain(|id, _| seen.contains(id));
    }
}

/// Steps the presence of `id` in the layout's kept state.
pub(crate) fn step(kept: &Handle, ctx: &Context, id: Id, motion: &Motion) -> Step {
    kept.with(|kept| kept.presences.step(ctx, id, motion))
}

/// What to run once an overlay has left.
pub(crate) type ExitedFn<A> = DismissFn<A>;

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(shown: Shown) -> Motion {
        Motion {
            shown,
            enter: Duration::from_millis(100),
            exit: Duration::from_millis(100),
            easing: Easing::Linear,
            entrance: Entrance::Fade,
        }
    }

    fn at(presences: &mut Presences, time: f64, shown: Shown) -> Step {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput {
            time: Some(time),
            ..egui::RawInput::default()
        });
        let step = presences.step(&ctx, Id::NULL.with("overlay"), &motion(shown));
        ctx.end_pass().textures_delta.clear();
        step
    }

    fn phase(step: &Step) -> Option<Phase> {
        match step {
            Step::Present(staged) => Some(staged.phase),
            Step::Closed { .. } => None,
        }
    }

    #[test]
    fn an_overlay_never_shown_stays_closed_and_does_not_exit() {
        let mut presences = Presences::default();
        assert!(matches!(
            at(&mut presences, 0.0, Shown::Closed),
            Step::Closed { exited: false }
        ));
    }

    #[test]
    fn the_phase_runs_entering_open_exiting_closed() {
        let mut presences = Presences::default();
        let phases: Vec<_> = [
            (0.0, Shown::Open),
            (0.05, Shown::Open),
            (0.2, Shown::Open),
            (0.25, Shown::Closed),
            (0.3, Shown::Closed),
            (0.4, Shown::Closed),
            (0.5, Shown::Closed),
        ]
        .into_iter()
        .map(|(time, shown)| phase(&at(&mut presences, time, shown)))
        .collect();
        assert_eq!(
            phases,
            [
                Some(Phase::Entering),
                Some(Phase::Entering),
                Some(Phase::Open),
                Some(Phase::Exiting),
                Some(Phase::Exiting),
                None,
                None,
            ]
        );
    }

    #[test]
    fn exited_is_reported_once() {
        let mut presences = Presences::default();
        at(&mut presences, 0.0, Shown::Open);
        at(&mut presences, 0.2, Shown::Open);
        at(&mut presences, 0.25, Shown::Closed);
        assert!(matches!(
            at(&mut presences, 0.4, Shown::Closed),
            Step::Closed { exited: true }
        ));
        assert!(matches!(
            at(&mut presences, 0.45, Shown::Closed),
            Step::Closed { exited: false }
        ));
    }

    #[test]
    fn reopening_mid_exit_reverses_from_where_it_was() {
        let mut presences = Presences::default();
        at(&mut presences, 0.0, Shown::Open);
        at(&mut presences, 0.2, Shown::Open);
        let Step::Present(leaving) = at(&mut presences, 0.25, Shown::Closed) else {
            panic!("exiting");
        };
        let Step::Present(deeper) = at(&mut presences, 0.3, Shown::Closed) else {
            panic!("exiting");
        };
        let Step::Present(back) = at(&mut presences, 0.31, Shown::Open) else {
            panic!("entering");
        };
        assert!(deeper.progress < leaving.progress);
        assert_eq!(back.phase, Phase::Entering);
        assert!(
            (f32::from(back.progress) - f32::from(deeper.progress)).abs() < 0.2,
            "continues from {:?}, not from the start: {:?}",
            deeper.progress,
            back.progress
        );
        assert!(f32::from(back.progress) > 0.0);
    }

    #[test]
    fn zero_durations_are_never_in_flight() {
        let mut presences = Presences::default();
        let instant = |shown| Motion {
            enter: Duration::ZERO,
            exit: Duration::ZERO,
            ..motion(shown)
        };
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let opened = presences.step(&ctx, Id::NULL.with("overlay"), &instant(Shown::Open));
        let closed = presences.step(&ctx, Id::NULL.with("overlay"), &instant(Shown::Closed));
        ctx.end_pass().textures_delta.clear();
        assert_eq!(phase(&opened), Some(Phase::Open));
        assert!(matches!(closed, Step::Closed { exited: true }));
    }

    #[test]
    fn a_slide_is_the_boxs_own_size_from_its_side() {
        let staged = Staged {
            phase: Phase::Entering,
            progress: Progress::START,
            entrance: Entrance::Slide(Side::Right),
        };
        let form = Form::Modal(crate::overlay::Modal::Edge(Side::Right));
        assert_eq!(staged.offset(vec2(380.0, 600.0), form), vec2(380.0, 0.0));
        let settled = Staged {
            progress: Progress::END,
            ..staged
        };
        assert_eq!(settled.offset(vec2(380.0, 600.0), form), Vec2::ZERO);
    }
}
