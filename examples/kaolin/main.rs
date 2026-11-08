//! A gallery of the kaolin README layouts, built and drawn by bullseye in eframe.
//!
//! F12 toggles the layout debug overlay.

#[path = "../common/mod.rs"]
mod common;
mod layouts;

use bullseye::style::Mark;
use bullseye::widget::Button;
use bullseye::{Builder, LayoutState, TextStyle, tw};
use egui::load::SizedTexture;
use egui::{ImageSource, TextureHandle, TextureOptions};

/// What the gallery keeps between frames.
pub struct Gallery {
    pub selected: usize,
}

pub type Build = fn(&mut Builder<'_, '_, Gallery>, ImageSource<'static>);

pub const ENTRIES: [(&str, Build); 10] = [
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
///   <nav class="flex flex-col w-[200px] flex-none gap-1 p-2 bg-background border-r border-border text-foreground">
///     <button class="w-full">Hello World</button>
///     <!-- one button per layout -->
///   </nav>
///   <main class="relative flex-1 bg-background"><!-- the selected layout --></main>
/// </div>
/// ```
pub fn window(b: &mut Builder<'_, '_, Gallery>, selected: usize, logo: ImageSource<'static>) {
    let t = common::tokens(b.ctx());
    b.div(tw().flex().flex_row().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .w_px(200.0)
                .flex_none()
                .gap(1.0)
                .p(2.0)
                .bg(t.background)
                .border_r(1.0)
                .border_color(t.border)
                .text(t.foreground),
            |b| {
                for (index, (name, _)) in ENTRIES.into_iter().enumerate() {
                    b.widget(
                        ("entry", index),
                        tw().w_full(),
                        Button::selectable(name, Mark::from(index == selected)),
                        move |gallery, ui, button| {
                            if button.show(ui).clicked() {
                                gallery.selected = index;
                            }
                        },
                    );
                }
            },
        );
        b.div(tw().relative().flex_1().bg(t.background), |b| {
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
    let mut layout = LayoutState::new();
    let mut logo_texture: Option<TextureHandle> = None;
    let mut theme_bar = common::ThemeBar::new();
    eframe::run_ui_native("kaolin", eframe::NativeOptions::default(), move |ui, _| {
        let ctx = ui.ctx().clone();
        common::use_preset(&ctx);
        theme_bar.show(ui);
        layout.set_base_text(Some(TextStyle::new(
            layouts::BODY,
            common::tokens(&ctx).foreground,
        )));
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
