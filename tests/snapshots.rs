//! Golden images of the kaolin layouts, rendered by wgpu through kittest.
//!
//! Run them with `cargo xtask snapshots`, which selects a deterministic
//! software renderer; see the README for reviewing and updating goldens.

use std::sync::Once;

use bullseye::{Builder, LayoutState, TextStyle, tw};
use egui::{Color32, ImageSource, TextureHandle, TextureOptions, Vec2, load::SizedTexture};
use egui_kittest::Harness;

#[path = "../examples/kaolin/layouts.rs"]
mod layouts;

const SIZE: Vec2 = Vec2::new(800.0, 600.0);

fn report_renderer() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let state = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            Default::default(),
        );
        println!("snapshot renderer: {:?}", state.adapter.get_info());
    });
}

/// Draws `layout` over an 800 x 600 screen and compares it with the golden `name`.
fn snapshot(name: &str, layout: impl for<'f> Fn(&mut Builder<'_, 'f, ()>, ImageSource<'static>)) {
    report_renderer();
    let mut state =
        LayoutState::new().with_base_text(TextStyle::new(layouts::BODY, Color32::BLACK));
    let mut logo: Option<TextureHandle> = None;
    let mut harness = Harness::builder().with_size(SIZE).build_ui(|ui| {
        let logo = logo.get_or_insert_with(|| {
            ui.ctx()
                .load_texture("logo", layouts::logo_image(), TextureOptions::NEAREST)
        });
        let source = ImageSource::Texture(SizedTexture::from_handle(logo));
        let area = ui.max_rect();
        state
            .build(ui.ctx(), |b| layout(b, source))
            .layout(area)
            .draw(&mut (), ui);
    });
    harness.snapshot(name);
}

/// A layout that ignores the logo.
fn plain(
    layout: impl for<'f> Fn(&mut Builder<'_, 'f, ()>) + 'static,
) -> impl for<'f> Fn(&mut Builder<'_, 'f, ()>, ImageSource<'static>) {
    move |b, _| layout(b)
}

#[test]
fn kaolin_hello_world() {
    snapshot("kaolin_hello_world", plain(layouts::hello_world));
}

#[test]
fn kaolin_directions() {
    snapshot("kaolin_directions", plain(layouts::directions));
}

#[test]
fn kaolin_sizing() {
    snapshot("kaolin_sizing", plain(layouts::sizing));
}

#[test]
fn kaolin_styling() {
    snapshot("kaolin_styling", plain(layouts::styling));
}

#[test]
fn kaolin_responsive() {
    snapshot("kaolin_responsive", plain(layouts::responsive));
}

#[test]
fn kaolin_multi_column() {
    snapshot("kaolin_multi_column", plain(layouts::multi_column));
}

#[test]
fn kaolin_text_wrapping() {
    snapshot("kaolin_text_wrapping", plain(layouts::text_wrapping));
}

#[test]
fn kaolin_proportional_growth() {
    snapshot(
        "kaolin_proportional_growth",
        plain(layouts::proportional_growth),
    );
}

#[test]
fn kaolin_nested() {
    snapshot("kaolin_nested", plain(layouts::nested));
}

#[test]
fn kaolin_embedded_layout() {
    snapshot("kaolin_embedded_layout", layouts::embedded_layout);
}

/// An absolute overlay stacked by `z` inside a `relative` container, with
/// the chrome clipped by its parent.
///
/// ```html
/// <div class="relative size-full bg-[#dcdcdc] overflow-hidden">
///   <div class="absolute inset-4 bg-white rounded-lg border border-[#a0a0a0]"></div>
///   <div class="absolute top-8 left-8 w-[240px] h-[120px] z-20 bg-[#00008b] rounded-lg text-white p-2 ring-2 ring-black">z-20</div>
///   <div class="absolute top-16 left-16 w-[240px] h-[120px] z-10 bg-[#a0a0a0] rounded-lg p-2 text-right">z-10</div>
///   <div class="absolute -right-8 bottom-4 w-[200px] h-[80px] bg-[#606060] opacity-50"></div>
/// </div>
/// ```
#[test]
fn chrome_stacked_by_z() {
    snapshot(
        "chrome_stacked_by_z",
        plain(|b| {
            b.div(
                tw().relative()
                    .size_full()
                    .bg(Color32::LIGHT_GRAY)
                    .overflow_hidden(),
                |b| {
                    b.div(
                        tw().absolute()
                            .inset(4.0)
                            .bg(Color32::WHITE)
                            .rounded_lg()
                            .border(1.0)
                            .border_color(Color32::GRAY),
                        |_| {},
                    );
                    b.div(
                        tw().absolute()
                            .top(8.0)
                            .left(8.0)
                            .w_px(240.0)
                            .h_px(120.0)
                            .z(20)
                            .bg(Color32::DARK_BLUE)
                            .rounded_lg()
                            .text(Color32::WHITE)
                            .p(2.0)
                            .ring(2.0, Color32::BLACK),
                        |b| b.text(tw(), "z-20"),
                    );
                    b.div(
                        tw().absolute()
                            .top(16.0)
                            .left(16.0)
                            .w_px(240.0)
                            .h_px(120.0)
                            .z(10)
                            .bg(Color32::GRAY)
                            .rounded_lg()
                            .p(2.0)
                            .text_right(),
                        |b| b.text(tw(), "z-10"),
                    );
                    b.div(
                        tw().absolute()
                            .right_px(-32.0)
                            .bottom(4.0)
                            .w_px(200.0)
                            .h_px(80.0)
                            .bg(Color32::DARK_GRAY)
                            .opacity(bullseye::Opacity::new(0.5)),
                        |_| {},
                    );
                },
            );
        }),
    );
}
