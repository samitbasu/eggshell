//! What sliders and steppers share: the step between their values, and the
//! keys that move them.

use std::ops::RangeInclusive;

use egui::{EventFilter, Key, Modifiers, Ui};

use crate::NodeUi as _;

/// The distance between the values of a slider or a stepper, always positive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step(f64);

impl Step {
    /// One whole unit.
    pub const ONE: Self = Self(1.0);

    /// A step of `size`; one that is not positive, or not a number, is
    /// [`ONE`](Self::ONE).
    pub fn new(size: f64) -> Self {
        if size > 0.0 && size.is_finite() {
            Self(size)
        } else {
            Self::ONE
        }
    }

    /// The nearest value of `range` that is a whole number of steps from its
    /// start, or its end if that is nearer than the last whole step.
    pub fn snap(self, value: f64, range: &RangeInclusive<f64>) -> f64 {
        let (low, high) = (*range.start(), *range.end());
        let steps = ((value - low) / self.0).round();
        let snapped = low + steps * self.0;
        let tidy = format!("{snapped:.*}", self.decimals());
        tidy.parse()
            .map_or(snapped, |tidy: f64| tidy.clamp(low, high))
    }

    /// `value` moved `steps` steps, kept within `range`.
    pub fn moved(self, value: f64, steps: i32, range: &RangeInclusive<f64>) -> f64 {
        self.snap(value + f64::from(steps) * self.0, range)
    }

    /// The digits after the point that show a multiple of the step.
    pub fn decimals(self) -> usize {
        self.0
            .to_string()
            .split_once('.')
            .map_or(0, |(_, rest)| rest.len())
    }
}

impl Default for Step {
    fn default() -> Self {
        Self::ONE
    }
}

/// How far PageUp and PageDown move, in steps.
pub(super) const PAGE: i32 = 10;

/// Keeps the arrow keys (and, with `escape`, Escape) on the focused node, so
/// they do not move the focus.
pub(super) fn hold_keys(ui: &Ui, escape: bool) {
    let filter = EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        escape,
        ..EventFilter::default()
    };
    let id = ui.node_id();
    ui.ctx()
        .memory_mut(|memory| memory.set_focus_lock_filter(id, filter));
}

/// Consumes `key` if it was pressed.
pub(super) fn pressed(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|input| input.consume_key(Modifiers::NONE, key))
}

/// How many steps the keys pressed this frame move a value: arrows by one,
/// Page keys by [`PAGE`]. Consumes them.
pub(super) fn steps_pressed(ui: &Ui) -> i32 {
    let by = |up: &[Key], down: &[Key], amount: i32| {
        let up = up.iter().fold(false, |any, &key| pressed(ui, key) | any);
        let down = down.iter().fold(false, |any, &key| pressed(ui, key) | any);
        amount * (i32::from(up) - i32::from(down))
    };
    by(
        &[Key::ArrowUp, Key::ArrowRight],
        &[Key::ArrowDown, Key::ArrowLeft],
        1,
    ) + by(&[Key::PageUp], &[Key::PageDown], PAGE)
}
