//! Hover intent: which trigger's floating content is open, from where the
//! pointer rests, where the keyboard focus is, and how long each has been
//! there.
//!
//! [`Intent::tick`] is pure: it moves the state to a time, given the trigger
//! the frame wants and whether something interrupted, and says how long
//! until the next change it waits for. Tooltips are one instance; what an
//! instance shows rides along as its payload `P`.

use core::time::Duration;

use egui::{Id, Rect};

/// Why a trigger wants what it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// The pointer is on it.
    Pointer,
    /// It has keyboard focus.
    Focus,
    /// It was pressed, and what it shows opens on a press.
    Pressed,
}

/// A trigger drawn this frame that wants what it shows.
#[derive(Clone)]
pub(crate) struct Trigger<P> {
    pub(crate) id: Id,
    pub(crate) payload: P,
    pub(crate) cause: Cause,
    /// Where it is on screen.
    pub(crate) rect: Rect,
}

/// What closes the open content and keeps it closed until its trigger is
/// left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interruption {
    Press,
    Escape,
    Drag,
    Blur,
}

/// Whether a trigger opens at once shortly after another one closed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Skip {
    /// Within this long of a close, or while one is open, the next trigger
    /// opens without the delay.
    Grace(Duration),
    #[cfg_attr(not(test), expect(dead_code, reason = "hover cards (#376) never skip"))]
    Never,
}

/// When focus arriving on a trigger opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusOpens {
    AtOnce,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "hover cards (#376) wait on focus")
    )]
    AfterDelay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Timing {
    /// How long the pointer rests on a trigger before it opens.
    pub(crate) open: Duration,
    /// How long a trigger opened with this timing stays open once nothing
    /// wants it.
    pub(crate) close: Duration,
    pub(crate) skip: Skip,
    pub(crate) focus: FocusOpens,
}

#[derive(Clone)]
pub(crate) struct Present<P> {
    pub(crate) trigger: Id,
    pub(crate) payload: P,
    pub(crate) open: bool,
    close: Duration,
}

#[derive(Clone, Copy, Debug)]
struct Waiting {
    trigger: Id,
    since: Duration,
}

/// Ticks in which focus restored to a trigger may land on it: egui applies a
/// requested focus a pass later.
pub(crate) const RESTORE_TICKS: u8 = 3;

/// A trigger that an overlay's dismissal gave focus back to. Until the focus
/// leaves it, its focus wants nothing opened: focus given back is focus
/// staying put, not arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Restored {
    trigger: Id,
    landing: Landing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Landing {
    /// Requested, and given up after this many more ticks without landing.
    Pending(u8),
    Landed,
}

pub(crate) struct Intent<P> {
    /// The triggers this frame's draw has registered so far.
    pub(crate) requests: Vec<Trigger<P>>,
    pub(crate) present: Vec<Present<P>>,
    waiting: Option<Waiting>,
    /// Since when the open one has not been wanted.
    unwanted_since: Option<Duration>,
    latched: Option<Id>,
    /// The trigger whose show a press opened, kept open until the next
    /// interruption.
    pinned: Option<Id>,
    restored: Option<Restored>,
    closed_at: Option<Duration>,
    /// Where the trigger last opened is.
    last: Option<Rect>,
}

impl<P> Default for Intent<P> {
    fn default() -> Self {
        Self {
            requests: Vec::new(),
            present: Vec::new(),
            waiting: None,
            unwanted_since: None,
            latched: None,
            pinned: None,
            restored: None,
            closed_at: None,
            last: None,
        }
    }
}

impl<P: Clone> Intent<P> {
    /// Focus was given back to `target` by an overlay closing, whatever closed
    /// it.
    pub(crate) fn focus_restored(&mut self, target: Id) {
        self.restored = Some(Restored {
            trigger: target,
            landing: Landing::Pending(RESTORE_TICKS),
        });
    }

    /// Follows the focus given back to a trigger: it lands, then the
    /// trigger's focus is an arrival again once the focus leaves it.
    pub(crate) fn follow_focus(&mut self, focused: Option<Id>) {
        let Some(restored) = self.restored else {
            return;
        };
        let here = focused == Some(restored.trigger);
        self.restored = match (restored.landing, here) {
            (_, true) => Some(Restored {
                landing: Landing::Landed,
                ..restored
            }),
            (Landing::Pending(left), false) if left > 1 => Some(Restored {
                landing: Landing::Pending(left - 1),
                ..restored
            }),
            _ => None,
        };
    }

    /// Whether `trigger` wants to open for a reason that opens it.
    fn arrives(&self, trigger: &Trigger<P>) -> bool {
        trigger.cause == Cause::Pointer
            || self
                .restored
                .is_none_or(|restored| restored.trigger != trigger.id)
    }

    /// Drops what `trigger`, which is gone, shows, at once.
    pub(crate) fn forget(&mut self, trigger: Id) {
        self.present.retain(|present| present.trigger != trigger);
    }

    /// The triggers that are open, in order.
    pub(crate) fn open_triggers(&self) -> Vec<Id> {
        self.present
            .iter()
            .filter(|present| present.open)
            .map(|present| present.trigger)
            .collect()
    }

    fn is_open(&self) -> bool {
        self.present.iter().any(|present| present.open)
    }

    fn is_open_for(&self, trigger: Id) -> bool {
        self.present
            .iter()
            .any(|present| present.open && present.trigger == trigger)
    }

    fn in_grace(&self, now: Duration, grace: Duration) -> bool {
        self.is_open()
            || self
                .closed_at
                .is_some_and(|at| now.saturating_sub(at) <= grace)
    }

    fn close_all(&mut self, now: Duration) {
        if self.is_open() {
            self.closed_at = Some(now);
        }
        for present in &mut self.present {
            present.open = false;
        }
        self.unwanted_since = None;
    }

    /// Whether `trigger` opens without the delay: within the grace it does
    /// for a neighbour of the trigger last opened, not for one around it,
    /// which the pointer crosses on its way out.
    fn skips_delay(&self, trigger: &Trigger<P>, now: Duration, skip: Skip) -> bool {
        match skip {
            Skip::Grace(grace) => {
                self.in_grace(now, grace)
                    && !self
                        .last
                        .is_some_and(|last| trigger.rect.contains_rect(last))
            }
            Skip::Never => false,
        }
    }

    fn open(&mut self, trigger: &Trigger<P>, now: Duration, timing: Timing) {
        self.close_all(now);
        self.closed_at = None;
        self.last = Some(trigger.rect);
        match self
            .present
            .iter_mut()
            .find(|present| present.trigger == trigger.id)
        {
            Some(present) => {
                present.payload.clone_from(&trigger.payload);
                present.close = timing.close;
                present.open = true;
            }
            None => self.present.push(Present {
                trigger: trigger.id,
                payload: trigger.payload.clone(),
                open: true,
                close: timing.close,
            }),
        }
    }

    /// Closes the open one once it has gone unwanted for its close delay.
    /// Returns how long it has left, if it is still open.
    fn linger(&mut self, now: Duration) -> Option<Duration> {
        let close = self.present.iter().find(|present| present.open)?.close;
        let since = *self.unwanted_since.get_or_insert(now);
        let left = close.saturating_sub(now.saturating_sub(since));
        if left.is_zero() {
            self.close_all(now);
            None
        } else {
            Some(left)
        }
    }

    /// Moves the intent to `now`, given what the frame's triggers want and
    /// whether something interrupted. Returns how long until the next change
    /// it waits for: a delay running out, or an unwanted one closing.
    pub(crate) fn tick(
        &mut self,
        now: Duration,
        timing: Timing,
        wanted: Option<&Trigger<P>>,
        interruption: Option<Interruption>,
    ) -> Option<Duration> {
        if let Some(trigger) = wanted.filter(|trigger| trigger.cause == Cause::Pressed) {
            self.pinned = Some(trigger.id);
            self.latched = None;
            self.waiting = None;
            self.open(trigger, now, timing);
            return None;
        }
        if let Some(pinned) = self.pinned {
            if interruption.is_none() && self.is_open_for(pinned) {
                self.waiting = None;
                return None;
            }
            self.pinned = None;
        }
        if interruption.is_some() {
            self.latched = wanted.map(|trigger| trigger.id);
            self.waiting = None;
            self.close_all(now);
            return None;
        }
        let wanted = wanted.filter(|trigger| self.arrives(trigger));
        if self.latched != wanted.map(|trigger| trigger.id) {
            self.latched = None;
        }
        let wanted = wanted.filter(|_| self.latched.is_none());
        if let Some(trigger) = wanted.filter(|trigger| self.is_open_for(trigger.id)) {
            self.open(trigger, now, timing);
            self.waiting = None;
            return None;
        }
        let closing = self.linger(now);
        let Some(trigger) = wanted else {
            self.waiting = None;
            return closing;
        };
        let since = match self.waiting {
            Some(waiting) if waiting.trigger == trigger.id => waiting.since,
            _ => {
                self.waiting = Some(Waiting {
                    trigger: trigger.id,
                    since: now,
                });
                now
            }
        };
        let delay = match (trigger.cause, timing.focus) {
            (Cause::Focus, FocusOpens::AtOnce) | (Cause::Pressed, _) => Duration::ZERO,
            _ if self.skips_delay(trigger, now, timing.skip) => Duration::ZERO,
            _ => timing.open,
        };
        let remaining = delay.saturating_sub(now.saturating_sub(since));
        if remaining.is_zero() {
            self.open(trigger, now, timing);
            self.waiting = None;
            None
        } else {
            [Some(remaining), closing].into_iter().flatten().min()
        }
    }
}

/// The triggers this frame, resolved: a pointer over a trigger beats focus
/// on another, and the one drawn last (the deepest) wins.
pub(crate) fn wanted<P: Clone>(triggers: Vec<Trigger<P>>) -> Option<Trigger<P>> {
    let of = |cause| triggers.iter().rfind(|trigger| trigger.cause == cause);
    of(Cause::Pressed)
        .or_else(|| of(Cause::Pointer))
        .or_else(|| triggers.iter().next_back())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMING: Timing = Timing {
        open: Duration::from_millis(500),
        close: Duration::ZERO,
        skip: Skip::Grace(Duration::from_millis(300)),
        focus: FocusOpens::AtOnce,
    };

    type Tooltips = Intent<()>;

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn trigger(name: &str, cause: Cause) -> Trigger<()> {
        Trigger {
            id: Id::NULL.with(name),
            payload: (),
            cause,
            rect: Rect::NOTHING,
        }
    }

    fn at(trigger: Trigger<()>, rect: Rect) -> Trigger<()> {
        Trigger { rect, ..trigger }
    }

    #[test]
    fn the_grace_opens_a_neighbour_at_once_but_not_the_trigger_around_the_last() {
        let inner = Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(20.0, 20.0));
        let around = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 30.0));
        let beside = Rect::from_min_max(egui::pos2(50.0, 0.0), egui::pos2(60.0, 10.0));
        let cell = at(trigger("cell", Cause::Pointer), inner);
        let control = at(trigger("control", Cause::Pointer), around);
        let neighbour = at(trigger("neighbour", Cause::Pointer), beside);
        let mut tooltips = Tooltips::default();
        tooltips.tick(ms(0), TIMING, Some(&cell), None);
        tooltips.tick(ms(600), TIMING, Some(&cell), None);
        assert_eq!(
            open_ids(&tooltips),
            [cell.id],
            "precondition: the cell's is open"
        );
        tooltips.tick(ms(650), TIMING, Some(&control), None);
        assert_eq!(open_ids(&tooltips), [], "the control around it waits");
        tooltips.tick(ms(700), TIMING, Some(&neighbour), None);
        assert_eq!(open_ids(&tooltips), [neighbour.id], "a neighbour does not");
    }

    fn id_of(trigger: Option<Trigger<()>>) -> Option<Id> {
        trigger.map(|trigger| trigger.id)
    }

    fn open_ids(tooltips: &Tooltips) -> Vec<Id> {
        tooltips.open_triggers()
    }

    #[test]
    fn the_pointer_waits_for_the_delay_and_reports_what_is_left() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        assert_eq!(
            tooltips.tick(ms(1000), TIMING, Some(&a), None),
            Some(ms(500))
        );
        assert_eq!(
            tooltips.tick(ms(1200), TIMING, Some(&a), None),
            Some(ms(300))
        );
        assert!(open_ids(&tooltips).is_empty());
        assert_eq!(tooltips.tick(ms(1500), TIMING, Some(&a), None), None);
        assert_eq!(open_ids(&tooltips), [a.id]);
    }

    #[test]
    fn leaving_before_the_delay_forgets_the_wait() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(400), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(450), TIMING, Some(&a), None),
            Some(ms(500))
        );
    }

    #[test]
    fn focus_restored_to_the_trigger_is_not_an_arrival() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Focus);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(50), TIMING, None, Some(Interruption::Escape));
        tooltips.focus_restored(a.id);
        tooltips.follow_focus(None);
        tooltips.tick(ms(100), TIMING, None, None);
        for at in [150, 200, 5000] {
            tooltips.follow_focus(Some(a.id));
            tooltips.tick(ms(at), TIMING, Some(&a), None);
            assert!(open_ids(&tooltips).is_empty(), "open at {at}");
        }
        let pointer = trigger("a", Cause::Pointer);
        tooltips.tick(ms(5100), TIMING, Some(&pointer), None);
        tooltips.tick(ms(5600), TIMING, Some(&pointer), None);
        assert_eq!(open_ids(&tooltips), [a.id], "the pointer arriving opens it");
        tooltips.tick(ms(5700), TIMING, None, Some(Interruption::Press));
        tooltips.follow_focus(None);
        tooltips.tick(ms(5800), TIMING, None, None);
        tooltips.follow_focus(Some(a.id));
        tooltips.tick(ms(5900), TIMING, Some(&a), None);
        assert_eq!(
            open_ids(&tooltips),
            [a.id],
            "focus that left and came back arrives"
        );
    }

    #[test]
    fn focus_restored_that_never_lands_is_forgotten() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Focus);
        tooltips.focus_restored(a.id);
        for _ in 0..RESTORE_TICKS {
            tooltips.follow_focus(None);
        }
        tooltips.follow_focus(Some(a.id));
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        assert_eq!(open_ids(&tooltips), [a.id], "a later arrival is an arrival");
    }

    #[test]
    fn focus_opens_at_once() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Focus);
        assert_eq!(tooltips.tick(ms(0), TIMING, Some(&a), None), None);
        assert_eq!(open_ids(&tooltips), [a.id]);
    }

    #[test]
    fn another_trigger_opens_at_once_within_the_grace_and_not_after_it() {
        let mut tooltips = Tooltips::default();
        let (a, b) = (trigger("a", Cause::Pointer), trigger("b", Cause::Pointer));
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(600), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(850), TIMING, Some(&b), None),
            None,
            "grace"
        );
        assert_eq!(open_ids(&tooltips), [b.id]);
        tooltips.tick(ms(900), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(1300), TIMING, Some(&a), None),
            Some(ms(500)),
            "grace is over"
        );
    }

    #[test]
    fn moving_straight_to_another_trigger_swaps_which_is_open() {
        let mut tooltips = Tooltips::default();
        let (a, b) = (trigger("a", Cause::Pointer), trigger("b", Cause::Pointer));
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(550), TIMING, Some(&b), None);
        assert_eq!(open_ids(&tooltips), [b.id]);
        assert_eq!(tooltips.present.len(), 2, "the first is leaving");
    }

    #[test]
    fn an_interruption_closes_and_keeps_it_closed_until_the_trigger_is_left() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(600), TIMING, Some(&a), Some(Interruption::Press));
        assert!(open_ids(&tooltips).is_empty());
        assert_eq!(tooltips.tick(ms(2000), TIMING, Some(&a), None), None);
        assert!(open_ids(&tooltips).is_empty(), "still latched");
        tooltips.tick(ms(2100), TIMING, None, None);
        assert!(tooltips.tick(ms(2200), TIMING, Some(&a), None).is_some());
    }

    #[test]
    fn a_pointer_over_one_trigger_beats_focus_on_another_and_the_deepest_wins() {
        let focus = trigger("focus", Cause::Focus);
        let outer = trigger("outer", Cause::Pointer);
        let inner = trigger("inner", Cause::Pointer);
        assert_eq!(
            id_of(wanted(vec![focus.clone(), outer, inner.clone()])),
            Some(inner.id)
        );
        assert_eq!(id_of(wanted(vec![focus.clone()])), Some(focus.id));
        assert_eq!(id_of(wanted(vec![])), None);
    }

    /// A hover card's timing: it waits on focus too, lingers once left, and
    /// never skips the delay.
    const CARD: Timing = Timing {
        open: Duration::from_millis(700),
        close: Duration::from_millis(300),
        skip: Skip::Never,
        focus: FocusOpens::AfterDelay,
    };

    fn open_card(a: &Trigger<()>) -> Intent<()> {
        let mut cards = Intent::default();
        cards.tick(ms(0), CARD, Some(a), None);
        cards.tick(ms(700), CARD, Some(a), None);
        assert_eq!(open_ids(&cards), [a.id], "precondition: the card is open");
        cards
    }

    #[test]
    fn a_left_trigger_closes_after_the_close_delay_and_reports_what_is_left() {
        let a = trigger("a", Cause::Pointer);
        let mut cards = open_card(&a);
        assert_eq!(cards.tick(ms(1000), CARD, None, None), Some(ms(300)));
        assert_eq!(cards.tick(ms(1200), CARD, None, None), Some(ms(100)));
        assert_eq!(open_ids(&cards), [a.id], "still open within the delay");
        assert_eq!(cards.tick(ms(1300), CARD, None, None), None);
        assert!(open_ids(&cards).is_empty());
    }

    #[test]
    fn coming_back_within_the_close_delay_keeps_it_open() {
        let a = trigger("a", Cause::Pointer);
        let mut cards = open_card(&a);
        cards.tick(ms(1000), CARD, None, None);
        cards.tick(ms(1200), CARD, Some(&a), None);
        assert_eq!(
            cards.tick(ms(1250), CARD, None, None),
            Some(ms(300)),
            "the close delay starts over"
        );
        cards.tick(ms(1500), CARD, None, None);
        assert_eq!(open_ids(&cards), [a.id]);
    }

    #[test]
    fn focus_waits_for_the_delay_when_the_timing_says_so() {
        let mut cards = Intent::default();
        let a = trigger("a", Cause::Focus);
        assert_eq!(cards.tick(ms(0), CARD, Some(&a), None), Some(ms(700)));
        assert!(open_ids(&cards).is_empty());
        cards.tick(ms(700), CARD, Some(&a), None);
        assert_eq!(open_ids(&cards), [a.id]);
    }

    #[test]
    fn without_a_grace_the_next_trigger_waits_while_the_last_lingers() {
        let (a, b) = (trigger("a", Cause::Pointer), trigger("b", Cause::Pointer));
        let mut cards = open_card(&a);
        assert_eq!(
            cards.tick(ms(1000), CARD, Some(&b), None),
            Some(ms(300)),
            "the sooner change is the first one closing"
        );
        assert_eq!(open_ids(&cards), [a.id]);
        assert_eq!(cards.tick(ms(1300), CARD, Some(&b), None), Some(ms(400)));
        assert!(open_ids(&cards).is_empty(), "the first closed on time");
        cards.tick(ms(1700), CARD, Some(&b), None);
        assert_eq!(open_ids(&cards), [b.id]);
    }

    #[test]
    fn an_interruption_closes_a_lingering_one_at_once() {
        let a = trigger("a", Cause::Pointer);
        let mut cards = open_card(&a);
        cards.tick(ms(1000), CARD, None, None);
        cards.tick(ms(1100), CARD, None, Some(Interruption::Escape));
        assert!(open_ids(&cards).is_empty());
    }
}
