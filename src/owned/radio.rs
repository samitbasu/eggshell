//! The owned radio group: circles that choose one value, with arrow keys.

use std::rc::Rc;

use egui::Role;

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::{Enablement, OnResponse, Tooltip, choice_label, choice_row, described};
use crate::builder::Slot;
use crate::contract::Size;
use crate::overlay::{Nav, Wrap};
use crate::{Builder, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Circle(usize),
    Label(usize),
}

/// Radio buttons that choose one of `options`, drawn from the [`Style`](crate::Style). Start
/// a group with [`Builder::radio_group`] and finish it with
/// [`on_change`](Self::on_change).
///
/// The group is one Tab stop, on the chosen option (the first enabled one when
/// none is chosen), as in HTML. The arrow keys move the choice, and the focus
/// with it, to the next or previous enabled option, wrapping around; Tab
/// leaves the group.
#[must_use = "a radio group is not added until you call `.on_change(..)`"]
pub struct RadioGroupChain<'a, 'f, A, T> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    current: T,
    options: Vec<Choice<T>>,
    classes: Tw,
    enablement: Enablement,
    size: Option<Size>,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> RadioGroupChain<'a, 'f, A, T> {
    /// A group of `options` of which `current` is chosen.
    pub(super) fn new<K: crate::Key, O: IntoChoice<T>>(
        b: Builder<'a, 'f, A>,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            current,
            options: choice::collect(options),
            classes: tw(),
            enablement: Enablement::Enabled,
            size: None,
            on_response: OnResponse::default(),
        }
    }

    /// Classes merged over the styled ones of the whole group (a column with a gap of 12 points), such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Runs `then` with the response of each circle each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the group takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.enablement = enablement;
        self
    }

    /// Adds the group. `select` runs with the value of the option that was
    /// chosen with the pointer or the keyboard.
    #[track_caller]
    pub fn on_change(self, select: impl Fn(&mut A, T) + 'static) {
        let select = Rc::new(select);
        let Self {
            mut b,
            slot,
            current,
            options,
            classes,
            enablement,
            size,
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        let style = b.style().clone();
        let radio = &style.radio;
        let metrics = *radio.sizes.lookup(b.ctx(), b.size_of(size));
        let group = tw().flex().flex_col().gap(3.0).merge(classes);
        b.div_id_with(&slot, group, |b| {
            for (index, option) in options.into_iter().enumerate() {
                let control = b.id_of(Part::Circle(index));
                let Choice {
                    value,
                    label,
                    tooltip,
                    enablement: own,
                    ..
                } = option;
                let tooltip = tooltip.map(Tooltip::text);
                let enablement = enablement.and(own);
                let chosen = value == current;
                let look = if chosen { &radio.on } else { &radio.off };
                let circle = look.indicator(
                    radio.border_width,
                    &style,
                    u8::MAX,
                    enablement,
                    radio.transition,
                    metrics.size,
                );
                let on_response = on_response.clone();
                let (on_circle, on_label) = (select.clone(), select.clone());
                let dot = metrics.dot;
                b.div(
                    choice_row(
                        &style,
                        &style.role_sized(&radio.text, metrics.text),
                        enablement,
                        metrics.gap,
                    ),
                    |b| {
                        b.div_id(Part::Circle(index), circle, |b| {
                            if chosen {
                                b.paint(tw().size_px(dot), |painter, rect, color| {
                                    painter.circle_filled(rect.center(), rect.width() / 2.0, color);
                                });
                            }
                        })
                        .role(Role::RadioButton)
                        .draw(move |app, ui| {
                            let response = described(ui, Role::RadioButton, None, "", &tooltip);
                            if response.clicked() || crate::nav::take_activation(ui, response.id) {
                                on_circle(app, value);
                            }
                            on_response.run(app, &response);
                        })
                        .selected(Mark::from(chosen));
                        choice_label(
                            b,
                            Part::Label(index),
                            control,
                            label,
                            enablement,
                            move |app| {
                                on_label(app, value);
                            },
                        );
                    },
                );
            }
        })
        .nav(Nav::Radio(Wrap::Around))
        .selection_follows_focus();
    }
}
