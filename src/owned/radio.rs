//! The owned radio group: circles that choose one value, with arrow keys.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Id, Key, Role, Ui, WidgetText};

use super::Mark;
use super::{Enablement, choice_label, choice_row, described, number};
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part<T> {
    Circle(T),
    Label(T),
}

/// Radio buttons that choose one of `options`, drawn from the [`Style`].
///
/// The group is one Tab stop, on the chosen option (the first when none is
/// chosen), as in HTML. The arrow keys move the choice, and the focus with it,
/// to the next or previous option, wrapping around; Tab leaves the group.
#[derive(Clone, Debug)]
pub struct RadioGroup<T> {
    current: T,
    options: Vec<(T, String)>,
    tooltips: Vec<(T, WidgetText)>,
    classes: Tw,
    enablement: Enablement,
}

impl<T: Copy + PartialEq + Hash + Debug + 'static> RadioGroup<T> {
    /// A group of `(value, label)` options of which `current` is chosen.
    /// Values must be distinct.
    pub fn new(current: T, options: &[(T, &str)]) -> Self {
        Self {
            current,
            options: options
                .iter()
                .map(|&(value, label)| (value, label.to_owned()))
                .collect(),
            tooltips: Vec::new(),
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }

    /// Classes merged over the styled ones of the whole group (a column with a gap of 12 points), such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
        self
    }

    /// Shows `text` while the pointer hovers the circle of the option `value`,
    /// as egui's `on_hover_text` does and also when the group is disabled. It
    /// adds to the option's accessible name and does not replace it.
    #[must_use]
    pub fn option_tooltip(mut self, value: T, text: impl Into<WidgetText>) -> Self {
        debug_assert!(
            self.options.iter().any(|&(option, _)| option == value),
            "no option {value:?} to give a tooltip"
        );
        self.tooltips.push((value, text.into()));
        self
    }

    /// Whether the group takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the group to `b` under `key`. `select` runs with the value of the
    /// option that was chosen with the pointer or the keyboard.
    pub(crate) fn show<A: 'static, K: crate::Key>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        select: fn(&mut A, T),
    ) {
        let style = Style::of(b.ctx());
        let radio = &style.radio;
        let enablement = self.enablement;
        let values: Vec<T> = self.options.iter().map(|&(value, _)| value).collect();
        let count = values.len();
        let tab_stop = values
            .iter()
            .position(|&value| value == self.current)
            .unwrap_or(0);
        b.scope(key, |b| {
            let ids: Vec<_> = values
                .iter()
                .map(|&value| b.id_of(Part::Circle(value)))
                .collect();
            let group = tw().flex().flex_col().gap(3.0).merge(self.classes);
            b.div(group, |b| {
                for (index, (value, label)) in self.options.into_iter().enumerate() {
                    let chosen = value == self.current;
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
                    let tooltip = self
                        .tooltips
                        .iter()
                        .find(|&&(option, _)| option == value)
                        .map(|(_, text)| text.clone());
                    let around = |offset: usize| {
                        let at = (index + offset) % count;
                        (values[at], ids[at])
                    };
                    let (previous, next) = (around(count - 1), around(1));
                    let dot = radio.dot;
                    let control = ids[index];
                    b.div(
                        choice_row(&style, &radio.text, enablement, radio.gap),
                        |b| {
                            b.div_id(
                                Part::Circle(value),
                                circle,
                                move |app, ui| {
                                    let response = described(
                                        ui,
                                        Role::RadioButton,
                                        Some(Mark::from(chosen)),
                                        "",
                                        &tooltip,
                                    );
                                    if response.clicked() {
                                        select(app, value);
                                    }
                                    take_focus_if_asked(ui, response.id);
                                    if response.has_focus() {
                                        navigate(app, ui, select, previous, next);
                                    }
                                },
                                |b| {
                                    if chosen {
                                        b.paint(tw().size_px(dot), |painter, rect, color| {
                                            painter.circle_filled(
                                                rect.center(),
                                                rect.width() / 2.0,
                                                color,
                                            );
                                        });
                                    }
                                },
                            );
                            choice_label(
                                b,
                                Part::Label(value),
                                control,
                                label,
                                enablement,
                                move |app| {
                                    select(app, value);
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
    select: fn(&mut A, T),
    previous: (T, Id),
    next: (T, Id),
) {
    number::hold_keys(ui, false);
    let pressed = |key| number::pressed(ui, key);
    let forward = pressed(Key::ArrowDown) | pressed(Key::ArrowRight);
    let backward = pressed(Key::ArrowUp) | pressed(Key::ArrowLeft);
    let (value, id) = match (forward, backward) {
        (true, false) => next,
        (false, true) => previous,
        _ => return,
    };
    select(app, value);
    ui.data_mut(|data| data.insert_temp(focus_request(), id));
    ui.ctx().request_repaint();
}

/// Where the circle that is to take the focus is recorded. egui drops focus
/// given to a node that is not focusable yet, and a circle becomes its
/// group's Tab stop only once the choice has moved to it.
const FOCUS_REQUEST: &str = "bullseye-radio-focus";

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
