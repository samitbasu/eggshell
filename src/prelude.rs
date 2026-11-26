//! TODO(prose): what an app imports with `use eggshell_ui::prelude::*`: the
//! builder, the classes and the style, the traits of the options the owned
//! controls share ([`Configure`](crate::owned::options::Configure),
//! [`Interactive`](crate::owned::options::Interactive) and
//! [`Respond`](crate::owned::options::Respond)), and every compiled-in
//! widget's extension trait, all unnamed, so that `.tooltip(..)`,
//! `b.badge(..)`, `b.tabs(..)` and the rest are in scope.

pub use crate::owned::options::{Configure as _, Interactive as _, Respond as _};
pub use crate::{Builder, Style, Tw, tw};

#[cfg(feature = "accordion")]
pub use crate::owned::accordion::Ext as _;
#[cfg(feature = "alert")]
pub use crate::owned::alert::Ext as _;
#[cfg(feature = "badge")]
pub use crate::owned::badge::Ext as _;
#[cfg(feature = "command")]
pub use crate::owned::command::Ext as _;
#[cfg(feature = "dialog")]
pub use crate::owned::dialog::Ext as _;
#[cfg(feature = "dialog")]
pub use crate::owned::dialog::PopoverExt as _;
#[cfg(feature = "disclosure")]
pub use crate::owned::disclosure::Ext as _;
#[cfg(feature = "disclosure")]
pub use crate::owned::disclosure::SectionExt as _;
#[cfg(feature = "form")]
pub use crate::owned::form::Ext as _;
#[cfg(feature = "hover-card")]
pub use crate::owned::hover_card::Ext as _;
#[cfg(feature = "markdown")]
pub use crate::owned::markdown::Ext as _;
#[cfg(feature = "navigation-menu")]
pub use crate::owned::navigation_menu::Ext as _;
#[cfg(feature = "notify")]
pub use crate::owned::notify::Ext as _;
#[cfg(feature = "skeleton")]
pub use crate::owned::skeleton::Ext as _;
#[cfg(feature = "spinner")]
pub use crate::owned::spinner::Ext as _;
#[cfg(feature = "tabs")]
pub use crate::owned::tabs::Ext as _;
