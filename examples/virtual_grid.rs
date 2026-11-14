//! A grid of 100,000 rows of buttons whose rows are virtualized: rows of two
//! cells alternate with rows of one cell spanning both columns, under a
//! two-row header.
//!
//! A port of egui_taffy's "Virtual grid row" demo (`examples/demo.rs`,
//! github.com/PPakalns/egui_taffy, MIT), rebuilt with `virtual_list`. The
//! rows are one height, so the list takes `fixed_px` and builds only the
//! rows in view. Two differences, both from building only some rows: the
//! two columns share the width equally, since a grid's auto columns would
//! need every row's content, and the header sits above the scrolled rows
//! rather than sticking at the top of them. The buttons are placeholders.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Scope, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many rows the grid has below its header.
pub const ROWS: u32 = 100_000;

/// The height of a row: a button and the padding around it.
const ROW: f32 = 44.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Grid,
    Cell(u8),
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
/// <div class="grid grid-cols-2 gap-1 p-1">
///   <div class="col-span-2 flex justify-center p-1">Colspan 2 header</div>
///   <div class="p-1">Header 1</div> <div class="p-1">Header 2</div>
/// </div>
/// ```
fn header(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().grid()
            .grid_cols(2)
            .gap(1.0)
            .p(1.0)
            .border_b(1.0)
            .border_color(t.border)
            .bg(t.muted),
        |b| {
            b.div(tw().col_span(2).flex().justify_center().p(1.0), |b| {
                b.text(tw(), "Colspan 2 header");
            });
            for column in 1..=2 {
                b.div(tw().p(1.0), |b| b.text(tw(), format!("Header {column}")));
            }
        },
    );
}

/// An odd row has two cells, an even row one that spans both.
///
/// ```html
/// <div class="grid grid-cols-2 gap-1 px-1 h-11 items-center">
///   <div class="flex justify-center"><button>Cell 1 1</button></div> …
/// </div>
/// ```
fn row(s: &mut Scope<'_, '_, State>, Row(index): Row) {
    s.div(
        tw().grid()
            .grid_cols(2)
            .gap(1.0)
            .px(1.0)
            .h_full()
            .items_center(),
        |s| {
            if index % 2 == 1 {
                for column in 1..=2 {
                    s.div(tw().flex().justify_center(), |s| {
                        s.button(Part::Cell(column), format!("Cell {index} {column}"))
                            .add();
                    });
                }
            } else {
                s.div(tw().col_span(2).flex().justify_center(), |s| {
                    s.button(Part::Cell(0), format!("Cell {index} - Colspan 2"))
                        .add();
                });
            }
        },
    );
}

/// ```html
/// <div class="flex flex-col size-full">
///   <div class="grid …">header</div>
///   <div class="grow min-h-0 overflow-y-auto">rows</div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    b.div(tw().flex().flex_col().size_full(), |b| {
        header(b);
        b.virtual_list(Part::Grid, tw().grow().min_h_0(), &state.rows)
            .fixed_px(ROW)
            .items(row);
    });
}

fn main() -> eframe::Result {
    common::run_app("virtual_grid", State::default(), layout)
}
