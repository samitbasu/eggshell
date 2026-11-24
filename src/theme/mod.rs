//! TODO(prose): shadcn's tokens, the values every look is drawn from: the
//! palette, light and dark, the shadcn style's sizes, the density tokens,
//! the font and the contrast check.

mod contrast;
mod fonts;
mod palette;
mod styles;
mod theme_color;
mod tokens;

pub use contrast::{ContrastPair, ContrastRatio, REGISTRY_MISSES, RegistryMiss, warn_low_contrast};
pub use fonts::FontSet;
pub use palette::{BaseColor, Palette};
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
