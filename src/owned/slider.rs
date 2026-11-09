//! The owned slider: a track with a range and a thumb, which a pointer drags
//! and the keyboard steps.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{Key, Rect, Ui, WidgetInfo};

use super::number::{self, Step};
use super::{Enablement, painted};
use crate::{Builder, NodeUi as _, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Track,
    Thumb,
}

/// A horizontal slider drawn from the [`Style`].
///
/// Pressing the track jumps the thumb there, and dragging the track or the
/// thumb moves it. With the thumb focused, the arrow keys move it a step and
/// PageUp and PageDown ten, and Home and End go to the ends of the range.
///
/// ```html
/// <span class="relative flex w-full items-center h-4">
///   <span class="h-1.5 w-full rounded-full bg-muted overflow-hidden">
///     <span class="h-full rounded-full bg-primary" style="width: 40%"></span>
///   </span>
///   <span role="slider" class="absolute size-4 rounded-full border border-primary bg-background shadow-sm
///     focus-visible:ring-[3px] focus-visible:ring-ring/50"></span>
/// </span>
/// ```
#[derive(Clone, Debug)]
pub struct Slider {
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    name: Option<String>,
    classes: Tw,
    enablement: Enablement,
}

/// What a draw closure needs to turn the pointer and the keys into a value.
#[derive(Clone, Debug)]
struct Scale {
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    thumb: f32,
}

impl Scale {
    /// The value under horizontal position `x` of a track occupying `track`;
    /// the thumb's centre travels the track less one thumb.
    fn at(&self, x: f32, track: Rect) -> f64 {
        let (low, high) = (*self.range.start(), *self.range.end());
        let travel = track.width() - self.thumb;
        if travel <= 0.0 {
            return low;
        }
        let along = ((x - track.left() - self.thumb / 2.0) / travel).clamp(0.0, 1.0);
        self.step
            .snap(low + f64::from(along) * (high - low), &self.range)
    }

    /// The value after the keys pressed this frame, if any.
    fn after_keys(&self, ui: &Ui) -> Option<f64> {
        number::hold_keys(ui, false);
        let ends = [
            (Key::Home, *self.range.start()),
            (Key::End, *self.range.end()),
        ];
        let end = ends.into_iter().fold(None, |end, (key, value)| {
            number::pressed(ui, key).then_some(value).or(end)
        });
        let steps = number::steps_pressed(ui);
        match (end, steps) {
            (Some(value), _) => Some(value),
            (None, 0) => None,
            (None, steps) => Some(self.step.moved(self.value, steps, &self.range)),
        }
    }
}

impl Slider {
    /// A slider at `value` within `range`, moving by [`Step::ONE`].
    pub fn new(value: f64, range: RangeInclusive<f64>) -> Self {
        Self {
            value,
            range,
            step: Step::ONE,
            name: None,
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }

    /// The distance between the values the slider takes.
    pub fn step(mut self, step: Step) -> Self {
        self.step = step;
        self
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the whole slider (its hit area, which is as wide as the space it is given), such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Whether the slider takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the slider to `b` under `key`, as wide as the space it is given.
    /// `set` runs with the new value when the pointer or the keyboard changes it.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        set: impl Fn(&mut A, f64) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let slider = &style.slider;
        let enablement = self.enablement;
        let (low, high) = (*self.range.start(), *self.range.end());
        let fraction = if high > low {
            ((self.value - low) / (high - low)).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let thumb = slider.thumb_size;
        let radius = f32::from(slider.radius);
        let fill = |states| enablement.colors(states, &style);
        let hit = tw()
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h_px(thumb)
            .sense_click_drag()
            .focusable_none();
        let hit = enablement.apply(hit, &style).merge(self.classes);
        let bar = painted(
            tw().w_full()
                .h_px(slider.track_height)
                .rounded_px(radius)
                .overflow_hidden(),
            fill(slider.track),
        );
        let range = painted(
            tw().h_full().w_frac(fraction, 1.0).rounded_px(radius),
            fill(slider.range),
        );
        let travel = tw().absolute().left_px(0.0).right_px(thumb).inset_y(0.0);
        let knob = slider
            .thumb
            .classes(&style, slider.radius, enablement, slider.transition)
            .absolute()
            .top_px(0.0)
            .left_frac(fraction, 1.0)
            .size_px(thumb)
            .sense_click_drag();
        let scale = Scale {
            value: self.value,
            range: self.range,
            step: self.step,
            thumb,
        };
        let name = self.name.unwrap_or_default();
        b.scope(key, |b| {
            let track = b.id_of(Part::Track);
            let on_track = scale.clone();
            b.div_id(
                Part::Track,
                hit,
                move |app, ui| {
                    let response = ui.node_response();
                    let pressed = response.is_pointer_button_down_on();
                    if let (true, Some(at)) = (pressed, response.interact_pointer_pos()) {
                        let value = on_track.at(at.x, response.rect);
                        if value != on_track.value {
                            set(app, value);
                        }
                    }
                },
                |b| {
                    b.div(bar, |b| b.div(range, |_| {}));
                    b.div(travel, |b| {
                        b.div_id(
                            Part::Thumb,
                            knob,
                            move |app, ui| {
                                let response = ui.node_response();
                                let track = ui.ctx().read_response(track).map(|r| r.rect);
                                let pointer = response
                                    .interact_pointer_pos()
                                    .zip(track)
                                    .filter(|_| response.is_pointer_button_down_on())
                                    .map(|(at, track)| scale.at(at.x, track));
                                let keys = if response.has_focus() {
                                    scale.after_keys(ui)
                                } else {
                                    None
                                };
                                if let Some(value) = keys.or(pointer).filter(|&v| v != scale.value)
                                {
                                    set(app, value);
                                }
                                response.widget_info(|| {
                                    WidgetInfo::slider(ui.is_enabled(), scale.value, &name)
                                });
                                let (low, high) = (*scale.range.start(), *scale.range.end());
                                ui.ctx().accesskit_node_builder(response.id, |node| {
                                    node.set_min_numeric_value(low);
                                    node.set_max_numeric_value(high);
                                });
                            },
                            |_| {},
                        );
                    });
                },
            );
        });
    }
}
