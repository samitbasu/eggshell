//! The splitter: a handle between two panes of a flex container that sizes
//! one of them, which the pointer drags and the keyboard steps.

use std::ops::{Bound, RangeBounds};
use std::rc::Rc;

use egui::{CursorIcon, Key, Modifiers, Pos2, Rect, Role, Stroke, StrokeKind, Ui, Vec2, pos2};
use taffy::FlexDirection;

use super::number;
use crate::builder::Slot;
use crate::contract::{Focus, SplitterStyle};
use crate::draw::pane_rect;
use crate::plane::Axis;
use crate::{Builder, Easing, NodeUi as _, Progress, transition, tw};

/// TODO(prose): a pane's size along its container's axis, in points; never
/// negative or NaN.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct PaneSize(f32);

impl PaneSize {
    /// TODO(prose): `points`, at least zero; NaN is zero.
    pub fn new(points: f32) -> Self {
        Self(if points.is_nan() {
            0.0
        } else {
            points.max(0.0)
        })
    }
}

impl From<f32> for PaneSize {
    fn from(points: f32) -> Self {
        Self::new(points)
    }
}

impl From<PaneSize> for f32 {
    fn from(size: PaneSize) -> f32 {
        size.0
    }
}

/// Which neighbour of the splitter it sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Before,
    After,
}

/// The sizes a pane may take.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Limits {
    min: PaneSize,
    max: Option<PaneSize>,
}

impl Limits {
    const ANY: Self = Self {
        min: PaneSize(0.0),
        max: None,
    };

    /// The limits `range` sets, or `Err` with them ended at their start
    /// when `range` ends before it starts.
    fn of(range: impl RangeBounds<f32>) -> Result<Self, Self> {
        let end = |bound: Bound<&f32>| match bound {
            Bound::Included(&points) | Bound::Excluded(&points) => Some(PaneSize::new(points)),
            Bound::Unbounded => None,
        };
        let min = end(range.start_bound()).unwrap_or_default();
        match end(range.end_bound()) {
            Some(max) if max < min => Err(Self {
                min,
                max: Some(min),
            }),
            max => Ok(Self { min, max }),
        }
    }

    fn clamp(self, points: f32) -> PaneSize {
        let at_least = PaneSize::new(points).max(self.min);
        self.max.map_or(at_least, |max| at_least.min(max))
    }
}

impl PaneSize {
    fn max(self, other: Self) -> Self {
        if other > self { other } else { self }
    }

    fn min(self, other: Self) -> Self {
        if other < self { other } else { self }
    }
}

/// The axis a splitter moves along, and which way along it grows its pane.
#[derive(Clone, Copy, Debug)]
struct Travel {
    axis: Axis,
    grows: Grows,
}

/// Whether the splitter is hovered, dragged or focused, and drawn in its hot
/// colours, or at rest.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Heat {
    Rest,
    Hot,
}

/// Whether moving the handle towards the end of the axis grows the pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grows {
    TowardsEnd,
    TowardsStart,
}

/// The arrow keys that move a handle along `axis`, towards the start first.
fn keys(axis: Axis) -> (Key, Key) {
    match axis {
        Axis::X => (Key::ArrowLeft, Key::ArrowRight),
        Axis::Y => (Key::ArrowUp, Key::ArrowDown),
    }
}

fn cursor(axis: Axis) -> CursorIcon {
    match axis {
        Axis::X => CursorIcon::ResizeColumn,
        Axis::Y => CursorIcon::ResizeRow,
    }
}

impl Travel {
    fn of(direction: FlexDirection, side: Side) -> Self {
        let (horizontal, reversed) = match direction {
            FlexDirection::Row => (Axis::X, false),
            FlexDirection::RowReverse => (Axis::X, true),
            FlexDirection::Column => (Axis::Y, false),
            FlexDirection::ColumnReverse => (Axis::Y, true),
        };
        let grows = if (side == Side::Before) != reversed {
            Grows::TowardsEnd
        } else {
            Grows::TowardsStart
        };
        Self {
            axis: horizontal,
            grows,
        }
    }

    /// How much the pane grows when the handle moves `by` along the axis.
    fn growth(self, by: f32) -> f32 {
        match self.grows {
            Grows::TowardsEnd => by,
            Grows::TowardsStart => -by,
        }
    }
}

/// Where a press on the handle started, and the pane's size then.
#[derive(Clone, Copy, Debug)]
struct Press {
    pointer: f32,
    size: f32,
}

/// The arrow keys' steps, in points.
const STEP: f32 = 10.0;
const SHIFT_STEP: f32 = 50.0;

/// TODO(prose): a splitter that does not yet know which pane it sizes. Start
/// one with [`Builder::splitter`].
///
/// ```compile_fail
/// # use eggshell_ui::Builder;
/// # fn view(b: &mut Builder<'_, '_, f32>) {
/// b.splitter("left").on_resize(|_: &mut f32, _| {});
/// # }
/// ```
///
/// ```compile_fail
/// # use eggshell_ui::Builder;
/// # use eggshell_ui::owned::PaneSize;
/// # fn view(b: &mut Builder<'_, '_, f32>) {
/// b.splitter("left")
///     .before(PaneSize::new(240.0))
///     .after(PaneSize::new(240.0))
///     .on_resize(|_: &mut f32, _| {});
/// # }
/// ```
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # use eggshell_ui::Builder;
/// # use eggshell_ui::owned::PaneSize;
/// # fn view(b: &mut Builder<'_, '_, f32>) {
/// b.splitter("left").before(PaneSize::new(240.0)).range(160.0..=480.0);
/// # }
/// ```
#[must_use = "a splitter is not added until you call `.before(..)` or `.after(..)`, then `.on_resize(..)`"]
pub struct SplitterStart<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
}

impl<'a, 'f, A: 'static> SplitterStart<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K) -> Self {
        Self {
            slot: b.slot(&key),
            b,
        }
    }

    /// TODO(prose): sizes the sibling before the splitter, now `size`.
    pub fn before(self, size: PaneSize) -> SplitterChain<'a, 'f, A> {
        SplitterChain::new(self, Side::Before, size)
    }

    /// TODO(prose): sizes the sibling after the splitter, now `size`.
    pub fn after(self, size: PaneSize) -> SplitterChain<'a, 'f, A> {
        SplitterChain::new(self, Side::After, size)
    }
}

/// TODO(prose): a splitter sizing one of its neighbours. Finish it with
/// [`on_resize`](Self::on_resize).
///
/// ```
/// use eggshell_ui::owned::PaneSize;
/// use eggshell_ui::{Builder, tw};
///
/// struct App {
///     sidebar: PaneSize,
/// }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.div(tw().flex().size_full(), |b| {
///         b.div(tw().w_px(app.sidebar.into()).shrink_0(), |_| {});
///         b.splitter("left")
///             .before(app.sidebar)
///             .range(160.0..=480.0)
///             .reset_to(240.0)
///             .named("Sidebar")
///             .on_resize(|app: &mut App, size| app.sidebar = size);
///         b.div(tw().flex_1(), |_| {});
///     });
/// }
/// ```
#[must_use = "a splitter is not added until you call `.on_resize(..)`"]
pub struct SplitterChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    side: Side,
    size: PaneSize,
    limits: Limits,
    reset: Option<PaneSize>,
    name: Option<String>,
}

impl<'a, 'f, A> SplitterChain<'a, 'f, A> {
    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        crate::owned::options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        crate::owned::options::when_some(self, value, then)
    }
}

impl<'a, 'f, A: 'static> SplitterChain<'a, 'f, A> {
    fn new(start: SplitterStart<'a, 'f, A>, side: Side, size: PaneSize) -> Self {
        Self {
            b: start.b,
            slot: start.slot,
            side,
            size,
            limits: Limits::ANY,
            reset: None,
            name: None,
        }
    }

    /// TODO(prose): the sizes the pane may take, in points: `a..=b`, `a..`
    /// or `..=b`.
    #[track_caller]
    pub fn range(mut self, range: impl RangeBounds<f32>) -> Self {
        let bounds = (range.start_bound().cloned(), range.end_bound().cloned());
        let limits = Limits::of(bounds).unwrap_or_else(|ended| {
            crate::report::report(
                self.b.ctx(),
                crate::Problem::BadRange,
                Some(self.slot.id),
                || {
                    format!(
                        "the splitter range {bounds:?} ends before it starts; it ends at its start"
                    )
                },
            );
            ended
        });
        self.limits = limits;
        self
    }

    /// TODO(prose): the size a double-click puts the pane back to.
    pub fn reset_to(mut self, size: impl Into<PaneSize>) -> Self {
        self.reset = Some(size.into());
        self
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): adds the splitter; `set` runs with the pane's new size.
    #[track_caller]
    pub fn on_resize(self, set: impl Fn(&mut A, PaneSize) + 'static) {
        let Self {
            mut b,
            slot,
            side,
            size,
            limits,
            reset,
            name,
        } = self;
        let direction = b.flex();
        if direction.is_none() {
            crate::report::report(
                b.ctx(),
                crate::Problem::MisplacedSplitter,
                Some(slot.id),
                || {
                    format!(
                        "the splitter {} is not in a flex container; it travels as in a row",
                        slot.readable()
                    )
                },
            );
        }
        let travel = Travel::of(direction.unwrap_or(FlexDirection::Row), side);
        let (style, ring) = {
            let style = b.style().clone();
            (style.splitter, style.focus)
        };
        let thickness = style.thickness.max(0.0);
        let classes = match travel.axis {
            Axis::X => tw().w_px(thickness).self_stretch(),
            Axis::Y => tw().h_px(thickness).self_stretch(),
        }
        .shrink_0()
        .sense_click_drag();
        let set = Rc::new(set);
        let handle = Handle {
            travel,
            size,
            limits,
            reset,
            style,
            ring,
        };
        if b.ctx().is_being_dragged(slot.id) {
            b.size_in_flight();
        }
        let node = b
            .div_id_with(&slot, classes, |_| {})
            .role(Role::Splitter)
            .sizes_pane(side);
        let node = match name {
            Some(name) => node.named(name),
            None => node,
        };
        node.draw(move |app, ui| handle.draw(ui, |size| set(app, size)));
    }
}

/// What a splitter's draw closure needs.
struct Handle {
    travel: Travel,
    size: PaneSize,
    limits: Limits,
    reset: Option<PaneSize>,
    style: SplitterStyle,
    ring: Focus,
}

impl Handle {
    fn draw(&self, ui: &Ui, set: impl FnOnce(PaneSize)) {
        let response = ui.node_response();
        let axis = self.travel.axis;
        let laid = pane_rect(ui).map_or(f32::from(self.size), |pane| axis.of(pane.size()));
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(cursor(axis));
        }
        let wanted = self
            .dragged_to(ui, &response, laid)
            .or_else(|| {
                response
                    .double_clicked()
                    .then_some(self.reset)
                    .flatten()
                    .map(f32::from)
            })
            .or_else(|| {
                response
                    .has_focus()
                    .then(|| self.keyed_to(ui, laid))
                    .flatten()
            });
        if let Some(size) = wanted.map(|points| self.limits.clamp(points))
            && size != self.size
        {
            set(size);
        }
        let heat = if response.hovered() || response.dragged() || response.has_focus() {
            Heat::Hot
        } else {
            Heat::Rest
        };
        self.paint(ui, response.rect, heat);
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                0.0,
                Stroke::new(self.ring.width, self.ring.color),
                StrokeKind::Inside,
            );
        }
        let (min, max) = (self.limits.min, self.limits.max);
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_numeric_value(f64::from(f32::from(self.size)));
            node.set_min_numeric_value(f64::from(f32::from(min)));
            if let Some(max) = max {
                node.set_max_numeric_value(f64::from(f32::from(max)));
            }
            node.set_orientation(match axis {
                Axis::X => egui::accesskit::Orientation::Vertical,
                Axis::Y => egui::accesskit::Orientation::Horizontal,
            });
        });
    }

    /// Where a drag puts the pane: its size at the press plus the pointer's
    /// travel since, so that clamping never accumulates drift.
    fn dragged_to(&self, ui: &Ui, response: &egui::Response, laid: f32) -> Option<f32> {
        let id = response.id.with("press");
        let along = |at: Pos2| self.travel.axis.at(at);
        if response.drag_started() {
            let origin = ui.input(|input| input.pointer.press_origin());
            if let Some(origin) = origin.or(response.interact_pointer_pos()) {
                let press = Press {
                    pointer: along(origin),
                    size: laid,
                };
                ui.data_mut(|data| data.insert_temp(id, press));
            }
        }
        let press = ui.data(|data| data.get_temp::<Press>(id));
        if response.drag_stopped() {
            ui.data_mut(|data| data.remove::<Press>(id));
        }
        let (press, at) = press.zip(response.interact_pointer_pos())?;
        response
            .dragged()
            .then(|| press.size + self.travel.growth(along(at) - press.pointer))
    }

    /// Where the keys pressed this frame put the pane.
    fn keyed_to(&self, ui: &Ui, laid: f32) -> Option<f32> {
        number::hold_arrows(ui);
        let (start, end) = keys(self.travel.axis);
        let consume = |modifiers, key| ui.input_mut(|input| input.consume_key(modifiers, key));
        let stepped = [(Modifiers::SHIFT, SHIFT_STEP), (Modifiers::NONE, STEP)]
            .into_iter()
            .find_map(|(modifiers, step)| {
                let by = if consume(modifiers, end) {
                    step
                } else if consume(modifiers, start) {
                    -step
                } else {
                    return None;
                };
                Some(laid + self.travel.growth(by))
            });
        let home = number::pressed(ui, Key::Home).then_some(f32::from(self.limits.min));
        let end = number::pressed(ui, Key::End)
            .then_some(self.limits.max)
            .flatten()
            .map(f32::from);
        stepped.or(home).or(end)
    }

    fn paint(&self, ui: &Ui, rect: Rect, heat: Heat) {
        let style = &self.style;
        let target = match heat {
            Heat::Hot => Progress::END,
            Heat::Rest => Progress::START,
        };
        let progress = match crate::draw::node_id_or_report(ui, "the splitter's hover transition") {
            Some(id) => transition::animate(
                ui.ctx(),
                id.with("hot"),
                target,
                style.transition,
                Easing::Default,
            ),
            None => target,
        };
        let colors = style.rest.towards(style.hot, progress);
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, colors.band);
        let centre = rect.center();
        let (from, to) = match self.travel.axis {
            Axis::X => (pos2(centre.x, rect.top()), pos2(centre.x, rect.bottom())),
            Axis::Y => (pos2(rect.left(), centre.y), pos2(rect.right(), centre.y)),
        };
        painter.line_segment([from, to], Stroke::new(style.line_width, colors.line));
        if let Some(grip) = style.grip {
            let size = match self.travel.axis {
                Axis::X => Vec2::new(grip.thickness, grip.length),
                Axis::Y => Vec2::new(grip.length, grip.thickness),
            };
            let pill = Rect::from_center_size(centre, size);
            painter.rect_filled(pill, grip.thickness / 2.0, colors.grip);
        }
    }
}
