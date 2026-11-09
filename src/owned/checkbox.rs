//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::Role;

use super::Mark;
use super::{Enablement, choice_label, choice_row, described, paint_check};
use crate::{Builder, Style, tw};

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Box,
    Label,
}

/// A checkbox drawn from the [`Style`], with an optional label.
#[derive(Clone, Debug)]
pub struct Checkbox {
    label: Option<String>,
    name: Option<String>,
    mark: Mark,
    enablement: Enablement,
}

impl Checkbox {
    /// A checkbox labelled `label`, ticked when `mark` is on.
    pub fn new(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            label: Some(label.into()),
            name: None,
            mark,
            enablement: Enablement::Enabled,
        }
    }

    /// A checkbox without a label.
    pub fn unlabelled(mark: Mark) -> Self {
        Self {
            label: None,
            ..Self::new("", mark)
        }
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the checkbox takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the checkbox to `b` under `key`. `on_toggle` runs when the box or
    /// the label is clicked, or the box is activated with Space or Enter.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        on_toggle: impl Fn(&mut A) + Copy + 'static,
    ) {
        let style = Style::of(b.ctx());
        let checkbox = &style.checkbox;
        let look = match self.mark {
            Mark::Off => &checkbox.off,
            Mark::On => &checkbox.on,
        };
        let enablement = self.enablement;
        let box_classes = look.indicator(
            checkbox.border_width,
            &style,
            checkbox.radius,
            enablement,
            checkbox.transition,
            checkbox.size,
        );
        let (glyph, mark) = (checkbox.glyph, self.mark);
        let name = self.name.unwrap_or_default();
        b.scope(key, |b| {
            let control = b.id_of(Part::Box);
            b.div(choice_row(&style, enablement, checkbox.gap), |b| {
                b.div_id(
                    Part::Box,
                    box_classes,
                    move |app, ui| {
                        if described(ui, Role::CheckBox, Some(mark), &name).clicked() {
                            on_toggle(app);
                        }
                    },
                    |b| {
                        if mark == Mark::On {
                            b.paint(tw().size_px(glyph), paint_check);
                        }
                    },
                );
                if let Some(label) = self.label {
                    choice_label(b, Part::Label, control, label, enablement, on_toggle);
                }
            });
        });
    }
}
