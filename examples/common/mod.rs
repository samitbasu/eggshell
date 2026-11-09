//! What the examples share: the base text role, the eframe shell, and the
//! controls panel the demos put beside their layout, built from bullseye
//! widgets inside the same tree.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout
//! ([`bullseye::Debug`]) in every example
//! that shows a [`ThemeBar`] or calls [`toggle_debug`].
#![allow(
    dead_code,
    reason = "each crate that includes this file uses a subset of it"
)]

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use bullseye::owned::{Cell, Segmented, Slider};
use bullseye::prim::text_field::{self, Field};
use bullseye::style::{Accent, Shadcn, Tokens};
use bullseye::{Builder, LayoutState, NodeUi, TextSpec, TextStyle, Tw, none, tw};
use egui::{Color32, Context, FontFamily, Key, Role, Theme, ThemePreference, WidgetInfo};

/// The controls panel's width.
const PANEL_WIDTH: f32 = 240.0;

/// The most segments a track keeps on one row.
const MAX_ONE_ROW: usize = 3;

/// Width of the focus ring, shadcn's `ring-[3px]`.
const RING_WIDTH: f32 = 3.0;

/// The accent of the examples that use one: the forms' primary button and
/// focus.
pub const ACCENT: Accent = Accent::Blue;

/// The height of the strip that holds the theme toggle.
const THEME_BAR_HEIGHT: f32 = 44.0;

/// The choices of the theme toggle, in order.
pub const APPEARANCES: [(ThemePreference, &str); 3] = [
    (ThemePreference::System, "System"),
    (ThemePreference::Light, "Light"),
    (ThemePreference::Dark, "Dark"),
];

/// F12 flips the layout debug overlay and its frame-timing readout.
pub fn toggle_debug(ctx: &Context) {
    bullseye::Debug::toggle_on(ctx, Key::F12);
}

/// The tokens the examples are drawn with: the preset applied to the
/// context, or the preset's own for its current theme.
pub fn tokens(ctx: &Context) -> Tokens {
    Shadcn::of(ctx).tokens
}

/// Applies the neutral shadcn preset to both egui themes, once per context.
pub fn use_preset(ctx: &Context) {
    if Shadcn::applied(ctx).is_none() {
        Shadcn::apply_both(ctx);
    }
}

/// [`use_preset`] with [`ACCENT`], for an example's `prepare`.
pub fn use_accent<S>(ctx: &Context, _: &S) {
    Shadcn::apply_both_with(ctx, ACCENT);
}

/// The colour of the focus ring: `ring-ring/50`.
pub fn ring(tokens: &Tokens) -> Color32 {
    tokens.ring.gamma_multiply(0.5)
}

/// The base text role: shadcn's `text-sm`.
pub fn body(tokens: &Tokens) -> TextSpec {
    tokens.text_role()
}

/// A proportional text role of `size` points.
pub fn sized(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Proportional,
    }
}

/// A monospace text role of `size` points.
pub fn mono(size: f32) -> TextSpec {
    TextSpec {
        size,
        line_height: size * 1.25,
        family: FontFamily::Monospace,
    }
}

/// The text every layout starts from in `theme`.
pub fn base_text(theme: Theme) -> TextStyle {
    let tokens = Tokens::for_theme(theme);
    TextStyle::new(body(&tokens), tokens.foreground)
}

/// Builds one frame of a layout from the example's state.
pub type Layout<S> = for<'b, 'f> fn(&mut Builder<'b, 'f, ()>, &S);

/// Builds one frame of a layout from the example's state, which is also the
/// application its draw closures act on.
pub type AppLayout<S> = for<'b, 'f> fn(&mut Builder<'b, 'f, S>, &S);

/// Runs `layout` over the whole window.
pub fn run<S: 'static>(title: &str, state: S, layout: Layout<S>) -> eframe::Result {
    shell(
        title,
        None,
        state,
        |_, _| {},
        move |state, built_in, area, ui| {
            built_in
                .build(ui.ctx(), |b| layout(b, state))
                .layout(area)
                .draw(&mut (), ui);
        },
    )
}

/// [`run`] for examples whose state is the application: their nodes sense
/// pointer events and their draw closures change the state.
pub fn run_app<S: 'static>(title: &str, state: S, layout: AppLayout<S>) -> eframe::Result {
    run_app_styled(title, state, |_, _| {}, layout)
}

/// [`run_app`] for examples whose state also sets egui's style, which
/// `prepare` does before every frame is built.
pub fn run_app_styled<S: 'static>(
    title: &str,
    state: S,
    prepare: fn(&egui::Context, &S),
    layout: AppLayout<S>,
) -> eframe::Result {
    run_styled_in(title, None, state, prepare, layout)
}

/// [`run_app_styled`] in a native window of `inner_size`.
pub fn run_app_sized<S: 'static>(
    title: &str,
    inner_size: egui::Vec2,
    state: S,
    prepare: fn(&egui::Context, &S),
    layout: AppLayout<S>,
) -> eframe::Result {
    run_styled_in(title, Some(inner_size), state, prepare, layout)
}

fn run_styled_in<S: 'static>(
    title: &str,
    inner_size: Option<egui::Vec2>,
    state: S,
    prepare: fn(&egui::Context, &S),
    layout: AppLayout<S>,
) -> eframe::Result {
    shell(
        title,
        inner_size,
        state,
        prepare,
        move |state, layout_state, area, ui| {
            layout_state
                .build(ui.ctx(), |b| layout(b, state))
                .layout(area)
                .draw(state, ui);
        },
    )
}

fn shell<S: 'static>(
    title: &str,
    inner_size: Option<egui::Vec2>,
    mut state: S,
    prepare: fn(&egui::Context, &S),
    mut frame: impl FnMut(&mut S, &mut LayoutState, egui::Rect, &mut egui::Ui) + 'static,
) -> eframe::Result {
    let mut layout_state = LayoutState::new();
    let mut theme_bar = ThemeBar::new();
    launch(title, inner_size, move |ui| {
        let ctx = ui.ctx().clone();
        use_preset(&ctx);
        prepare(&ctx, &state);
        theme_bar.show(ui);
        layout_state.set_base_text(Some(base_text(ctx.theme())));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(tokens(&ctx).background))
            .show(ui, |ui| {
                let area = ui.max_rect();
                frame(&mut state, &mut layout_state, area, ui);
            });
    })
}

/// The id of the `<canvas>` element the web build draws into.
pub const WEB_CANVAS_ID: &str = "bullseye_canvas";

/// Runs `ui_fun` as the whole application: in a native window of
/// `inner_size` (the platform default when `None`), or on the page's
/// [`WEB_CANVAS_ID`] canvas on wasm32, where this returns once the runner is
/// spawned. Every example's `main` ends here, so both builds construct the
/// same app.
pub fn launch(
    title: &str,
    inner_size: Option<egui::Vec2>,
    ui_fun: impl FnMut(&mut egui::Ui) + 'static,
) -> eframe::Result {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut options = eframe::NativeOptions::default();
        if let Some(size) = inner_size {
            options.viewport = egui::ViewportBuilder::default().with_inner_size(size);
        }
        let mut ui_fun = ui_fun;
        eframe::run_ui_native(title, options, move |ui, _| ui_fun(ui))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (title, inner_size);
        web::start(ui_fun);
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::JsCast as _;

    struct App<F>(F);

    impl<F: FnMut(&mut egui::Ui)> eframe::App for App<F> {
        fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
            (self.0)(ui);
        }
    }

    pub fn start(ui_fun: impl FnMut(&mut egui::Ui) + 'static) {
        wasm_bindgen_futures::spawn_local(async move {
            let canvas = eframe::web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(super::WEB_CANVAS_ID))
                .and_then(|element| {
                    element
                        .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
                        .ok()
                })
                .expect("the page has a canvas with id `bullseye_canvas`");
            let started = eframe::WebRunner::new()
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(|_| Ok(Box::new(App(ui_fun)))),
                )
                .await;
            if let Err(error) = started {
                panic!("could not start eframe: {error:?}");
            }
        });
    }
}

/// The System, Light and Dark choices as the preset's tab list. The choice
/// lives in the egui context, which is what follows the OS theme under
/// [`ThemePreference::System`].
///
/// ```html
/// <div class="flex items-center h-9 p-[3px] rounded-lg bg-muted w-fit">
///   <span class="… text-muted-foreground">System</span>
///   <span class="… bg-background text-foreground shadow-sm">Light</span>  <!-- pressed -->
///   <span class="… text-muted-foreground">Dark</span>
/// </div>
/// ```
pub fn theme_toggle<A, K>(b: &mut Builder<'_, '_, A>, key: K)
where
    A: 'static,
    K: Hash + Debug + 'static,
{
    let current = b.ctx().options(|options| options.theme_preference);
    Segmented::new(current, &APPEARANCES).show(b, key, |_: &mut A, ctx, preference| {
        ctx.set_theme(preference)
    });
}

/// The strip every example shows along its top edge, with the theme toggle
/// at its right: for examples that run their own eframe loop.
///
/// ```html
/// <header class="flex items-center justify-end h-11 px-3 bg-background border-b border-border">…toggle…</header>
/// ```
#[derive(Default)]
pub struct ThemeBar {
    layout: LayoutState,
}

impl ThemeBar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows the strip as a top panel of `ui`.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        use_preset(&ctx);
        toggle_debug(&ctx);
        self.layout.set_base_text(Some(base_text(ctx.theme())));
        let t = tokens(&ctx);
        egui::Panel::top("theme_bar")
            .exact_size(THEME_BAR_HEIGHT)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(t.background))
            .show(ui, |ui| {
                let area = ui.max_rect();
                self.layout
                    .build(&ctx, |b| {
                        b.div(
                            tw().flex()
                                .items_center()
                                .justify_end()
                                .size_full()
                                .px(3.0)
                                .border_b(1.0)
                                .border_color(t.border)
                                .text(t.foreground),
                            |b| theme_toggle(b, "theme"),
                        );
                    })
                    .layout(area)
                    .draw(&mut (), ui);
            });
    }
}

/// A draw closure that runs `act` on the application when the node is clicked.
pub fn on_click<S>(
    act: impl FnOnce(&mut S) + 'static,
) -> impl FnOnce(&mut S, &mut egui::Ui) + 'static {
    move |state, ui| {
        if ui.node_response().clicked() {
            act(state);
            ui.ctx().request_repaint();
        }
    }
}

/// A demo with its controls: a panel on the left, built in the same tree, and
/// the demo filling the rest.
///
/// ```html
/// <div class="flex size-full">
///   <aside class="flex flex-col gap-3 p-3 w-[240px] shrink-0 bg-card border-r border-border text-foreground">…</aside>
///   <div class="grow min-w-0 h-full">…</div>
/// </div>
/// ```
pub fn with_controls<'f, A>(
    b: &mut Builder<'_, 'f, A>,
    controls: impl FnOnce(&mut Builder<'_, 'f, A>),
    demo: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    with_panel(b, PANEL_WIDTH, controls, demo);
}

/// [`with_controls`] with a panel `width` points wide.
pub fn with_panel<'f, A>(
    b: &mut Builder<'_, 'f, A>,
    width: f32,
    controls: impl FnOnce(&mut Builder<'_, 'f, A>),
    demo: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    let t = tokens(b.ctx());
    b.div(tw().flex().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .gap(3.0)
                .p(3.0)
                .w_px(width)
                .shrink_0()
                .bg(t.card)
                .border_r(1.0)
                .border_color(t.border)
                .text(t.foreground),
            controls,
        );
        b.div(tw().grow().min_w_0().h_full(), demo);
    });
}

/// A caption over a control.
///
/// ```html
/// <div class="flex flex-col gap-1"><span class="text-muted-foreground">Gap</span>…</div>
/// ```
pub fn captioned<'f, A>(
    b: &mut Builder<'_, 'f, A>,
    caption: &str,
    control: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    let muted = tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.text(tw().text(muted), caption);
        control(b);
    });
}

/// A captioned slider that edits the `f32` at `lens`, with its value beside
/// the caption.
///
/// ```html
/// <label class="flex flex-col gap-1">
///   <span class="flex justify-between"><span class="text-muted-foreground">Gap</span><span>2</span></span>
///   <input type="range" class="w-full">
/// </label>
/// ```
pub fn slider_row<S: 'static>(
    b: &mut Builder<'_, '_, S>,
    key: &'static str,
    caption: &str,
    value: f32,
    range: RangeInclusive<f32>,
    lens: fn(&mut S) -> &mut f32,
) {
    let name = caption.to_owned();
    let muted = tokens(b.ctx()).muted_foreground;
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.div(tw().flex().justify_between(), |b| {
            b.text(tw().text(muted), caption);
            b.text(tw(), format!("{value:.0}"));
        });
        let bounds = f64::from(*range.start())..=f64::from(*range.end());
        Slider::new(f64::from(value), bounds).named(name).show(
            b,
            key,
            move |state: &mut S, edited| {
                *lens(state) = edited as f32;
            },
        );
    });
}

/// A row in a list of choices: a half-strength accent fill under the pointer,
/// and the full accent fill while it is the chosen one.
///
/// ```html
/// <span class="px-2 py-1 rounded-md text-sm hover:bg-accent/50 focus-visible:ring-[3px] focus-visible:ring-ring/50">flex-wrap</span>
/// <span class="px-2 py-1 rounded-md text-sm bg-accent text-accent-foreground">flex-nowrap</span>  <!-- pressed -->
/// ```
pub fn menu_item(t: &Tokens, cell: Cell) -> Tw {
    let look = tw()
        .px(2.0)
        .py(1.0)
        .rounded_px(f32::from(t.radius_md()))
        .focus(|p| p.ring(RING_WIDTH, ring(t)));
    match cell {
        Cell::Pressed => look.bg(t.accent).text(t.accent_foreground),
        Cell::Released => look.hover(|p| p.bg(t.accent_50())),
    }
}

/// A row of segments that sets the `T` at `lens`. A track of more than
/// [`MAX_ONE_ROW`] segments wraps.
///
/// ```html
/// <div class="flex items-center h-9 p-[3px] rounded-lg bg-muted">
///   <span class="flex-1 flex items-center justify-center px-2 py-1 rounded-md text-muted-foreground">…</span>
///   <span class="… bg-background text-foreground shadow-sm">…</span>  <!-- pressed -->
/// </div>
/// ```
pub fn choice<S, K, T>(
    b: &mut Builder<'_, '_, S>,
    key: K,
    current: T,
    options: &[(T, &str)],
    lens: fn(&mut S) -> &mut T,
) where
    S: 'static,
    K: Hash + Debug + 'static,
    T: Copy + PartialEq + Hash + Debug + 'static,
{
    let group = Segmented::new(current, options);
    let group = if options.len() > MAX_ONE_ROW {
        group.wrapping()
    } else {
        group
    };
    group.show(b, key, move |state, _, value| *lens(state) = value);
}

/// A frameless text field filling its row, in shadcn's input box: `h-9`,
/// bordered, with a ring while the field has focus; the box is the field's
/// only frame. `name` is its accessible name. The box is the node `key`; the
/// leaf inside it is `"field"`.
///
/// ```html
/// <div class="flex items-center w-full min-h-9 px-3 py-1 border border-input rounded-md shadow-xs bg-background dark:bg-input/30
///   focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50">
///   <input class="w-full bg-transparent outline-none" placeholder="Your name">
/// </div>
/// ```
pub fn text_box<S, K>(
    b: &mut Builder<'_, '_, S>,
    key: K,
    (name, role): (&'static str, Role),
    field: Field,
    lens: fn(&mut S) -> &mut String,
) where
    S: 'static,
    K: Hash + Debug + 'static,
{
    let t = tokens(b.ctx());
    let fill = b.ctx().global_style().visuals.text_edit_bg_color();
    let field = field.bare();
    let measured = field.clone();
    b.div_id(
        key,
        tw().flex()
            .items_center()
            .w_full()
            .min_h_px(t.control_height)
            .px(t.input_padding.x / 4.0)
            .py(t.input_padding.y / 4.0)
            .border(1.0)
            .border_color(t.input)
            .rounded_px(f32::from(t.radius_md()))
            .bg(fill)
            .shadow(t.shadow_xs)
            .focus_within(|p| p.border_color(t.ring).ring(RING_WIDTH, ring(&t))),
        none,
        |b| {
            b.leaf(
                "field",
                tw().w_full(),
                move |m| text_field::measure(m, &measured),
                move |state, ui| {
                    let response = text_field::show(ui, lens(state), &field);
                    response.widget_info(|| WidgetInfo::labeled(role, ui.is_enabled(), name));
                },
            );
        },
    );
}
