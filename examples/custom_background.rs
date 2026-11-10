//! Port of egui_taffy's "Custom background demo": boxes with default,
//! transparent, custom, bordered and rounded backgrounds, then the same boxes
//! as buttons whose fill and border react to the pointer.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with eggshell: taffy places, egui draws. A paint variant cannot
//! change a border's width, which would move layout; where the demo widens a
//! border on hover, the buttons add a ring instead. The box size slider is a
//! stock egui slider in a leaf of the same tree.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// The fill and border egui's dark theme gives a non-interactive widget.
pub const DEFAULT_FILL: Color32 = Color32::from_gray(27);
const DEFAULT_BORDER: Color32 = Color32::from_gray(60);
/// The fills egui's dark theme gives a hovered and a pressed button.
pub const BUTTON_HOVER: Color32 = Color32::from_gray(70);
pub const BUTTON_PRESSED: Color32 = Color32::from_gray(100);

pub struct State {
    /// Side of each box, in points.
    pub size: f32,
    pub clicks: u32,
    pub selected: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            size: 200.0,
            clicks: 0,
            selected: false,
        }
    }
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <!-- a range input for the box size, with its caption and value -->
/// </aside>
/// ```
fn controls(b: &mut B<'_, '_, State>, state: &State) {
    common::slider_row(
        b,
        "size",
        "Box size (points)",
        state.size,
        80.0..=300.0,
        |s| &mut s.size,
    );
}

/// ```html
/// <div class="flex items-center justify-center size-[200px] p-5 bg-[#1b1b1b] uppercase">bg default</div>
/// ```
fn background_box(b: &mut B<'_, '_, State>, state: &State, paint: Tw, text: Color32, label: &str) {
    b.div(
        paint
            .flex()
            .items_center()
            .justify_center()
            .size_px(state.size)
            .p(5.0)
            .text(text),
        |b| b.text(tw().uppercase(), label),
    );
}

/// ```html
/// <div class="flex flex-col items-center justify-center gap-2.5 size-full">
///   <h2 class="uppercase">backgrounds</h2>
///   <div class="flex flex-wrap justify-center gap-2.5 p-2.5 bg-black">
///     <div class="... bg-[#1b1b1b]">bg default</div>
///     <div class="... bg-[#1b1b1b] border border-[#3c3c3c]">bg + border default</div>
///     <div class="...">bg transparent</div>
///     <div class="... bg-[#606060]">bg custom</div>
///     <div class="... bg-[#dcdcdc] border-[20px] border-white rounded-[40px] text-black">ext 1</div>
///     <div class="... border-[40px] border-[#3c3c3c] rounded-[200px]">ext 2</div>
///   </div>
///   <h2 class="uppercase">buttons</h2>
///   <div class="flex flex-wrap justify-center gap-2.5 p-2.5 bg-black">...five buttons...</div>
///   <div class="flex gap-5 uppercase"><span>clicked: 0x</span><span>selected: false</span></div>
/// </div>
/// ```
pub fn demo(b: &mut B<'_, '_, State>, state: &State) {
    let white = Color32::WHITE;
    b.div(
        tw().flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(2.5)
            .size_full(),
        |b| {
            b.text(tw().role(common::sized(24.0)).uppercase(), "backgrounds");
            b.div(
                tw().flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(2.5)
                    .p(2.5)
                    .bg(Color32::BLACK),
                |b| {
                    background_box(b, state, tw().bg(DEFAULT_FILL), white, "bg default");
                    background_box(
                        b,
                        state,
                        tw().bg(DEFAULT_FILL)
                            .border(1.0)
                            .border_color(DEFAULT_BORDER),
                        white,
                        "bg + border default",
                    );
                    background_box(
                        b,
                        state,
                        tw().bg(Color32::TRANSPARENT),
                        white,
                        "bg transparent",
                    );
                    background_box(b, state, tw().bg(Color32::DARK_GRAY), white, "bg custom");
                    background_box(
                        b,
                        state,
                        tw().bg(Color32::LIGHT_GRAY)
                            .border(20.0)
                            .border_color(white)
                            .rounded_px(40.0),
                        Color32::BLACK,
                        "ext 1",
                    );
                    background_box(
                        b,
                        state,
                        tw().border(40.0)
                            .border_color(DEFAULT_BORDER)
                            .rounded_px(200.0),
                        white,
                        "ext 2",
                    );
                },
            );
            b.text(tw().role(common::sized(24.0)).uppercase(), "buttons");
            b.div(
                tw().flex()
                    .flex_wrap()
                    .justify_center()
                    .gap(2.5)
                    .p(2.5)
                    .bg(Color32::BLACK),
                |b| buttons(b, state),
            );
            b.div(tw().flex().gap(5.0), |b| {
                b.text(tw().uppercase(), format!("clicked: {}x", state.clicks));
                b.text(tw().uppercase(), format!("selected: {}", state.selected));
            });
        },
    );
}

/// A box that is a button: it senses clicks and `act` runs on each one.
fn button(
    b: &mut B<'_, '_, State>,
    key: &'static str,
    state: &State,
    paint: Tw,
    text: Color32,
    label: &str,
    act: fn(&mut State),
) {
    b.div_id(
        key,
        paint
            .flex()
            .items_center()
            .justify_center()
            .size_px(state.size)
            .p(5.0)
            .text(text)
            .sense_click(),
        |b| b.text(tw().uppercase(), label),
    )
    .on_click(act);
}

/// ```html
/// <div class="... bg-[#1b1b1b] hover:bg-[#464646] active:bg-[#646464]">button</div>
/// <div class="... bg-[#1b1b1b]">selectable</div>
/// <div class="... hover:bg-[#464646]">ext clickable transparent</div>
/// <div class="... border-2 rounded-[20px] hover:ring-8 hover:ring-[#3c3c3c]">ext clickable</div>
/// <div class="... bg-[#a0a0a0] border-[20px] border-white hover:border-[#606060] rounded-[200px]">ext selectable</div>
/// ```
fn buttons(b: &mut B<'_, '_, State>, state: &State) {
    let white = Color32::WHITE;
    let count: fn(&mut State) = |state| state.clicks += 1;
    let toggle: fn(&mut State) = |state| state.selected = !state.selected;
    let (selectable, ext_selectable) = if state.selected {
        ("selected", "ext selected")
    } else {
        ("selectable", "ext selectable")
    };
    button(
        b,
        "button",
        state,
        tw().bg(DEFAULT_FILL)
            .hover(|p| p.bg(BUTTON_HOVER))
            .active(|p| p.bg(BUTTON_PRESSED)),
        white,
        "button",
        count,
    );
    let selected_fill = if state.selected {
        BUTTON_PRESSED
    } else {
        DEFAULT_FILL
    };
    button(
        b,
        "selectable",
        state,
        tw().bg(selected_fill).hover(|p| p.bg(BUTTON_HOVER)),
        white,
        selectable,
        toggle,
    );
    button(
        b,
        "clickable transparent",
        state,
        tw().bg(Color32::TRANSPARENT).hover(|p| p.bg(BUTTON_HOVER)),
        white,
        "ext clickable transparent",
        count,
    );
    button(
        b,
        "clickable",
        state,
        tw().border(2.0)
            .border_color(DEFAULT_BORDER)
            .rounded_px(20.0)
            .hover(|p| p.ring(8.0, DEFAULT_BORDER)),
        white,
        "ext clickable",
        count,
    );
    let ext_fill = if state.selected {
        Color32::GRAY
    } else {
        Color32::LIGHT_GRAY
    };
    button(
        b,
        "ext selectable",
        state,
        tw().bg(ext_fill)
            .border(20.0)
            .border_color(white)
            .rounded_px(200.0)
            .hover(|p| p.border_color(Color32::DARK_GRAY)),
        Color32::BLACK,
        ext_selectable,
        toggle,
    );
}

/// The controls beside the demo.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, |b| controls(b, state), |b| demo(b, state));
}

fn main() -> eframe::Result {
    common::run_app("custom_background", State::default(), layout)
}
