//! TODO(prose): the skeleton, a loading placeholder after shadcn/ui's
//! Skeleton: a muted box that pulses.
//!
//! ```
//! use eggshell_ui::{Builder, tw};
//! use eggshell_ui_extras::skeleton::SkeletonExt;
//!
//! fn view(b: &mut Builder<'_, '_, ()>) {
//!     b.skeleton(tw().h_px(16.0).w_px(250.0));
//!     b.skeleton(tw().size_px(48.0).rounded_full());
//! }
//! ```

use std::time::Duration;

use eggshell_ui::{Builder, Easing, Opacity, Progress, Style, Tw, cycle, tw};
use egui::Color32;

/// TODO(prose): how skeletons look: their fill, corners and pulse.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SkeletonStyle {
    /// TODO(prose): the fill at full opacity.
    pub fill: Color32,
    /// TODO(prose): the corner radius.
    pub radius: u8,
    /// TODO(prose): how long one pulse takes: down to [`low`](Self::low) and
    /// back.
    pub period: Duration,
    /// TODO(prose): the opacity at the pulse's lowest.
    pub low: Opacity,
}

impl eggshell_ui::contract::Extension for SkeletonStyle {
    fn derive(style: &Style) -> Self {
        Self::from_style(style)
    }
}

impl SkeletonStyle {
    /// TODO(prose): a translucent grey over any ground, `rounded-md` of the
    /// style's menus, and Tailwind's `animate-pulse`: two seconds down to
    /// half.
    pub fn from_style(style: &Style) -> Self {
        Self {
            fill: Color32::from_gray(128).gamma_multiply(0.2),
            radius: style.menu().radius,
            period: Duration::from_secs(2),
            low: Opacity::new(0.5),
        }
    }
}

/// TODO(prose): the skeleton, added to a [`Builder`].
pub trait SkeletonExt {
    /// TODO(prose): a skeleton box laid out by `classes` (its size), drawn in
    /// the style's fill and pulsing. It is not announced to assistive
    /// technology, and under reduced motion it stands still at full opacity.
    fn skeleton(&mut self, classes: Tw);
}

impl<A: 'static> SkeletonExt for Builder<'_, '_, A> {
    fn skeleton(&mut self, classes: Tw) {
        let style = self.style().extension::<SkeletonStyle>();
        let opacity = cycle(self.ctx(), style.period)
            .map_or(Opacity::OPAQUE, |phase| pulse(phase, style.low));
        let styled = tw()
            .shrink_0()
            .bg(style.fill)
            .rounded_px(f32::from(style.radius))
            .opacity(opacity);
        self.div(styled.merge(classes), |_| {});
    }
}

/// The opacity `phase` of the way through a pulse: Tailwind's `pulse`
/// keyframes, 1 at the start, `low` at the middle, each half eased.
fn pulse(phase: Progress, low: Opacity) -> Opacity {
    let twice = f32::from(phase) * 2.0;
    let (from, to, along) = if twice < 1.0 {
        (1.0, f32::from(low), twice)
    } else {
        (f32::from(low), 1.0, twice - 1.0)
    };
    let eased = f32::from(Easing::Pulse.ease(Progress::new(along)));
    Opacity::new(from + (to - from) * eased)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(phase: f32) -> f32 {
        pulse(Progress::new(phase), Opacity::new(0.5)).into()
    }

    #[test]
    fn a_pulse_starts_and_ends_opaque_and_is_lowest_in_the_middle() {
        assert_eq!(at(0.0), 1.0);
        assert_eq!(at(1.0), 1.0);
        assert_eq!(at(0.5), 0.5);
    }

    #[test]
    fn the_two_halves_mirror_each_other() {
        for p in [0.1, 0.2, 0.3] {
            assert!((at(p) - at(1.0 - p)).abs() < 1e-5, "{p}");
        }
    }

    #[test]
    fn a_pulse_never_leaves_its_range() {
        for step in 0..=100 {
            let opacity = at(step as f32 / 100.0);
            assert!((0.5..=1.0).contains(&opacity), "{step}: {opacity}");
        }
    }
}
