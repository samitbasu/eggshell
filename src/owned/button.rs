//! The owned button: a node that senses clicks, with a text child.

use std::fmt::Debug;

use egui::Role;

use super::Mark;
use super::{Enablement, centered, described};
use crate::contract::{Size, Variant};
use crate::{Builder, Icon, Style, Tw, tw};

/// Whether pressing a button shifts it down, as web buttons commonly do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Shifts,
    Stays,
}

/// A button drawn from the [`Style`]: one of its variants, or a toggle, with
/// an icon, a label or both.
///
/// A button without a visible label has nothing for assistive technology to
/// read, so it needs a [`named`](Self::named) name:
/// [`icon_only`](Self::icon_only) takes it as an argument, which makes the
/// omission a compile error, and a button given an empty label by hand
/// ([`new`](Self::new), [`toggle`](Self::toggle)) without a name panics in a
/// debug build when it is shown.
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
    icon: Option<Icon>,
    toggle: Option<Mark>,
    variant: Option<Variant>,
    size: Size,
    name: Option<String>,
    enablement: Enablement,
    press: Press,
    classes: Tw,
}

impl Button {
    /// A button labelled `label` in the style's default variant.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            toggle: None,
            variant: None,
            size: Size::DEFAULT,
            name: None,
            enablement: Enablement::Enabled,
            press: Press::Shifts,
            classes: tw(),
        }
    }

    /// A button that shows only `icon`, sized by the style's glyph size for
    /// the button's size, and that assistive technology calls `name`.
    pub fn icon_only(icon: Icon, name: impl Into<String>) -> Self {
        Self::new("").icon(icon).named(name)
    }

    /// A toggle that shows whether it is `mark`ed on.
    pub fn toggle(label: impl Into<String>, mark: Mark) -> Self {
        Self {
            toggle: Some(mark),
            ..Self::new(label)
        }
    }

    /// Puts `icon` before the label, tinted by the button's text colour in
    /// each state and sized by the style's glyph size for the button's size.
    pub fn icon(self, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// Draws the button as the style's variant `variant`. A toggle takes the
    /// variant's off and on looks from the style's toggle variants.
    pub fn variant(self, variant: Variant) -> Self {
        Self {
            variant: Some(variant),
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

    /// Classes merged over the styled ones, such as the button's width or how
    /// it grows; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = classes;
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
    pub(crate) fn show<A: 'static, K: crate::Key>(
        self,
        b: &mut Builder<'_, '_, A>,
        key: K,
        on_click: impl FnOnce(&mut A) + 'static,
    ) {
        debug_assert!(
            !self.label.is_empty() || self.name.is_some(),
            "the button {key:?} has no label and no name: call `named`",
        );
        let style = Style::of(b.ctx());
        let button = &style.button;
        let variant = |keyed_default: Variant| self.variant.unwrap_or(keyed_default);
        let (look, metrics, selected) = match self.toggle {
            None => (
                *button.variants.get(variant(button.variants.default_key())),
                *button.sizes.get(self.size),
                None,
            ),
            Some(mark) => {
                let looks = match mark {
                    Mark::Off => &button.toggle_off,
                    Mark::On => &button.toggle_on,
                };
                (
                    looks
                        .get(variant(looks.default_key()))
                        .bordered(button.toggle_border_width),
                    *button.toggle_sizes.get(self.size),
                    Some(mark),
                )
            }
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
            .role(style.role(&button.text).clone())
            .whitespace_nowrap();
        let classes = self.enablement.fade(classes, &style).merge(self.classes);
        let name = self.name.unwrap_or_else(|| self.label.clone());
        let glyph = metrics.glyph;
        b.div_id(
            key,
            classes,
            move |app, ui| {
                if described(ui, Role::Button, selected, &name).clicked() {
                    on_click(app);
                }
            },
            |b| {
                if let Some(icon) = self.icon {
                    b.glyph(tw().size_px(glyph).shrink_0(), icon);
                }
                if !self.label.is_empty() {
                    b.text(tw(), self.label);
                }
            },
        );
    }
}
