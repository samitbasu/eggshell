use eggshell_ui::testing::screenshot::{self, Scale};
use eggshell_ui::{LayoutState, tw};
use egui::{Color32, Rect, Theme, vec2};
use egui_kittest::Harness;
use image::RgbaImage;

const SIZE: egui::Vec2 = vec2(120.0, 80.0);
const FILL: Color32 = Color32::from_rgb(200, 30, 60);

struct Scene {
    layout: LayoutState,
    swatch: Option<Rect>,
}

fn harness(scale: Scale, theme: Theme) -> Harness<'static, Scene> {
    screenshot::builder(scale, theme)
        .expect("lavapipe is installed")
        .with_size(SIZE)
        .build_ui_state(
            |ui, scene: &mut Scene| {
                let page = ui.max_rect();
                ui.painter().rect_filled(page, 0.0, ui.visuals().panel_fill);
                let placed = scene
                    .layout
                    .build(ui.ctx(), |b| {
                        b.div_id("swatch", tw().w_px(40.0).h_px(30.0).bg(FILL), |_| {});
                    })
                    .layout(page);
                scene.swatch = Some(placed.at("swatch").rect());
                placed.draw(&mut (), ui);
            },
            Scene {
                layout: LayoutState::new(),
                swatch: None,
            },
        )
}

fn shot(scale: Scale, theme: Theme) -> (RgbaImage, Rect) {
    let mut harness = harness(scale, theme);
    harness.run();
    let swatch = harness.state().swatch.expect("the swatch was laid out");
    (screenshot::render(&mut harness).expect("renders"), swatch)
}

fn pixel(image: &RgbaImage, at: egui::Pos2, scale: Scale) -> [u8; 4] {
    let at = at * scale.pixels_per_point();
    image.get_pixel(at.x as u32, at.y as u32).0
}

#[test]
fn screenshot_sizes_follow_the_scale() {
    for scale in Scale::ALL {
        let (image, _) = shot(scale, Theme::Light);
        let factor = scale.pixels_per_point();
        assert_eq!(
            image.dimensions(),
            ((SIZE.x * factor) as u32, (SIZE.y * factor) as u32),
            "{scale:?}"
        );
    }
}

#[test]
fn screenshot_draws_the_swatch_at_both_scales() {
    for scale in Scale::ALL {
        let (image, swatch) = shot(scale, Theme::Light);
        assert!(
            Rect::from_min_size(egui::Pos2::ZERO, SIZE).contains_rect(swatch),
            "the swatch is on screen: {swatch:?}"
        );
        assert_eq!(
            pixel(&image, swatch.center(), scale),
            FILL.to_array(),
            "{scale:?}"
        );
    }
}

#[test]
fn screenshot_light_and_dark_differ() {
    let (light, swatch) = shot(Scale::X1, Theme::Light);
    let (dark, _) = shot(Scale::X1, Theme::Dark);
    assert_eq!(light.dimensions(), dark.dimensions());
    assert_eq!(
        pixel(&light, swatch.center(), Scale::X1),
        pixel(&dark, swatch.center(), Scale::X1)
    );
    assert_ne!(light, dark);
}

#[test]
fn screenshot_writes_both_themes_at_both_scales() {
    let dir = std::env::temp_dir().join(format!("eggshell-screenshot-{}", std::process::id()));
    let written = screenshot::write_shots(&dir, "swatch", |builder, _| {
        builder.with_size(SIZE).build_ui(|ui| {
            ui.label("shot");
        })
    })
    .expect("writes");
    let names: Vec<_> = written
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "swatch-light@1x.png",
            "swatch-light@2x.png",
            "swatch-dark@1x.png",
            "swatch-dark@2x.png"
        ]
    );
    let doubled = image::open(&written[1]).expect("a PNG").to_rgba8();
    assert_eq!(doubled.dimensions(), (240, 160));
    std::fs::remove_dir_all(&dir).expect("cleans up");
}
