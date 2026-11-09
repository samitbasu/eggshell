//! What Tailwind renders and bullseye cannot yet: negative-spread shadows and font weights.
//!
//! Two deficiencies are recorded, not fixed, and this example makes each
//! visible by putting what CSS would render beside what bullseye renders today.
//!
//! **Negative-spread shadows
//! ([#12](https://github.com/samitbasu/bullseye/issues/12)).** Tailwind's
//! `shadow`, `shadow-md`, `shadow-lg` and `shadow-xl` stack two layers, the
//! larger of them with a negative spread. egui's `Shadow::spread` is a `u8` and a
//! `BoxPaint` holds one shadow, so bullseye's `shadow(..)` class can express
//! one layer with the spread clamped to zero. The *CSS reference* cards are drawn
//! with a `paint` closure instead: each layer is an `egui::Shadow` cast by the
//! card rect shrunk by the spread, with the corner radii shrunk to match. That
//! is exactly what CSS does for the geometry, offset and colour. The blur is
//! not exact: CSS blurs with a Gaussian of standard deviation half the blur
//! radius, while egui feathers over a linear ramp of the given width, and the
//! example passes the CSS blur radius straight through. Expect the same
//! silhouette with a slightly different falloff. Definitions are Tailwind v3's;
//! v4 renames `shadow` to `shadow-sm` and shifts the rest down a step.
//!
//! **Font weights
//! ([#13](https://github.com/samitbasu/bullseye/issues/13)).** There is no
//! `font-medium` / `font-semibold` / `font-bold` class and egui does not
//! synthesise weights, so a heading in any weight renders as Regular. The
//! *with real faces* column registers weighted files as named `FontFamily`s at
//! startup, which is the pattern an app can follow today. It scans the standard
//! system font directories for the first family with Regular, Medium, SemiBold
//! and Bold files (`egui_system_font_provider` is a glyph fallback, not a weight
//! lookup), and shows a note when it finds none. No font files are committed.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use bullseye::style::Tokens;
use bullseye::{Builder, LayoutState, TextSpec, TextStyle, Tw, tw};
use egui::epaint::RectShape;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Rect, Shadow, Theme};

type B<'b, 'f, A> = Builder<'b, 'f, A>;

pub const CARD_SIZE: [f32; 2] = [140.0, 72.0];
/// `rounded-lg`.
pub const CARD_RADIUS: CornerRadius = CornerRadius::same(8);

const BODY: TextSpec = text_spec(14.0, FontFamily::Proportional);
const SMALL: TextSpec = text_spec(12.0, FontFamily::Proportional);
const MONO: TextSpec = text_spec(11.0, FontFamily::Monospace);
const HEADING: TextSpec = text_spec(18.0, FontFamily::Proportional);
const SAMPLE: TextSpec = text_spec(24.0, FontFamily::Proportional);

const fn text_spec(size: f32, family: FontFamily) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family,
    }
}

/// The base text of the example in `theme`.
pub fn base_text(theme: Theme) -> TextStyle {
    TextStyle::new(BODY, Tokens::for_theme(theme).foreground)
}

/// One layer of a CSS `box-shadow` of the form `0 <y>px <blur>px <spread>px rgb(0 0 0 / <alpha>)`,
/// shown on screen as `0 <y>px <blur>px <spread>px / <alpha>`.
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub offset_y: i8,
    pub blur: u8,
    pub spread: i8,
    pub alpha_percent: u8,
}

impl Layer {
    fn color(self) -> Color32 {
        Color32::from_black_alpha((u32::from(self.alpha_percent) * 255).div_ceil(100) as u8)
    }

    fn css(self) -> String {
        let Self {
            offset_y,
            blur,
            spread,
            alpha_percent,
        } = self;
        format!("0 {offset_y}px {blur}px {spread}px / 0.{alpha_percent:02}")
    }

    /// The layer as CSS paints it: the shadow of `rect` grown by the spread,
    /// which for a negative spread is the shadow of the shrunken rect.
    pub fn reference(self, rect: Rect, radius: CornerRadius) -> RectShape {
        let shrink = u8::try_from(-i16::from(self.spread).min(0)).unwrap_or(u8::MAX);
        let shrunk = rect.shrink(f32::from(shrink));
        let radius = CornerRadius {
            nw: radius.nw.saturating_sub(shrink),
            ne: radius.ne.saturating_sub(shrink),
            sw: radius.sw.saturating_sub(shrink),
            se: radius.se.saturating_sub(shrink),
        };
        Shadow {
            offset: [0, self.offset_y],
            blur: self.blur,
            spread: u8::try_from(self.spread.max(0)).unwrap_or(0),
            color: self.color(),
        }
        .as_shape(shrunk, radius)
    }

    /// The nearest `Shadow` bullseye's `shadow(..)` accepts: a negative spread becomes zero.
    pub fn clamped(self) -> Shadow {
        Shadow {
            offset: [0, self.offset_y],
            blur: self.blur,
            spread: u8::try_from(self.spread.max(0)).unwrap_or(0),
            color: self.color(),
        }
    }
}

/// A Tailwind v3 shadow class and its CSS layers, front to back.
#[derive(Clone, Copy, Debug)]
pub struct ShadowClass {
    pub class: &'static str,
    pub layers: &'static [Layer],
}

const fn layer(offset_y: i8, blur: u8, spread: i8, alpha_percent: u8) -> Layer {
    Layer {
        offset_y,
        blur,
        spread,
        alpha_percent,
    }
}

pub const SHADOWS: [ShadowClass; 5] = [
    ShadowClass {
        class: "shadow-sm",
        layers: &[layer(1, 2, 0, 5)],
    },
    ShadowClass {
        class: "shadow",
        layers: &[layer(1, 3, 0, 10), layer(1, 2, -1, 10)],
    },
    ShadowClass {
        class: "shadow-md",
        layers: &[layer(4, 6, -1, 10), layer(2, 4, -2, 10)],
    },
    ShadowClass {
        class: "shadow-lg",
        layers: &[layer(10, 15, -3, 10), layer(4, 6, -4, 10)],
    },
    ShadowClass {
        class: "shadow-xl",
        layers: &[layer(20, 25, -5, 10), layer(8, 10, -6, 10)],
    },
];

/// A Tailwind font weight class.
#[derive(Clone, Copy, Debug)]
pub struct Weight {
    pub class: &'static str,
    pub number: u16,
    /// File name suffixes of the weight, such as `Regular` in `Montserrat-Regular.otf`.
    pub styles: &'static [&'static str],
}

pub const WEIGHTS: [Weight; 4] = [
    Weight {
        class: "font-normal",
        number: 400,
        styles: &["Regular"],
    },
    Weight {
        class: "font-medium",
        number: 500,
        styles: &["Medium"],
    },
    Weight {
        class: "font-semibold",
        number: 600,
        styles: &["SemiBold", "Semibold"],
    },
    Weight {
        class: "font-bold",
        number: 700,
        styles: &["Bold"],
    },
];

impl Weight {
    /// The family the app registers this weight under.
    pub fn family(self) -> FontFamily {
        FontFamily::Name(format!("weight-{}", self.number).into())
    }
}

/// Weighted font files of one family, in [`WEIGHTS`] order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Faces {
    pub name: &'static str,
    pub files: [PathBuf; 4],
}

const FAMILIES: [&str; 7] = [
    "Montserrat",
    "Inter",
    "Roboto",
    "NotoSans",
    "OpenSans",
    "SourceSans3",
    "IBMPlexSans",
];

/// The browser has no font directories to scan.
#[cfg(target_arch = "wasm32")]
fn font_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn font_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut roots: Vec<PathBuf> = [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/Library/Fonts",
        "/System/Library/Fonts",
        "C:\\Windows\\Fonts",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    if let Some(home) = home {
        roots.extend(
            [".local/share/fonts", ".fonts", "Library/Fonts"]
                .into_iter()
                .map(|dir| home.join(dir)),
        );
    }
    roots
}

fn collect_files(dir: &Path, depth: u8, into: &mut HashMap<String, PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            if depth > 0 {
                collect_files(&path, depth - 1, into);
            }
        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            into.entry(name.to_owned()).or_insert(path);
        }
    }
}

/// The first of [`FAMILIES`] with a file for every weight under `roots`.
pub fn find_faces(roots: &[PathBuf]) -> Option<Faces> {
    let mut files = HashMap::new();
    for root in roots {
        collect_files(root, 4, &mut files);
    }
    FAMILIES.into_iter().find_map(|name| {
        let found = WEIGHTS.map(|weight| {
            weight.styles.iter().find_map(|style| {
                ["ttf", "otf"]
                    .into_iter()
                    .find_map(|ext| files.get(&format!("{name}-{style}.{ext}")).cloned())
            })
        });
        let [Some(a), Some(b), Some(c), Some(d)] = found else {
            return None;
        };
        Some(Faces {
            name,
            files: [a, b, c, d],
        })
    })
}

/// Registers each weight of `faces` as its own named family, falling back to egui's defaults for missing glyphs.
pub fn register(ctx: &egui::Context, faces: &Faces) -> std::io::Result<()> {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    for (weight, path) in WEIGHTS.into_iter().zip(&faces.files) {
        let key = format!("{}-{}", faces.name, weight.number);
        fonts
            .font_data
            .insert(key.clone(), FontData::from_owned(fs::read(path)?).into());
        let mut chain = vec![key];
        chain.extend(fallback.iter().cloned());
        fonts.families.insert(weight.family(), chain);
    }
    ctx.set_fonts(fonts);
    Ok(())
}

/// The tokens of the surface the shadow comparison sits on. Shadows are
/// black at 5 to 10 percent, which is invisible on a dark surface, so the
/// comparison stays on the light theme's surface in both themes.
fn specimen() -> Tokens {
    Tokens::light()
}

/// The text that says which rendering a card is.
fn card_text<A>(b: &mut B<'_, '_, A>, label: &'static str) {
    b.text(
        tw().role(SMALL)
            .text(specimen().muted_foreground)
            .text_center(),
        label,
    );
}

fn card_box() -> Tw {
    tw().flex()
        .items_center()
        .justify_center()
        .w_px(CARD_SIZE[0])
        .h_px(CARD_SIZE[1])
        .rounded_lg()
        .bg(specimen().card)
}

/// One card per `ShadowClass`, shadowed by CSS's layers.
///
/// ```html
/// <div class="relative w-[140px] h-[72px]">
///   <!-- one absolutely positioned layer per CSS box-shadow, drawn first -->
///   <div class="absolute inset-0 flex items-center justify-center rounded-lg bg-white shadow-lg">CSS reference</div>
/// </div>
/// ```
fn reference_card<A>(b: &mut B<'_, '_, A>, class: ShadowClass) {
    b.div(tw().relative().w_px(CARD_SIZE[0]).h_px(CARD_SIZE[1]), |b| {
        b.paint(tw().absolute().inset(0.0), move |painter, rect, _| {
            for layer in class.layers.iter().rev() {
                painter.add(layer.reference(rect, CARD_RADIUS));
            }
        });
        b.div(card_box().absolute().inset(0.0), |b| {
            card_text(b, "CSS reference")
        });
    });
}

/// The same card with the best `shadow(..)` can say today: the first layer, spread clamped to zero.
///
/// ```html
/// <div class="flex items-center justify-center w-[140px] h-[72px] rounded-lg bg-white shadow-[0_10px_15px_0_rgb(0_0_0/0.1)]">bullseye today</div>
/// ```
fn today_card<A>(b: &mut B<'_, '_, A>, class: ShadowClass) {
    let shadow = class.layers[0].clamped();
    b.div(card_box().shadow(shadow), |b| {
        card_text(b, "bullseye today")
    });
}

/// ```html
/// <div class="flex flex-col items-center gap-10">
///   <code class="text-[11px] text-gray-500">shadow-lg</code>
///   <!-- reference card, today card -->
///   <code class="text-[11px] text-gray-500">0 10px 15px -3px / 0.10</code>
///   <code class="text-[11px] text-gray-500">0 4px 6px -4px / 0.10</code>
/// </div>
/// ```
fn shadow_column<A>(b: &mut B<'_, '_, A>, class: ShadowClass) {
    b.div(tw().flex().flex_col().items_center().gap(10.0), move |b| {
        b.text(tw().role(MONO).text(specimen().foreground), class.class);
        reference_card(b, class);
        today_card(b, class);
        b.div(tw().flex().flex_col().items_center(), |b| {
            for layer in class.layers {
                b.text(
                    tw().role(MONO)
                        .text(specimen().muted_foreground)
                        .whitespace_nowrap(),
                    layer.css(),
                );
            }
        });
    });
}

/// The shadow comparison, on a light stage in both themes.
///
/// ```html
/// <section class="flex flex-col gap-6 p-8 size-full bg-muted text-foreground">
///   <h2 class="text-lg">Negative-spread shadows (#12)</h2>
///   <p class="text-xs text-muted-foreground">…</p>
///   <div class="flex flex-row gap-8 p-6 rounded-lg bg-[light muted]">
///     <!-- one column per shadow class -->
///   </div>
/// </section>
/// ```
pub fn shadows<A>(b: &mut B<'_, '_, A>) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .gap(6.0)
            .p(8.0)
            .size_full()
            .bg(t.muted)
            .text(t.foreground),
        |b| {
            b.text(
                tw().role(HEADING),
                "Negative-spread shadows (#12): CSS reference above, bullseye today below",
            );
            b.text(
                tw().role(SMALL).text(t.muted_foreground),
                "Each layer is box-shadow: 0 <y> <blur> <spread> rgb(0 0 0 / <alpha>); the reference stacks them, bullseye keeps the first with spread 0.",
            );
            b.div(
                tw().flex()
                    .flex_row()
                    .gap(8.0)
                    .p(6.0)
                    .rounded_lg()
                    .bg(specimen().muted),
                |b| {
                    for class in SHADOWS {
                        shadow_column(b, class);
                    }
                },
            );
        },
    );
}

fn weight_row<A>(b: &mut B<'_, '_, A>, weight: Weight, faces: Option<&Faces>) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_row().items_center().gap(6.0), |b| {
        b.div(tw().w_px(150.0).flex_none(), |b| {
            b.text(
                tw().role(MONO).text(muted),
                format!("{} ({})", weight.class, weight.number),
            );
        });
        b.div(tw().w_px(340.0).flex_none(), |b| {
            b.text(tw().role(SAMPLE), "Settings and preferences");
        });
        if faces.is_some() {
            b.text(
                tw().role(SAMPLE).font(weight.family()),
                "Settings and preferences",
            );
        }
    });
}

/// The weights comparison. `faces` is what [`register`] installed, if anything.
///
/// ```html
/// <section class="flex flex-col gap-4 p-8 bg-background text-foreground">
///   <h2 class="text-lg">Font weights (#13)</h2>
///   <div class="flex items-center gap-6">
///     <code class="w-[150px]">font-semibold (600)</code>
///     <span class="w-[340px] text-2xl font-normal">Settings and preferences</span>
///     <span class="text-2xl font-semibold">Settings and preferences</span>
///   </div>
///   <!-- one row per weight -->
/// </section>
/// ```
pub fn weights<A>(b: &mut B<'_, '_, A>, faces: Option<&Faces>) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .gap(4.0)
            .p(8.0)
            .bg(t.background)
            .text(t.foreground),
        |b| {
            b.text(tw().role(HEADING), "Font weights (#13)");
            b.div(tw().flex().flex_row().gap(6.0), |b| {
                b.div(tw().w_px(150.0).flex_none(), |_| {});
                b.div(tw().w_px(340.0).flex_none(), |b| {
                    b.text(tw().role(SMALL).text(t.muted_foreground), "bullseye today");
                });
                b.text(
                    tw().role(SMALL).text(t.muted_foreground),
                    match faces {
                        Some(faces) => format!("with real faces ({})", faces.name),
                        None => "with real faces".to_owned(),
                    },
                );
            });
            for weight in WEIGHTS {
                weight_row(b, weight, faces);
            }
            if faces.is_none() {
                b.text(
                    tw().role(BODY).text(t.warning),
                    format!(
                        "No weighted faces found: looked for {} with Regular, Medium, SemiBold and Bold files in the system font directories.",
                        FAMILIES.join(", ")
                    ),
                );
            }
        },
    );
}

/// ```html
/// <main class="flex flex-col size-full">
///   <section class="flex-1"><!-- shadows --></section>
///   <section><!-- weights --></section>
/// </main>
/// ```
pub fn window<A>(b: &mut B<'_, '_, A>, faces: Option<&Faces>) {
    b.div(tw().flex().flex_col().size_full(), |b| {
        b.div(tw().flex_1(), shadows);
        weights(b, faces);
    });
}

fn main() -> eframe::Result {
    let mut layout = LayoutState::new();
    let mut faces: Option<Faces> = None;
    let mut registered = false;
    let mut theme_bar = common::ThemeBar::new();
    common::launch("known_gaps", Some(egui::vec2(1100.0, 860.0)), move |ui| {
        let ctx = ui.ctx().clone();
        if !registered {
            registered = true;
            faces = find_faces(&font_roots());
            if let Some(found) = &faces
                && let Err(error) = register(&ctx, found)
            {
                eprintln!("could not load {}: {error}", found.name);
                faces = None;
            }
            // egui applies new fonts at the start of the next pass, and a
            // named family used before then panics.
            ctx.request_repaint();
            return;
        }
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout.set_base_text(Some(base_text(ctx.theme())));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let area = ui.max_rect();
                layout
                    .build(&ctx, |b| window(b, faces.as_ref()))
                    .layout(area)
                    .draw(&mut (), ui);
            });
    })
}
