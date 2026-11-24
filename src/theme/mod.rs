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
