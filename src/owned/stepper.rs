//! The owned stepper: a number field with two stacked buttons that step it.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;
use std::rc::Rc;

use egui::{Key, Role, WidgetInfo};

use super::number::{Step, pressed};
use super::{
    Chevron, Enablement, FieldHooks, Tip, centered, described, field_classes, paint_chevron, tipped,
};
use crate::builder::Slot;
use crate::prim::text_field::{self, Field};
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Field,
    Buttons,
    Up,
    Down,
}

/// A number between the ends of a range, edited as text and stepped by two
/// buttons, drawn from the [`Style`]. Start one with [`Builder::stepper`] and
/// finish it with [`on_change`](Self::on_change).
///
/// With the field focused, ArrowUp and ArrowDown step the value. Typed
/// numbers apply as they parse, are clamped to the range, and snap to the
/// step when the field loses focus. A button is disabled at its end of the
/// range. The buttons are not Tab stops: the keyboard steps in the field.
///
/// ```html
/// <div class="flex items-stretch gap-px divide-x divide-input w-28 h-9 rounded-md border border-input shadow-xs
///   focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50">
///   <div class="flex items-center grow px-3"><input inputmode="decimal"></div>
///   <div class="flex flex-col gap-px divide-y divide-input w-6">
///     <button class="grow rounded-tr-[5px]">▲</button><button class="grow rounded-br-[5px]">▼</button>
///   </div>
/// </div>
/// ```
#[must_use = "a stepper is not added until you call `.on_change(..)`"]
pub struct StepperChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    name: Option<String>,
    tooltip: Tip,
    classes: Tw,
    enablement: Enablement,
    hooks: FieldHooks<A>,
}

impl<'a, 'f, A: 'static> StepperChain<'a, 'f, A> {
    /// A stepper at `value` within `range`, stepping by [`Step::ONE`].
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        value: f64,
        range: RangeInclusive<f64>,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            value,
            range,
            step: Step::ONE,
            name: None,
            tooltip: None,
            classes: tw(),
            enablement: Enablement::Enabled,
            hooks: FieldHooks::default(),
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

    /// Classes merged over the styled ones of the whole stepper, such as its
    /// width, which is 112 points until then; the later class wins
    /// ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shown while the pointer hovers the control, as egui's `on_hover_text`
    /// shows it: with egui's timing and style, and also when the control is
    /// disabled, as an HTML `title` is. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Runs `then` with the field's response each frame: the escape hatch for
    /// what the builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.hooks.response.set(then);
        self
    }

    /// Runs `then` when Enter is pressed in the field.
    pub fn when_submitted(mut self, then: impl Fn(&mut A) + 'static) -> Self {
        self.hooks.submitted(then);
        self
    }

    /// Runs `then` when the field loses focus, also when Enter does it.
    pub fn when_blurred(mut self, then: impl Fn(&mut A) + 'static) -> Self {
        self.hooks.blurred(then);
        self
    }

    /// Whether the stepper takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the stepper. `set` runs with the new value.
    #[track_caller]
    pub fn on_change(self, set: impl Fn(&mut A, f64) + 'static) {
        let set = Rc::new(set);
        let Self {
            mut b,
            slot,
            value,
            range,
            step,
            name,
            tooltip,
            classes,
            enablement,
            hooks,
        } = self;
        let style = Style::of(b.ctx());
        let stepper = &style.stepper;
        let (low, high) = (*range.start(), *range.end());
        let spec = Spec {
            value,
            range,
            step,
            name: name.unwrap_or_default(),
            tooltip,
            hooks,
        };
        let group = enablement
            .fade(
                field_classes(&stepper.group, &style, stepper.radius, enablement)
                    .flex()
                    .items_stretch()
                    .gap(1.0 / 4.0)
                    .h_px(stepper.height)
                    .w_px(112.0),
                &style,
            )
            .merge(classes);
        let divider = match enablement {
            Enablement::Enabled => stepper.divider,
            Enablement::Disabled => enablement.colors(stepper.group.paint.states, &style).border,
        };
        let group = group.divide_x(1.0, divider);
        let field_box = centered(tw())
            .justify_start()
            .grow()
            .min_w_0()
            .px(stepper.padding_x / 4.0);
        let inner_radius = f32::from(stepper.radius) - stepper.group.border_width;
        let button = |at_end: bool, corner: fn(Tw, f32) -> Tw| {
            let enablement = if at_end {
                Enablement::Disabled
            } else {
                enablement
            };
            let classes = centered(stepper.button.classes(
                &style,
                0,
                enablement,
                stepper.transition,
            ))
            .focusable_none()
            .grow();
            corner(classes, inner_radius)
        };
        let (up, down) = (
            button(value >= high, Tw::rounded_tr_px),
            button(value <= low, Tw::rounded_br_px),
        );
        let glyph = stepper.glyph;
        let field = Field::singleline("").bare();
        let measured = field.clone();
        b.claim(&slot);
        b.scope_with(slot.id, &slot.label, |b| {
            let editing = b.id_of(Part::Field).with("editing");
            b.div_id(Part::Box, group, |b| {
                b.div(field_box, |b| {
                    let spec = spec.clone();
                    let set = set.clone();
                    b.leaf(Part::Field, tw().w_full().h_full(), move |m| {
                        text_field::measure(m, &measured)
                    })
                    .draw_widgets(move |app, ui| spec.run_field(app, ui, &field, editing, &*set));
                });
                let buttons = tw()
                    .flex()
                    .flex_col()
                    .gap(1.0 / 4.0)
                    .divide_y(1.0, divider)
                    .w_px(stepper.button_width)
                    .shrink_0();
                b.div_id(Part::Buttons, buttons, |b| {
                    for (part, classes, chevron, steps, label) in [
                        (Part::Up, up, Chevron::Up, 1, "Increase"),
                        (Part::Down, down, Chevron::Down, -1, "Decrease"),
                    ] {
                        let spec = spec.clone();
                        let set = set.clone();
                        let name = format!("{label} {}", spec.name);
                        b.div_id(part, classes, |b| {
                            b.paint(tw().size_px(glyph), paint_chevron(chevron))
                        })
                        .draw(move |app, ui| {
                            if described(ui, Role::Button, None, name.trim(), &spec.tooltip)
                                .clicked()
                            {
                                set(app, spec.step.moved(spec.value, steps, &spec.range));
                            }
                        });
                    }
                });
            });
        });
    }
}

/// What a stepper's field and buttons act on.
struct Spec<A> {
    value: f64,
    range: RangeInclusive<f64>,
    step: Step,
    name: String,
    tooltip: Tip,
    hooks: FieldHooks<A>,
}

impl<A> Clone for Spec<A> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            range: self.range.clone(),
            step: self.step,
            name: self.name.clone(),
            tooltip: self.tooltip.clone(),
            hooks: self.hooks.clone(),
        }
    }
}

impl<A> Spec<A> {
    /// Shows the field for a frame: the arrow keys step it, typing applies
    /// numbers as they parse, and losing focus snaps the number to the step.
    /// What is typed is kept in `editing` while the field is focused.
    fn run_field(
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
        let response = tipped(text_field::show(ui, &mut text, field), &self.tooltip);
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
        self.hooks.run(app, ui, &response);
    }
}
