//! The owned toggle group: toggles of which exactly one is on.

use std::fmt::{self, Debug};
use std::hash::Hash;
use std::rc::Rc;

use egui::Id;

use super::toggle::{ToggleChain, ToggleItemChain};
use super::{ButtonOptions, Face};
use crate::key::child_id;
use crate::{Builder, Key};

type OnChange<A, T> = Rc<dyn Fn(&mut A, T)>;

/// A set of toggles of which exactly one is on, as Radix's ToggleGroup of
/// `type="single"`: radio semantics with toggle looks, with icons, labels or
/// any [`children`](super::ButtonChain::children) as the members' content.
///
/// A group is a plain value: its key, the application's current value and
/// the options and handler the members share. A member is on exactly when the
/// current value is its own, and the current value is read from the
/// application each frame, so a write from elsewhere shows up. Members are
/// placed with [`Builder::toggle_item`], anywhere below the builder, with
/// other nodes between them; the group is borrowed, so nested closures can
/// place members from it.
///
/// Pressing a member calls the group's `on_change` with the member's value.
/// Pressing the member that is already on keeps it on and does not call
/// `on_change`.
///
/// A member's key is the group's key with the member's value, so two members
/// with the same value in one scope are a duplicate path, a bug that panics
/// in a debug build. Members are reported to assistive technology as radio
/// buttons.
///
/// ```
/// use bullseye::owned::{ButtonOptions, ToggleGroup};
/// use bullseye::{Builder, tw};
///
/// #[derive(Clone, Copy, Debug, Hash, PartialEq)]
/// enum Tool { Select, Pan, Zoom }
/// struct App { tool: Tool }
///
/// fn view(b: &mut Builder<'_, '_, App>, app: &App) {
///     let tools = ToggleGroup::new("tools", app.tool)
///         .with_options(ButtonOptions::new().classes(tw().w_px(80.0)))
///         .on_change(|app, tool| app.tool = tool);
///     b.div(tw().flex().gap(2.0), |b| {
///         b.toggle_item(&tools, Tool::Select, "Select").add();
///         b.div(tw().flex(), |b| {
///             b.toggle_item(&tools, Tool::Pan, "Pan").add();
///         });
///         b.toggle_item(&tools, Tool::Zoom, "Zoom").tooltip("Z").add();
///     });
/// }
/// ```
pub struct ToggleGroup<A, T> {
    id: Id,
    label: Rc<str>,
    current: T,
    options: ButtonOptions,
    // Shared by reference count: each member's click closure is `'static` and
    // outlives the borrow of the group, and the handler is not `Clone`.
    on_change: Option<OnChange<A, T>>,
}

impl<A: 'static, T: Copy + PartialEq + Hash + Debug + 'static> ToggleGroup<A, T> {
    /// A group named `key` whose `current` member is on.
    pub fn new<K: Key>(key: K, current: T) -> Self {
        Self {
            id: child_id(Id::NULL, &key),
            label: format!("{key:?}").into(),
            current,
            options: ButtonOptions::new(),
            on_change: None,
        }
    }

    /// Options every member starts from; a member's own settings override
    /// them. Merged over earlier options, the later winning.
    pub fn with_options(mut self, options: ButtonOptions) -> Self {
        self.options = self.options.merge(options);
        self
    }

    /// Runs `on_change` with a member's value when it is pressed and was not
    /// on.
    pub fn on_change(mut self, on_change: impl Fn(&mut A, T) + 'static) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }

    pub(super) fn item<'a, 'f>(
        &self,
        b: Builder<'a, 'f, A>,
        value: T,
        face: Face,
    ) -> ToggleItemChain<'a, 'f, A, T> {
        let key = MemberKey {
            group: self.id,
            label: self.label.clone(),
            value,
        };
        ToggleChain::new(b, key, face, self.current)
            .with_options(self.options.clone())
            .member(Member {
                value,
                on_change: self.on_change.clone(),
            })
    }
}

/// What a [`ToggleItemChain`] knows of its group: its own value and the
/// group's handler.
pub struct Member<A, T> {
    pub(super) value: T,
    pub(super) on_change: Option<OnChange<A, T>>,
}

#[derive(Hash)]
struct MemberKey<T> {
    group: Id,
    label: Rc<str>,
    value: T,
}

impl<T: Debug> Debug for MemberKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={:?}", self.label, self.value)
    }
}
