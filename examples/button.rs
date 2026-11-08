//! Port of egui_taffy's "Button demo": buttons that hold other layouts, a plain
//! one that counts clicks and a selectable one that toggles.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with bullseye. A button is a `div_id` that senses clicks, with
//! hover and active paint as variants; being selected is app state known at
//! build time, so it is base paint. The reset button is a stock egui button in
//! a leaf of the same tree.

#[path = "common/mod.rs"]
mod common;

use bullseye::style::Tokens;
use bullseye::widget::Button;
use bullseye::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// The fills of a button in each state, from the theme's tokens.
#[derive(Clone, Copy, Debug)]
pub struct Fills {
    pub rest: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
}

impl Fills {
    pub fn of(t: &Tokens) -> Self {
        Self {
            rest: t.secondary,
            hover: t.border,
            pressed: t.ring,
            selected: t.primary,
        }
    }
}

#[derive(Default)]
pub struct State {
    pub clicks: u32,
    pub selected: bool,
}

/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]"><button>Reset</button></aside>
/// ```
fn controls(b: &mut B<'_, '_, State>) {
    b.widget(
        "reset",
        tw().self_start(),
        Button::new("Reset"),
        |state, ui, button| {
            if button.show(ui).clicked() {
                *state = State::default();
            }
        },
    );
}

type Align = fn(Tw) -> Tw;

const ALIGNMENTS: [(&str, Align); 4] = [
    ("START", Tw::self_start),
    ("CENTER", Tw::self_center),
    ("END", Tw::self_end),
    ("STRETCH", Tw::self_stretch),
];

/// A button's look. A selected one keeps its fill under the pointer.
///
/// ```html
/// <div class="flex flex-col items-center p-2 rounded border border-input text-foreground
///             bg-secondary hover:bg-border active:bg-ring">...</div>
/// <div class="... bg-primary text-primary-foreground active:bg-ring">...</div>
/// ```
fn button(t: &Tokens, selected: bool) -> Tw {
    let fills = Fills::of(t);
    let look = tw()
        .flex()
        .flex_col()
        .items_center()
        .p(2.0)
        .rounded()
        .border(1.0)
        .border_color(t.input)
        .active(|p| p.bg(fills.pressed));
    if selected {
        look.bg(fills.selected).text(t.primary_foreground)
    } else {
        look.bg(fills.rest)
            .text(t.foreground)
            .hover(|p| p.bg(fills.hover))
    }
}

/// The button's content: a title and one row per cross-axis alignment, each
/// aligned within the button as its name says.
///
/// ```html
/// <span>Button</span>
/// <div class="flex flex-col p-1 self-start"><span class="self-center">START</span></div>
/// <div class="flex flex-col p-1 self-center"><span class="self-center">CENTER</span></div>
/// ...
/// ```
fn content<A>(b: &mut B<'_, '_, A>, title: &str) {
    b.text(tw(), title);
    for (name, align) in ALIGNMENTS {
        b.div(align(tw().flex().flex_col().p(1.0)), |b| {
            b.text(tw().self_center(), name);
        });
    }
}

/// ```html
/// <div class="flex flex-col items-stretch gap-2 w-full max-w-full p-2">
///   <div class="button">...</div>
///   <span>Button clicked 0 times</span>
///   <hr>
///   <div class="button selected">...</div>
///   <span>Selected: false</span>
/// </div>
/// ```
pub fn demo(b: &mut B<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .items_stretch()
            .gap(2.0)
            .w_full()
            .max_w_full()
            .p(2.0),
        |b| {
            b.div_id(
                "button",
                button(&t, false),
                common::on_click::<State>(|state| state.clicks += 1),
                |b| content(b, "Button"),
            );
            b.text(tw(), format!("Button clicked {} times", state.clicks));
            b.rule(tw().h_px(1.0).bg(t.border));
            b.div_id(
                "selectable",
                button(&t, state.selected),
                common::on_click::<State>(|state| state.selected = !state.selected),
                |b| content(b, "Selectable button"),
            );
            b.text(tw(), format!("Selected: {}", state.selected));
        },
    );
}

/// The controls beside the demo.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, controls, |b| demo(b, state));
}

fn main() -> eframe::Result {
    common::run_app("button", State::default(), layout)
}
