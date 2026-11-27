//! A virtual list of 10,000 rows navigated as a listbox: Tab reaches the
//! list once, the arrow, Home, End, PageUp and PageDown keys move the
//! highlight over every row, and Enter or Space activates it.

use eggshell_examples::common;

use eggshell_ui::vlist::ListNav;
use eggshell_ui::{Builder, tw};

pub const ROWS: u32 = 10_000;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Row(pub u32);

pub struct State {
    pub rows: Vec<Row>,
    pub picked: Option<Row>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rows: (0..ROWS).map(Row).collect(),
            picked: None,
        }
    }
}

/// ```html
/// <ul role="listbox" tabindex="0" aria-activedescendant="row-3" class="h-64 overflow-auto">
///   <li role="option" id="row-3" class="data-[active]:bg-accent">Row 3</li>
/// </ul>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    let t = common::tokens(b.ctx());
    let accent = t.accent;
    b.div_id(
        "Page",
        tw().flex().flex_col().gap(3.0).p(4.0).w_px(260.0),
        |s| {
            s.virtual_list(
                "list",
                tw().h_px(220.0).border(1.0).border_color(t.border),
                &state.rows,
            )
            .fixed_px(28.0)
            .nav(ListNav::Listbox)
            .row_name(|row| format!("Row {}", row.0))
            .row_classes(
                tw().px(2.0)
                    .flex()
                    .items_center()
                    .hover(move |p| p.bg(accent)),
            )
            .on_activate(|state: &mut State, row| state.picked = Some(row))
            .items(|s, row| {
                let marker = if s.is_active() { "> " } else { "  " };
                s.text(tw(), format!("{marker}Row {}", row.0));
            });
            let picked = state.picked.map_or("Nothing picked".to_owned(), |row| {
                format!("Picked row {}", row.0)
            });
            s.span("picked", tw().text(t.muted_foreground), picked);
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("list_nav", State::default(), layout)
}
