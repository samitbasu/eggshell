//! Golden images of the kaolin layouts, rendered by wgpu through kittest.
//!
//! Run them with `cargo xtask snapshots`, which selects a deterministic
//! software renderer; see the README for reviewing and updating goldens.

use std::sync::Once;

use eggshell_ui::{Builder, LayoutState, TextStyle, tw};
use eggshell_ui_shadcn::{ShadcnStyle, Tokens};
use egui::{Color32, ImageSource, TextureHandle, TextureOptions, Vec2, load::SizedTexture};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
mod common;

#[path = "../../examples/kaolin/layouts.rs"]
mod layouts;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button.rs"]
mod button;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/kaolin/main.rs"]
mod kaolin_gallery;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/custom_background.rs"]
mod custom_background;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/button_children.rs"]
mod button_children;
#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/toggle_group.rs"]
mod toggle_group;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dialog.rs"]
mod dialog;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/menu.rs"]
mod menu;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/chat_shell/main.rs"]
mod chat_shell;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/controls_gallery.rs"]
mod controls_gallery;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/form.rs"]
mod form;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/signup_form.rs"]
mod signup_form;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_demos.rs"]
mod flex_demos;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_example.rs"]
mod flex_example;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex.rs"]
mod flex;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex_grid.rs"]
mod flex_grid;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flex_wrap.rs"]
mod flex_wrap;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/flexbox_guide.rs"]
mod flexbox_guide;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/grow.rs"]
mod grow;

#[allow(dead_code, reason = "the example's main and controls are not run here")]
#[path = "../../examples/holy_grail.rs"]
mod holy_grail;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/known_gaps.rs"]
mod known_gaps;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/sidebar.rs"]
mod sidebar;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/todo.rs"]
mod todo;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/dnd_gallery.rs"]
mod dnd_gallery;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/scroll.rs"]
mod scroll_example;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/overflow.rs"]
mod overflow;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/flex_scroll.rs"]
mod flex_scroll;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/image_gallery.rs"]
mod image_gallery;

#[allow(dead_code, reason = "the example's main is not run here")]
#[path = "../../examples/crate_ui.rs"]
mod crate_ui;

use crate::support;

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
    snapshot_ink(name, (Color32::BLACK, egui::Theme::Dark), layout);
}

/// [`snapshot`] for the kaolin layouts, whose text is the light theme's foreground.
fn kaolin_snapshot(
    name: &str,
    layout: impl for<'f> Fn(&mut Builder<'_, 'f, ()>, ImageSource<'static>),
) {
    snapshot_ink(
        name,
        (Tokens::light().foreground, egui::Theme::Light),
        layout,
    );
}

fn snapshot_ink(
    name: &str,
    (ink, theme): (Color32, egui::Theme),
    layout: impl for<'f> Fn(&mut Builder<'_, 'f, ()>, ImageSource<'static>),
) {
    report_renderer();
    let mut state = LayoutState::new().with_base_text(TextStyle::new(layouts::BODY, ink));
    let mut logo: Option<TextureHandle> = None;
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_theme(theme)
        .build_ui(|ui| {
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
    kaolin_snapshot("kaolin_hello_world", plain(layouts::hello_world));
}

#[test]
fn kaolin_directions() {
    kaolin_snapshot("kaolin_directions", plain(layouts::directions));
}

#[test]
fn kaolin_sizing() {
    kaolin_snapshot("kaolin_sizing", plain(layouts::sizing));
}

#[test]
fn kaolin_styling() {
    snapshot("kaolin_styling", plain(layouts::styling));
}

#[test]
fn kaolin_responsive() {
    kaolin_snapshot("kaolin_responsive", plain(layouts::responsive));
}

#[test]
fn kaolin_multi_column() {
    kaolin_snapshot("kaolin_multi_column", plain(layouts::multi_column));
}

#[test]
fn kaolin_text_wrapping() {
    kaolin_snapshot("kaolin_text_wrapping", plain(layouts::text_wrapping));
}

#[test]
fn kaolin_proportional_growth() {
    kaolin_snapshot(
        "kaolin_proportional_growth",
        plain(layouts::proportional_growth),
    );
}

#[test]
fn kaolin_nested() {
    kaolin_snapshot("kaolin_nested", plain(layouts::nested));
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
                            .opacity(eggshell_ui::Opacity::new(0.5)),
                        |_| {},
                    );
                },
            );
        }),
    );
}

/// Translucent rounded cards with a deep offset shadow over a light ground:
/// the shadow must reach around the rounded corners (#52).
#[test]
fn glass_card_shadows() {
    let shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(90),
    };
    snapshot(
        "glass_card_shadows",
        plain(move |b| {
            b.div(
                tw().size_full()
                    .bg(Color32::from_rgb(236, 238, 242))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(12.0),
                |b| {
                    for radius in [6.0, 14.0, 28.0] {
                        b.div(
                            tw().w_px(180.0)
                                .h_px(120.0)
                                .rounded_px(radius)
                                .bg(Color32::from_white_alpha(150))
                                .shadow(shadow),
                            |_| {},
                        );
                    }
                },
            );
        }),
    );
}

#[test]
fn port_flex() {
    light_snapshot("port_flex", SIZE, (), flex::layout);
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
    light_snapshot("port_holy_grail", SIZE, (), holy_grail::layout);
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
fn snapshot_controlled<A>(name: &str, app: A, view: support::View<A>) {
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
fn snapshot_app<A>(
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
fn snapshot_themed<A>(
    name: &str,
    size: Vec2,
    app: (A, support::View<A>, TextStyle),
    pointer: Option<egui::Pos2>,
    prepare: (egui::Theme, fn(&egui::Context, &A)),
) {
    snapshot_themed_in(ShadcnStyle::Vega, name, size, app, pointer, prepare);
}

/// [`snapshot_themed`] with the examples switched to `style`.
fn snapshot_themed_in<A>(
    style: ShadcnStyle,
    name: &str,
    size: Vec2,
    (app, view, base_text): (A, support::View<A>, TextStyle),
    pointer: Option<egui::Pos2>,
    (theme, prepare): (egui::Theme, fn(&egui::Context, &A)),
) {
    report_renderer();
    let mut harness = support::harness_prepared(size, base_text, app, view, prepare);
    common::set_style(&harness.ctx, style);
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(2);
    if let Some(pointer) = pointer {
        harness.hover_at(pointer);
        harness.run_steps(12);
    }
    harness.snapshot(name);
}

/// Icon toggles whose active one shows a `.children(..)` accent bar.
#[test]
fn button_children_light() {
    button_children_in("button_children_light", egui::Theme::Light);
}

#[test]
fn button_children_dark() {
    button_children_in("button_children_dark", egui::Theme::Dark);
}

fn button_children_in(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(200.0, 80.0),
        (
            button_children::State::default(),
            button_children::layout,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        None,
        (theme, |_, _| {}),
    );
}

/// A tool cluster and a card picker, each one `ToggleGroup`.
#[test]
fn toggle_group_light() {
    toggle_group_in("toggle_group_light", egui::Theme::Light);
}

#[test]
fn toggle_group_dark() {
    toggle_group_in("toggle_group_dark", egui::Theme::Dark);
}

fn toggle_group_in(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(380.0, 420.0),
        (
            toggle_group::State::default(),
            toggle_group::layout,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        None,
        (theme, |_, _| {}),
    );
}

/// The todo list with the pointer over the second row: its highlight, and the
/// Remove action revealed by group hover.
#[test]
fn todo_row_hovered() {
    todo_hovered("todo_row_hovered", egui::Theme::Light);
}

#[test]
fn todo_row_hovered_dark() {
    todo_hovered("todo_row_hovered_dark", egui::Theme::Dark);
}

fn todo_hovered(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(520.0, 260.0),
        (
            todo::State::default(),
            todo::layout,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        Some(egui::pos2(200.0, 88.0)),
        (theme, |_, _| {}),
    );
}

/// The todo list with the layout debug overlay on and the pointer over the
/// second row: every node's hairline, the hovered node's path and rect, and
/// the frame-timing readout. The readout's numbers are measured, so its box is
/// masked (magenta): the golden pins where it sits and how much room it takes.
#[test]
fn debug_overlay_todo() {
    report_renderer();
    let theme = egui::Theme::Light;
    let mut harness = support::harness_prepared(
        Vec2::new(520.0, 260.0),
        TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        todo::State::default(),
        todo::layout,
        |ctx, _| eggshell_ui::Debug::set(ctx, eggshell_ui::Debug::Outlines),
    );
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(2);
    harness.hover_at(egui::pos2(200.0, 88.0));
    harness.run_steps(12);
    let readout = harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| match shape {
                egui::Shape::Text(text) if text.galley.text().contains("fps") => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            }),
            _ => None,
        })
        .expect("the overlay draws the readout");
    harness.mask(readout.expand2(egui::vec2(4.0, 2.0)));
    harness.snapshot("debug_overlay_todo");
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
    port_button_in("port_button", egui::Theme::Light);
}

#[test]
fn port_button_dark() {
    port_button_in("port_button_dark", egui::Theme::Dark);
}

fn port_button_in(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(640.0, 520.0),
        (
            button::State {
                clicks: 3,
                selected: true,
            },
            button::layout,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        Some(egui::pos2(440.0, 120.0)),
        (theme, |_, _| {}),
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
    gallery_in(name, width, egui::Theme::Light);
}

fn gallery_in(name: &str, width: f32, theme: egui::Theme) {
    gallery_styled(ShadcnStyle::Vega, name, width, theme);
}

fn gallery_styled(style: ShadcnStyle, name: &str, width: f32, theme: egui::Theme) {
    report_renderer();
    let state = controls_gallery::State {
        cell_width: controls_gallery::CellWidth::new(width),
        ..Default::default()
    };
    let mut harness = support::harness_prepared(
        Vec2::new(1400.0, 800.0),
        common::base_text_styled(theme, style),
        state,
        controls_gallery::layout,
        controls_gallery::prepare,
    );
    common::set_style(&harness.ctx, style);
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(4);
    let reports = &harness.state().app.reports;
    assert!(
        controls_gallery::Control::ALL
            .iter()
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
fn controls_gallery_dark() {
    gallery_in("controls_gallery_dark", 200.0, egui::Theme::Dark);
}

/// One owned section at the size the example opens with: the golden, and the
/// assertion that everything in it fits the window.
fn owned_section(name: &str, section: controls_gallery::Section, theme: egui::Theme) {
    owned_section_after(name, section, theme, |_| {});
}

fn owned_section_lyra(name: &str, section: controls_gallery::Section, theme: egui::Theme) {
    owned_section_in(ShadcnStyle::Lyra, name, section, theme, |_| {});
}

/// [`owned_section`] after `act` has driven the harness.
fn owned_section_after(
    name: &str,
    section: controls_gallery::Section,
    theme: egui::Theme,
    act: impl FnOnce(&mut Harness<'static, support::Rig<controls_gallery::State>>),
) {
    owned_section_in(ShadcnStyle::Vega, name, section, theme, act);
}

fn owned_section_in(
    style: ShadcnStyle,
    name: &str,
    section: controls_gallery::Section,
    theme: egui::Theme,
    act: impl FnOnce(&mut Harness<'static, support::Rig<controls_gallery::State>>),
) {
    report_renderer();
    let state = controls_gallery::State {
        section,
        ..Default::default()
    };
    let mut harness = support::harness_prepared(
        controls_gallery::WINDOW,
        common::base_text_styled(theme, style),
        state,
        controls_gallery::layout,
        controls_gallery::prepare,
    );
    common::set_style(&harness.ctx, style);
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(12);
    act(&mut harness);
    let bottom = harness
        .state()
        .nodes
        .iter()
        .map(|(_, rect)| rect.bottom())
        .fold(0.0, f32::max);
    assert!(
        bottom > 100.0 && bottom <= controls_gallery::WINDOW.y,
        "{name} reaches {bottom} in a {} high window",
        controls_gallery::WINDOW.y
    );
    harness.snapshot(name);
}

macro_rules! owned_section_goldens {
    ($($light:ident, $dark:ident: $section:ident;)*) => {$(
        #[test]
        fn $light() {
            owned_section(stringify!($light), controls_gallery::Section::$section, egui::Theme::Light);
        }

        #[test]
        fn $dark() {
            owned_section(stringify!($dark), controls_gallery::Section::$section, egui::Theme::Dark);
        }
    )*};
}

owned_section_goldens! {
    controls_gallery_buttons_light, controls_gallery_buttons_dark: Buttons;
    controls_gallery_choices_light, controls_gallery_choices_dark: Choices;
    controls_gallery_values_light, controls_gallery_values_dark: Values;
}

/// The enabled stepper with its text field focused: the ring surrounds the
/// whole group and stops short of the buttons.
fn focused_stepper(name: &str, theme: egui::Theme) {
    owned_section_after(name, controls_gallery::Section::Values, theme, |harness| {
        let field = harness
            .state()
            .nodes
            .iter()
            .map(|(path, _)| path.clone())
            .find(|path| path.contains("stepper") && path.ends_with("/Box/Field"))
            .expect("the stepper's field");
        support::click_node(harness, &field);
        harness.run_steps(12);
        let group = field.trim_end_matches("/Field");
        let group = support::rect_of(harness, group);
        assert!(
            support::ring_at(harness, group).is_some(),
            "the focused stepper is ringed"
        );
    });
}

#[test]
fn controls_gallery_stepper_focused_light() {
    focused_stepper("controls_gallery_stepper_focused_light", egui::Theme::Light);
}

#[test]
fn controls_gallery_stepper_focused_dark() {
    focused_stepper("controls_gallery_stepper_focused_dark", egui::Theme::Dark);
}

/// The values section with the enabled select's list open: the chosen
/// option marked, the first highlighted.
fn select_open(name: &str, theme: egui::Theme) {
    owned_section_after(name, controls_gallery::Section::Values, theme, |harness| {
        let trigger = harness
            .state()
            .nodes
            .iter()
            .map(|(path, _)| path.clone())
            .find(|path| path.ends_with("(\"select\", Enabled)"))
            .expect("the enabled select");
        support::click_node(harness, &trigger);
        harness.run_steps(12);
        let first = harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.contains("List") && path.ends_with("/Box/Option(0)"));
        assert!(first, "the list is open");
    });
}

#[test]
fn controls_gallery_select_open_light() {
    select_open("controls_gallery_select_open_light", egui::Theme::Light);
}

#[test]
fn controls_gallery_select_open_dark() {
    select_open("controls_gallery_select_open_dark", egui::Theme::Dark);
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

fn light_snapshot<A>(name: &str, size: Vec2, app: A, view: support::View<A>) {
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
    known_gaps_in("known_gaps_shadows", egui::Theme::Light);
}

/// The shadow stage stays light, so the comparison still reads.
#[test]
fn known_gaps_shadows_dark() {
    known_gaps_in("known_gaps_shadows_dark", egui::Theme::Dark);
}

fn known_gaps_in(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(1100.0, 560.0),
        (
            (),
            |b, _| known_gaps::shadows(b),
            known_gaps::base_text(theme),
        ),
        None,
        (theme, |_, _| {}),
    );
}

fn kaolin_window(
    b: &mut Builder<'_, '_, kaolin_gallery::Gallery>,
    gallery: &kaolin_gallery::Gallery,
) {
    kaolin_gallery::window(b, gallery.selected, ImageSource::Uri("bytes://logo".into()));
}

/// The gallery's sidebar and stage, with Directions selected.
#[test]
fn kaolin_gallery_light() {
    kaolin_gallery_in("kaolin_gallery_light", egui::Theme::Light);
}

#[test]
fn kaolin_gallery_dark() {
    kaolin_gallery_in("kaolin_gallery_dark", egui::Theme::Dark);
}

fn kaolin_gallery_in(name: &str, theme: egui::Theme) {
    snapshot_themed(
        name,
        Vec2::new(900.0, 560.0),
        (
            kaolin_gallery::Gallery { selected: 1 },
            kaolin_window,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        None,
        (theme, |_, _| {}),
    );
}

/// shadcn's Lyra: square, compact, `text-xs`, a 1 pt ring. The same layouts
/// as the Vega goldens, switched with no code change.
mod lyra {
    use super::*;

    macro_rules! lyra_sections {
        ($($light:ident, $dark:ident: $section:ident;)*) => {$(
            #[test]
            fn $light() {
                owned_section_lyra(stringify!($light), controls_gallery::Section::$section, egui::Theme::Light);
            }

            #[test]
            fn $dark() {
                owned_section_lyra(stringify!($dark), controls_gallery::Section::$section, egui::Theme::Dark);
            }
        )*};
    }

    lyra_sections! {
        controls_gallery_lyra_buttons_light, controls_gallery_lyra_buttons_dark: Buttons;
        controls_gallery_lyra_choices_light, controls_gallery_lyra_choices_dark: Choices;
        controls_gallery_lyra_values_light, controls_gallery_lyra_values_dark: Values;
    }

    /// The text edits of the cells section, whose computed and drawn rects
    /// must agree under Lyra.
    #[test]
    fn controls_gallery_lyra_cells() {
        gallery_styled(
            ShadcnStyle::Lyra,
            "controls_gallery_lyra_cells",
            200.0,
            egui::Theme::Light,
        );
    }

    #[test]
    fn controls_gallery_lyra_cells_dark() {
        gallery_styled(
            ShadcnStyle::Lyra,
            "controls_gallery_lyra_cells_dark",
            200.0,
            egui::Theme::Dark,
        );
    }

    fn form_in(name: &str, theme: egui::Theme) {
        let state = form::State {
            name: "Ada Lovelace".into(),
            push_alerts: true,
            language: 1,
            theme: form::Theme::Dark,
            marks: vec![form::Mark::Bold, form::Mark::Underline],
            accent: form::SWATCHES[4],
            ..Default::default()
        };
        snapshot_themed_in(
            ShadcnStyle::Lyra,
            name,
            Vec2::new(760.0, 900.0),
            (
                state,
                form::layout,
                common::base_text_styled(theme, ShadcnStyle::Lyra),
            ),
            None,
            (theme, form::prepare),
        );
    }

    #[test]
    fn form_lyra() {
        form_in("form_lyra", egui::Theme::Light);
    }

    #[test]
    fn form_lyra_dark() {
        form_in("form_lyra_dark", egui::Theme::Dark);
    }

    fn chat_shell_lyra(name: &str, theme: egui::Theme) {
        let base = chat_shell::theme::Theme::new(theme).base_text();
        snapshot_themed_in(
            ShadcnStyle::Lyra,
            name,
            Vec2::new(1200.0, 760.0),
            (
                chat_shell::state::App::default(),
                chat_shell::view::layout,
                base,
            ),
            None,
            (theme, chat_shell::view::prepare),
        );
    }

    #[test]
    fn chat_shell_lyra_light() {
        chat_shell_lyra("chat_shell_lyra_light", egui::Theme::Light);
    }

    #[test]
    fn chat_shell_lyra_dark() {
        chat_shell_lyra("chat_shell_lyra_dark", egui::Theme::Dark);
    }
}

/// The telemetry gallery at rest, three columns, at a fixed time so the
/// traces are the same every run.
#[test]
fn dnd_gallery_light() {
    dnd_gallery_in("dnd_gallery_light", egui::Theme::Light);
}

#[test]
fn dnd_gallery_dark() {
    dnd_gallery_in("dnd_gallery_dark", egui::Theme::Dark);
}

fn dnd_gallery_in(name: &str, theme: egui::Theme) {
    report_renderer();
    let mut harness = support::frame_harness(
        Vec2::new(520.0, 400.0),
        common::base_text(theme),
        dnd_gallery::State::default(),
        dnd_gallery::frame,
    );
    harness.ctx.set_theme(theme);
    harness.run_steps(6);
    harness.snapshot(name);
}

/// A `with_drag_chip` chip being dragged over the gallery, at 2x.
#[test]
fn dnd_gallery_drag_chip_light() {
    dnd_gallery_chip_in("dnd_gallery_drag_chip_light", egui::Theme::Light);
}

#[test]
fn dnd_gallery_drag_chip_dark() {
    dnd_gallery_chip_in("dnd_gallery_drag_chip_dark", egui::Theme::Dark);
}

fn dnd_gallery_chip_in(name: &str, theme: egui::Theme) {
    report_renderer();
    let mut state = dnd_gallery::State::default();
    state.image = dnd_gallery::Image::Chip;
    let mut harness = support::frame_harness_scaled(
        Vec2::new(520.0, 440.0),
        2.0,
        common::base_text(theme),
        state,
        |ui, app, layout| dnd_gallery::window(ui, app, layout, &mut LayoutState::new()),
    );
    harness.ctx.set_theme(theme);
    harness.run_steps(6);
    let ctx = harness.ctx.clone();
    let panel = |key: u32| {
        support::layout_rect(
            &ctx,
            &format!("{}/PanelKey({key})/\"face\"", dnd_gallery::GRID),
        )
    };
    let from = panel(0).center();
    harness.hover_at(from);
    harness.run_steps(3);
    harness.drag_at(from);
    harness.step();
    harness.hover_at(from + egui::vec2(0.0, 8.0));
    harness.step();
    let over = panel(4).center();
    harness.hover_at(over);
    harness.run_steps(24);
    harness.snapshot(name);
}

/// The scroll example at rest: a vertical, a horizontal, a two-way and a
/// nested container.
#[test]
fn scroll_light() {
    scroll_in("scroll_light", egui::Theme::Light, false);
}

#[test]
fn scroll_dark() {
    scroll_in("scroll_dark", egui::Theme::Dark, false);
}

/// Every container of the scroll example scrolled, the nested ones both.
#[test]
fn scroll_scrolled_light() {
    scroll_in("scroll_scrolled_light", egui::Theme::Light, true);
}

fn scroll_in(name: &str, theme: egui::Theme, scrolled: bool) {
    use eggshell_ui::scroll::scroll_by;
    report_renderer();
    let mut harness = support::harness_prepared(
        Vec2::new(1360.0, 640.0),
        common::base_text(theme),
        scroll_example::State::default(),
        scroll_example::layout,
        |_, _| {},
    );
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(3);
    if scrolled {
        let id = |path: &str| support::id_of(&harness, path);
        let by = |path: &str, x, y| scroll_by(&harness.ctx, id(path), Vec2::new(x, y));
        by("/Vertical", 0.0, 330.0);
        by("/Horizontal", 260.0, 0.0);
        by("/Both", 150.0, 120.0);
        by("/Nested", 0.0, 90.0);
        by("/Nested/Carousel", 180.0, 0.0);
        by("/Nested/List", 0.0, 150.0);
        harness.run_steps(3);
    }
    harness.snapshot(name);
}

/// The overflow demo: visible, clip, hidden and scroll, twenty labels each.
#[test]
fn overflow_light() {
    light_snapshot(
        "overflow_light",
        Vec2::new(620.0, 360.0),
        overflow::State::default(),
        overflow::layout,
    );
}

#[test]
fn flex_scroll_light() {
    light_snapshot(
        "flex_scroll_light",
        Vec2::new(900.0, 700.0),
        flex_scroll::flex_example::State::default(),
        flex_scroll::layout,
    );
}

#[test]
fn image_gallery_light() {
    light_snapshot(
        "image_gallery_light",
        Vec2::new(900.0, 700.0),
        image_gallery::State::default(),
        image_gallery::layout,
    );
}

#[test]
fn crate_ui_light() {
    light_snapshot(
        "crate_ui_light",
        Vec2::new(800.0, 700.0),
        crate_ui::State::default(),
        crate_ui::layout,
    );
}

/// The dialog example's modals over its scrolling page: the dialog, a
/// confirmation opened from it, its role menu, and a sheet.
fn dialog_in(name: &str, theme: egui::Theme, state: dialog::State) {
    let state = dialog::State {
        name: "Pedro Duarte".to_owned(),
        ..state
    };
    snapshot_themed(
        name,
        SIZE,
        (
            state,
            dialog::layout,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
        ),
        Some(egui::pos2(2.0, 2.0)),
        (theme, common::use_accent::<dialog::State>),
    );
}

fn open_dialog() -> dialog::State {
    dialog::State {
        dialog: eggshell_ui::overlay::Shown::Open,
        ..dialog::State::default()
    }
}

#[test]
fn dialog_light() {
    dialog_in("dialog_light", egui::Theme::Light, open_dialog());
}

#[test]
fn dialog_dark() {
    dialog_in("dialog_dark", egui::Theme::Dark, open_dialog());
}

fn confirming() -> dialog::State {
    dialog::State {
        confirm: eggshell_ui::overlay::Shown::Open,
        ..open_dialog()
    }
}

#[test]
fn dialog_confirm_light() {
    dialog_in("dialog_confirm_light", egui::Theme::Light, confirming());
}

#[test]
fn dialog_confirm_dark() {
    dialog_in("dialog_confirm_dark", egui::Theme::Dark, confirming());
}

#[test]
fn dialog_role_menu_light() {
    let picking = dialog::State {
        picker: true,
        ..open_dialog()
    };
    dialog_in("dialog_role_menu_light", egui::Theme::Light, picking);
}

fn sheet() -> dialog::State {
    dialog::State {
        sheet: eggshell_ui::overlay::Shown::Open,
        ..dialog::State::default()
    }
}

#[test]
fn sheet_light() {
    dialog_in("sheet_light", egui::Theme::Light, sheet());
}

#[test]
fn sheet_dark() {
    dialog_in("sheet_dark", egui::Theme::Dark, sheet());
}

/// The menu example's dropdown, opened and driven through the real path.
fn menu_after(
    name: &str,
    theme: egui::Theme,
    act: impl FnOnce(&mut Harness<'static, support::Rig<menu::State>>),
) {
    report_renderer();
    let style = ShadcnStyle::Vega;
    let mut harness = support::harness_prepared(
        Vec2::new(420.0, 400.0),
        common::base_text_styled(theme, style),
        menu::State {
            grid: true,
            ..menu::State::default()
        },
        menu::layout,
        common::use_accent::<menu::State>,
    );
    common::set_style(&harness.ctx, style);
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(12);
    act(&mut harness);
    harness.snapshot(name);
}

const ACTIONS: &str = "/Page/Top/(PopoverButton, Menu)/Box";

/// The dropdown with its Arrange submenu open and the pointer on one of its
/// items: shortcuts, a disabled item, headings, a checked box, the chosen
/// radio, the chevron and the destructive item.
fn menu_open(name: &str, theme: egui::Theme) {
    menu_after(name, theme, |harness| {
        support::click_node(harness, "/Page/Top/PopoverButton");
        harness.run_steps(8);
        support::click_node(harness, &format!("{ACTIONS}/Item(Arrange)"));
        harness.run_steps(8);
        let front = format!("{ACTIONS}/(Item(Arrange), Submenu)/Box/Item(Front)");
        support::hover_node(harness, &front);
        harness.run_steps(8);
        assert!(
            harness.state().nodes.iter().any(|(path, _)| *path == front),
            "the submenu is open"
        );
    });
}

#[test]
fn menu_open_light() {
    menu_open("menu_open_light", egui::Theme::Light);
}

#[test]
fn menu_open_dark() {
    menu_open("menu_open_dark", egui::Theme::Dark);
}

/// The destructive item under the pointer.
fn menu_destructive(name: &str, theme: egui::Theme) {
    menu_after(name, theme, |harness| {
        support::click_node(harness, "/Page/Top/PopoverButton");
        harness.run_steps(8);
        support::hover_node(harness, &format!("{ACTIONS}/Item(Delete)"));
        harness.run_steps(8);
    });
}

#[test]
fn menu_destructive_light() {
    menu_destructive("menu_destructive_light", egui::Theme::Light);
}

#[test]
fn menu_destructive_dark() {
    menu_destructive("menu_destructive_dark", egui::Theme::Dark);
}

/// Frames of the dialog, the sheet and the menu entering and leaving, for
/// review (the motion is the user's choice): `cargo test --features
/// snapshots --test it motion_strips -- --ignored` with `EGGSHELL_MOTION_DIR`
/// naming where the PNGs go, and `VK_ICD_FILENAMES` the software renderer.
mod motion {
    use super::*;
    use eggshell_ui::overlay::Shown;

    fn out() -> std::path::PathBuf {
        let dir = std::env::var("EGGSHELL_MOTION_DIR")
            .expect("EGGSHELL_MOTION_DIR names the output directory");
        std::fs::create_dir_all(&dir).expect("the directory can be made");
        dir.into()
    }

    /// Five frames a quarter of `duration` apart, from 0% to 100%.
    fn frames<A>(harness: &mut Harness<'static, support::Rig<A>>, name: &str) {
        for quarter in 0..5 {
            harness.step();
            let image = harness.render().expect("renders");
            image
                .save(out().join(format!("{name}_{}.png", quarter * 25)))
                .expect("saves");
        }
    }

    fn dialog_harness(
        theme: egui::Theme,
        step_dt: f32,
        state: dialog::State,
    ) -> Harness<'static, support::Rig<dialog::State>> {
        let style = ShadcnStyle::Vega;
        let mut harness = support::harness_stepped(
            SIZE,
            step_dt,
            TextStyle::new(layouts::BODY, Tokens::for_theme(theme).foreground),
            dialog::State {
                name: "Pedro Duarte".to_owned(),
                ..state
            },
            dialog::layout,
        );
        common::set_style(&harness.ctx, style);
        harness.state_mut().page = support::Page::Themed;
        harness.state_mut().companion = support::Companion::Alone;
        harness.ctx.set_theme(theme);
        harness.run_steps(8);
        harness
    }

    fn modal_strips(
        kind: &str,
        open: fn(&mut dialog::State, eggshell_ui::overlay::Shown),
        enter: f32,
        exit: f32,
    ) {
        report_renderer();
        for (theme, label) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let mut harness = dialog_harness(theme, enter / 4.0, dialog::State::default());
            open(&mut harness.state_mut().app, Shown::Open);
            frames(&mut harness, &format!("{kind}_enter_{label}"));

            let mut harness = dialog_harness(theme, exit / 4.0, dialog::State::default());
            open(&mut harness.state_mut().app, Shown::Open);
            harness.run_steps(40);
            open(&mut harness.state_mut().app, Shown::Closed);
            frames(&mut harness, &format!("{kind}_exit_{label}"));
        }
    }

    #[test]
    #[ignore = "writes review images, see the module comment"]
    fn motion_strips_dialog() {
        modal_strips("dialog", |state, on| state.dialog = on, 0.2, 0.2);
    }

    #[test]
    #[ignore = "writes review images, see the module comment"]
    fn motion_strips_sheet() {
        modal_strips("sheet", |state, on| state.sheet = on, 0.5, 0.3);
    }

    fn menu_harness(theme: egui::Theme) -> Harness<'static, support::Rig<menu::State>> {
        let style = ShadcnStyle::Vega;
        let mut harness = support::harness_stepped(
            Vec2::new(420.0, 400.0),
            0.0375,
            common::base_text_styled(theme, style),
            menu::State {
                grid: true,
                ..menu::State::default()
            },
            menu::layout,
        );
        common::set_style(&harness.ctx, style);
        harness.state_mut().page = support::Page::Themed;
        harness.state_mut().companion = support::Companion::Alone;
        harness.ctx.set_theme(theme);
        harness.run_steps(8);
        harness
    }

    #[test]
    #[ignore = "writes review images, see the module comment"]
    fn motion_strips_menu() {
        report_renderer();
        for (theme, label) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let mut harness = menu_harness(theme);
            let at = support::rect_of(&harness, "/Page/Top/PopoverButton").center();
            support::hover_over(&mut harness, at);
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
            harness.step();
            frames(&mut harness, &format!("menu_enter_{label}"));
            harness.run_steps(12);
            harness.key_press(egui::Key::Escape);
            frames(&mut harness, &format!("menu_exit_{label}"));
        }
    }
}

/// The tooltip look candidates, for review (the look is the user's choice):
/// `cargo test --features snapshots --test it tooltip_candidates -- --ignored`
/// with `EGGSHELL_TOOLTIP_DIR` naming where the PNGs go, and
/// `VK_ICD_FILENAMES` the software renderer.
mod tooltip_candidates {
    use super::*;
    use eggshell_ui::Style;
    use eggshell_ui::overlay::Distance;
    use eggshell_ui_shadcn::Shadcn;

    fn out() -> std::path::PathBuf {
        let dir = std::env::var("EGGSHELL_TOOLTIP_DIR")
            .expect("EGGSHELL_TOOLTIP_DIR names the output directory");
        std::fs::create_dir_all(&dir).expect("the directory can be made");
        dir.into()
    }

    fn change(ctx: &egui::Context, edit: impl FnOnce(&mut Style)) {
        let mut style = (*Style::of(ctx)).clone();
        edit(&mut style);
        style.install(ctx);
    }

    /// (a) shadcn's TooltipContent: inverted, with its arrow.
    fn inverted_with_arrow(_: &egui::Context, _: &()) {}

    /// (b) the same without the arrow.
    fn inverted_without_arrow(ctx: &egui::Context, _: &()) {
        change(ctx, |style| {
            style.tooltip.arrow = None;
            style.tooltip.offset = Distance::new(4.0);
        });
    }

    /// (c) a popover: `bg-popover`, a border and `shadow-md`, no arrow.
    fn popover(ctx: &egui::Context, _: &()) {
        let tokens = Shadcn::of(ctx).tokens;
        change(ctx, |style| {
            let colors = eggshell_ui::contract::Colors {
                fill: tokens.popover,
                border: tokens.border,
                text: tokens.popover_foreground,
            };
            style.tooltip.popup = eggshell_ui::contract::Look::new(
                eggshell_ui::contract::States::still(colors, tokens.popover),
            )
            .bordered(1.0)
            .shadowed(tokens.shadow_md);
            style.tooltip.arrow = None;
            style.tooltip.offset = Distance::new(4.0);
        });
    }

    fn view(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.div(tw().flex().justify_center().p(4.0).pt(16.0), |b| {
            b.button("add", "Add to library")
                .tooltip("Add to library")
                .add();
        });
    }

    fn render(name: &str, theme: egui::Theme, scale: f32, prepare: fn(&egui::Context, &())) {
        let tokens = Tokens::for_theme(theme);
        let mut harness = support::harness_scaled(
            Vec2::new(240.0, 120.0),
            scale,
            TextStyle::new(layouts::BODY, tokens.foreground),
            (),
            view,
            prepare,
        );
        common::set_style(&harness.ctx, ShadcnStyle::Vega);
        harness.state_mut().page = support::Page::Themed;
        harness.state_mut().companion = support::Companion::Alone;
        harness.ctx.set_theme(theme);
        harness.run_steps(8);
        support::hover_node(&mut harness, "/\"add\"");
        harness.run_steps(30);
        let label = if theme == egui::Theme::Light {
            "light"
        } else {
            "dark"
        };
        let suffix = if scale > 1.0 { "@2x" } else { "" };
        harness
            .render()
            .expect("renders")
            .save(out().join(format!("{name}_{label}{suffix}.png")))
            .expect("saves");
    }

    #[test]
    #[ignore = "writes review images, see the module comment"]
    fn tooltip_candidates() {
        report_renderer();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            for scale in [1.0, 2.0] {
                render("a_inverted_arrow", theme, scale, inverted_with_arrow);
                render("b_inverted_plain", theme, scale, inverted_without_arrow);
                render("c_popover", theme, scale, popover);
            }
        }
    }
}

/// A button's built tooltip, text and a key chip, open beside the page's
/// own key chip.
fn rich_tooltip_view(b: &mut Builder<'_, '_, ()>, _: &()) {
    b.div(
        tw().flex()
            .items_center()
            .justify_center()
            .gap(3.0)
            .p(4.0)
            .pt(16.0),
        |b| {
            b.button("save", "Save")
                .tooltip_with(|s| {
                    s.div(tw().flex().items_center().gap(2.0), |s| {
                        s.text(tw(), "Save");
                        s.kbd("Ctrl+S");
                    });
                })
                .add();
            b.kbd("Ctrl+S");
        },
    );
}

fn rich_tooltip_open(harness: &mut Harness<'static, support::Rig<()>>, theme: egui::Theme) {
    common::set_style(&harness.ctx, ShadcnStyle::Vega);
    harness.state_mut().page = support::Page::Themed;
    harness.state_mut().companion = support::Companion::Alone;
    harness.ctx.set_theme(theme);
    harness.run_steps(8);
    support::hover_node(harness, "/\"save\"");
    harness.run_steps(30);
    assert!(
        harness
            .state()
            .nodes
            .iter()
            .any(|(path, _)| path.contains("Tooltip(")),
        "the tooltip is open"
    );
}

fn rich_tooltip_in(name: &str, theme: egui::Theme) {
    report_renderer();
    let tokens = Tokens::for_theme(theme);
    let mut harness = support::harness_prepared(
        Vec2::new(240.0, 120.0),
        TextStyle::new(layouts::BODY, tokens.foreground),
        (),
        rich_tooltip_view,
        |_, _| {},
    );
    rich_tooltip_open(&mut harness, theme);
    harness.snapshot(name);
}

#[test]
fn rich_tooltip_light() {
    rich_tooltip_in("rich_tooltip_light", egui::Theme::Light);
}

#[test]
fn rich_tooltip_dark() {
    rich_tooltip_in("rich_tooltip_dark", egui::Theme::Dark);
}

/// Candidate looks for a key chip on a tooltip, rendered for review:
/// `cargo test --features snapshots --test it kbd_candidates -- --ignored`
/// with `EGGSHELL_TOOLTIP_DIR` naming where the PNGs go, and
/// `VK_ICD_FILENAMES` the software renderer.
mod kbd_candidates {
    use super::*;
    use eggshell_ui::Style;
    use eggshell_ui::contract::Colors;
    use eggshell_ui_shadcn::Shadcn;

    fn out() -> std::path::PathBuf {
        let dir = std::env::var("EGGSHELL_TOOLTIP_DIR")
            .expect("EGGSHELL_TOOLTIP_DIR names the output directory");
        std::fs::create_dir_all(&dir).expect("the directory can be made");
        dir.into()
    }

    fn change(ctx: &egui::Context, edit: impl FnOnce(&mut Style)) {
        let mut style = (*Style::of(ctx)).clone();
        edit(&mut style);
        style.install_for(ctx, ctx.theme());
    }

    /// (a) shadcn's Kbd in a tooltip: `bg-background/20 text-background`,
    /// `/10` in the dark theme.
    fn tinted(_: &egui::Context, _: &()) {}

    /// (b) an outline: no fill, a 1 pt `background/40` border.
    fn outlined(ctx: &egui::Context, _: &()) {
        let tokens = Shadcn::of(ctx).tokens;
        change(ctx, |style| {
            style.kbd.tooltip = Colors {
                fill: Color32::TRANSPARENT,
                border: tokens.background.gamma_multiply(0.4),
                text: tokens.background,
            };
            style.kbd.border_width = 1.0;
        });
    }

    /// (c) the page's chip as it is: `bg-muted text-muted-foreground`.
    fn muted(ctx: &egui::Context, _: &()) {
        change(ctx, |style| style.kbd.tooltip = style.kbd.page);
    }

    fn render(name: &str, theme: egui::Theme, scale: f32, prepare: fn(&egui::Context, &())) {
        let tokens = Tokens::for_theme(theme);
        let mut harness = support::harness_scaled(
            Vec2::new(240.0, 120.0),
            scale,
            TextStyle::new(layouts::BODY, tokens.foreground),
            (),
            rich_tooltip_view,
            prepare,
        );
        rich_tooltip_open(&mut harness, theme);
        let label = if theme == egui::Theme::Light {
            "light"
        } else {
            "dark"
        };
        let suffix = if scale > 1.0 { "@2x" } else { "" };
        harness
            .render()
            .expect("renders")
            .save(out().join(format!("kbd_{name}_{label}{suffix}.png")))
            .expect("saves");
    }

    #[test]
    #[ignore = "writes review images, see the module comment"]
    fn kbd_candidates() {
        report_renderer();
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            for scale in [1.0, 2.0] {
                render("a_tinted", theme, scale, tinted);
                render("b_outlined", theme, scale, outlined);
                render("c_muted", theme, scale, muted);
            }
        }
    }
}
