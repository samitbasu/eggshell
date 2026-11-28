//! The owned radio group: circles that choose one value, with arrow keys.

use std::rc::Rc;

use egui::{Context, Role};

use super::Mark;
use super::choice::{self, Choice, IntoChoice};
use super::options::{Configure, Interactive, Options, Resolved, Respond, Validity};
use super::{
    Card, Form, OnResponse, announce_group, choice_card, choice_label, choice_row, described,
    tip_describes, tipped_whole,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::Autofocus;
use crate::overlay::{Nav, Wrap};
use crate::{Builder, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
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
///
/// Its shared options come from [`Configure`],
/// [`Interactive`] and
/// [`Respond`], in scope through the
/// [`prelude`](crate::prelude). Its tooltip shows over the whole
/// group, and an option's own over it; its response is each circle's.
#[must_use = "a radio group is not added until you call `.on_change(..)`"]
pub struct Chain<'a, 'f, A, T> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    current: T,
    options: Vec<Choice<T>>,
    form: Form,
    opts: Options,
    validity: Validity,
    size: Option<Size>,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T> {
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
            opts: Options::new(),
            size: None,
            on_response: OnResponse::default(),
            validity: Validity::Valid,
        }
    }

    /// TODO(prose): draws each option as a choice card: its label holds the circle, after the label and the option's [`description`](Choice::description), and a click anywhere on the card chooses it.
    pub fn cards(mut self) -> Self {
        self.form = Form::Card;
        self
    }

    /// TODO(prose): marks the radio group invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::owned::text_field::TextFieldChain::invalid)
    /// does: its circles' borders and rings turn destructive, and so do their fields' labels.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Adds the group. `select` runs with the value of the option that was
    /// chosen with the pointer or the keyboard.
    #[track_caller]
    pub fn on_change(self, select: impl Fn(&mut A, T) + 'static) {
        self.on_change_with(move |app, _, value| select(app, value));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, select: impl Fn(&mut A, &Context, T) + 'static) {
        let select = Rc::new(select);
        let Self {
            mut b,
            slot,
            current,
            options,
            form,
            opts,
            size,
            on_response,
            validity,
        } = self;
        let Resolved {
            name,
            classes,
            enablement,
            tooltip: group_tip,
            refused,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let style = b.style().clone();
        let radio = &style.radio;
        let size = b.size_of(size);
        let metrics = *radio.sizes.lookup(b.ctx(), size);
        let size = radio.sizes.key_for(size);
        let group = b.part(Part::Root, &[], classes);
        let whole_tip = group_tip.clone();
        let target = choice::focus_target(&options, &current, enablement);
        b.div_id_with(&slot, group, |b| {
            for (index, option) in options.into_iter().enumerate() {
                let autofocus = if Some(index) == target {
                    autofocus
                } else {
                    Autofocus::Never
                };
                let control = b.id_of(Node::Circle(index));
                let (enablement, tooltip) = option.gated(enablement, refused);
                let Choice {
                    value,
                    label,
                    description,
                    ..
                } = option;

                let group_tip = group_tip.clone();
                let chosen = value == current;
                let mut keys = form.keys(Part::item_keys(size, Mark::from(chosen), enablement));
                keys.extend(validity.key(enablement));
                let circle = b.part(Part::Item, &keys, enablement.senses());
                let on_response = on_response.clone();
                let (on_circle, on_label) = (select.clone(), select.clone());
                let row = choice_row(
                    b,
                    &style.role_sized(&radio.text, metrics.text),
                    (enablement, &validity),
                    metrics.gap,
                );
                // A card's label is drawn before the circle it holds, so the
                // circle names itself by the card's title.
                let circle_name = match form {
                    Form::Row => String::new(),
                    Form::Card => label.clone(),
                };
                let validity = validity.clone();
                let build_circle = move |b: &mut Builder<'_, '_, A>| {
                    b.div_id(Node::Circle(index), circle, |b| {
                        // The dot of an unchosen circle is there, unseen, so
                        // choosing moves no node.
                        let shown = if chosen { tw() } else { tw().invisible() };
                        let indicator = b.part(Part::Indicator, &keys, tw());
                        let icon = b.part(Part::IndicatorIcon, &keys, shown);
                        b.div(indicator, |b| b.div(icon, |_| {}));
                    })
                    .role(Role::RadioButton)
                    .open_focus(autofocus.open_focus())
                    .draw(move |app, ui| {
                        let response =
                            described(ui, Role::RadioButton, None, &circle_name, &tooltip);
                        validity.describe(ui.ctx(), response.id);
                        autofocus.apply(ui.ctx(), &response);
                        if tooltip.is_none() {
                            tip_describes(&response, &group_tip);
                        }
                        if response.clicked() || crate::nav::take_activation(ui, response.id) {
                            on_circle(app, ui.ctx(), value);
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
                            Node::Label(index),
                            control,
                            label,
                            enablement,
                            move |app, ctx| {
                                on_label(app, ctx, value);
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
                        let on_card = move |app: &mut A, ctx: &Context| on_label(app, ctx, value);
                        choice_card(b, Node::Label(index), card, row, on_card, build_circle);
                    }
                }
            }
        })
        .nav(Nav::Radio(Wrap::Around))
        .selection_follows_focus()
        .senses_for(&whole_tip)
        .draw(move |_, ui| {
            tipped_whole(ui, &whole_tip);
            announce_group(ui, Role::RadioGroup, name.as_deref());
        });
    }
}

/// TODO(prose): the anatomy of a radio group: shadcn's `radio-group`.
///
/// TODO(prose): the keys its rules see: an item's size, `aria-checked`
/// (shadcn's `data-checked`) and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the group, holding a field per option.
    Root,
    /// TODO(prose): one option's circle.
    Item,
    /// TODO(prose): what the chosen circle holds: the room its dot is
    /// centred in.
    Indicator,
    /// TODO(prose): the dot in the chosen circle.
    IndicatorIcon,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "radio-group.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "radio-group",
            Self::Item => "radio-group-item",
            Self::Indicator => "radio-group-indicator",
            Self::IndicatorIcon => "radio-group-indicator-icon",
        }
    }
}

impl Part {
    /// TODO(prose): the keys of an item of `size` and its dot, chosen if `mark` is on, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{Enablement, Mark, radio::Part};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = Part::item_keys(Size::DEFAULT, Mark::Off, Enablement::Disabled);
    /// assert!(keys.contains(&Key::Aria(Aria::Disabled)));
    /// ```
    pub fn item_keys(
        size: crate::contract::Size,
        mark: super::Mark,
        enablement: super::Enablement,
    ) -> Vec<crate::sheet::Key> {
        super::field::choice_keys(size, mark, enablement)
    }
}

impl<A, T> Configure for Chain<'_, '_, A, T> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, T> Interactive for Chain<'_, '_, A, T> {}

impl<A, T> Respond for Chain<'_, '_, A, T> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// Radio buttons for `options` (anything [`IntoChoice`]), of which
    /// `current` is chosen. Values must be distinct. Assistive technology
    /// sees each option as a `Role::RadioButton`.
    pub fn radio_group<K: crate::Key, T: Copy + PartialEq + 'static, O: IntoChoice<T>>(
        &mut self,
        key: K,
        current: T,
        options: impl IntoIterator<Item = O>,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, current, options)
    }
}
