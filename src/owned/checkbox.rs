//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::Role;

use super::{
    Card, Enablement, Form, Gate, IntoTip, OnResponse, Unset, Validity, check, choice_card,
    choice_label, choice_row, described, flip, options,
};
use super::{CheckboxPart, Mark};
use crate::builder::Slot;
use crate::contract::Size;
use crate::focus::{Autofocus, Select};
use crate::{Builder, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Label,
}

/// A checkbox drawn from the [`Style`](crate::Style), with an optional label. Start one with
/// [`Builder::checkbox`] and finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is checked when that is its `on` value: `true` for
/// a `bool`, or the second of [`values`](Self::values). A click passes the
/// other value to `on_change`.
///
/// ```
/// # use eggshell_ui::Builder;
/// #[derive(Clone, Copy, PartialEq)]
/// enum Terms { Declined, Accepted }
/// struct App { terms: Terms, news: bool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.checkbox("terms", "I agree", app.terms)
///         .values(Terms::Declined, Terms::Accepted)
///         .on_change(|app, terms| app.terms = terms);
///     b.checkbox("news", "Send me news", app.news)
///         .on_change(|app, news| app.news = news);
/// }
/// ```
#[must_use = "a checkbox is not added until you call `.on_change(..)`"]
pub struct CheckboxChain<'a, 'f, A, T, V = Unset> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: Option<String>,
    description: Option<String>,
    form: Form,
    name: Option<String>,
    gate: Gate,
    validity: Validity,
    current: T,
    values: V,
    classes: Tw,
    enablement: Enablement,
    size: Option<Size>,
    on_response: OnResponse<A>,
    autofocus: Autofocus,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> CheckboxChain<'a, 'f, A, T> {
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        label: Option<String>,
        current: T,
    ) -> Self {
        Self {
            slot: b.slot(&key),
            b,
            label,
            description: None,
            form: Form::Row,
            name: None,
            gate: Gate::default(),
            current,
            values: Unset,
            classes: tw(),
            enablement: Enablement::Enabled,
            size: None,
            on_response: OnResponse::default(),
            autofocus: Autofocus::Never,
            validity: Validity::Valid,
        }
    }

    /// The values the checkbox switches between: `off` when unchecked and
    /// `on` when checked.
    pub fn values(self, off: T, on: T) -> CheckboxChain<'a, 'f, A, T, [T; 2]> {
        CheckboxChain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            description: self.description,
            form: self.form,
            name: self.name,
            gate: self.gate,
            current: self.current,
            values: [off, on],
            classes: self.classes,
            enablement: self.enablement,
            size: self.size,
            on_response: self.on_response,
            autofocus: self.autofocus,
            validity: self.validity,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> CheckboxChain<'a, 'f, A, T, V> {
    /// TODO(prose): marks the checkbox invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::prim::text_field::TextFieldChain::invalid)
    /// does: its box's border and ring turn destructive, and so does its field's label.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
        self
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// TODO(prose): draws the checkbox as a choice card: the label holds it, the box after the label and the description, and a click anywhere on the card toggles it.
    pub fn card(mut self) -> Self {
        self.form = Form::Card;
        self
    }

    /// TODO(prose): the text under a card's label.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Classes merged over the styled ones of the whole row, such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// Shown while the pointer hovers the control, in a tooltip overlay
    /// ([`TooltipStyle`](crate::contract::TooltipStyle)): after the style's
    /// delay, at once when it has keyboard focus, and also when the control is
    /// disabled, as an HTML `title` is. Rich text keeps egui's formatting. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.gate.tooltip(text);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](crate::Builder::kbd) chip.
    /// The content takes no input, and its text describes the control to
    /// assistive technology.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.gate.tooltip_with(build);
        self
    }

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Whether the checkbox takes input.
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

    /// Runs `then` with the box's response each frame: the escape hatch for
    /// what the builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }
}

impl<'a, 'f, A: 'static> CheckboxChain<'a, 'f, A, bool> {
    /// Adds the checkbox. `on_change` runs with the new value when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> CheckboxChain<'a, 'f, A, T, [T; 2]> {
    /// Adds the checkbox. `on_change` runs with the new value when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            description,
            form,
            name,
            gate,
            current,
            values: [off, on],
            classes: caller,
            enablement,
            size,
            on_response,
            autofocus,
            validity,
        } = self;
        let (enablement, tooltip) = gate.resolve(&mut b, slot.id, enablement);
        let (mark, toggled) = flip([off, on], current);
        let on_toggle = Rc::new(move |app: &mut A| on_change(app, toggled));
        let style = b.style().clone();
        let checkbox = &style.checkbox;
        let size = b.size_of(size);
        let metrics = *checkbox.sizes.lookup(b.ctx(), size);
        let mut keys = form.keys(CheckboxPart::keys(
            checkbox.sizes.key_for(size),
            mark,
            enablement,
        ));
        keys.extend(validity.key(enablement));
        let box_classes = b.part(CheckboxPart::Root, &keys, enablement.senses());
        // A card's label is drawn before the control it holds, so the
        // control names itself by the card's title.
        let name = match form {
            Form::Row => name.unwrap_or_default(),
            Form::Card => name.or_else(|| label.clone()).unwrap_or_default(),
        };
        b.claim(&slot);
        b.scope_with(&slot, |b| {
            let control = b.id_of(Part::Box);
            let row = choice_row(
                b,
                &style.role_sized(&checkbox.text, metrics.text),
                (enablement, &validity),
                metrics.gap,
            );
            let on_press = on_toggle.clone();
            let validity = validity.clone();
            let build_box = move |b: &mut Builder<'_, '_, A>| {
                b.div_id(Part::Box, box_classes, |b| {
                    let indicator = b.part(CheckboxPart::Indicator, &keys, tw());
                    b.div(indicator, |b| {
                        if mark == Mark::On {
                            b.glyph(tw(), check());
                        }
                    });
                })
                .open_focus(autofocus.open_focus())
                .draw(move |app, ui| {
                    let response = described(ui, Role::CheckBox, Some(mark), &name, &tooltip);
                    validity.describe(ui.ctx(), response.id);
                    autofocus.apply(ui.ctx(), &response);
                    if response.clicked() {
                        on_press(app);
                    }
                    on_response.run(app, &response);
                });
            };
            match form {
                Form::Row => b.div(row.merge(caller), |b| {
                    build_box(b);
                    if let Some(label) = label {
                        choice_label(b, Part::Label, control, label, enablement, move |app| {
                            on_toggle(app)
                        });
                    }
                }),
                Form::Card => {
                    let card = Card {
                        control,
                        title: label.unwrap_or_default(),
                        description,
                        enablement,
                        classes: caller,
                    };
                    choice_card(
                        b,
                        Part::Label,
                        card,
                        row,
                        move |app| on_toggle(app),
                        build_box,
                    );
                }
            }
        });
    }
}
