//! The glyphs of the modifier and editing keys, which no text face of egui's
//! carries: a key chip or a menu shortcut that writes ⌘ or ⇧ would otherwise
//! panic in a debug build.

use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};

const NAME: &str = "eggshell-symbols";

/// Adds the symbol face behind the proportional family's own faces.
/// Cheap once it is there, and puts it back if the host replaced the fonts.
pub(crate) fn install(ctx: &Context) {
    ctx.add_font(FontInsert::new(
        NAME,
        FontData::from_static(include_bytes!("fonts/DejaVuSans-symbols.ttf")),
        vec![InsertFontFamily {
            family: FontFamily::Proportional,
            priority: FontPriority::Lowest,
        }],
    ));
}
