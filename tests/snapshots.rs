//! Golden images of the kaolin layouts, rendered by wgpu through kittest.
//!
//! Run them with `cargo xtask snapshots`, which selects a deterministic
//! software renderer; see the README for reviewing and updating goldens.

#![allow(
    clippy::duplicate_mod,
    reason = "each example includes the shared examples/common module"
)]

use std::sync::Once;

use bullseye::{Builder, LayoutState, TextStyle, tw};
use egui::{Color32, ImageSource, TextureHandle, TextureOptions, Vec2, load::SizedTexture};
use egui_kittest::Harness;

#[path = "../examples/kaolin/layouts.rs"]
mod layouts;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/button.rs"]
mod button;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/custom_background.rs"]
mod custom_background;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/chat_shell/main.rs"]
mod chat_shell;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/flex.rs"]
mod flex;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/flex_grid.rs"]
mod flex_grid;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/flex_wrap.rs"]
mod flex_wrap;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/flexbox_guide.rs"]
mod flexbox_guide;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/grow.rs"]
mod grow;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../examples/holy_grail.rs"]
mod holy_grail;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/sidebar.rs"]
mod sidebar;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/todo.rs"]
mod todo;

mod support;

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

#[test]
fn port_flex() {
    snapshot("port_flex", plain(|b| flex::layout(b, &())));
}

#[test]
fn port_flex_wrap() {
    snapshot(
        "port_flex_wrap",
        plain(|b| flex_wrap::layout(b, &flex_wrap::State::default())),
    );
}

#[test]
fn port_flex_grid() {
    snapshot(
        "port_flex_grid",
        plain(|b| flex_grid::layout(b, &flex_grid::State::default())),
    );
}

#[test]
fn port_grow() {
    snapshot(
        "port_grow",
        plain(|b| grow::layout(b, &grow::State::default())),
    );
}

#[test]
fn port_holy_grail() {
    snapshot("port_holy_grail", plain(|b| holy_grail::layout(b, &())));
}

#[test]
fn flexbox_guide_centering() {
    snapshot(
        "flexbox_guide_centering",
        plain(|b| flexbox_guide::layout(b, &flexbox_guide::State::showing("Perfect centering", 0))),
    );
}

#[test]
fn flexbox_guide_six_items() {
    snapshot(
        "flexbox_guide_six_items",
        plain(|b| {
            flexbox_guide::layout(
                b,
                &flexbox_guide::State::showing("Six items, evenly distributed", 0),
            )
        }),
    );
}

#[test]
fn flexbox_guide_align_content_stretch() {
    snapshot(
        "flexbox_guide_align_content_stretch",
        plain(|b| flexbox_guide::layout(b, &flexbox_guide::State::showing("align-content", 6))),
    );
}

#[test]
fn flexbox_guide_justify_space_evenly() {
    snapshot(
        "flexbox_guide_justify_space_evenly",
        plain(|b| flexbox_guide::layout(b, &flexbox_guide::State::showing("justify-content", 5))),
    );
}

/// Draws an interactive example of `size` with the pointer at `pointer` (in
/// the layout's coordinates, 8 points in from the window's corner) and compares
/// it with the golden `name`.
fn snapshot_app<A: 'static>(
    name: &str,
    size: Vec2,
    app: A,
    base_text: TextStyle,
    view: support::View<A>,
    pointer: Option<egui::Pos2>,
) {
    report_renderer();
    let mut harness = support::harness(size, base_text, app, view);
    if let Some(pointer) = pointer {
        harness.hover_at(pointer);
        harness.run_steps(12);
    }
    harness.snapshot(name);
}

/// The todo list with the pointer over the second row: its highlight, and the
/// Remove action revealed by group hover.
#[test]
fn todo_row_hovered() {
    snapshot_app(
        "todo_row_hovered",
        Vec2::new(520.0, 260.0),
        todo::State::default(),
        TextStyle::new(layouts::BODY, Color32::BLACK),
        todo::layout,
        Some(egui::pos2(200.0, 88.0)),
    );
}

/// Every box of the demo, with the pointer over the `ext clickable` button.
#[test]
fn port_custom_background() {
    snapshot_app(
        "port_custom_background",
        Vec2::new(1400.0, 700.0),
        custom_background::State::default(),
        TextStyle::new(layouts::BODY, Color32::BLACK),
        custom_background::layout,
        Some(egui::pos2(910.0, 490.0)),
    );
}

/// The selectable button selected, the counting button under the pointer.
#[test]
fn port_button() {
    snapshot_app(
        "port_button",
        Vec2::new(400.0, 520.0),
        button::State {
            clicks: 3,
            selected: true,
        },
        TextStyle::new(layouts::BODY, Color32::BLACK),
        button::layout,
        Some(egui::pos2(200.0, 120.0)),
    );
}

/// Grid selected, a crate pill under the pointer.
#[test]
fn port_sidebar() {
    let mut state = sidebar::State::default();
    state.route = sidebar::Route::Example("Grid");
    snapshot_app(
        "port_sidebar",
        Vec2::new(640.0, 760.0),
        state,
        TextStyle::new(layouts::BODY, Color32::BLACK),
        sidebar::layout,
        Some(egui::pos2(60.0, 560.0)),
    );
}

/// The chat shell with the pointer over a recent session.
#[test]
fn chat_shell() {
    let app = chat_shell::state::App::default();
    let base = app.theme.base_text();
    snapshot_app(
        "chat_shell",
        Vec2::new(1200.0, 760.0),
        app,
        base,
        chat_shell::view::layout,
        Some(egui::pos2(120.0, 400.0)),
    );
}
