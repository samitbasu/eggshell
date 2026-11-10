//! The owned toggle: a button that shows whether it is on.

use super::{ButtonChain, ButtonOptions, Enablement, Face, Unset, flip};
use crate::contract::{Size, Variant};
use crate::{Builder, Icon, Scope, Tw};

/// A toggle button drawn from the [`Style`](crate::Style)'s toggle looks,
/// with an icon, a label or both. Start one with [`Builder::toggle`] and
/// finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is pressed when that is its `on` value: `true` for a
/// `bool`, or the second of [`values`](Self::values). A click passes the other
/// value to `on_change`. Its other options are those of a
/// [`ButtonChain`].
///
/// ```
/// # use bullseye::Builder;
/// #[derive(Clone, Copy, PartialEq)]
/// enum Weight { Normal, Bold }
/// struct App { weight: Weight, italic: bool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     b.toggle("bold", "Bold", app.weight)
///         .values(Weight::Normal, Weight::Bold)
///         .on_change(|app, weight| app.weight = weight);
///     b.toggle("italic", "Italic", app.italic)
///         .on_change(|app, italic| app.italic = italic);
/// }
/// ```
#[must_use = "a toggle is not added until you call `.on_change(..)`"]
pub struct ToggleChain<'a, 'f, A, T, V = Unset> {
    button: ButtonChain<'a, 'f, A>,
    current: T,
    values: V,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> ToggleChain<'a, 'f, A, T> {
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        face: Face,
        current: T,
    ) -> Self {
        Self {
            button: ButtonChain::new(b, key, face),
            current,
            values: Unset,
        }
    }

    /// The values the toggle switches between: `off` when released and `on`
    /// when pressed.
    pub fn values(self, off: T, on: T) -> ToggleChain<'a, 'f, A, T, [T; 2]> {
        ToggleChain {
            button: self.button,
            current: self.current,
            values: [off, on],
        }
    }
}

impl<'a, 'f, A: 'static, T, V> ToggleChain<'a, 'f, A, T, V> {
    /// Applies shared `options`: what they set overrides what the chain has
    /// so far, and what is set after them overrides them.
    pub fn with_options(mut self, options: ButtonOptions) -> Self {
        self.button = self.button.with_options(options);
        self
    }

    /// Puts `icon` before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.button = self.button.icon(icon);
        self
    }

    /// Draws the toggle as the off and on looks of the style's toggle variant
    /// `variant`.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.button = self.button.variant(variant);
        self
    }

    /// Draws the toggle at the style's size `size`.
    pub fn size(mut self, size: Size) -> Self {
        self.button = self.button.size(size);
        self
    }

    /// Classes merged over the styled ones, such as the toggle's width; the
    /// later class wins ([`Tw::merge`]).
    pub fn classes(mut self, classes: Tw) -> Self {
        self.button = self.button.classes(classes);
        self
    }

    /// The accessible name, when it is not the label.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.button = self.button.named(name);
        self
    }

    /// Shown while the pointer hovers the control, also when it is disabled.
    pub fn tooltip(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.button = self.button.tooltip(text);
        self
    }

    /// Whether the toggle takes input.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.button = self.button.enablement(enablement);
        self
    }

    /// Builds `body`'s nodes inside the toggle after its face; see
    /// [`ButtonChain::children`].
    pub fn children(mut self, body: impl for<'s> FnOnce(&mut Scope<'s, 'f, A>) + 'a) -> Self {
        self.button = self.button.children(body);
        self
    }

    /// Runs `then` with the toggle's response each frame: the escape hatch
    /// for what the builder does not offer, such as hover.
    pub fn on_response(mut self, then: impl Fn(&mut A, &egui::Response) + 'static) -> Self {
        self.button = self.button.on_response(then);
        self
    }
}

impl<'a, 'f, A: 'static> ToggleChain<'a, 'f, A, bool> {
    /// Adds the toggle. `on_change` runs with the new value when it is
    /// clicked or activated with Space or Enter.
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> ToggleChain<'a, 'f, A, T, [T; 2]> {
    /// Adds the toggle. `on_change` runs with the new value when it is
    /// clicked or activated with Space or Enter.
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        let (mark, toggled) = flip(self.values, self.current);
        self.button
            .marked(mark)
            .on_click(move |app| on_change(app, toggled));
    }
}
