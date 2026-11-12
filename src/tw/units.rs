//! Constrained values used by style classes and display primitives.

use core::cmp::Ordering;
use core::ops::Mul;

/// Opacity in the closed unit interval.
///
/// Construction clamps, and NaN becomes fully transparent, so values are
/// totally ordered. Multiplying two opacities cannot leave the interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opacity(f32);

impl Opacity {
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self(0.0);
    /// Fully opaque; the default.
    pub const OPAQUE: Self = Self(1.0);

    /// Clamps `value` to `0.0..=1.0`; NaN becomes [`Opacity::TRANSPARENT`].
    pub fn new(value: f32) -> Self {
        if value.is_nan() {
            Self::TRANSPARENT
        } else {
            Self(value.clamp(0.0, 1.0))
        }
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

impl Eq for Opacity {}

impl PartialOrd for Opacity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Opacity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Mul for Opacity {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl From<Opacity> for f32 {
    fn from(opacity: Opacity) -> f32 {
        opacity.0
    }
}

/// How far something has come, in the closed unit interval: a transition, or
/// a task shown by a progress bar.
///
/// Construction clamps, and NaN becomes [`Progress::START`].
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Progress(f32);

impl Progress {
    /// Not started: the base paint.
    pub const START: Self = Self(0.0);
    /// Arrived: the variant paint.
    pub const END: Self = Self(1.0);

    /// Clamps `value` to `0.0..=1.0`; NaN becomes [`Progress::START`].
    pub fn new(value: f32) -> Self {
        if value.is_nan() {
            Self::START
        } else {
            Self(value.clamp(0.0, 1.0))
        }
    }

    /// `from` moved this far towards `to`.
    pub fn lerp(self, from: f32, to: f32) -> f32 {
        from + (to - from) * self.0
    }
}

impl From<Progress> for f32 {
    fn from(progress: Progress) -> f32 {
        progress.0
    }
}

/// A rotation. Non-finite input becomes no rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Angle(f32);

impl Angle {
    /// No rotation.
    pub const ZERO: Self = Self(0.0);

    /// An angle in radians.
    pub fn radians(radians: f32) -> Self {
        if radians.is_finite() {
            Self(radians)
        } else {
            Self::ZERO
        }
    }

    /// An angle in degrees.
    pub fn degrees(degrees: f32) -> Self {
        Self::radians(degrees.to_radians())
    }
}

impl From<Angle> for f32 {
    /// The angle in radians.
    fn from(angle: Angle) -> f32 {
        angle.0
    }
}

/// Mirroring of an image or icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Flip {
    /// Not mirrored.
    #[default]
    None,
    /// Mirrored left to right.
    Horizontal,
    /// Mirrored top to bottom.
    Vertical,
    /// Mirrored both ways.
    Both,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_clamps_and_orders() {
        assert_eq!(f32::from(Opacity::new(1.5)), 1.0);
        assert_eq!(f32::from(Opacity::new(-0.5)), 0.0);
        assert_eq!(Opacity::new(f32::NAN), Opacity::TRANSPARENT);
        assert!(Opacity::new(0.25) < Opacity::new(0.75));
        assert_eq!(Opacity::default(), Opacity::OPAQUE);
    }

    #[test]
    fn progress_clamps_and_interpolates() {
        assert_eq!(Progress::new(2.0), Progress::END);
        assert_eq!(Progress::new(f32::NAN), Progress::START);
        assert_eq!(Progress::new(0.25).lerp(10.0, 20.0), 12.5);
    }

    #[test]
    fn opacity_multiplies_within_the_interval() {
        let half = Opacity::new(0.5);
        assert_eq!(f32::from(half * half), 0.25);
        assert_eq!(half * Opacity::OPAQUE, half);
    }

    #[test]
    fn angle_converts_degrees_and_rejects_non_finite() {
        assert!((f32::from(Angle::degrees(180.0)) - core::f32::consts::PI).abs() < 1e-6);
        assert_eq!(Angle::radians(f32::INFINITY), Angle::ZERO);
        assert_eq!(Angle::degrees(f32::NAN), Angle::ZERO);
    }
}
