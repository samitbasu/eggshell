//! The `known_gaps` example: the CSS reference shadows against what
//! `shadow(..)` can express, asserted on the shapes egui is handed.

use egui::epaint::{ClippedShape, RectShape, Shape};
use egui::{RawInput, Rect, pos2};

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/known_gaps.rs"]
mod known_gaps;

use known_gaps::{CARD_RADIUS, CARD_SIZE, SHADOWS, WEIGHTS, find_faces};

const SCREEN: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(1000.0, 800.0),
};

fn draw(build: impl for<'f> FnOnce(&mut bullseye::Builder<'_, 'f, ()>)) -> Vec<ClippedShape> {
    let ctx = egui::Context::default();
    let input = RawInput {
        screen_rect: Some(SCREEN),
        ..RawInput::default()
    };
    ctx.run_ui(input.clone(), |_| {})
        .drop_without_applying_deltas();
    let mut build = Some(build);
    let mut output = ctx.run_ui(input, |ui| {
        let build = build.take().expect("one pass");
        bullseye::LayoutState::new()
            .with_base_text(known_gaps::base_text())
            .build(ui.ctx(), build)
            .layout(SCREEN)
            .draw(&mut (), ui);
    });
    output.textures_delta.clear();
    output.shapes
}

fn shadow_shapes(shapes: &[ClippedShape]) -> Vec<&RectShape> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.blur_width > 0.0 => Some(rect),
            _ => None,
        })
        .collect()
}

#[test]
fn the_reference_is_tighter_than_the_clamped_shadow_where_the_spread_is_negative() {
    let shapes = draw(known_gaps::shadows);
    let shadows = shadow_shapes(&shapes);
    let expected: usize = SHADOWS.iter().map(|class| class.layers.len() + 1).sum();
    assert_eq!(
        shadows.len(),
        expected,
        "layers plus one clamped shadow each"
    );

    let cards: Vec<Rect> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.fill == egui::Color32::WHITE => Some(rect.rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        cards.len(),
        2 * SHADOWS.len(),
        "a reference and a today card per class"
    );

    let mut at = 0;
    for (class, cards) in SHADOWS.into_iter().zip(cards.chunks(2)) {
        let (reference, rest) = shadows[at..].split_at(class.layers.len());
        let today = rest[0];
        at += class.layers.len() + 1;

        let first = class.layers[0];
        let [card, today_card] = [cards[0], cards[1]];
        assert_eq!(card.size(), egui::Vec2::from(CARD_SIZE), "{}", class.class);
        assert_eq!(
            today.rect,
            today_card.translate(egui::vec2(0.0, f32::from(first.offset_y)))
        );
        assert_eq!(today.blur_width, f32::from(first.blur));

        for (shape, layer) in reference.iter().zip(class.layers.iter().rev()) {
            let shrink = f32::from(layer.spread.unsigned_abs());
            let expected = card
                .translate(egui::vec2(0.0, f32::from(layer.offset_y)))
                .shrink(shrink);
            assert_eq!(shape.rect, expected, "{} layer {layer:?}", class.class);
            assert_eq!(shape.blur_width, f32::from(layer.blur));
            assert_eq!(
                shape.corner_radius.nw,
                CARD_RADIUS.nw.saturating_sub(layer.spread.unsigned_abs())
            );
        }
        if class.layers.iter().any(|layer| layer.spread < 0) {
            assert!(
                reference
                    .iter()
                    .any(|shape| shape.rect.width() < today.rect.width()),
                "{}: some reference layer is narrower than the clamped one",
                class.class
            );
        }
    }
}

#[test]
fn the_whole_window_builds_without_weighted_faces() {
    let shapes = draw(|b| known_gaps::window(b, None));
    assert!(!shadow_shapes(&shapes).is_empty());
}

#[test]
fn faces_need_every_weight_in_one_family() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("known_gaps_fonts");
    let _ = std::fs::remove_dir_all(&root);
    let nested = root.join("deep");
    std::fs::create_dir_all(&nested).unwrap();
    let touch = |name: &str| std::fs::write(nested.join(name), b"").unwrap();

    for name in ["Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-Bold.ttf"] {
        touch(name);
    }
    assert_eq!(
        find_faces(std::slice::from_ref(&root)),
        None,
        "SemiBold is missing"
    );

    touch("Inter-SemiBold.otf");
    let faces = find_faces(&[root]).expect("all four weights present");
    assert_eq!(faces.name, "Inter");
    assert_eq!(faces.files.len(), WEIGHTS.len());
    assert!(faces.files[2].ends_with("Inter-SemiBold.otf"));
}
