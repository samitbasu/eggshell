//! What the examples share: the base text role, the eframe shell, and the
//! controls panel the demos put beside their layout, built from bullseye
//! widgets inside the same tree.
#![allow(
    dead_code,
    reason = "each crate that includes this file uses a subset of it"
)]

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use bullseye::prim::segmented::{Cell, segmented};
use bullseye::prim::text_field::{self, Field};
use bullseye::style::{Shadcn, Tokens};
use bullseye::widget::Slider;
use bullseye::{Builder, LayoutState, NodeUi, TextSpec, TextStyle, Tw, none, tw};
use egui::{Color32, Context, FontFamily, Role, Theme, WidgetInfo};

/// The controls panel's width.
const PANEL_WIDTH: f32 = 240.0;

/// The most segments a track keeps on one row.
const MAX_ONE_ROW: usize = 3;

/// Width of the focus ring, shadcn's `ring-[3px]`.
const RING_WIDTH: f32 = 3.0;

/// The tokens the examples are drawn with: the preset applied to the
/// context, or the preset's own for its current theme.
pub fn tokens(ctx: &Context) -> Tokens {
    bullseye::style::tokens(ctx).unwrap_or_else(|| Tokens::for_theme(ctx.theme()))
}

/// Applies the shadcn preset to both egui themes, once per context.
pub fn use_preset(ctx: &Context) {
    if bullseye::style::tokens(ctx).is_none() {
        Shadcn::apply_both(ctx);
    }
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
    shell(
        title,
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
    mut state: S,
    prepare: fn(&egui::Context, &S),
    mut frame: impl FnMut(&mut S, &mut LayoutState, egui::Rect, &mut egui::Ui) + 'static,
) -> eframe::Result {
    let mut layout_state = LayoutState::new();
    let mut first_frame = true;
    eframe::run_ui_native(title, eframe::NativeOptions::default(), move |ui, _| {
        let ctx = ui.ctx().clone();
        if std::mem::take(&mut first_frame) {
            ctx.set_theme(Theme::Light);
        }
        use_preset(&ctx);
        prepare(&ctx, &state);
        layout_state.set_base_text(Some(base_text(ctx.theme())));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(tokens(&ctx).background))
            .show(ui, |ui| {
                let area = ui.max_rect();
                frame(&mut state, &mut layout_state, area, ui);
            });
    })
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
    let t = tokens(b.ctx());
    b.div(tw().flex().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .gap(3.0)
                .p(3.0)
                .w_px(PANEL_WIDTH)
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
        b.widget(
            key,
            tw().w_full(),
            Slider::new(),
            move |state, ui, slider| {
                let mut edited = f64::from(*lens(state));
                let bounds = f64::from(*range.start())..=f64::from(*range.end());
                let response = slider.show(ui, &mut edited, bounds);
                if response.changed() {
                    *lens(state) = edited as f32;
                }
                response.widget_info(|| WidgetInfo::slider(ui.is_enabled(), edited, &name));
            },
        );
    });
}

/// A row in a list of choices: an accent fill under the pointer, and kept
/// while it is the chosen one.
///
/// ```html
/// <span class="px-2 py-1 rounded-md text-sm hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50">flex-wrap</span>
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
        Cell::Released => look.hover(|p| p.bg(t.accent)),
    }
}

/// The shadcn tab-list classes for the context's theme.
pub fn segmented_classes(ctx: &Context) -> bullseye::style::SegmentedClasses {
    Shadcn {
        theme: ctx.theme(),
        tokens: tokens(ctx),
    }
    .segmented()
}

/// A row of segments that sets the `T` at `lens`. A track of more than
/// [`MAX_ONE_ROW`] segments wraps and takes its height from its rows, which
/// the preset's fixed-height track cannot do.
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
    let classes = segmented_classes(b.ctx());
    let wraps = options.len() > MAX_ONE_ROW;
    let track = if wraps {
        classes.container().flex_wrap().h_auto().gap(0.5)
    } else {
        classes.container()
    };
    segmented(
        b,
        key,
        track,
        current,
        options,
        |cell| {
            let look = classes.cell(cell);
            if wraps { look.h_auto() } else { look }
        },
        lens,
    );
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
