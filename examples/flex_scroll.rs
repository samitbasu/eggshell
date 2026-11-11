//! Port of egui_flex's "flex_scroll": the flex example's wrapping row of
//! growing items, repeated fifty times in a vertical scroll area.
//!
//! After the example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with eggshell rather than copied. egui_flex
//! needs a scope with a maximum height around each row to work in a scroll
//! area, because it sizes its items from the space it is offered; here the
//! rows are laid out by taffy at the scroll container's width, in the same
//! frame, with no such help. The row is the one of `flex_example`.

#[allow(
    dead_code,
    reason = "only the row and the state of the example are used"
)]
#[path = "flex_example.rs"]
pub mod flex_example;

use eggshell_ui::{Builder, tw};
use flex_example::{State, common};

type B<'b, 'f> = Builder<'b, 'f, State>;

const ROWS: u32 = 50;

/// ```html
/// <div class="size-full overflow-y-auto p-2">
///   <div class="flex flex-col gap-2">  <!-- fifty times: -->
///     <span>Item n</span> <div class="flex flex-wrap items-stretch w-full gap-2">…</div>
///   </div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.scroll("rows", tw().size_full().p(2.0)).children(|s| {
        s.div(tw().flex().flex_col().gap(2.0).text(t.foreground), |s| {
            for n in 0..ROWS {
                s.scope(n, |s| {
                    s.text(tw().text(t.muted_foreground), format!("Item {n}"));
                    flex_example::row(s, state);
                });
            }
        });
    });
}

fn main() -> eframe::Result {
    common::run_app("flex_scroll", State::default(), layout)
}
