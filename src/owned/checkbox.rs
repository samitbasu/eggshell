//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use egui::{Context, Role};

use super::Mark;
use super::options::{Configure, Interactive, Options, Resolved, Respond, Validity};
use super::{
    Card, Form, OnResponse, Unset, check, choice_card, choice_label, choice_row, described, flip,
};
use crate::builder::Slot;
use crate::contract::Size;
use crate::{Builder, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Node {
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
/// Its shared options come from [`Configure`], [`Interactive`] and
/// [`Respond`]: its [`classes`](Configure::classes) are the whole row's, and
/// its response is the box's.
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
pub struct Chain<'a, 'f, A, T, V = Unset> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: Option<String>,
    description: Option<String>,
    form: Form,
    opts: Options,
    validity: Validity,
    current: T,
    values: V,
    size: Option<Size>,
    on_response: OnResponse<A>,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T> {
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
            opts: Options::new(),
            current,
            values: Unset,
            size: None,
            on_response: OnResponse::default(),
            validity: Validity::Valid,
        }
    }

    /// The values the checkbox switches between: `off` when unchecked and
    /// `on` when checked.
    pub fn values(self, off: T, on: T) -> Chain<'a, 'f, A, T, [T; 2]> {
        Chain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            description: self.description,
            form: self.form,
            opts: self.opts,
            current: self.current,
            values: [off, on],
            size: self.size,
            on_response: self.on_response,
            validity: self.validity,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> Chain<'a, 'f, A, T, V> {
    /// TODO(prose): marks the checkbox invalid while `error` is `Some`, as
    /// [`TextFieldChain::invalid`](crate::owned::text_field::TextFieldChain::invalid)
    /// does: its box's border and ring turn destructive, and so does its field's label.
    pub fn invalid(mut self, error: Option<impl Into<String>>) -> Self {
        self.validity = Validity::of(error);
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

    /// TODO(prose): draws at the style's size `size`, over the one the subtree inherits.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}

impl<'a, 'f, A: 'static> Chain<'a, 'f, A, bool> {
    /// Adds the checkbox. `on_change` runs with the new value when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, on_change: impl Fn(&mut A, &Context, bool) + 'static) {
        self.values(false, true).on_change_with(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T, [T; 2]> {
    /// Adds the checkbox. `on_change` runs with the new value when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        self.on_change_with(move |app, _, value| on_change(app, value));
    }

    /// Like [`on_change`](Self::on_change), with a handler that also gets
    /// the egui context, for an effect the context performs.
    #[track_caller]
    pub fn on_change_with(self, on_change: impl Fn(&mut A, &Context, T) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            description,
            form,
            opts,
            current,
            values: [off, on],
            size,
            on_response,
            validity,
        } = self;
        let Resolved {
            name,
            classes: caller,
            enablement,
            tooltip,
            autofocus,
            ..
        } = opts.resolve(&mut b, slot.id);
        let (mark, toggled) = flip([off, on], current);
        let on_toggle = Rc::new(move |app: &mut A, ctx: &Context| on_change(app, ctx, toggled));
        let style = b.style().clone();
        let checkbox = &style.checkbox;
        let size = b.size_of(size);
        let metrics = *checkbox.sizes.lookup(b.ctx(), size);
        let mut keys = form.keys(Part::keys(checkbox.sizes.key_for(size), mark, enablement));
        keys.extend(validity.key(enablement));
        let box_classes = b.part(Part::Root, &keys, enablement.senses());
        // A card's label is drawn before the control it holds, so the
        // control names itself by the card's title.
        let name = match form {
            Form::Row => name.unwrap_or_default(),
            Form::Card => name.or_else(|| label.clone()).unwrap_or_default(),
        };
        b.claim(&slot);
        b.scope_with(&slot, |b| {
            let control = b.id_of(Node::Box);
            let row = choice_row(
                b,
                &style.role_sized(&checkbox.text, metrics.text),
                (enablement, &validity),
                metrics.gap,
            );
            let on_press = on_toggle.clone();
            let validity = validity.clone();
            let build_box = move |b: &mut Builder<'_, '_, A>| {
                b.div_id(Node::Box, box_classes, |b| {
                    let indicator = b.part(Part::Indicator, &keys, tw());
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
                        on_press(app, ui.ctx());
                    }
                    on_response.run(app, &response);
                });
            };
            match form {
                Form::Row => b.div(row.merge(caller), |b| {
                    build_box(b);
                    if let Some(label) = label {
                        choice_label(
                            b,
                            Node::Label,
                            control,
                            label,
                            enablement,
                            move |app, ctx| on_toggle(app, ctx),
                        );
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
                        Node::Label,
                        card,
                        row,
                        move |app, ctx| on_toggle(app, ctx),
                        build_box,
                    );
                }
            }
        });
    }
}

/// TODO(prose): the anatomy of a checkbox: shadcn's `checkbox`.
///
/// TODO(prose): the keys its rules see: the size, `aria-checked` (shadcn's
/// `data-checked`) and `aria-disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// TODO(prose): the box.
    Root,
    /// TODO(prose): the check glyph in a ticked box.
    Indicator,
}

impl crate::sheet::Part for Part {
    const SOURCE: &'static str = "checkbox.tsx";

    fn slot(self) -> &'static str {
        match self {
            Self::Root => "checkbox",
            Self::Indicator => "checkbox-indicator",
        }
    }
}

impl Part {
    /// TODO(prose): the keys of a checkbox of `size`, ticked if `mark` is on, for [`Builder::part`](crate::Builder::part).
    ///
    /// ```
    /// use eggshell_ui::Aria;
    /// use eggshell_ui::contract::Size;
    /// use eggshell_ui::owned::{checkbox::Part, Enablement, Mark};
    /// use eggshell_ui::sheet::Key;
    ///
    /// let keys = Part::keys(Size::DEFAULT, Mark::On, Enablement::Enabled);
    /// assert!(keys.contains(&Key::Aria(Aria::Checked)));
    /// ```
    pub fn keys(
        size: crate::contract::Size,
        mark: super::Mark,
        enablement: super::Enablement,
    ) -> Vec<crate::sheet::Key> {
        super::field::choice_keys(size, mark, enablement)
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A checkbox labelled `label` that shows `current`: a `bool`, or with
    /// [`values`](Chain::values) any two-valued type. Assistive
    /// technology sees a `Role::CheckBox`.
    pub fn checkbox<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        label: impl Into<String>,
        current: T,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, Some(label.into()), current)
    }

    /// A checkbox without a label.
    pub fn checkbox_unlabelled<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        current: T,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, None, current)
    }
}

impl<A, T, V> Configure for Chain<'_, '_, A, T, V> {
    fn options(&mut self) -> &mut Options {
        &mut self.opts
    }
}

impl<A, T, V> Interactive for Chain<'_, '_, A, T, V> {}

impl<A, T, V> Respond for Chain<'_, '_, A, T, V> {
    type App = A;

    fn responses(&mut self) -> &mut OnResponse<A> {
        &mut self.on_response
    }
}
