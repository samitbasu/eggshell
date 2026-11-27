//! TODO(prose): cards in a grid that pop out into windows of their own and
//! back.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

use eggshell_examples::common;

use eggshell_ui::window::{MinSize, Popped, Window, WindowFrame};
use eggshell_ui::{Builder, Scope, tw};

/// The cards' titles.
pub const CARDS: [&str; 4] = ["Revenue", "Latency", "Errors", "Queue"];

/// The rows of each card's body.
const ROWS: usize = 30;

#[derive(Clone, Copy, Debug, Hash)]
enum Part {
    Grid,
    Card(usize),
    Toggle,
    Body,
    Frames,
}

/// Which cards are popped out, and where their windows were last.
#[derive(Default)]
pub struct State {
    pub popped: [Popped; CARDS.len()],
    pub frames: [Option<WindowFrame>; CARDS.len()],
}

/// ```html
/// <div class="flex flex-col gap-3 p-4">
///   <div class="grid grid-cols-2 gap-3"> a card per title </div>
///   <div class="text-sm text-muted-foreground"> each window's frame </div>
/// </div>
/// ```
pub fn layout(b: &mut Builder<'_, '_, State>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(tw().flex().flex_col().gap(3.0).p(4.0), |b| {
        b.div_id(Part::Grid, tw().grid().grid_cols(2).gap(3.0), |s| {
            for (at, title) in CARDS.iter().enumerate() {
                card(s, state, at, title);
            }
        });
        b.div_id(Part::Frames, tw().flex().flex_col().gap(1.0), |s| {
            for (title, frame) in CARDS.iter().zip(state.frames) {
                let said = frame.map_or_else(
                    || format!("{title}: not opened"),
                    |frame| {
                        let at = frame.position.map_or_else(
                            || "an unknown place".to_owned(),
                            |at| format!("{:.0}, {:.0}", at.x, at.y),
                        );
                        format!(
                            "{title}: at {at}, {:.0} x {:.0}",
                            frame.size.x, frame.size.y
                        )
                    },
                );
                s.text(tw().text(t.muted_foreground), said);
            }
        });
    });
}

/// ```html
/// <div class="flex flex-col gap-2 p-3 h-[220px] rounded-lg border bg-card">
///   <div class="flex items-center justify-between"> title, Pop out </div>
///   <div class="flex-1 overflow-y-auto"> rows </div>
/// </div>
/// ```
fn card(s: &mut Scope<'_, '_, State>, state: &State, at: usize, title: &'static str) {
    let t = common::tokens(s.ctx());
    let popped = state.popped[at];
    s.pop_out(Part::Card(at))
        .classes(
            tw().flex()
                .flex_col()
                .gap(2.0)
                .p(3.0)
                .h_px(220.0)
                .rounded_lg()
                .border(1.0)
                .border_color(t.border)
                .bg(t.card),
        )
        .popped(popped)
        .window(
            Window::new(title)
                .frame(state.frames[at])
                .min_size(MinSize::Content),
        )
        .on_close(move |state: &mut State| state.popped[at] = Popped::In)
        .on_frame(move |state: &mut State, frame| state.frames[at] = Some(frame))
        .placeholder(move |s| {
            s.text(
                tw().text(t.muted_foreground),
                format!("{title} is in its own window"),
            );
        })
        .children(|s| {
            s.div(tw().flex().items_center().justify_between(), |s| {
                s.text(tw().font_medium(), title);
                let (label, next) = match popped {
                    Popped::In => ("Pop out", Popped::Out),
                    Popped::Out => ("Pop in", Popped::In),
                };
                s.button(Part::Toggle, label)
                    .on_click(move |state: &mut State| state.popped[at] = next);
            });
            s.scroll(Part::Body, tw().flex_1()).children(|s| {
                s.div(tw().flex().flex_col().gap(1.0), |s| {
                    for row in 0..ROWS {
                        s.text(tw(), format!("{title} row {row}"));
                    }
                });
            });
        });
}

fn main() -> eframe::Result {
    common::run_app("pop_out", State::default(), layout)
}
