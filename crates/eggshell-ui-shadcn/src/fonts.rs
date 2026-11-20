//! The font a preset draws its text in: shadcn's `--font-sans`.

use eggshell_ui::{Faces, Weight};
use egui::epaint::text::{FontInsert, FontPriority, FontTweak, InsertFontFamily, VariationCoords};
use egui::{Context, FontData, FontFamily};

/// A font with a face for each of the weights it has, which
/// [`Shadcn::apply`](crate::Shadcn::apply) registers with egui: the
/// [`Normal`](Weight::Normal) face as the first font of
/// [`FontFamily::Proportional`], every other as a named family that the
/// preset's [`Faces`] give its weight.
///
/// Faces are registered under `<name>/<weight>`, and egui keeps a font it
/// already has under a name, so the name identifies the font: two sets with
/// one name are one font to egui.
#[derive(Clone, Debug, PartialEq)]
pub struct FontSet {
    name: String,
    faces: Vec<(Weight, FontData)>,
}

impl FontSet {
    /// A set of one static face per weight. A weight left out falls back to
    /// the nearest lighter one the set has.
    pub fn faces(name: &str, faces: impl IntoIterator<Item = (Weight, FontData)>) -> Self {
        Self {
            name: name.to_owned(),
            faces: faces.into_iter().collect(),
        }
    }

    /// A set from one variable font with a `wght` axis, drawn at every
    /// [`Weight`]. The file is shared, not copied, between the weights.
    pub fn variable(name: &str, font: &FontData) -> Self {
        Self::faces(
            name,
            Weight::ALL.map(|weight| {
                let tweak = FontTweak {
                    coords: VariationCoords::new([(b"wght", f32::from(weight.number()))]),
                    ..font.tweak.clone()
                };
                (weight, font.clone().tweak(tweak))
            }),
        )
    }

    /// Geist, shadcn's default sans-serif, from the bundled variable font
    /// (SIL Open Font License 1.1, see `fonts/OFL.txt`).
    #[cfg(feature = "fonts")]
    pub fn geist() -> Self {
        static GEIST: std::sync::LazyLock<FontData> = std::sync::LazyLock::new(|| {
            FontData::from_static(include_bytes!("../fonts/Geist-Variable.ttf"))
        });
        Self::variable("Geist", &GEIST)
    }

    /// The set a preset starts with: [`geist`](Self::geist) with the `fonts`
    /// feature, else none.
    pub(crate) fn bundled() -> Option<Self> {
        #[cfg(feature = "fonts")]
        return Some(Self::geist());
        #[cfg(not(feature = "fonts"))]
        None
    }

    fn face_name(&self, weight: Weight) -> String {
        format!("{}/{}", self.name, weight.number())
    }

    fn family(&self, weight: Weight) -> FontFamily {
        match weight {
            Weight::Normal => FontFamily::Proportional,
            _ => FontFamily::Name(self.face_name(weight).into()),
        }
    }

    fn weighted(&self) -> impl Iterator<Item = Weight> + '_ {
        self.faces
            .iter()
            .map(|(weight, _)| *weight)
            .filter(|weight| *weight != Weight::Normal)
    }

    /// The named family of each weight but Normal, for the style contract.
    pub(crate) fn declared(&self) -> Faces {
        self.weighted().fold(Faces::default(), |faces, weight| {
            faces.with(FontFamily::Proportional, weight, self.family(weight))
        })
    }

    /// Adds the faces to `ctx`'s fonts. The Normal face also backs every
    /// named family, for the glyphs a weighted face lacks. Adding a face egui
    /// has under its name already does nothing.
    pub(crate) fn install(&self, ctx: &Context) {
        for font in self.inserts() {
            ctx.add_font(font);
        }
    }

    /// The faces as egui font insertions: each weight's face first in its
    /// own family, and the Normal face last in every weighted family.
    pub(crate) fn inserts(&self) -> Vec<FontInsert> {
        let first = |family| InsertFontFamily {
            family,
            priority: FontPriority::Highest,
        };
        self.faces
            .iter()
            .map(|(weight, font)| {
                let mut families = vec![first(self.family(*weight))];
                if *weight == Weight::Normal {
                    families.extend(self.weighted().map(|weighted| InsertFontFamily {
                        family: self.family(weighted),
                        priority: FontPriority::Lowest,
                    }));
                }
                FontInsert::new(&self.face_name(*weight), font.clone(), families)
            })
            .collect()
    }
}
