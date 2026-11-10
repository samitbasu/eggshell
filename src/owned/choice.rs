//! The options of the choice widgets: [`Choice`] and the [`IntoChoice`]
//! conversions that let a call site pass whatever it already has.

use std::fmt::Display;

use egui::WidgetText;

use super::Enablement;
use crate::Icon;

/// One option of a radio group, a segmented control or a select: the `value`
/// it chooses and how it is shown.
///
/// ```
/// # use bullseye::owned::{Choice, Enablement};
/// let beta = Choice::new(2, "Beta")
///     .tooltip("Not final")
///     .enablement(Enablement::Disabled);
/// ```
#[derive(Clone, Debug)]
pub struct Choice<T> {
    /// What choosing the option gives to the handler.
    pub value: T,
    /// The visible text, and the option's accessible name.
    pub label: String,
    /// Drawn before the label by a segmented control; the other widgets show no icon.
    pub icon: Option<Icon>,
    /// Shown while the pointer hovers the option, also when it is disabled.
    /// It adds to the option's accessible name and does not replace it.
    pub tooltip: Option<WidgetText>,
    /// Whether this option takes input, on top of its widget's own.
    pub enablement: Enablement,
}

impl<T> Choice<T> {
    /// An enabled option without icon or tooltip.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            icon: None,
            tooltip: None,
            enablement: Enablement::Enabled,
        }
    }

    /// Puts `icon` before the label, tinted by the text colour of the option
    /// in each state and sized by the style.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Shows `text` while the pointer hovers the option.
    pub fn tooltip(mut self, text: impl Into<WidgetText>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Whether the option takes input. A disabled option cannot be chosen with
    /// the pointer or the keyboard, is skipped by the arrow keys, is announced
    /// as disabled and is drawn in the disabled look.
    pub fn enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }
}

/// What a choice widget accepts as one of its options.
///
/// Implemented for:
/// - `T: Copy + Display`: the value, labelled by its `Display`;
/// - `&T` with `T: Copy + Display`, so a slice of values (`ALL.iter()`) can be
///   passed as it is;
/// - `(T, impl Into<String>)`: the value and its label;
/// - `(T, Icon, impl Into<String>)`: the value, an icon and its label;
/// - [`Choice<T>`], for a tooltip or a disabled option.
pub trait IntoChoice<T> {
    /// The option.
    fn into_choice(self) -> Choice<T>;
}

impl<T: Copy + Display> IntoChoice<T> for T {
    fn into_choice(self) -> Choice<T> {
        Choice::new(self, self.to_string())
    }
}

impl<T: Copy + Display> IntoChoice<T> for &T {
    fn into_choice(self) -> Choice<T> {
        Choice::new(*self, self.to_string())
    }
}

impl<T, S: Into<String>> IntoChoice<T> for (T, S) {
    fn into_choice(self) -> Choice<T> {
        Choice::new(self.0, self.1)
    }
}

impl<T, S: Into<String>> IntoChoice<T> for (T, Icon, S) {
    fn into_choice(self) -> Choice<T> {
        Choice::new(self.0, self.2).icon(self.1)
    }
}

impl<T> IntoChoice<T> for Choice<T> {
    fn into_choice(self) -> Choice<T> {
        self
    }
}

pub(super) fn collect<T, O: IntoChoice<T>>(options: impl IntoIterator<Item = O>) -> Vec<Choice<T>> {
    options.into_iter().map(IntoChoice::into_choice).collect()
}
