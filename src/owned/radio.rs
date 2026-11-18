//! The owned radio group: circles that choose one value, with arrow keys.

use std::rc::Rc;

use egui::{Id, Key, Role, Ui};

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::{
    Enablement, Gate, IntoTip, OnResponse, Tooltip, announce_group, choice_label, choice_row,
    described, number, options, tip_describes, tipped_whole,
};
use crate::builder::Slot;
use crate::contract::Size;
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
    gate: Gate,
    name: Option<String>,
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
            gate: Gate::default(),
            name: None,
            size: None,
            on_response: OnResponse::default(),
        }
    }

    /// Classes merged over the styled ones of the whole group (a column with a gap of 12 points), such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// The accessible name of the group, the equivalent of `aria-label`.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): shown while the pointer is over the group or focus is in
    /// it, also when it is disabled. An option's own tooltip shows over it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.gate.tooltip_with(build);
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

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.gate.refuse(reason);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<U>(self, value: Option<U>, then: impl FnOnce(Self, U) -> Self) -> Self {
        options::when_some(self, value, then)
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
            gate,
            name,
            size,
            on_response,
        } = self;
        let refused = gate.refused();
        let (enablement, group_tip) = gate.resolve(&mut b, slot.id, enablement);
        let style = b.style().clone();
        let radio = &style.radio;
        let metrics = *radio.sizes.lookup(b.ctx(), b.size_of(size));
        let values: Vec<T> = options.iter().map(|option| option.value).collect();
        let gated: Vec<_> = options
            .iter()
            .map(|option| option.gated(enablement, refused))
            .collect();
        let enabled: Vec<bool> = gated
            .iter()
            .map(|(enablement, _)| *enablement == Enablement::Enabled)
            .collect();
        let count = values.len();
        let tab_stop = values
            .iter()
            .zip(&enabled)
            .position(|(&value, &enabled)| enabled && value == current)
            .or_else(|| enabled.iter().position(|&enabled| enabled))
            .unwrap_or(0);
        let group = tw().flex().flex_col().gap(3.0).merge(classes);
        let group = match &group_tip {
            Some(_) => group.sense_hover(),
            None => group,
        };
        let whole_tip = group_tip.clone();
        b.div_id_with(&slot, group, |b| {
            let ids: Vec<_> = (0..count)
                .map(|index| b.id_of(Part::Circle(index)))
                .collect();
            for (((index, option), &control), (enablement, tooltip)) in
                options.into_iter().enumerate().zip(&ids).zip(gated)
            {
                let Choice { value, label, .. } = option;
                let tooltip = tooltip.map(Tooltip::text);
                let group_tip = group_tip.clone();
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
                let circle = if index == tab_stop {
                    circle
                } else {
                    circle.focusable_none()
                };
                let on_response = on_response.clone();
                let (on_circle, on_label) = (select.clone(), select.clone());
                let stop = |at: usize| {
                    let value = *values.get(at)?;
                    let id = *ids.get(at)?;
                    enabled.get(at)?.then_some((value, id))
                };
                let previous = (1..count).find_map(|step| stop((index + count - step) % count));
                let next = (1..count).find_map(|step| stop((index + step) % count));
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
                        .draw(move |app, ui| {
                            let response = described(
                                ui,
                                Role::RadioButton,
                                Some(Mark::from(chosen)),
                                "",
                                &tooltip,
                            );
                            if tooltip.is_none() {
                                tip_describes(ui, &response, &group_tip);
                            }
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
        })
        .draw(move |_, ui| {
            tipped_whole(ui, &whole_tip);
            announce_group(ui, Role::RadioGroup, name.as_deref());
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
    number::hold_arrows(ui);
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
    let key = focus_request(ui.ctx());
    ui.data_mut(|data| data.insert_temp(key, id));
    ui.ctx().request_repaint();
}

/// Where the circle that is to take the focus is recorded. egui drops focus
/// given to a node that is not focusable yet, and a circle becomes its
/// group's Tab stop only once the choice has moved to it.
const FOCUS_REQUEST: &str = "eggshell-radio-focus";

fn focus_request(ctx: &egui::Context) -> Id {
    crate::key::viewport_id(ctx, FOCUS_REQUEST)
}

fn take_focus_if_asked(ui: &Ui, circle: Id) {
    let key = focus_request(ui.ctx());
    let asked = ui.data_mut(|data| {
        let asked = data.get_temp::<Id>(key) == Some(circle);
        if asked {
            data.remove::<Id>(key);
        }
        asked
    });
    if asked {
        ui.memory_mut(|memory| memory.request_focus(circle));
    }
}
