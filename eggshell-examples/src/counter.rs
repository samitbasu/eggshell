//! A button that counts its clicks.

use eggshell_ui::{Builder, tw};

#[derive(Default)]
pub struct State {
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Add,
    Count,
}

/// ```html
/// <div class="flex items-center gap-3"> <button>Add one</button> <span>Counted 0</span> </div>
/// ```
pub fn view<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.div(tw().flex().flex_row().items_center().gap(3.0), |b| {
        b.button(Part::Add, "Add one")
            .on_click(|state: &mut State| state.count += 1);
        b.span(Part::Count, tw(), format!("Counted {}", state.count));
    });
}
