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
use bullseye::widget::Slider;
use bullseye::{Builder, LayoutState, NodeUi, TextSpec, TextStyle, Tw, none, tw};
use egui::{Color32, FontFamily, Role, WidgetInfo};

/// Text.
pub const INK: Color32 = Color32::from_gray(20);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_gray(100);
/// Borders and dividers.
pub const EDGE: Color32 = Color32::from_gray(176);
/// The controls panel.
pub const PANEL: Color32 = Color32::from_gray(245);
/// The chosen cell of a segmented control.
pub const SELECTED: Color32 = Color32::from_rgb(170, 196, 232);
/// A control under the pointer.
pub const HOVER: Color32 = Color32::from_gray(228);
/// The ring of a control with keyboard focus.
pub const FOCUS: Color32 = Color32::from_rgb(70, 120, 200);

/// The controls panel's width.
const PANEL_WIDTH: f32 = 240.0;

/// The base text role.
pub const BODY: TextSpec = TextSpec {
    size: 16.0,
    line_height: 20.0,
    family: FontFamily::Proportional,
};

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

/// The text every layout starts from.
pub fn base_text() -> TextStyle {
    TextStyle::new(BODY, Color32::BLACK)
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
    let mut layout_state = LayoutState::new().with_base_text(base_text());
    eframe::run_ui_native(title, eframe::NativeOptions::default(), move |ui, _| {
        ui.ctx().set_theme(egui::Theme::Light);
        prepare(ui.ctx(), &state);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
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
///   <aside class="flex flex-col gap-3 p-3 w-[240px] shrink-0 bg-[#f5f5f5] border-r border-[#b0b0b0]">…</aside>
///   <div class="grow min-w-0 h-full">…</div>
/// </div>
/// ```
pub fn with_controls<'f, A>(
    b: &mut Builder<'_, 'f, A>,
    controls: impl FnOnce(&mut Builder<'_, 'f, A>),
    demo: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    b.div(tw().flex().size_full(), |b| {
        b.div(
            tw().flex()
                .flex_col()
                .gap(3.0)
                .p(3.0)
                .w_px(PANEL_WIDTH)
                .shrink_0()
                .bg(PANEL)
                .border_r(1.0)
                .border_color(EDGE)
                .text(INK),
            controls,
        );
        b.div(tw().grow().min_w_0().h_full(), demo);
    });
}

/// A caption over a control.
///
/// ```html
/// <div class="flex flex-col gap-1"><span class="text-[#646464]">Gap</span>…</div>
/// ```
pub fn captioned<'f, A>(
    b: &mut Builder<'_, 'f, A>,
    caption: &str,
    control: impl FnOnce(&mut Builder<'_, 'f, A>),
) {
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.text(tw().text(MUTED), caption);
        control(b);
    });
}

/// A captioned slider that edits the `f32` at `lens`, with its value beside
/// the caption.
///
/// ```html
/// <label class="flex flex-col gap-1">
///   <span class="flex justify-between"><span class="text-[#646464]">Gap</span><span>2</span></span>
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
    b.div(tw().flex().flex_col().gap(1.0), |b| {
        b.div(tw().flex().justify_between(), |b| {
            b.text(tw().text(MUTED), caption);
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

/// The look of a segmented control's cells.
///
/// ```html
/// <span class="px-2.5 py-1 rounded hover:bg-[#e4e4e4] focus-visible:ring-2 focus-visible:ring-[#4678c8]">flex-wrap</span>
/// <span class="px-2.5 py-1 rounded bg-[#aac4e8]">flex-nowrap</span>  <!-- pressed -->
/// ```
pub fn segment(cell: Cell) -> Tw {
    let look = tw().px(2.5).py(1.0).rounded().focus(|p| p.ring(2.0, FOCUS));
    match cell {
        Cell::Pressed => look.bg(SELECTED),
        Cell::Released => look.hover(|p| p.bg(HOVER)),
    }
}

/// A row of segments that sets the `T` at `lens`.
///
/// ```html
/// <div class="flex flex-wrap gap-1 p-0.5 border border-[#b0b0b0] rounded-md">…</div>
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
    segmented(
        b,
        key,
        tw().flex()
            .flex_wrap()
            .gap(1.0)
            .p(0.5)
            .border(1.0)
            .border_color(EDGE)
            .rounded_md(),
        current,
        options,
        segment,
        lens,
    );
}

/// A text field filling its row, in a box that rings while the field has
/// focus. `name` is its accessible name. The box is the node `key`; the leaf
/// inside it is `"field"`.
///
/// ```html
/// <div class="w-full border border-[#b0b0b0] rounded-md focus-within:ring-2 focus-within:ring-[#4678c8]">
///   <input class="w-full" placeholder="Your name">
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
    let measured = field.clone();
    b.div_id(
        key,
        tw().w_full()
            .border(1.0)
            .border_color(EDGE)
            .rounded_md()
            .focus_within(|p| p.ring(2.0, FOCUS)),
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
