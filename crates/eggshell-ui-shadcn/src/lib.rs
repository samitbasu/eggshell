#![deny(missing_docs)]
#![cfg_attr(
    not(test),
    deny(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        clippy::get_unwrap,
        clippy::string_slice,
        clippy::indexing_slicing,
        clippy::unchecked_time_subtraction,
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::fn_params_excessive_bools
    )
)]
//! The shadcn/ui design system as a [`eggshell_ui::Style`].
//!
//! [`Shadcn`] is a preset: a [`ShadcnStyle`] (which of shadcn's styles, such as
//! [`Vega`](ShadcnStyle::Vega) or [`Lyra`](ShadcnStyle::Lyra)), a
//! [`Palette`] (shadcn's base and theme colours) and an egui theme (light or dark),
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
//! ```
//! use eggshell_ui_shadcn::{Palette, Shadcn, ShadcnStyle, ThemeColor};
//!
//! let ctx = egui::Context::default();
//! // Once at startup: shadcn's default preset, light and dark.
//! Shadcn::apply_both(&ctx);
//! // Or every frame, to switch style or colour at run time; a no-op while
//! // nothing changes.
//! Shadcn::apply_both_with(&ctx, ShadcnStyle::Lyra, Palette::new().theme(ThemeColor::Blue));
//! // The tokens, for the application's own colours.
//! let background = Shadcn::of(&ctx).tokens.background;
//! # let _ = background;
//! ```
//!
//! What eggshell cannot draw exactly and the preset therefore approximates is
//! documented on [`eggshell_ui::contract::InputStyle`] for the wrapped text edit;
//! the owned widgets are exact except for the negative spread of `shadow-md`.
//!
//! # Fonts
//!
//! With the default `fonts` feature the preset bundles Geist, shadcn's
//! `--font-sans`, as one variable font (165 KiB, SIL Open Font License 1.1,
//! `fonts/OFL.txt`) and [`Shadcn::apply`] registers it at every [`Weight`]:
//! Normal as the first font of [`FontFamily::Proportional`], the others as
//! named families its [`Faces`] declare. [`Shadcn::with_fonts`] swaps in
//! another [`FontSet`], or `None` for egui's fonts as the application set
//! them. Without the feature nothing is embedded and the preset starts with
//! no font, so its medium controls are drawn in the regular face, with a
//! debug warning.
//!
//! The application's own text picks a weight with eggshell's classes
//! (`tw().font_medium()`, `font_semibold`, `font_bold`). A [`FontSet`] of
//! static faces that leaves a weight out draws it in the nearest lighter
//! face it has. eggshell's symbol face (⌘, ⇧, ⌥) sits last in every weight's
//! family, behind Geist, so key chips and menu shortcuts draw in any weight.
//!
//! egui's `default_fonts` feature embeds its own Ubuntu-Light, which Geist
//! makes a fallback only. An application that does not need it can depend on
//! egui with `default-features = false` (and the features it does use) to
//! drop it from the binary.
//!
//! [`Weight`]: eggshell_ui::Weight
//! [`Faces`]: eggshell_ui::Faces
//! [`FontFamily::Proportional`]: egui::FontFamily::Proportional
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

mod contrast;
mod drag_chip;
mod fonts;
mod palette;
mod preset;
mod sheets;
mod styles;
mod theme_color;
mod tokens;

pub use contrast::{ContrastPair, ContrastRatio, REGISTRY_MISSES, RegistryMiss};
pub use drag_chip::{DragChip, drag_chip};
pub use fonts::FontSet;
pub use palette::{BaseColor, Palette};
pub use preset::{Shadcn, tokens};
pub use styles::{Elevation, HoverSurface, ShadcnStyle};
pub use theme_color::ThemeColor;
pub use tokens::Tokens;

/// The names of the button sizes this preset defines besides the default.
pub mod size {
    use eggshell_ui::contract::Size;

    /// `icon`: a square the height of a control, for a button that is only an
    /// icon.
    pub const ICON: Size = Size("icon");

    /// TODO(prose): `sm`, a control four points shorter.
    pub const SM: Size = Size("sm");

    /// TODO(prose): `xs`, a control twelve points shorter, in 12 pt text.
    pub const XS: Size = Size("xs");

    /// `icon-sm`: [`ICON`] at [`SM`].
    pub const ICON_SM: Size = Size("icon-sm");

    /// `icon-xs`: [`ICON`] at [`XS`].
    pub const ICON_XS: Size = Size("icon-xs");

    /// TODO(prose): `lg`, a control four points taller.
    pub const LG: Size = Size("lg");

    /// TODO(prose): `icon-lg`: [`ICON`] at [`LG`].
    pub const ICON_LG: Size = Size("icon-lg");
}

/// The names of shadcn's button variants, as the preset's sheet names them
/// for [`ButtonPart`](eggshell_ui::owned::ButtonPart). shadcn's `default` is
/// [`PRIMARY`]; a button that names no variant is [`SECONDARY`].
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
    /// TODO(prose): `link`: primary-coloured text, underlined under the pointer.
    pub const LINK: Variant = Variant("link");
}
