//! Spike: simple controls built from bullseye nodes instead of wrapped egui
//! widgets (issue #19). A control is an identified node with a sense,
//! `hover`/`active`/`focus` variants and text or paint children, styled from
//! [`Style`] and measured by taffy, so there is no size estimate to drift.

mod button;
mod checkbox;
mod style;

pub use button::Button;
pub use checkbox::Checkbox;
pub use style::{ButtonStyle, CheckboxStyle, Colors, Focus, Look, Metrics, States, Style, variant};

/// Whether a control takes input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Enablement {
    /// Responds to the pointer and the keyboard.
    #[default]
    Enabled,
    /// Inert, and drawn in the style's explicit disabled colours.
    Disabled,
}
