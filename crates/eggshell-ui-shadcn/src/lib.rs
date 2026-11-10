#![deny(missing_docs)]
//! The shadcn/ui design system as a [`eggshell_ui::Style`].
//!
//! [`Shadcn`] is a preset: a [`ShadcnStyle`] (which of shadcn's styles, such as
//! [`Vega`](ShadcnStyle::Vega) or [`Lyra`](ShadcnStyle::Lyra)), a
//! [`ThemeColor`] (shadcn's theme colour) and an egui theme (light or dark),
//! chosen independently. Nothing here runs unless the application calls
//! [`Shadcn::apply`]: the library never changes egui's style on its own.
//!
//! The crate only fills the contract. `eggshell` defines what its widgets
//! need and has no shadcn names or values; a style is data, so changing
//! [`ShadcnStyle`] at run time changes sizes, radii, text size and ring width
//! with no code change in the application.
//!
//! Applying a preset does three things. It fills the [`eggshell_ui::Style`]
//! contract with the tokens ([`Shadcn::style`]) and installs it for the
//! preset's theme, which the owned widgets and the text edit of `eggshell`
//! draw from. It maps the [`Tokens`] onto the egui [`Style`](egui::Style) of
//! the preset's theme (widget fills, strokes, radii, selection, spacing) for
//! the egui widgets an application draws outside eggshell. It also remembers
//! the tokens in the context, for [`tokens`] and [`Shadcn::of`].
//!
//! What eggshell cannot draw exactly and the preset therefore approximates is
//! documented on [`eggshell_ui::contract::InputStyle`] for the wrapped text edit;
//! the owned widgets are exact except for font weights (`font-medium` needs a
//! registered family) and the negative spread of `shadow-md`.
//!
//! # Hovered and selected
//!
//! shadcn's components pair the two as follows, and the helpers on
//! [`Tokens`] name the pairs:
//!
//! | Component | Hovered | Selected |
//! |---|---|---|
//! | List or menu row on the page | `accent/50` ([`Tokens::accent_50`]) | `accent` with `accent-foreground` |
//! | Table row | `muted/50` ([`Tokens::muted_50`]) | `muted` |
//! | Ghost and outline button | `accent` with `accent-foreground` (dark: `accent/50`, `input/50`) | |
//! | Secondary, primary, destructive button | `secondary/80`, `primary/90`, `destructive/90` | |
//!
//! In the neutral palette `muted`, `accent` and `secondary` are one colour,
//! so a hover on a `muted` surface needs the half-strength variant of the
//! selected colour, not a different token.

mod egui_map;
mod preset;
mod styles;
mod theme_color;
mod tokens;

pub use preset::{Shadcn, tokens};
pub use styles::{DestructiveLook, Elevation, HoverSurface, SecondaryHover, ShadcnStyle};
pub use theme_color::ThemeColor;
pub use tokens::Tokens;

/// The names of the button sizes this preset defines besides the default.
pub mod size {
    use eggshell_ui::contract::Size;

    /// `icon`: a square the height of a control, for a button that is only an
    /// icon.
    pub const ICON: Size = Size("icon");
}

/// The names of shadcn's button variants, the keys of the contract's
/// [`ButtonStyle::variants`](eggshell_ui::contract::ButtonStyle::variants) this
/// preset defines.
pub mod variant {
    use eggshell_ui::contract::Variant;

    /// `default`.
    pub const PRIMARY: Variant = Variant("primary");
    /// `secondary`: the variant of a button that names none.
    pub const SECONDARY: Variant = Variant("secondary");
    /// `destructive`.
    pub const DESTRUCTIVE: Variant = Variant("destructive");
    /// `outline`.
    pub const OUTLINE: Variant = Variant("outline");
    /// `ghost`.
    pub const GHOST: Variant = Variant("ghost");
}
