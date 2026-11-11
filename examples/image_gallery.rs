//! Port of the gallery of hello_egui's fancy example: images in rows that are
//! scaled to fill the width, in a vertical scroll area.
//!
//! After the example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with eggshell rather than copied. The original
//! counts how many images fit a row and scales the row to the width it is
//! offered; here the rows are a wrapping flex row whose items grow in
//! proportion to their aspect ratio, so taffy breaks and fills the rows at the
//! scroll container's width, in the same frame. The images are not loaded:
//! each is a tile of the size the original's index gives it (48 of its 100),
//! with its id on it. Clicking one selects it.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, tw};
use egui::Color32;
use egui::ecolor::Hsva;

type B<'b, 'f> = Builder<'b, 'f, State>;

/// The height an image is shown at before its row is scaled to the width.
const HEIGHT: f32 = 300.0;

/// `(id, width, height)` of the images, at the height the original loads them.
const IMAGES: [(u32, u32, u32); 48] = [
    (244225, 177, 300),
    (242038, 202, 300),
    (252850, 199, 300),
    (253017, 300, 300),
    (250877, 140, 300),
    (245585, 481, 300),
    (244308, 605, 300),
    (244078, 520, 300),
    (244514, 229, 300),
    (248929, 246, 300),
    (254582, 190, 300),
    (244077, 506, 300),
    (241069, 278, 300),
    (254433, 356, 300),
    (249207, 461, 300),
    (250045, 502, 300),
    (251099, 402, 300),
    (253069, 147, 300),
    (251367, 362, 300),
    (251487, 244, 300),
    (252443, 546, 300),
    (246954, 272, 300),
    (251051, 523, 300),
    (243243, 240, 300),
    (242632, 306, 300),
    (245242, 192, 300),
    (245335, 219, 300),
    (253136, 259, 289),
    (251319, 466, 300),
    (241072, 401, 300),
    (249139, 204, 300),
    (247595, 300, 300),
    (250554, 185, 300),
    (251656, 269, 300),
    (245832, 300, 300),
    (247942, 300, 300),
    (248695, 242, 300),
    (245488, 685, 300),
    (248801, 362, 300),
    (246795, 331, 300),
    (251574, 242, 300),
    (258592, 297, 300),
    (257402, 251, 300),
    (250504, 276, 300),
    (250002, 263, 300),
    (257193, 299, 300),
    (245834, 300, 300),
    (259132, 235, 300),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Gallery,
    Image(u32),
}

#[derive(Default)]
pub struct State {
    pub selected: Option<u32>,
}

/// A colour of the image's own, from its id.
fn tint(id: u32) -> Color32 {
    let hue = (id.wrapping_mul(2_654_435_761) >> 8) as f32 / (1u32 << 24) as f32;
    Hsva::new(hue, 0.45, 0.8, 1.0).into()
}

/// ```html
/// <div class="size-full overflow-y-auto p-4">
///   <p class="text-muted-foreground">This is a gallery of images…</p>
///   <div class="flex flex-wrap items-start gap-4">
///     <div class="relative rounded-lg" style="flex: w/h 1 calc(300px * w/h)"><div style="aspect-ratio: w/h"/>…</div> …
///   </div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Gallery, tw().size_full().p(4.0))
        .children(|s| {
            s.div(tw().flex().flex_col().gap(4.0), |s| {
                s.text(
                tw().text(t.muted_foreground),
                "This is a gallery of images from malmal.io and hellopaint.io. For the sake of the \
                 example the images are tiles of the same sizes.",
            );
                s.div(tw().flex().flex_wrap().items_start().gap(4.0), |s| {
                    for (id, width, height) in IMAGES {
                        let ratio = width as f32 / height as f32;
                        let selected = state.selected == Some(id);
                        let tile = tw()
                            .relative()
                            .flex_col()
                            .grow_by(ratio)
                            .basis_px(HEIGHT * ratio)
                            .min_w_0()
                            .rounded_lg()
                            .bg(tint(id))
                            .text(Color32::BLACK)
                            .sense_click();
                        let tile = if selected {
                            tile.ring(2.0, t.ring)
                        } else {
                            tile
                        };
                        s.div_id(Part::Image(id), tile, |s| {
                            s.rule(tw().w_full().aspect(ratio));
                            s.text(tw().absolute().left(2.0).bottom(2.0), id.to_string());
                        })
                        .on_click(move |app: &mut State| app.selected = Some(id));
                    }
                });
            });
        });
}

fn main() -> eframe::Result {
    common::run_app("image_gallery", State::default(), layout)
}
