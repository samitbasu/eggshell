//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::Role;

use super::Mark;
use super::{Enablement, Tip, choice_label, choice_row, described, paint_check};
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Label,
}

/// A checkbox drawn from the [`Style`], with an optional label. Start one with
/// [`Builder::checkbox`] and finish it with [`bind`](Self::bind) or
/// [`on_toggle`](Self::on_toggle).
#[must_use = "a checkbox is not added until you call `.bind(..)` or `.on_toggle(..)`"]
pub struct CheckboxBuilder<'a, 'b, 'f, A, K> {
    b: &'a mut Builder<'b, 'f, A>,
    key: K,
    label: Option<String>,
    name: Option<String>,
    tooltip: Tip,
    mark: Mark,
    classes: Tw,
    enablement: Enablement,
}

impl<'a, 'b, 'f, A: 'static, K: crate::Key> CheckboxBuilder<'a, 'b, 'f, A, K> {
    pub(super) fn new(
        b: &'a mut Builder<'b, 'f, A>,
        key: K,
        label: Option<String>,
        mark: Mark,
    ) -> Self {
        Self {
            b,
            key,
            label,
            name: None,
            tooltip: None,
            mark,
            classes: tw(),
            enablement: Enablement::Enabled,
        }
    }

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

    /// Adds the checkbox, which flips the bool that `lens` picks out of the
    /// application's state when it is toggled.
    pub fn bind(self, lens: impl Fn(&mut A) -> &mut bool + Copy + 'static) {
        self.on_toggle(move |app| {
            let ticked = lens(app);
            *ticked = !*ticked;
        });
    }

    /// Adds the checkbox. `on_toggle` runs when the box or the label is
    /// clicked, or the box is activated with Space or Enter.
    pub fn on_toggle(self, on_toggle: impl Fn(&mut A) + Copy + 'static) {
        let Self {
            b,
            key,
            label,
            name,
            tooltip,
            mark,
            classes: caller,
            enablement,
        } = self;
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
        b.scope(key, |b| {
            let control = b.id_of(Part::Box);
            let row = choice_row(&style, &checkbox.text, enablement, checkbox.gap).merge(caller);
            b.div(row, |b| {
                b.div_id(
                    Part::Box,
                    box_classes,
                    move |app, ui| {
                        if described(ui, Role::CheckBox, Some(mark), &name, &tooltip).clicked() {
                            on_toggle(app);
                        }
                    },
                    |b| {
                        if mark == Mark::On {
                            b.paint(tw().size_px(glyph), paint_check);
                        }
                    },
                );
                if let Some(label) = label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
