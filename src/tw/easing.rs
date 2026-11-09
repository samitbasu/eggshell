//! Transition timing functions.

use super::Progress;

/// A transition's timing function, as Tailwind's `ease-*` classes.
///
/// Egui's animation supplies the clock as linear [`Progress`]; [`ease`](Self::ease)
/// bends it before anything is blended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    /// `ease-linear`: `linear`.
    Linear,
    /// `ease-in`: `cubic-bezier(0.4, 0, 1, 1)`.
    In,
    /// `ease-out`: `cubic-bezier(0, 0, 0.2, 1)`.
    Out,
    /// `ease-in-out`: `cubic-bezier(0.4, 0, 0.2, 1)`.
    InOut,
    /// What `transition` uses without an `ease-*` class: Tailwind's default
    /// `cubic-bezier(0.4, 0, 0.2, 1)`.
    #[default]
    Default,
}

/// A CSS `cubic-bezier(x1, y1, x2, y2)` from (0, 0) to (1, 1).
#[derive(Clone, Copy)]
struct CubicBezier {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl CubicBezier {
    const TOLERANCE: f64 = 1e-7;

    fn coordinate(c1: f64, c2: f64, s: f64) -> f64 {
        let u = 1.0 - s;
        3.0 * u * u * s * c1 + 3.0 * u * s * s * c2 + s * s * s
    }

    fn slope(c1: f64, c2: f64, s: f64) -> f64 {
        let u = 1.0 - s;
        3.0 * u * u * c1 + 6.0 * u * s * (c2 - c1) + 3.0 * s * s * (1.0 - c2)
    }

    /// `y` at the point where `x` equals `x`. Newton's method from the
    /// linear guess, falling back to bisection where the slope is too flat to
    /// trust; `x(s)` is monotonic because both x control points are in 0..=1.
    fn at(self, x: f64) -> f64 {
        let (mut lo, mut hi) = (0.0, 1.0);
        let mut s = x;
        for _ in 0..16 {
            let error = Self::coordinate(self.x1, self.x2, s) - x;
            if error.abs() < Self::TOLERANCE {
                return Self::coordinate(self.y1, self.y2, s);
            }
            if error > 0.0 {
                hi = s;
            } else {
                lo = s;
            }
            let slope = Self::slope(self.x1, self.x2, s);
            let step = s - error / slope;
            s = if slope.abs() > 1e-6 && (lo..=hi).contains(&step) {
                step
            } else {
                (lo + hi) / 2.0
            };
        }
        while hi - lo > Self::TOLERANCE {
            s = (lo + hi) / 2.0;
            if Self::coordinate(self.x1, self.x2, s) < x {
                lo = s;
            } else {
                hi = s;
            }
        }
        Self::coordinate(self.y1, self.y2, (lo + hi) / 2.0)
    }
}

impl Easing {
    fn curve(self) -> Option<CubicBezier> {
        let bezier = |x1, y1, x2, y2| Some(CubicBezier { x1, y1, x2, y2 });
        match self {
            Self::Linear => None,
            Self::In => bezier(0.4, 0.0, 1.0, 1.0),
            Self::Out => bezier(0.0, 0.0, 0.2, 1.0),
            Self::InOut | Self::Default => bezier(0.4, 0.0, 0.2, 1.0),
        }
    }

    /// The eased progress for a linear one. Fixes 0 and 1.
    pub fn ease(self, linear: Progress) -> Progress {
        match self.curve() {
            None => linear,
            Some(curve) => Progress::new(curve.at(f64::from(f32::from(linear))) as f32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Easing; 5] = [
        Easing::Linear,
        Easing::In,
        Easing::Out,
        Easing::InOut,
        Easing::Default,
    ];

    fn at(easing: Easing, p: f32) -> f32 {
        easing.ease(Progress::new(p)).into()
    }

    #[test]
    fn every_curve_fixes_the_endpoints() {
        for easing in ALL {
            assert_eq!(easing.ease(Progress::START), Progress::START, "{easing:?}");
            assert_eq!(easing.ease(Progress::END), Progress::END, "{easing:?}");
        }
    }

    #[test]
    fn linear_is_the_identity() {
        for p in [0.1, 0.5, 0.9] {
            assert_eq!(at(Easing::Linear, p), p);
        }
    }

    #[test]
    fn the_default_curve_is_ease_in_out() {
        for p in [0.1, 0.3, 0.7] {
            assert_eq!(at(Easing::Default, p), at(Easing::InOut, p));
        }
    }

    /// Reference values at 0.25, 0.5 and 0.75 from a brute-force 80-step
    /// bisection of the Bezier definition, computed outside this crate.
    #[test]
    fn curves_match_known_css_values() {
        let cases = [
            (Easing::InOut, [0.2366, 0.7756, 0.9594]),
            (Easing::In, [0.0986, 0.3248, 0.6301]),
            (Easing::Out, [0.5776, 0.8392, 0.9642]),
        ];
        for (easing, expected) in cases {
            for (p, want) in [0.25, 0.5, 0.75].into_iter().zip(expected) {
                let got = at(easing, p);
                assert!(
                    (got - want).abs() < 1e-3,
                    "{easing:?} at {p}: {got} vs {want}"
                );
            }
        }
    }

    #[test]
    fn ease_in_lags_and_ease_out_leads_the_linear_clock() {
        assert!(at(Easing::In, 0.25) < 0.25);
        assert!(at(Easing::Out, 0.25) > 0.25);
    }

    #[test]
    fn every_curve_is_monotonic() {
        for easing in ALL {
            let samples: Vec<f32> = (0..=1000).map(|i| at(easing, i as f32 / 1000.0)).collect();
            assert!(
                samples.windows(2).all(|pair| pair[0] <= pair[1] + 1e-6),
                "{easing:?}"
            );
        }
    }
}
