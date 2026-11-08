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

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/controls_gallery.rs"]
mod controls_gallery;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/form.rs"]
mod form;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/signup_form.rs"]
mod signup_form;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/flex_demos.rs"]
mod flex_demos;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../examples/flex_example.rs"]
mod flex_example;

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
#[path = "../examples/known_gaps.rs"]
mod known_gaps;

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
    snapshot_controlled(
        "port_flex_wrap",
        flex_wrap::State::default(),
        flex_wrap::layout,
    );
}

#[test]
fn port_flex_grid() {
    snapshot_controlled(
        "port_flex_grid",
        flex_grid::State::default(),
        flex_grid::layout,
    );
}

#[test]
fn port_grow() {
    snapshot_controlled("port_grow", grow::State::default(), grow::layout);
}

#[test]
fn port_holy_grail() {
    snapshot("port_holy_grail", plain(|b| holy_grail::layout(b, &())));
}

fn guide(name: &str, title: &str, choice: usize) {
    snapshot_controlled(
        name,
        flexbox_guide::State::showing(title, choice),
        flexbox_guide::layout,
    );
}

#[test]
fn flexbox_guide_centering() {
    guide("flexbox_guide_centering", "Perfect centering", 0);
}

#[test]
fn flexbox_guide_six_items() {
    guide(
        "flexbox_guide_six_items",
        "Six items, evenly distributed",
        0,
    );
}

#[test]
fn flexbox_guide_align_content_stretch() {
    guide("flexbox_guide_align_content_stretch", "align-content", 6);
}

#[test]
fn flexbox_guide_justify_space_evenly() {
    guide("flexbox_guide_justify_space_evenly", "justify-content", 5);
}

/// An example with its controls panel beside an 800 x 600 demo.
fn snapshot_controlled<A: 'static>(name: &str, app: A, view: support::View<A>) {
    snapshot_app(
        name,
        Vec2::new(1040.0, 600.0),
        app,
        TextStyle::new(layouts::BODY, Color32::BLACK),
        view,
        None,
    );
}

/// Draws an interactive example of `size` in the light theme the examples
/// run in, with the pointer at `pointer` (in the layout's coordinates, 8
/// points in from the window's corner), and compares it with the golden `name`.
fn snapshot_app<A: 'static>(
    name: &str,
    size: Vec2,
    app: A,
    base_text: TextStyle,
    view: support::View<A>,
    pointer: Option<egui::Pos2>,
) {
    snapshot_themed(
        name,
        size,
        (app, view, base_text),
        pointer,
        (egui::Theme::Light, |_, _| {}),
    );
}

/// [`snapshot_app`] in `theme`, which stock widgets draw themselves in.
fn snapshot_themed<A: 'static>(
    name: &str,
    size: Vec2,
    (app, view, base_text): (A, support::View<A>, TextStyle),
    pointer: Option<egui::Pos2>,
    (theme, prepare): (egui::Theme, fn(&egui::Context, &A)),
) {
    report_renderer();
    let mut harness = support::harness_prepared(size, base_text, app, view, prepare);
    harness.ctx.set_theme(theme);
    harness.run_steps(2);
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
        Vec2::new(1640.0, 700.0),
        custom_background::State::default(),
        TextStyle::new(layouts::BODY, Color32::BLACK),
        custom_background::layout,
        Some(egui::pos2(1150.0, 490.0)),
    );
}

/// The selectable button selected, the counting button under the pointer.
#[test]
fn port_button() {
    snapshot_app(
        "port_button",
        Vec2::new(640.0, 520.0),
        button::State {
            clicks: 3,
            selected: true,
        },
        TextStyle::new(layouts::BODY, Color32::BLACK),
        button::layout,
        Some(egui::pos2(440.0, 120.0)),
    );
}

/// Grid selected, a crate pill under the pointer.
#[test]
fn port_sidebar() {
    let state = sidebar::State {
        route: sidebar::Route::Example("Grid"),
    };
    snapshot_app(
        "port_sidebar",
        Vec2::new(640.0, 760.0),
        state,
        TextStyle::new(layouts::BODY, Color32::BLACK),
        sidebar::layout,
        Some(egui::pos2(60.0, 560.0)),
    );
}

/// The chat shell showing `theme`, with `edit` applied to the app.
fn chat_shell_in(
    name: &str,
    theme: egui::Theme,
    edit: impl FnOnce(&mut chat_shell::state::App),
    pointer: Option<egui::Pos2>,
) {
    let mut app = chat_shell::state::App::default();
    edit(&mut app);
    let base = chat_shell::theme::Theme::new(theme).base_text();
    snapshot_themed(
        name,
        Vec2::new(1200.0, 760.0),
        (app, chat_shell::view::layout, base),
        pointer,
        (theme, chat_shell::view::prepare),
    );
}

/// The chat shell in the light theme with the pointer over a recent session.
#[test]
fn chat_shell_light() {
    chat_shell_in(
        "chat_shell_light",
        egui::Theme::Light,
        |_| {},
        Some(egui::pos2(120.0, 400.0)),
    );
}

/// The chat shell in the dark theme with the pointer over a recent session.
#[test]
fn chat_shell_dark() {
    chat_shell_in(
        "chat_shell_dark",
        egui::Theme::Dark,
        |_| {},
        Some(egui::pos2(120.0, 400.0)),
    );
}

/// The chat shell with a six-line draft, the thorough mode and the second
/// model: the composer has grown and the conversation has made room.
#[test]
fn chat_shell_composer_grown() {
    chat_shell_in(
        "chat_shell_composer_grown",
        egui::Theme::Dark,
        |app| {
            app.draft =
                "First line\nSecond line\nThird line\nFourth line\nFifth line\nSixth line".into();
            app.mode = chat_shell::state::Mode::Thorough;
            app.model = 1;
        },
        None,
    );
}

fn eleven_lines(app: &mut chat_shell::state::App) {
    app.draft = (1..=11)
        .map(|line| format!("Line number {line} of a long draft"))
        .collect::<Vec<_>>()
        .join("\n");
}

/// The chat shell with an eleven-line draft, past the composer's cap: the
/// text scrolls, and the fade at its edge blends into the composer's surface.
#[test]
fn chat_shell_composer_scrolling() {
    chat_shell_in(
        "chat_shell_composer_scrolling",
        egui::Theme::Dark,
        eleven_lines,
        None,
    );
}

/// [`chat_shell_composer_scrolling`] in the light theme, where the surface the
/// fade blends into is a different colour.
#[test]
fn chat_shell_composer_scrolling_light() {
    chat_shell_in(
        "chat_shell_composer_scrolling_light",
        egui::Theme::Light,
        eleven_lines,
        None,
    );
}

/// The controls gallery with every cell `width` points wide: the golden, and
/// the assertion that no widget's rect differs from the library's.
fn gallery(name: &str, width: f32) {
    report_renderer();
    let state = controls_gallery::State {
        cell_width: controls_gallery::CellWidth::new(width),
        ..Default::default()
    };
    let mut harness = support::harness_prepared(
        Vec2::new(1400.0, 800.0),
        controls_gallery::base_text(),
        state,
        controls_gallery::layout,
        controls_gallery::prepare,
    );
    harness.ctx.set_theme(egui::Theme::Light);
    harness.run_steps(4);
    let reports = &harness.state().app.reports;
    assert!(
        controls_gallery::Control::ALL
            .iter()
            .filter(|control| control.is_checked())
            .all(|control| reports.contains_key(control)),
        "precondition: every checked cell drew: {:?}",
        reports.keys()
    );
    let mismatched: Vec<_> = reports
        .iter()
        .filter(|(_, report)| report.mismatch().is_some())
        .collect();
    assert!(mismatched.is_empty(), "{mismatched:?}");
    harness.snapshot(name);
}

#[test]
fn controls_gallery_narrow() {
    gallery("controls_gallery_narrow", 110.0);
}

#[test]
fn controls_gallery_default() {
    gallery("controls_gallery_default", 200.0);
}

#[test]
fn controls_gallery_wide() {
    gallery("controls_gallery_wide", 300.0);
}

/// The form part-way through: a name, the push switch on, the beta switch and
/// region select inert, two marks on, another accent, and Save inert for want
/// of an email.
#[test]
fn form() {
    let state = form::State {
        name: "Ada Lovelace".into(),
        push_alerts: true,
        language: 1,
        theme: form::Theme::Dark,
        marks: vec![form::Mark::Bold, form::Mark::Underline],
        accent: form::SWATCHES[4],
        ..Default::default()
    };
    snapshot_themed(
        "form",
        Vec2::new(760.0, 900.0),
        (
            state,
            form::layout,
            TextStyle::new(layouts::BODY, Color32::BLACK),
        ),
        None,
        (egui::Theme::Light, form::prepare),
    );
}

fn signup_snapshot(name: &str, size: Vec2, state: signup_form::State) {
    snapshot_themed(
        name,
        size,
        (
            state,
            signup_form::layout,
            TextStyle::new(layouts::BODY, Color32::BLACK),
        ),
        None,
        (egui::Theme::Light, signup_form::prepare),
    );
}

fn light_snapshot<A: 'static>(name: &str, size: Vec2, app: A, view: support::View<A>) {
    snapshot_app(
        name,
        size,
        app,
        TextStyle::new(layouts::BODY, Color32::BLACK),
        view,
        None,
    );
}

/// The demos laid out horizontally, items at their natural size.
#[test]
fn flex_demos_horizontal() {
    light_snapshot(
        "flex_demos_horizontal",
        Vec2::new(1280.0, 700.0),
        flex_demos::State::default(),
        flex_demos::layout,
    );
}

/// The demos laid out vertically, with every item growing and the content of
/// the stretched items at the bottom right.
#[test]
fn flex_demos_vertical_grow() {
    light_snapshot(
        "flex_demos_vertical_grow",
        Vec2::new(1280.0, 900.0),
        flex_demos::State {
            direction: flex_demos::Direction::Vertical,
            grow: true,
            anchor: 8,
        },
        flex_demos::layout,
    );
}

#[test]
fn flex_example() {
    light_snapshot(
        "flex_example",
        Vec2::new(900.0, 700.0),
        flex_example::State::default(),
        flex_example::layout,
    );
}

#[test]
fn signup_form_empty() {
    signup_snapshot(
        "signup_form_empty",
        Vec2::new(640.0, 760.0),
        signup_form::State::default(),
    );
}

/// Submit pressed with a short name, a mismatched password and no consent:
/// every error shows.
#[test]
fn signup_form_rejected() {
    signup_snapshot(
        "signup_form_rejected",
        Vec2::new(640.0, 900.0),
        signup_form::State {
            fields: signup_form::Fields {
                first_name: "A".into(),
                email: "ada@example".into(),
                password: "hunter2".into(),
                age: 17.0,
                ..Default::default()
            },
            progress: signup_form::Progress::Rejected,
        },
    );
}

/// The shadow half of `known_gaps` (#12): CSS's layered negative-spread
/// shadows above, `shadow(..)` clamped below. The weights half depends on the
/// fonts installed on the machine, so it has no golden.
#[test]
fn known_gaps_shadows() {
    report_renderer();
    let mut state = LayoutState::new().with_base_text(known_gaps::base_text());
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1100.0, 560.0))
        .build_ui(|ui| {
            let area = ui.max_rect();
            state
                .build(ui.ctx(), known_gaps::shadows)
                .layout(area)
                .draw(&mut (), ui);
        });
    harness.snapshot("known_gaps_shadows");
}
