//! The owned checkbox: a box node that senses clicks and holds a check
//! glyph, beside a label node that toggles it too.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, WidgetInfo};

use super::{Enablement, choice_label, choice_row, paint_check};
use crate::style::Mark;
use crate::{Builder, NodeUi as _, Style, tw};

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
        on_toggle: fn(&mut A),
    ) {
        let style = Style::of(b.ctx());
        let checkbox = &style.checkbox;
        let look = match self.mark {
            Mark::Off => &checkbox.off,
            Mark::On => &checkbox.on,
        };
        let enablement = self.enablement;
        let box_classes = look
            .classes(&style, checkbox.radius, enablement, checkbox.transition)
            .flex()
            .items_center()
            .justify_center()
            .shrink_0()
            .size_px(checkbox.size);
        let (glyph, mark) = (checkbox.glyph, self.mark);
        let name = self.name.unwrap_or_default();
        b.scope(key, |b| {
            let control = b.id_of(Part::Box);
            b.div(choice_row(&style, enablement, checkbox.gap), |b| {
                b.div_id(
                    Part::Box,
                    box_classes,
                    move |app, ui| {
                        let response = ui.node_response();
                        if response.clicked() {
                            on_toggle(app);
                        }
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                Role::CheckBox,
                                ui.is_enabled(),
                                mark == Mark::On,
                                &name,
                            )
                        });
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
