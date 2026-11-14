//! The ids of a virtual list's rows stay with the rows while they are
//! scrolled out of view and back.
//!
//! A port of egui_virtual_list's `virtual_list_stable_id_test`
//! (github.com/lucasmerlin/hello_egui, `crates/egui_virtual_list`, MIT),
//! rebuilt with `virtual_list`. Each of 100 rows shows a button and the id
//! egui keeps its state under. A row is keyed by its own data, so its ids
//! are the same whichever rows are built around it, and whatever egui keeps
//! for its widgets is there when it comes back.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Scope, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many rows the list has.
pub const ROWS: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    List,
    Hello,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row(pub u32);

pub struct State {
    pub rows: Vec<Row>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rows: (0..ROWS).map(Row).collect(),
        }
    }
}

/// ```html
/// <div class="flex items-center gap-2 py-1"><button>Hello</button> <span>id: …, index: 12</span></div>
/// ```
fn row(s: &mut Scope<'_, '_, State>, Row(index): Row) {
    let id = s.id_of(Part::Hello);
    s.div(tw().flex().items_center().gap(2.0).py(1.0), |s| {
        s.button(Part::Hello, "Hello").add();
        s.text(tw(), format!("id: {:016X}, index: {index}", id.value()));
    });
}

/// ```html
/// <div class="size-full px-2 overflow-y-auto">rows</div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    b.virtual_list(Part::List, tw().size_full().px(2.0), &state.rows)
        .items(row);
}

fn main() -> eframe::Result {
    common::run_app("virtual_list_stable_id", State::default(), layout)
}
