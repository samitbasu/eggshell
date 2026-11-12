//! What sliders and steppers share: the step between their values, and the
//! keys that move them.

use std::ops::RangeInclusive;
use std::rc::Rc;

use egui::{EventFilter, Key, Modifiers, Ui};

use crate::NodeUi as _;

/// The start or the end of an edit of a slider or a stepper, for what the
/// application does while a value is being changed (not re-syncing a draft
/// from live state) or only when the change is done (sending an intent).
///
/// One edit is one gesture: a press on the thumb or the track to its release;
/// one key press, which begins and ends at once; for a stepper's field, the
/// typing from the first change to the field losing focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    /// The edit starts, before the first value it sets.
    Began,
    /// The edit is over, after the last value it sets.
    Ended,
}

/// The `on_edit` handler of a slider or a stepper, if it has one.
pub(super) struct Edits<A>(Option<Rc<EditFn<A>>>);

type EditFn<A> = dyn Fn(&mut A, Edit);

impl<A> Clone for Edits<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<A> Default for Edits<A> {
    fn default() -> Self {
        Self(None)
    }
}

impl<A> Edits<A> {
    pub(super) fn set(&mut self, then: impl Fn(&mut A, Edit) + 'static) {
        self.0 = Some(Rc::new(then));
    }

    pub(super) fn emit(&self, app: &mut A, edit: Edit) {
        if let Some(then) = &self.0 {
            then(app, edit);
        }
    }

    /// Runs `change` as one edit.
    pub(super) fn once(&self, app: &mut A, change: impl FnOnce(&mut A)) {
        self.emit(app, Edit::Began);
        change(app);
        self.emit(app, Edit::Ended);
    }

    /// Runs `change` in the frame of a pointer press on the node `id`,
    /// which is `down` while the button is held on it. Call it every frame:
    /// the press is [`Began`](Edit::Began) when `down` turns on, and the
    /// release [`Ended`](Edit::Ended) when it turns off.
    pub(super) fn pointer(
        &self,
        app: &mut A,
        ui: &Ui,
        (id, down): (egui::Id, bool),
        change: impl FnOnce(&mut A),
    ) {
        let key = id.with("eggshell-edit");
        let was = ui.data(|data| data.get_temp::<bool>(key)).unwrap_or(false);
        ui.data_mut(|data| data.insert_temp(key, down));
        if down && !was {
            self.emit(app, Edit::Began);
        }
        change(app);
        if was && !down {
            self.emit(app, Edit::Ended);
        }
    }
}

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
        let on_lattice = tidy
            .parse()
            .map_or(snapped, |tidy: f64| tidy.clamp(low, high));
        if (high - value).abs() < (on_lattice - value).abs() {
            high
        } else {
            on_lattice
        }
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

/// Keeps the arrow keys on the focused node, so they do not move the focus.
pub(super) fn hold_arrows(ui: &Ui) {
    let filter = EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
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
