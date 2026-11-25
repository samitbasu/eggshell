//! The owned toggle: a button that shows whether it is on.

use egui::Role;

use super::toggle_group::Member;
use super::{Enablement, Mark, Unset, button, button::Face, flip};
use crate::contract::{Size, Variant};
use crate::{Builder, Icon, Scope, Tw};

/// A member of a [`toggle_group::Group`](super::toggle_group::Group): a toggle with the
/// group's options, finished with [`add`](Chain::add). Start one with
/// [`Builder::toggle_item`].
pub type ItemChain<'a, 'f, A, T> = Chain<'a, 'f, A, T, Member<A, T>>;

/// A toggle button drawn from the [`Style`](crate::Style)'s toggle looks,
/// with an icon, a label or both. Start one with [`Builder::toggle`] and
/// finish it with [`on_change`](Self::on_change).
///
/// It shows `current` and is pressed when that is its `on` value: `true` for a
/// `bool`, or the second of [`values`](Self::values). A click passes the other
/// value to `on_change`. Its other options are those of a
/// [`button::Chain`].
///
/// ```
/// # use eggshell_ui::Builder;
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
pub struct Chain<'a, 'f, A, T, V = Unset> {
    button: button::Chain<'a, 'f, A>,
    current: T,
    values: V,
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T> {
    pub(super) fn new<K: crate::Key>(
        b: Builder<'a, 'f, A>,
        key: K,
        face: Face,
        current: T,
    ) -> Self {
        Self {
            button: button::Chain::new(b, key, face),
            current,
            values: Unset,
        }
    }

    /// Makes the toggle a member of a toggle group: on when `current` is the
    /// member's value, reported as a radio button, and finished with
    /// [`add`](Chain::add).
    pub(super) fn member(self, member: Member<A, T>) -> ItemChain<'a, 'f, A, T> {
        Chain {
            button: self.button.role(Role::RadioButton),
            current: self.current,
            values: member,
        }
    }

    /// The values the toggle switches between: `off` when released and `on`
    /// when pressed.
    pub fn values(self, off: T, on: T) -> Chain<'a, 'f, A, T, [T; 2]> {
        Chain {
            button: self.button,
            current: self.current,
            values: [off, on],
        }
    }
}

impl<'a, 'f, A: 'static, T, V> Chain<'a, 'f, A, T, V> {
    /// TODO(prose): takes focus as the overlay it is in opens, as
    /// [`NodeHandle::focus_on_open`](crate::NodeHandle::focus_on_open) does.
    pub fn focus_on_open(mut self) -> Self {
        self.button = self.button.focus_on_open();
        self
    }

    /// TODO(prose): takes focus on every frame this is set.
    pub fn focus_now(mut self) -> Self {
        self.button = self.button.focus_now();
        self
    }

    /// Applies shared `options`: what they set overrides what the chain has
    /// so far, and what is set after them overrides them.
    pub fn with_options(mut self, options: button::Options) -> Self {
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

    /// Shown while the pointer hovers the control, also when it is disabled.
    pub fn tooltip(mut self, text: impl super::options::IntoTip) -> Self {
        self.button = self.button.tooltip(text);
        self
    }

    /// TODO(prose): the accessible name, in place of the face's, as
    /// `aria-label` is.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.button = self.button.named(name);
        self
    }

    /// Like [`tooltip`](Self::tooltip), with content `build` makes inside the
    /// tooltip's box, such as text and a [`kbd`](crate::Builder::kbd) chip.
    pub fn tooltip_with(
        mut self,
        build: impl Fn(&mut crate::Builder<'_, '_, ()>) + Send + Sync + 'static,
    ) -> Self {
        self.button = self.button.tooltip_with(build);
        self
    }

    /// Whether the toggle takes input.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.button = self.button.enablement(enablement);
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason as
    /// the tooltip and the accessible description. `None` clears it; the last
    /// call wins.
    pub fn refused(mut self, reason: impl super::options::IntoTip) -> Self {
        self.button = self.button.refused(reason);
        self
    }

    /// TODO(prose): the chain through `then`, when `condition` holds.
    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        super::options::when(self, condition, then)
    }

    /// TODO(prose): the chain through `then` with the value, when there is one.
    pub fn when_some<V2>(self, value: Option<V2>, then: impl FnOnce(Self, V2) -> Self) -> Self {
        super::options::when_some(self, value, then)
    }

    /// Builds `body`'s nodes inside the toggle after its face; see
    /// [`button::Chain::children`].
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

impl<'a, 'f, A: 'static> Chain<'a, 'f, A, bool> {
    /// Adds the toggle. `on_change` runs with the new value when it is
    /// clicked or activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, bool) + 'static) {
        self.values(false, true).on_change(on_change);
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> Chain<'a, 'f, A, T, [T; 2]> {
    /// Adds the toggle. `on_change` runs with the new value when it is
    /// clicked or activated with Space or Enter.
    #[track_caller]
    pub fn on_change(self, on_change: impl Fn(&mut A, T) + 'static) {
        let (mark, toggled) = flip(self.values, self.current);
        self.button
            .marked(mark)
            .on_click(move |app| on_change(app, toggled));
    }
}

impl<'a, 'f, A: 'static, T: Copy + PartialEq + 'static> ItemChain<'a, 'f, A, T> {
    /// Adds the member. It is on when the group's current value is its own;
    /// pressing it calls the group's `on_change` with its value, unless it is
    /// already on.
    #[track_caller]
    pub fn add(self) {
        let Member { value, on_change } = self.values;
        let mark = Mark::from(self.current == value);
        self.button.marked(mark).on_click(move |app| {
            if mark == Mark::Off
                && let Some(on_change) = on_change
            {
                on_change(app, value);
            }
        });
    }
}

impl<'b, 'f, A: 'static> crate::Builder<'b, 'f, A> {
    /// A toggle button showing `face` and `current`: a `bool`, or with
    /// [`values`](Chain::values) any two-valued type. Assistive
    /// technology sees a `Role::Button`, toggled on or off.
    pub fn toggle<K: crate::Key, T: Copy + PartialEq + 'static>(
        &mut self,
        key: K,
        face: impl Into<Face>,
        current: T,
    ) -> Chain<'_, 'f, A, T> {
        Chain::new(self.reborrow(), key, face.into(), current)
    }

    /// A member of `group` that is on when the group's current value is
    /// `value`, showing `face`. Members may sit anywhere below the builder
    /// that places them, with other nodes between them. Assistive technology
    /// sees each member as a `Role::RadioButton`.
    pub fn toggle_item<T: Copy + PartialEq + std::hash::Hash + std::fmt::Debug + 'static>(
        &mut self,
        group: &super::toggle_group::Group<A, T>,
        value: T,
        face: impl Into<Face>,
    ) -> ItemChain<'_, 'f, A, T> {
        group.item(self.reborrow(), value, face.into())
    }
}
