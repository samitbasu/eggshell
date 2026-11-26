//! TODO(prose): shadcn's tokens, the values every look is drawn from: the
//! palette, light and dark, the shadcn style's sizes, the density tokens,
//! the font and the contrast check.
//!
//! The shadcn/ui design system as a [`Style`](crate::Style).
//!
//! [`Shadcn`] is a preset: a [`ShadcnStyle`] (which of shadcn's styles, such as
//! [`Vega`](ShadcnStyle::Vega) or [`Lyra`](ShadcnStyle::Lyra)), a
//! [`Palette`] (shadcn's base and theme colours) and an egui theme (light or dark),
//! chosen independently. TODO(prose): a layout given a preset's style gives
//! egui its look; nothing here changes egui's style on its own.
//!
//! The crate only fills the contract. `eggshell` defines what its widgets
//! need and has no shadcn names or values; a style is data, so changing
//! [`ShadcnStyle`] at run time changes sizes, radii, text size and ring width
//! with no code change in the application.
//!
//! TODO(prose): applying a preset is now giving a layout its style. It fills the [`Style`](crate::Style)
//! contract with the tokens ([`Shadcn::style`]) and installs it for the
//! preset's theme, which the owned widgets and the text edit of `eggshell`
//! draw from. It maps the [`Tokens`] onto the egui [`Style`](egui::Style) of
//! the preset's theme (widget fills, strokes, radii, selection, spacing) for
//! the egui widgets an application draws outside eggshell.
//!
//! ```
//! use eggshell_ui::theme::{Palette, Shadcn, ShadcnStyle, ThemeColor};
//! use eggshell_ui::{LayoutState, SharedStyle};
//!
//! let style = SharedStyle::new(Shadcn::themed(ShadcnStyle::Vega, Palette::new()));
//! let _page = LayoutState::new(&style);
//! let _bar = LayoutState::named("bar", &style);
//! style.set(Shadcn::themed(
//!     ShadcnStyle::Lyra,
//!     Palette::new().theme(ThemeColor::Blue),
//! ));
//! let background = Shadcn::light().tokens.background;
//! # let _ = background;
//! ```
//!
//! What eggshell cannot draw exactly and the preset therefore approximates is
//! documented on [`InputStyle`](crate::contract::InputStyle) for the wrapped text edit;
//! the owned widgets are exact except for the negative spread of `shadow-md`.
//!
//! # Fonts
//!
//! With the default `fonts` feature the preset bundles Geist, shadcn's
//! `--font-sans`, as one variable font (165 KiB, SIL Open Font License 1.1,
//! `fonts/OFL.txt`) and TODO(prose): a [`LayoutState`](crate::LayoutState) given the preset's style registers it at every [`Weight`]:
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
//! [`Weight`]: crate::Weight
//! [`Faces`]: crate::Faces
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
mod fonts;
mod palette;
mod preset;
mod styles;
mod theme_color;
mod tokens;

pub(crate) use contrast::warn_low_contrast;
pub use contrast::{ContrastPair, ContrastRatio, REGISTRY_MISSES, RegistryMiss};
pub use fonts::FontSet;
pub use palette::{BaseColor, Palette};
pub use preset::Shadcn;
pub use styles::{Elevation, HoverSurface, ShadcnStyle};
pub use theme_color::ThemeColor;
pub use tokens::Tokens;

/// The names of the button sizes this preset defines besides the default.
pub mod size {
    use crate::contract::Size;

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
/// for [`button::Part`](crate::owned::button::Part). shadcn's `default` is
/// [`PRIMARY`](variant::PRIMARY); a button that names no variant is
/// [`SECONDARY`](variant::SECONDARY).
pub mod variant {
    use crate::contract::Variant;

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
    /// TODO(prose): `info`: an alert in the info tone, which shadcn's alert
    /// does not have.
    pub const INFO: Variant = Variant("info");
    /// TODO(prose): `success`: an alert in the success tone.
    pub const SUCCESS: Variant = Variant("success");
    /// TODO(prose): `warning`: an alert in the warning tone.
    pub const WARNING: Variant = Variant("warning");
}
