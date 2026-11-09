//! The owned button: a node that senses clicks, with a text child.

use std::fmt::Debug;
use std::hash::Hash;

use egui::Role;

use super::Mark;
use super::{Enablement, centered, described};
use crate::contract::{Size, Variant};
use crate::{Builder, Style, Tw, tw};

#[derive(Clone, Copy, Debug)]
enum Kind {
    Variant(Option<Variant>),
    Toggle(Mark),
}

/// Whether pressing a button shifts it down, as web buttons commonly do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Shifts,
    Stays,
}

/// A button drawn from the [`Style`]: one of its variants, or a toggle.
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
    kind: Kind,
    size: Size,
    name: Option<String>,
    enablement: Enablement,
    press: Press,
    adjust: fn(Tw) -> Tw,
}

impl Button {
    /// A button labelled `label` in the style's default variant.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: Kind::Variant(None),
            size: Size::DEFAULT,
            name: None,
            enablement: Enablement::Enabled,
            press: Press::Shifts,
            adjust: std::convert::identity,
        }
    }

    /// A toggle that shows whether it is `mark`ed on.
    pub fn toggle(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            kind: Kind::Toggle(mark),
            ..Self::new(label)
        }
    }

    /// Draws the button as the style's variant `variant`.
    pub fn variant(self, variant: Variant) -> Self {
        Self {
            kind: Kind::Variant(Some(variant)),
            ..self
        }
    }

    /// Draws the button at the style's size `size`.
    pub fn size(self, size: Size) -> Self {
        Self { size, ..self }
    }

    /// Marks a button that opens a popup, which stays put when pressed.
    pub fn opens_popup(self) -> Self {
        Self {
            press: Press::Stays,
            ..self
        }
    }

    /// Changes the button's classes, such as its width or how it grows.
    pub fn classes(mut self, adjust: fn(Tw) -> Tw) -> Self {
        self.adjust = adjust;
        self
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
        let button = &style.button;
        let (look, metrics, selected) = match self.kind {
            Kind::Variant(variant) => (
                *button
                    .variants
                    .get(variant.unwrap_or(button.variants.default_key())),
                *button.sizes.get(self.size),
                None,
            ),
            Kind::Toggle(mark) => (
                match mark {
                    Mark::Off => button.toggle_off,
                    Mark::On => button.toggle_on,
                }
                .bordered(button.toggle_border_width),
                *button.toggle_sizes.get(self.size),
                Some(mark),
            ),
        };
        let mut classes = look.classes(&style, metrics.radius, self.enablement, button.transition);
        if self.enablement == Enablement::Enabled && self.press == Press::Shifts {
            classes = classes.active(|p| p.translate_y(button.press_shift));
        }
        let classes = centered(classes)
            .h_px(metrics.height)
            .min_w_px(metrics.min_width)
            .px(metrics.padding_x / 4.0)
            .gap(metrics.gap / 4.0)
            .role(style.text.clone())
            .whitespace_nowrap();
        let classes = (self.adjust)(classes);
        let name = self.name.unwrap_or_else(|| self.label.clone());
        b.div_id(
            key,
            classes,
            move |app, ui| {
                if described(ui, Role::Button, selected, &name).clicked() {
                    on_click(app);
                }
            },
            |b| b.text(tw(), self.label),
        );
    }
}
