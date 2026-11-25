//! How long a notice has been on screen, on a clock the caller passes in.

use core::time::Duration;

/// Whether the dwell counts the time since the last tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hold {
    Running,
    Held,
}

/// What a tick found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Countdown {
    /// This much is left.
    Left(Duration),
    /// The dwell ran out at this tick.
    Expires,
    /// It ran out at an earlier tick.
    Over,
}

/// One notice's dwell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dwell {
    Counting { elapsed: Duration, seen: Duration },
    Over,
}

impl Dwell {
    /// A dwell first seen at `now`.
    pub(super) fn new(now: Duration) -> Self {
        Self::Counting {
            elapsed: Duration::ZERO,
            seen: now,
        }
    }

    /// Moves the dwell to `now`: the time since the last tick counts unless
    /// it is `hold`'s. Reports [`Countdown::Expires`] once.
    pub(super) fn tick(&mut self, now: Duration, length: Duration, hold: Hold) -> Countdown {
        let Self::Counting { elapsed, seen } = *self else {
            return Countdown::Over;
        };
        let elapsed = match hold {
            Hold::Running => elapsed.saturating_add(now.saturating_sub(seen)),
            Hold::Held => elapsed,
        };
        let left = length.saturating_sub(elapsed);
        if left.is_zero() {
            *self = Self::Over;
            Countdown::Expires
        } else {
            *self = Self::Counting { elapsed, seen: now };
            Countdown::Left(left)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn a_dwell_counts_down_and_expires_once() {
        let mut dwell = Dwell::new(ms(1000));
        assert_eq!(
            dwell.tick(ms(1000), ms(500), Hold::Running),
            Countdown::Left(ms(500))
        );
        assert_eq!(
            dwell.tick(ms(1300), ms(500), Hold::Running),
            Countdown::Left(ms(200))
        );
        assert_eq!(
            dwell.tick(ms(1500), ms(500), Hold::Running),
            Countdown::Expires
        );
        assert_eq!(
            dwell.tick(ms(1600), ms(500), Hold::Running),
            Countdown::Over
        );
    }

    #[test]
    fn held_time_is_not_counted_and_the_dwell_resumes_where_it_left_off() {
        let mut dwell = Dwell::new(ms(0));
        dwell.tick(ms(300), ms(500), Hold::Running);
        assert_eq!(
            dwell.tick(ms(5000), ms(500), Hold::Held),
            Countdown::Left(ms(200))
        );
        assert_eq!(
            dwell.tick(ms(5100), ms(500), Hold::Running),
            Countdown::Left(ms(100))
        );
        assert_eq!(
            dwell.tick(ms(5200), ms(500), Hold::Running),
            Countdown::Expires
        );
    }

    #[test]
    fn a_clock_that_goes_back_counts_nothing() {
        let mut dwell = Dwell::new(ms(1000));
        assert_eq!(
            dwell.tick(ms(400), ms(500), Hold::Running),
            Countdown::Left(ms(500))
        );
    }
}
