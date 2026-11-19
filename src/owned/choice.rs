//! The options of the choice widgets: [`Choice`] and the [`IntoChoice`]
//! conversions that let a call site pass whatever it already has.

use std::fmt::Display;

use egui::WidgetText;

use super::options::Refused;
use super::{Enablement, IntoTip, Tooltip};
use crate::Icon;

/// One option of a radio group, a segmented control or a select: the `value`
/// it chooses and how it is shown.
///
/// ```
/// # use eggshell_ui::owned::{Choice, Enablement};
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
    /// TODO(prose): why the option is disabled, shown in place of its
    /// tooltip and read as its accessible description; `None` while it is not
    /// refused.
    pub refusal: Option<WidgetText>,
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
            refusal: None,
        }
    }

    /// Puts `icon` before the label, tinted by the text colour of the option
    /// in each state and sized by the style.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Shows `text` while the pointer hovers the option.
    pub fn tooltip(mut self, text: impl IntoTip) -> Self {
        self.tooltip = text.into_tip();
        self
    }

    /// Whether the option takes input. A disabled option cannot be chosen with
    /// the pointer or the keyboard, is skipped by the arrow keys, is announced
    /// as disabled and is drawn in the disabled look.
    pub fn enablement(mut self, enablement: impl Into<Enablement>) -> Self {
        let enablement = enablement.into();
        self.enablement = enablement;
        self
    }

    /// TODO(prose): disabled while `reason` is `Some`, with the reason in
    /// place of the option's tooltip and as its accessible description. `None`
    /// clears it; the last call wins.
    pub fn refused(mut self, reason: impl IntoTip) -> Self {
        self.refusal = reason.into_tip();
        self
    }

    /// The option's enablement within a group that is `group`, and the
    /// tooltip it shows. A refused group's reason shows once, on the control,
    /// so its options show none of their own.
    pub(super) fn gated(
        &self,
        group: Enablement,
        refused: Refused,
    ) -> (Enablement, Option<Tooltip>) {
        let own_refusal = match self.refusal {
            Some(_) => Enablement::Disabled,
            None => Enablement::Enabled,
        };
        let tooltip = match refused {
            Refused::Yes => None,
            Refused::No => match (&self.refusal, &self.tooltip) {
                (Some(reason), _) => Some(Tooltip::refusal(reason.clone())),
                (None, Some(text)) => Some(Tooltip::text(text.clone())),
                (None, None) => None,
            },
        };
        (group.and(self.enablement).and(own_refusal), tooltip)
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
