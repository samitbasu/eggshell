//! The glyphs of the modifier and editing keys, which no text face of egui's
//! carries: a key chip or a menu shortcut that writes ⌘ or ⇧ would otherwise
//! panic in a debug build.

use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};

const NAME: &str = "eggshell-symbols";

/// Adds the symbol face behind the faces of the proportional family and of
/// every family the installed style draws a weight with. Cheap once it is
/// there, and puts it back if the host replaced the fonts.
///
/// egui adds a font only under a name it does not have yet, so each family
/// gets the face under its own name; the bytes are shared.
pub(crate) fn install(ctx: &Context, faces: &crate::text::Faces) {
    let font = FontData::from_static(include_bytes!("fonts/DejaVuSans-symbols.ttf"));
    let weighted = faces
        .iter()
        .map(|face| face.face.clone())
        .collect::<Vec<_>>();
    for family in std::iter::once(FontFamily::Proportional).chain(weighted) {
        let name = match &family {
            FontFamily::Proportional => NAME.to_owned(),
            other => format!("{NAME}/{other}"),
        };
        ctx.add_font(FontInsert::new(
            &name,
            font.clone(),
            vec![InsertFontFamily {
                family,
                priority: FontPriority::Lowest,
            }],
        ));
    }
}
