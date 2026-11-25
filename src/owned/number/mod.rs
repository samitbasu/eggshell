//! What sliders and steppers share: the step between their values, and the
//! keys that move them.

pub(crate) mod field;
pub(crate) mod readout;

pub use readout::NumberChain;

use std::ops::RangeInclusive;
use std::rc::Rc;

use egui::{Context, EventFilter, Id, Key, Modifiers, Ui};

use crate::Problem;
use crate::finite::clamp;
use crate::report::report;

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

/// TODO(prose): the values a slider or a stepper takes, from its start to its end; both are finite and the start is not past the end.
///
/// A range converted from `start..=end` that is not one is mended: an end
/// that is not finite becomes the other end (both, `0.0`), and reversed ends
/// are swapped. A widget given a mended range reports
/// [`Problem::BadRange`].
///
/// ```
/// use eggshell_ui::owned::number::ValueRange;
///
/// let range = ValueRange::from(0.0..=10.0);
/// assert_eq!((range.start(), range.end()), (0.0, 10.0));
/// assert_eq!(ValueRange::new(10.0, 0.0), None);
/// assert_eq!(ValueRange::from(10.0..=0.0), ValueRange::from(0.0..=10.0));
/// assert_eq!(ValueRange::from(f64::NAN..=4.0), ValueRange::from(4.0..=4.0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ValueRange {
    start: f64,
    end: f64,
    given: Given,
}

/// What a [`ValueRange`] was made from, kept to report a mended one.
#[derive(Clone, Copy, Debug)]
enum Given {
    Valid,
    Mended { start: f64, end: f64 },
}

impl ValueRange {
    /// TODO(prose): the range from `start` to `end`, if both are finite and `start` is not past `end`.
    ///
    /// ```
    /// use eggshell_ui::owned::number::ValueRange;
    ///
    /// assert!(ValueRange::new(0.0, 1.0).is_some());
    /// assert!(ValueRange::new(0.0, f64::INFINITY).is_none());
    /// ```
    pub fn new(start: f64, end: f64) -> Option<Self> {
        (start.is_finite() && end.is_finite() && start <= end).then_some(Self {
            start,
            end,
            given: Given::Valid,
        })
    }

    /// TODO(prose): the lowest value.
    ///
    /// ```
    /// use eggshell_ui::owned::number::ValueRange;
    ///
    /// assert_eq!(ValueRange::from(2.0..=3.0).start(), 2.0);
    /// ```
    pub fn start(self) -> f64 {
        self.start
    }

    /// TODO(prose): the highest value.
    ///
    /// ```
    /// use eggshell_ui::owned::number::ValueRange;
    ///
    /// assert_eq!(ValueRange::from(2.0..=3.0).end(), 3.0);
    /// ```
    pub fn end(self) -> f64 {
        self.end
    }

    /// The smallest range holding both.
    pub(super) fn union(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            given: Given::Valid,
        }
    }

    /// Whether `other` lies within this range.
    pub(super) fn holds(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub(super) fn contains(self, value: f64) -> bool {
        self.start <= value && value <= self.end
    }

    /// `value` within the range; NaN is the start.
    pub(super) fn clamp(self, value: f64) -> f64 {
        clamp(value, self.start, self.end)
    }

    /// Reports a mended range and a `value` that is not a number, for the
    /// widget `subject`; the value is then the start.
    #[track_caller]
    pub(super) fn checked(self, ctx: &Context, subject: Id, value: f64) -> f64 {
        if let Given::Mended { start, end } = self.given {
            report(ctx, Problem::BadRange, Some(subject), || {
                format!(
                    "the range {start:?}..={end:?} is reversed or not finite; it is {:?}..={:?}",
                    self.start, self.end
                )
            });
        }
        if value.is_nan() {
            report(ctx, Problem::NanValue, Some(subject), || {
                format!(
                    "the value is NaN; it is {:?}, the start of its range",
                    self.start
                )
            });
            self.start
        } else {
            value
        }
    }
}

impl PartialEq for ValueRange {
    fn eq(&self, other: &Self) -> bool {
        (self.start, self.end) == (other.start, other.end)
    }
}

impl From<RangeInclusive<f64>> for ValueRange {
    fn from(range: RangeInclusive<f64>) -> Self {
        let (start, end) = range.into_inner();
        Self::new(start, end).unwrap_or_else(|| {
            let (low, high) = match (start.is_finite(), end.is_finite()) {
                (true, true) => (start, end),
                (true, false) => (start, start),
                (false, true) => (end, end),
                (false, false) => (0.0, 0.0),
            };
            Self {
                start: low.min(high),
                end: low.max(high),
                given: Given::Mended { start, end },
            }
        })
    }
}

impl From<&RangeInclusive<f64>> for ValueRange {
    fn from(range: &RangeInclusive<f64>) -> Self {
        Self::from(range.clone())
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
    pub fn snap(self, value: f64, range: impl Into<ValueRange>) -> f64 {
        let range = range.into();
        self.snap_from(range.start, value, range)
    }

    /// The nearest value of `range` that is a whole number of steps from
    /// `origin`, or an end of `range` if that is nearer: a slider's typed
    /// value stays on its track's lattice, whatever the typed range.
    pub(super) fn snap_from(self, origin: f64, value: f64, range: ValueRange) -> f64 {
        let (low, high) = (range.start, range.end);
        let steps = ((value - origin) / self.0).round();
        let snapped = origin + steps * self.0;
        let tidy = format!("{snapped:.*}", self.decimals());
        let on_lattice = clamp(tidy.parse().unwrap_or(snapped), low, high);
        [high, low].into_iter().fold(on_lattice, |best, end| {
            if (end - value).abs() < (best - value).abs() {
                end
            } else {
                best
            }
        })
    }

    /// The distance itself.
    pub(super) fn size(self) -> f64 {
        self.0
    }

    /// `value` moved `steps` steps, kept within `range`.
    pub fn moved(self, value: f64, steps: i32, range: impl Into<ValueRange>) -> f64 {
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
    let Some(id) = crate::draw::node_id_or_report(ui, "holding the arrow keys") else {
        return;
    };
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
