//! A gallery of the kaolin README layouts, built and drawn by bullseye in eframe.

mod layouts;

use bullseye::{Builder, LayoutState, TextStyle, tw};
use egui::load::SizedTexture;
use egui::{Button, Color32, ImageSource, TextureHandle, TextureOptions};

/// What the gallery keeps between frames.
struct Gallery {
    selected: usize,
}

type Build = fn(&mut Builder<'_, '_, Gallery>, ImageSource<'static>);

const ENTRIES: [(&str, Build); 10] = [
    ("Hello World", |b, _| layouts::hello_world(b)),
    ("Directions", |b, _| layouts::directions(b)),
    ("Sizing", |b, _| layouts::sizing(b)),
    ("Styling", |b, _| layouts::styling(b)),
    ("Responsive layout", |b, _| layouts::responsive(b)),
    ("Multi-column layout", |b, _| layouts::multi_column(b)),
    ("Text wrapping", |b, _| layouts::text_wrapping(b)),
    ("Proportional growth", |b, _| {
        layouts::proportional_growth(b)
    }),
    ("Nested layouts", |b, _| layouts::nested(b)),
    ("Embedded example", layouts::embedded_layout),
];

/// The window: a list of layouts on the left, the selected one on the right.
///
/// ```html
/// <div class="flex flex-row size-full">
///   <nav class="flex flex-col w-[200px] flex-none gap-1 p-2 bg-[#dcdcdc] text-black">
///     <button class="w-full h-[26px]">Hello World</button>
///     <!-- one button per layout -->
///   </nav>
///   <main class="relative flex-1 bg-white"><!-- the selected layout --></main>
/// </div>
/// ```
fn window(b: &mut Builder<'_, '_, Gallery>, selected: usize, logo: ImageSource<'static>) {
    b.div(tw().flex().flex_row().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .w_px(200.0)
                .flex_none()
                .gap(1.0)
                .p(2.0)
                .bg(Color32::LIGHT_GRAY)
                .text(Color32::BLACK),
            |b| {
                for (index, (name, _)) in ENTRIES.into_iter().enumerate() {
                    b.div_id(
                        ("entry", index),
                        tw().w_full().h_px(26.0),
                        move |gallery, ui| {
                            let button = Button::selectable(index == gallery.selected, name);
                            if ui.add_sized(ui.max_rect().size(), button).clicked() {
                                gallery.selected = index;
                            }
                        },
                        |_| {},
                    );
                }
            },
        );
        b.div(tw().relative().flex_1().bg(Color32::WHITE), |b| {
            let (_, build) = ENTRIES[selected];
            build(b, logo);
        });
    });
}

fn logo(ctx: &egui::Context) -> TextureHandle {
    ctx.load_texture("logo", layouts::logo_image(), TextureOptions::NEAREST)
}

fn main() -> eframe::Result {
    let mut gallery = Gallery { selected: 0 };
    let mut layout =
        LayoutState::new().with_base_text(TextStyle::new(layouts::BODY, Color32::BLACK));
    let mut logo_texture: Option<TextureHandle> = None;
    eframe::run_ui_native("kaolin", eframe::NativeOptions::default(), move |ui, _| {
        ui.ctx().set_theme(egui::Theme::Light);
        let logo = logo_texture.get_or_insert_with(|| logo(ui.ctx()));
        let source = ImageSource::Texture(SizedTexture::from_handle(logo));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let (area, selected) = (ui.max_rect(), gallery.selected);
                layout
                    .build(ui.ctx(), |b| window(b, selected, source))
                    .layout(area)
                    .draw(&mut gallery, ui);
            });
    })
}
