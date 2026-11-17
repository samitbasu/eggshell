//! Arithmetic that cannot panic on the numbers an application or a layout
//! hands the library: NaN, infinities and bounds in the wrong order.

use egui::{Pos2, Rect, Vec2, vec2};

/// `value` kept within `low..=high`, without the panics of `f32::clamp`:
/// NaN is `low`, a NaN bound is no bound, and reversed bounds give `high`.
/// The result is NaN only when all three are.
pub(crate) fn clamp<T: egui::emath::Numeric>(value: T, low: T, high: T) -> T {
    T::from_f64(value.to_f64().max(low.to_f64()).min(high.to_f64()))
}

/// [`clamp`] per component.
pub(crate) fn clamp_vec2(value: Vec2, low: Vec2, high: Vec2) -> Vec2 {
    vec2(clamp(value.x, low.x, high.x), clamp(value.y, low.y, high.y))
}

/// `length`, or zero when it is not finite.
pub(crate) fn length_or_zero(length: f32) -> f32 {
    if length.is_finite() { length } else { 0.0 }
}

/// `rect`, or an empty one at `fallback` (or the origin, if that is not
/// finite either) when any of its coordinates is not finite: egui
/// debug-asserts on NaN rects.
pub(crate) fn rect_or_empty(rect: Rect, fallback: Pos2) -> Rect {
    if rect.is_finite() {
        rect
    } else if fallback.is_finite() {
        Rect::from_min_size(fallback, Vec2::ZERO)
    } else {
        Rect::ZERO
    }
}

#[cfg(test)]
mod tests {
    use egui::pos2;

    use super::*;

    #[test]
    fn clamp_is_total() {
        assert_eq!(clamp(5.0_f32, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0_f32, 0.0, 10.0), 0.0);
        assert_eq!(clamp(11.0_f64, 0.0, 10.0), 10.0);
        assert_eq!(clamp(f32::NAN, 0.0, 10.0), 0.0);
        assert_eq!(clamp(5.0_f32, f32::NAN, 3.0), 3.0);
        assert_eq!(clamp(5.0_f32, 7.0, f32::NAN), 7.0);
        assert_eq!(clamp(5.0_f32, 10.0, 0.0), 0.0, "reversed bounds give high");
        assert_eq!(clamp(f64::INFINITY, 0.0, 10.0), 10.0);
        assert!(clamp(f32::NAN, f32::NAN, f32::NAN).is_nan());
    }

    #[test]
    fn a_rect_that_is_not_finite_is_empty_at_the_fallback() {
        let fine = Rect::from_min_size(pos2(1.0, 2.0), vec2(3.0, 4.0));
        assert_eq!(rect_or_empty(fine, Pos2::ZERO), fine);
        let nan = Rect::from_min_size(pos2(f32::NAN, 2.0), vec2(3.0, 4.0));
        assert_eq!(
            rect_or_empty(nan, pos2(5.0, 6.0)),
            Rect::from_min_size(pos2(5.0, 6.0), Vec2::ZERO)
        );
        let wide = Rect::from_min_size(pos2(0.0, 0.0), vec2(f32::INFINITY, 1.0));
        assert!(!rect_or_empty(wide, Pos2::ZERO).any_nan());
        assert_eq!(rect_or_empty(wide, Pos2::ZERO).size(), Vec2::ZERO);
        assert_eq!(rect_or_empty(nan, pos2(f32::NAN, 0.0)), Rect::ZERO);
    }
}
