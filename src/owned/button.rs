//! The owned button: a node that senses clicks, with a text child.

use std::fmt::Debug;

use egui::Role;

use super::Mark;
use super::{Enablement, OnResponse, Tip, centered, described};
use crate::builder::Slot;
use crate::contract::{Size, Variant};
use crate::{Builder, Icon, Style, Tw, tw};

/// Whether pressing a button shifts it down, as web buttons commonly do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Shifts,
    Stays,
}

/// What a button shows: a label, or only an icon.
///
/// A `&str` or `String` is a label and an [`Icon`] is an icon-only face,
/// which assistive technology can read only from [`named`](ButtonChain::named).
#[derive(Clone, Debug)]
pub enum Face {
    /// Text, with no icon unless [`icon`](ButtonChain::icon) adds one.
    Label(String),
    /// An icon and no text.
    Icon(Icon),
}

impl From<&str> for Face {
    fn from(label: &str) -> Self {
        Self::Label(label.to_owned())
    }
}

impl From<String> for Face {
    fn from(label: String) -> Self {
        Self::Label(label)
    }
}

impl From<Icon> for Face {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon)
    }
}

/// A button drawn from the [`Style`]: one of its variants, with
/// an icon, a label or both. Start one with [`Builder::button`] and finish it
/// with [`on_click`](Self::on_click), or [`add`](Self::add) for a button that
/// does nothing.
///
/// A button without a visible label has nothing for assistive technology to
/// read, so it needs a [`named`](Self::named) name; one without panics in a
/// debug build when it is added.
#[must_use = "a button is not added until you call `.on_click(..)` or `.add()`"]
pub struct ButtonChain<'a, 'f, A> {
    b: Builder<'a, 'f, A>,
    slot: Slot,
    label: String,
    toggle: Option<Mark>,
    opts: ButtonOptions,
    on_response: OnResponse<A>,
}

/// The settings of a button or toggle that do not depend on its key or its
/// face, as a plain value that can be built once and applied to many buttons
/// with [`with_options`](ButtonChain::with_options).
///
/// A setting that was never set leaves the button's own (or the style's) in
/// place. When two sets of options meet in [`merge`](Self::merge), a setting
/// the second one has set wins, and classes merge as [`Tw::merge`] does.
///
/// ```
/// use bullseye::owned::ButtonOptions;
/// use bullseye::Builder;
/// use bullseye::contract::Size;
///
/// fn verb() -> ButtonOptions {
///     ButtonOptions::new().size(Size::DEFAULT).tooltip("Does the thing")
/// }
///
/// fn view(b: &mut Builder<'_, '_, ()>) {
///     b.button("save", "Save").with_options(verb()).add();
///     b.button("send", "Send").with_options(verb()).named("Send now").add();
/// }
/// ```
#[derive(Clone)]
pub struct ButtonOptions {
    icon: Option<Icon>,
    variant: Option<Variant>,
    size: Option<Size>,
    name: Option<String>,
    tooltip: Tip,
    enablement: Option<Enablement>,
    press: Option<Press>,
    classes: Tw,
}

impl Default for ButtonOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonOptions {
    /// Options that set nothing.
    pub fn new() -> Self {
        Self {
            icon: None,
            variant: None,
            size: None,
            name: None,
            tooltip: None,
            enablement: None,
            press: None,
            classes: tw(),
        }
    }

    /// `self` with every setting `other` has set taking `other`'s value, and
    /// the classes of both merged, `other`'s winning ([`Tw::merge`]).
    pub fn merge(self, other: Self) -> Self {
        Self {
            icon: other.icon.or(self.icon),
            variant: other.variant.or(self.variant),
            size: other.size.or(self.size),
            name: other.name.or(self.name),
            tooltip: other.tooltip.or(self.tooltip),
            enablement: other.enablement.or(self.enablement),
            press: other.press.or(self.press),
            classes: self.classes.merge(other.classes),
        }
    }

    /// Puts `icon` before the label, tinted by the button's text colour in
    /// each state and sized by the style's glyph size for the button's size.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draws the button as the style's variant `variant`. A toggle takes the
    /// variant's off and on looks from the style's toggle variants.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Draws the button at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Marks a button that opens a popup, which stays put when pressed.
    pub fn opens_popup(mut self) -> Self {
        self.press = Some(Press::Stays);
        self
    }

    /// Classes merged over the styled ones, such as the button's width or how
    /// it grows; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.classes = self.classes.merge(classes);
        self
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
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

    /// Whether the button takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = Some(enablement);
        self
    }
}

impl<'a, 'f, A: 'static> ButtonChain<'a, 'f, A> {
    pub(super) fn new<K: crate::Key>(b: Builder<'a, 'f, A>, key: K, face: Face) -> Self {
        let (label, icon) = match face {
            Face::Label(label) => (label, None),
            Face::Icon(icon) => (String::new(), Some(icon)),
        };
        Self {
            slot: b.slot(&key),
            b,
            label,
            toggle: None,
            opts: ButtonOptions {
                icon,
                ..ButtonOptions::new()
            },
            on_response: OnResponse::default(),
        }
    }

    /// Makes the button a toggle that shows `mark`.
    pub(super) fn marked(mut self, mark: Mark) -> Self {
        self.toggle = Some(mark);
        self
    }

    /// Applies shared `options`: what they set overrides what the chain has
    /// so far, and what is set after them overrides them.
    pub fn with_options(mut self, options: ButtonOptions) -> Self {
        self.opts = self.opts.merge(options);
        self
    }

    /// Puts `icon` before the label, tinted by the button's text colour in
    /// each state and sized by the style's glyph size for the button's size.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.opts = self.opts.icon(icon);
        self
    }

    /// Draws the button as the style's variant `variant`. A toggle takes the
    /// variant's off and on looks from the style's toggle variants.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.opts = self.opts.variant(variant);
        self
    }

    /// Draws the button at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.opts = self.opts.size(size);
        self
    }

    /// Marks a button that opens a popup, which stays put when pressed.
    pub fn opens_popup(mut self) -> Self {
        self.opts = self.opts.opens_popup();
        self
    }

    /// Classes merged over the styled ones, such as the button's width or how
    /// it grows; the later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.opts = self.opts.classes(classes);
        self
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.opts = self.opts.named(name);
        self
    }

    /// Shown while the pointer hovers the control, as egui's `on_hover_text`
    /// shows it: with egui's timing and style, and also when the control is
    /// disabled, as an HTML `title` is. It adds to the control's accessible
    /// name and does not replace it.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.opts = self.opts.tooltip(text);
        self
    }

    /// Runs `then` with the button's response each frame: the escape hatch for what the
    /// builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.on_response.set(then);
        self
    }

    /// Whether the button takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.opts = self.opts.enablement(enablement);
        self
    }

    /// Adds a button that does nothing when it is clicked: a placeholder, or
    /// one that is disabled.
    pub fn add(self) {
        self.on_click(|_| {});
    }

    /// Adds the button. `on_click` runs when it is clicked or activated with
    /// Space or Enter.
    pub fn on_click(self, on_click: impl FnOnce(&mut A) + 'static) {
        let Self {
            mut b,
            slot,
            label,
            toggle,
            opts:
                ButtonOptions {
                    icon,
                    variant,
                    size,
                    name,
                    tooltip,
                    enablement,
                    press,
                    classes: caller,
                },
            on_response,
        } = self;
        let size = size.unwrap_or(Size::DEFAULT);
        let enablement = enablement.unwrap_or(Enablement::Enabled);
        let press = press.unwrap_or(Press::Shifts);
        debug_assert!(
            !label.is_empty() || name.is_some(),
            "the button {} has no label and no name: call `named`",
            slot.readable(),
        );
        let style = Style::of(b.ctx());
        let button = &style.button;
        let variant = |keyed_default: Variant| variant.unwrap_or(keyed_default);
        let (look, metrics, selected) = match toggle {
            None => (
                *button.variants.get(variant(button.variants.default_key())),
                *button.sizes.get(size),
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
                    *button.toggle_sizes.get(size),
                    Some(mark),
                )
            }
        };
        let mut classes = look.classes(&style, metrics.radius, enablement, button.transition);
        if enablement == Enablement::Enabled && press == Press::Shifts {
            classes = classes.active(|p| p.translate_y(button.press_shift));
        }
        let classes = centered(classes)
            .h_px(metrics.height)
            .min_w_px(metrics.min_width)
            .px(metrics.padding_x / 4.0)
            .gap(metrics.gap / 4.0)
            .role(style.role(&button.text).clone())
            .whitespace_nowrap();
        let classes = enablement.fade(classes, &style).merge(caller);
        let name = name.unwrap_or_else(|| label.clone());
        let glyph = metrics.glyph;
        b.div_id_with(
            slot.id,
            &slot.label,
            classes,
            move |app, ui| {
                let response = described(ui, Role::Button, selected, &name, &tooltip);
                if response.clicked() {
                    on_click(app);
                }
                on_response.run(app, &response);
            },
            |b| {
                if let Some(icon) = icon {
                    b.glyph(tw().size_px(glyph).shrink_0(), icon);
                }
                if !label.is_empty() {
                    b.text(tw(), label);
                }
            },
        );
    }
}
