//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::Role;

use super::Mark;
use super::{
    Enablement, OnResponse, Tip, Unset, choice_label, choice_row, described, flip, paint_check,
};
use crate::builder::Slot;
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Label,
}

/// A checkbox drawn from the [`Style`], with an optional label. Start one with
/// [`Builder::checkbox`] and finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is checked when that is its `on` value: `true` for
/// a `bool`, or the second of [`values`](Self::values). A click passes the
/// other value to `on_change`.
///
/// ```
/// # use bullseye::Builder;
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
    name: Option<String>,
    tooltip: Tip,
    current: T,
    values: V,
    classes: Tw,
    enablement: Enablement,
    on_response: OnResponse<A>,
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
            name: None,
            tooltip: None,
            current,
            values: Unset,
            classes: tw(),
            enablement: Enablement::Enabled,
            on_response: OnResponse::default(),
        }
    }

    /// The values the checkbox switches between: `off` when unchecked and
    /// `on` when checked.
    pub fn values(self, off: T, on: T) -> CheckboxChain<'a, 'f, A, T, [T; 2]> {
        CheckboxChain {
            b: self.b,
            slot: self.slot,
            label: self.label,
            name: self.name,
            tooltip: self.tooltip,
            current: self.current,
            values: [off, on],
            classes: self.classes,
            enablement: self.enablement,
            on_response: self.on_response,
        }
    }
}

impl<'a, 'f, A: 'static, T, V> CheckboxChain<'a, 'f, A, T, V> {
    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Classes merged over the styled ones of the whole row, such as its
    /// width or placement; the later class wins ([`Tw::merge`]).
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

    /// Whether the checkbox takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
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
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + Copy + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> CheckboxChain<'a, 'f, A, T, [T; 2]> {
    /// Adds the checkbox. `on_change` runs with the new value when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + Copy + 'static) {
        let Self {
            mut b,
            slot,
            label,
            name,
            tooltip,
            current,
            values: [off, on],
            classes: caller,
            enablement,
            on_response,
        } = self;
        let (mark, toggled) = flip([off, on], current);
        let on_toggle = move |app: &mut A| on_change(app, toggled);
        let style = Style::of(b.ctx());
        let checkbox = &style.checkbox;
        let look = match mark {
            Mark::Off => &checkbox.off,
            Mark::On => &checkbox.on,
        };
        let box_classes = look.indicator(
            checkbox.border_width,
            &style,
            checkbox.radius,
            enablement,
            checkbox.transition,
            checkbox.size,
        );
        let glyph = checkbox.glyph;
        let name = name.unwrap_or_default();
        b.scope_with(slot.id, &slot.label, |b| {
            let control = b.id_of(Part::Box);
            let row = choice_row(&style, &checkbox.text, enablement, checkbox.gap).merge(caller);
            b.div(row, |b| {
                b.div_id(Part::Box, box_classes, |b| {
                    if mark == Mark::On {
                        b.paint(tw().size_px(glyph), paint_check);
                    }
                })
                .draw(move |app, ui| {
                    let response = described(ui, Role::CheckBox, Some(mark), &name, &tooltip);
                    if response.clicked() {
                        on_toggle(app);
                    }
                    on_response.run(app, &response);
                });
                if let Some(label) = label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
