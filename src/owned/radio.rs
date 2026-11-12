//! The owned radio group: circles that choose one value, with arrow keys.

use std::rc::Rc;

use egui::{Id, Key, Role, Ui};

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::{Enablement, OnResponse, choice_label, choice_row, described, number};
use crate::builder::Slot;
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Circle(usize),
    Label(usize),
}

/// Radio buttons that choose one of `options`, drawn from the [`Style`]. Start
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

    /// Whether the group takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
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
            on_response,
        } = self;
        let enablement = b.enablement_of(enablement);
        let style = Style::of(b.ctx());
        let radio = &style.radio;
        let values: Vec<T> = options.iter().map(|option| option.value).collect();
        let enabled: Vec<bool> = options
            .iter()
            .map(|option| enablement.and(option.enablement) == Enablement::Enabled)
            .collect();
        let count = values.len();
        let tab_stop = (0..count)
            .find(|&at| enabled[at] && values[at] == current)
            .or_else(|| (0..count).find(|&at| enabled[at]))
            .unwrap_or(0);
        b.claim(&slot);
        b.scope_with(slot.id, &slot.label, |b| {
            let ids: Vec<_> = (0..count)
                .map(|index| b.id_of(Part::Circle(index)))
                .collect();
            let group = tw().flex().flex_col().gap(3.0).merge(classes);
            b.div(group, |b| {
                for (index, option) in options.into_iter().enumerate() {
                    let Choice {
                        value,
                        label,
                        tooltip,
                        enablement: own,
                        ..
                    } = option;
                    let enablement = enablement.and(own);
                    let chosen = value == current;
                    let look = if chosen { &radio.on } else { &radio.off };
                    let circle = look.indicator(
                        radio.border_width,
                        &style,
                        u8::MAX,
                        enablement,
                        radio.transition,
                        radio.size,
                    );
                    let circle = if index == tab_stop {
                        circle
                    } else {
                        circle.focusable_none()
                    };
                    let on_response = on_response.clone();
                    let (on_circle, on_label) = (select.clone(), select.clone());
                    let around = |offset: usize| {
                        (1..count)
                            .map(|step| (index + step * offset) % count)
                            .find(|&at| enabled[at])
                            .map(|at| (values[at], ids[at]))
                    };
                    let (previous, next) = (around(count - 1), around(1));
                    let dot = radio.dot;
                    let control = ids[index];
                    b.div(
                        choice_row(&style, &radio.text, enablement, radio.gap),
                        |b| {
                            b.div_id(Part::Circle(index), circle, |b| {
                                if chosen {
                                    b.paint(tw().size_px(dot), |painter, rect, color| {
                                        painter.circle_filled(
                                            rect.center(),
                                            rect.width() / 2.0,
                                            color,
                                        );
                                    });
                                }
                            })
                            .draw(move |app, ui| {
                                let response = described(
                                    ui,
                                    Role::RadioButton,
                                    Some(Mark::from(chosen)),
                                    "",
                                    &tooltip,
                                );
                                if response.clicked() {
                                    on_circle(app, value);
                                }
                                on_response.run(app, &response);
                                take_focus_if_asked(ui, response.id);
                                if response.has_focus() {
                                    navigate(app, ui, &*on_circle, previous, next);
                                }
                            });
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
            });
        });
    }
}

/// Moves the choice and the focus on an arrow key. The focused circle keeps
/// the arrows from moving the focus spatially.
fn navigate<A, T: Copy>(
    app: &mut A,
    ui: &Ui,
    select: impl Fn(&mut A, T),
    previous: Option<(T, Id)>,
    next: Option<(T, Id)>,
) {
    number::hold_keys(ui, false);
    let pressed = |key| number::pressed(ui, key);
    let forward = pressed(Key::ArrowDown) | pressed(Key::ArrowRight);
    let backward = pressed(Key::ArrowUp) | pressed(Key::ArrowLeft);
    let target = match (forward, backward) {
        (true, false) => next,
        (false, true) => previous,
        _ => return,
    };
    let Some((value, id)) = target else { return };
    select(app, value);
    ui.data_mut(|data| data.insert_temp(focus_request(), id));
    ui.ctx().request_repaint();
}

/// Where the circle that is to take the focus is recorded. egui drops focus
/// given to a node that is not focusable yet, and a circle becomes its
/// group's Tab stop only once the choice has moved to it.
const FOCUS_REQUEST: &str = "eggshell-radio-focus";

fn focus_request() -> Id {
    crate::key::root_id().with(FOCUS_REQUEST)
}

fn take_focus_if_asked(ui: &Ui, circle: Id) {
    let asked = ui.data_mut(|data| {
        let asked = data.get_temp::<Id>(focus_request()) == Some(circle);
        if asked {
            data.remove::<Id>(focus_request());
        }
        asked
    });
    if asked {
        ui.memory_mut(|memory| memory.request_focus(circle));
    }
}
