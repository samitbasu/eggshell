//! The owned radio group: circles that choose one value, with arrow keys.

use std::rc::Rc;

use egui::Role;

use super::choice::{self, Choice, IntoChoice};
use super::{
    Card, Enablement, Form, Gate, IntoTip, OnResponse, announce_group, choice_card, choice_label,
    choice_row, described, options, tip_describes, tipped_whole,
};
use super::{Mark, RadioGroupPart};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::{Autofocus, Select};
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
    form: Form,
    classes: Tw,
    enablement: Enablement,
    gate: Gate,
    name: Option<String>,
    size: Option<Size>,
    on_response: OnResponse<A>,
    autofocus: Autofocus,
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
            form: Form::Row,
            classes: tw(),
            enablement: Enablement::Enabled,
            gate: Gate::default(),
            name: None,
            size: None,
            on_response: OnResponse::default(),
            autofocus: Autofocus::Never,
        }
    }

    /// TODO(prose): draws each option as a choice card: its label holds the circle, after the label and the option's [`description`](Choice::description), and a click anywhere on the card chooses it.
    pub fn cards(mut self) -> Self {
        self.form = Form::Card;
        self
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

    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.autofocus = Autofocus::OnOverlayOpen;
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    pub fn focus_now(mut self) -> Self {
        self.autofocus = Autofocus::Now(Select::None);
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
            form,
            classes,
            enablement,
            gate,
            name,
            size,
            on_response,
            autofocus,
        } = self;
        let refused = gate.refused();
        let (enablement, group_tip) = gate.resolve(&mut b, slot.id, enablement);
        let style = b.style().clone();
        let radio = &style.radio();
        let size = b.size_of(size);
        let metrics = *radio.sizes.lookup(b.ctx(), size);
        let size = radio.sizes.key_for(size);
        let group = b.part(RadioGroupPart::Root, &[], classes);
        let group = match &group_tip {
            Some(_) => group.sense_hover(),
            None => group,
        };
        let whole_tip = group_tip.clone();
        let target = choice::focus_target(&options, &current, enablement);
        b.div_id_with(&slot, group, |b| {
            for (index, option) in options.into_iter().enumerate() {
                let autofocus = if Some(index) == target {
                    autofocus
                } else {
                    Autofocus::Never
                };
                let control = b.id_of(Part::Circle(index));
                let (enablement, tooltip) = option.gated(enablement, refused);
                let Choice {
                    value,
                    label,
                    description,
                    ..
                } = option;

                let group_tip = group_tip.clone();
                let chosen = value == current;
                let keys = form.keys(RadioGroupPart::item_keys(
                    size,
                    Mark::from(chosen),
                    enablement,
                ));
                let circle = b.part(RadioGroupPart::Item, &keys, enablement.senses());
                let on_response = on_response.clone();
                let (on_circle, on_label) = (select.clone(), select.clone());
                let row = choice_row(
                    b,
                    &style.role_sized(&radio.text, metrics.text),
                    enablement,
                    metrics.gap,
                );
                // A card's label is drawn before the circle it holds, so the
                // circle names itself by the card's title.
                let circle_name = match form {
                    Form::Row => String::new(),
                    Form::Card => label.clone(),
                };
                let build_circle = move |b: &mut Builder<'_, '_, A>| {
                    b.div_id(Part::Circle(index), circle, |b| {
                        // The dot of an unchosen circle is there, unseen, so
                        // choosing moves no node.
                        let shown = if chosen { tw() } else { tw().invisible() };
                        let indicator = b.part(RadioGroupPart::Indicator, &keys, tw());
                        let icon = b.part(RadioGroupPart::IndicatorIcon, &keys, shown);
                        b.div(indicator, |b| b.div(icon, |_| {}));
                    })
                    .role(Role::RadioButton)
                    .open_focus(autofocus.open_focus())
                    .draw(move |app, ui| {
                        let response =
                            described(ui, Role::RadioButton, None, &circle_name, &tooltip);
                        autofocus.apply(ui.ctx(), &response);
                        if tooltip.is_none() {
                            tip_describes(ui, &response, &group_tip);
                        }
                        if response.clicked() || crate::nav::take_activation(ui, response.id) {
                            on_circle(app, value);
                        }
                        on_response.run(app, &response);
                    })
                    .selected(Mark::from(chosen));
                };
                match form {
                    Form::Row => b.div(row, |b| {
                        build_circle(b);
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
                    }),
                    Form::Card => {
                        let card = Card {
                            control,
                            title: label,
                            description,
                            enablement,
                            classes: tw(),
                        };
                        let on_card = move |app: &mut A| on_label(app, value);
                        choice_card(b, Part::Label(index), card, row, on_card, build_circle);
                    }
                }
            }
        })
        .nav(Nav::Radio(Wrap::Around))
        .selection_follows_focus()
        .draw(move |_, ui| {
            tipped_whole(ui, &whole_tip);
            announce_group(ui, Role::RadioGroup, name.as_deref());
        });
    }
}
