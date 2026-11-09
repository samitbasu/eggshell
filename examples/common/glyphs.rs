//! Small icons rasterised at start-up, so the examples that show icon cells
//! and icon buttons need no image assets or loaders.

use bullseye::Icon;
use egui::{Color32, ColorImage, Context, Id, TextureHandle, TextureOptions, Vec2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    Sun,
    Moon,
    Monitor,
    Plus,
}

const SIDE: usize = 64;

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// How much of the pixel at `p` (in the unit square) the glyph covers.
fn coverage(glyph: Glyph, p: Vec2) -> f32 {
    let stroke = 0.075;
    let line = |a: Vec2, b: Vec2| stroke / 2.0 - distance_to_segment(p, a, b);
    let disc = |centre: Vec2, radius: f32| radius - (p - centre).length();
    let sdf = match glyph {
        Glyph::Sun => {
            let centre = vec2(0.5, 0.5);
            (0..8).fold(disc(centre, 0.19), |best, ray| {
                let angle = ray as f32 * std::f32::consts::FRAC_PI_4;
                let dir = vec2(angle.cos(), angle.sin());
                best.max(line(centre + dir * 0.31, centre + dir * 0.43))
            })
        }
        Glyph::Moon => disc(vec2(0.5, 0.5), 0.36).min(-disc(vec2(0.66, 0.38), 0.3)),
        Glyph::Monitor => {
            let (min, max) = (vec2(0.12, 0.2), vec2(0.88, 0.68));
            let q = (p - (min + max) / 2.0).abs() - (max - min) / 2.0;
            let outside = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
            let frame = stroke / 2.0 - outside.abs();
            frame
                .max(line(vec2(0.5, 0.68), vec2(0.5, 0.82)))
                .max(line(vec2(0.3, 0.82), vec2(0.7, 0.82)))
        }
        Glyph::Plus => {
            line(vec2(0.2, 0.5), vec2(0.8, 0.5)).max(line(vec2(0.5, 0.2), vec2(0.5, 0.8)))
        }
    };
    (sdf * SIDE as f32 + 0.5).clamp(0.0, 1.0)
}

fn texture(ctx: &Context, glyph: Glyph) -> TextureHandle {
    let id = Id::NULL.with(("example-glyph", glyph));
    if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return texture;
    }
    let pixels = (0..SIDE * SIDE)
        .map(|index| {
            let p = vec2(
                ((index % SIDE) as f32 + 0.5) / SIDE as f32,
                ((index / SIDE) as f32 + 0.5) / SIDE as f32,
            );
            Color32::from_white_alpha((coverage(glyph, p) * 255.0).round() as u8)
        })
        .collect();
    let image = ColorImage::new([SIDE, SIDE], pixels);
    let texture = ctx.load_texture(format!("glyph-{glyph:?}"), image, TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

/// `glyph` as an icon, tinted by whatever text colour it is drawn with.
pub fn icon(ctx: &Context, glyph: Glyph) -> Icon {
    let texture = texture(ctx, glyph);
    Icon::new(egui::ImageSource::Texture(egui::load::SizedTexture::new(
        texture.id(),
        vec2(SIDE as f32, SIDE as f32),
    )))
}
