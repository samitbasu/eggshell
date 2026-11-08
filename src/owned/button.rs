//! The owned button: a node that senses clicks, with a text child.

use std::fmt::Debug;
use std::hash::Hash;

use egui::{Role, WidgetInfo};

use super::style::{Look, Metrics, Style};
use super::{Enablement, variant};
use crate::style::Mark;
use crate::{Builder, NodeUi as _, tw};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Variant(&'static str),
    Toggle(Mark),
}

/// A button drawn from the [`Style`]: one of its variants, or a toggle.
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
    shape: Shape,
    name: Option<String>,
    enablement: Enablement,
}

impl Button {
    /// A `secondary` button labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shape: Shape::Variant(variant::SECONDARY),
            name: None,
            enablement: Enablement::Enabled,
        }
    }

    /// A toggle that shows whether it is `mark`ed on.
    pub fn toggle(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            shape: Shape::Toggle(mark),
            ..Self::new(label)
        }
    }

    /// Draws the button as the style's variant `name`.
    pub fn variant(self, name: &'static str) -> Self {
        Self {
            shape: Shape::Variant(name),
            ..self
        }
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the button takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// Adds the button to `b` under `key`. `on_click` runs when it is clicked
    /// or activated with Space or Enter.
    pub fn show<A: 'static, K: Hash + Debug + 'static>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        on_click: impl FnOnce(&mut A) + 'static,
    ) {
        let style = Style::of(b.ctx());
        let (look, metrics, selected): (&Look, &Metrics, Option<bool>) = match self.shape {
            Shape::Variant(name) => (style.button.variant(name), &style.button.metrics, None),
            Shape::Toggle(mark) => (
                style.button.toggle(mark),
                &style.button.toggle_metrics,
                Some(mark == Mark::On),
            ),
        };
        let classes = look
            .classes(metrics.radius, self.enablement, style.button.transition)
            .flex()
            .items_center()
            .justify_center()
            .h_px(metrics.height)
            .min_w_px(metrics.min_width)
            .px(metrics.padding_x / 4.0)
            .gap(metrics.gap / 4.0)
            .role(style.button.text.clone())
            .whitespace_nowrap();
        let name = self.name.unwrap_or_else(|| self.label.clone());
        b.div_id(
            key,
            classes,
            move |app, ui| {
                let response = ui.node_response();
                if response.clicked() {
                    on_click(app);
                }
                response.widget_info(|| match selected {
                    Some(on) => WidgetInfo::selected(Role::Button, ui.is_enabled(), on, &name),
                    None => WidgetInfo::labeled(Role::Button, ui.is_enabled(), &name),
                });
            },
            |b| b.text(tw(), self.label),
        );
    }
}
