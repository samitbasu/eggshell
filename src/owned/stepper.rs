//! The owned stepper: a number field with two stacked buttons that step it.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{Key, Role, WidgetInfo};

use super::number::{Step, pressed};
use super::{Chevron, Enablement, centered, described, field_classes, paint_chevron};
use crate::prim::text_field::{self, Field};
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Field,
    Up,
    Down,
}

/// A number between the ends of a range, edited as text and stepped by two
/// buttons, drawn from the [`Style`].
///
/// With the field focused, ArrowUp and ArrowDown step the value. Typed
/// numbers apply as they parse, are clamped to the range, and snap to the
/// step when the field loses focus. A button is disabled at its end of the
/// range. The buttons are not Tab stops: the keyboard steps in the field.
///
/// ```html
/// <div class="flex items-start gap-px w-28">
///   <div class="flex items-center grow h-9 px-3 rounded-md border border-input shadow-xs
///     focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50"><input inputmode="decimal"></div>
///   <div class="flex flex-col gap-px"><button class="w-6 h-[17.5px]">▲</button><button>▼</button></div>
/// </div>
/// ```
#[derive(Clone, Debug)]
pub struct Stepper {
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    name: Option<String>,
    adjust: fn(Tw) -> Tw,
    enablement: Enablement,
}

impl Stepper {
    /// A stepper at `value` within `range`, stepping by [`Step::ONE`].
    pub fn new(value: f64, range: RangeInclusive<f64>) -> Self {
        Self {
            value,
            range,
            step: Step::ONE,
            name: None,
            adjust: |classes| classes.w_px(112.0),
            enablement: Enablement::Enabled,
        }
    }

    /// The distance one button press moves the value.
    pub fn step(mut self, step: Step) -> Self {
        self.step = step;
        self
    }

    /// The accessible name, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Changes the classes of the whole stepper, such as its width; it is 112
    /// points wide until then.
    pub fn classes(mut self, adjust: fn(Tw) -> Tw) -> Self {
        self.adjust = adjust;
        self
    }

    /// Whether the stepper takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the stepper to `b` under `key`. `set` runs with the new value.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        set: impl Fn(&mut A, f64) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let stepper = &style.stepper;
        let Self {
            value,
            range,
            step,
            name,
            adjust,
            enablement,
        } = self;
        let (low, high) = (*range.start(), *range.end());
        let spec = Spec {
            value,
            range,
            step,
            name: name.unwrap_or_default(),
        };
        let row = adjust(tw().flex().items_start().gap(stepper.gap / 4.0));
        let field_box = field_classes(&stepper.field, &style, stepper.field_radius, enablement)
            .flex()
            .items_center()
            .grow()
            .min_w_0()
            .h_px(stepper.height)
            .px(stepper.padding_x / 4.0);
        let button = |at_end: bool| {
            let enablement = if at_end {
                Enablement::Disabled
            } else {
                enablement
            };
            centered(stepper.button.classes(
                &style,
                stepper.button_radius,
                enablement,
                stepper.transition,
            ))
            .focusable_none()
            .w_px(stepper.button_size.x)
            .h_px(stepper.button_size.y)
        };
        let (up, down) = (button(value >= high), button(value <= low));
        let glyph = stepper.glyph;
        let field = Field::singleline("").bare();
        let measured = field.clone();
        b.scope(key, |b| {
            let editing = b.id_of(Part::Field).with("editing");
            b.div(row, |b| {
                b.div_id(Part::Box, field_box, crate::none, |b| {
                    let spec = spec.clone();
                    b.leaf(
                        Part::Field,
                        tw().w_full(),
                        move |m| text_field::measure(m, &measured),
                        move |app, ui| spec.run_field(app, ui, &field, editing, set),
                    );
                });
                b.div(tw().flex().flex_col().gap(stepper.gap / 4.0), |b| {
                    for (part, classes, chevron, steps, label) in [
                        (Part::Up, up, Chevron::Up, 1, "Increase"),
                        (Part::Down, down, Chevron::Down, -1, "Decrease"),
                    ] {
                        let spec = spec.clone();
                        let name = format!("{label} {}", spec.name);
                        b.div_id(
                            part,
                            classes,
                            move |app, ui| {
                                if described(ui, Role::Button, None, name.trim()).clicked() {
                                    set(app, spec.step.moved(spec.value, steps, &spec.range));
                                }
                            },
                            |b| b.paint(tw().size_px(glyph), paint_chevron(chevron)),
                        );
                    }
                });
            });
        });
    }
}

/// What a stepper's field and buttons act on.
#[derive(Clone, Debug)]
struct Spec {
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    name: String,
}

impl Spec {
    /// Shows the field for a frame: the arrow keys step it, typing applies
    /// numbers as they parse, and losing focus snaps the number to the step.
    /// What is typed is kept in `editing` while the field is focused.
    fn run_field<A>(
        &self,
        app: &mut A,
        ui: &mut egui::Ui,
        field: &Field,
        editing: egui::Id,
        set: impl Fn(&mut A, f64),
    ) {
        let parse = |text: &str| text.trim().parse::<f64>().ok();
        let (low, high) = (*self.range.start(), *self.range.end());
        let mut value = self.value;
        let mut typed = ui.data(|data| data.get_temp::<String>(editing));
        if ui.memory(|memory| memory.has_focus(Field::id_in(ui))) {
            let current = typed.as_deref().and_then(parse).unwrap_or(value);
            let steps =
                i32::from(pressed(ui, Key::ArrowUp)) - i32::from(pressed(ui, Key::ArrowDown));
            if steps != 0 {
                value = self.step.moved(current, steps, &self.range);
                typed = None;
                set(app, value);
            }
        }
        let mut text = typed.unwrap_or_else(|| format!("{value:.*}", self.step.decimals()));
        let response = text_field::show(ui, &mut text, field);
        response.widget_info(|| WidgetInfo {
            label: Some(self.name.clone()),
            ..WidgetInfo::drag_value(ui.is_enabled(), value)
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_min_numeric_value(low);
            node.set_max_numeric_value(high);
        });
        if response.changed() {
            let number = parse(&text).map(|number| number.clamp(low, high));
            if let Some(number) = number.filter(|&number| number != self.value) {
                set(app, number);
            }
            ui.data_mut(|data| data.insert_temp(editing, text));
        } else if response.lost_focus() {
            if let Some(number) = parse(&text) {
                set(app, self.step.snap(number, &self.range));
            }
            ui.data_mut(|data| data.remove::<String>(editing));
        }
    }
}
