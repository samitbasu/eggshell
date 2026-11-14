//! A virtual list of 10,000 rows of mixed height: date headers, and notes
//! whose tags wrap.
//!
//! Only the rows in and near the viewport are built, so a frame costs the
//! same however long the list is. Each row is measured the first time it is
//! built and counts as the estimate until then; the list keeps the row under
//! the top of the viewport in place as rows above it are measured. Resize the
//! window: the tags wrap anew and the heights are measured again. The buttons
//! scroll to a row with `vlist::scroll_to`.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::vlist;
use eggshell_ui::{Builder, Tw, tw};
use eggshell_ui_shadcn::Tokens;
use egui::Align;

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many rows the list has, headers included.
pub const ROWS: usize = 10_000;

const TAGS: [&str; 12] = [
    "layout", "taffy", "egui", "scroll", "regions", "overlays", "focus", "keys", "text", "measure",
    "draw", "a11y",
];

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A row of the list: its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    /// The header of a day.
    Day(u32),
    /// A note, by its index in [`State::notes`].
    Note(u32),
}

/// The parts the layout names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    List,
    First,
    Middle,
    Last,
}

pub struct Note {
    pub title: String,
    pub tags: Vec<&'static str>,
}

pub struct State {
    pub rows: Vec<Row>,
    pub notes: Vec<Note>,
}

impl Default for State {
    fn default() -> Self {
        let mut seed: u32 = 7;
        let mut next = move |below: u32| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (seed >> 16) % below
        };
        let mut rows = Vec::with_capacity(ROWS);
        let mut notes = Vec::new();
        let mut day = 0;
        while rows.len() < ROWS {
            rows.push(Row::Day(day));
            day += 1;
            for _ in 0..=next(8) {
                if rows.len() == ROWS {
                    break;
                }
                let index = notes.len() as u32;
                let count = if index % 5 == 3 {
                    10 + next(5)
                } else {
                    1 + next(5)
                };
                let tags = (0..count)
                    .map(|_| TAGS[next(TAGS.len() as u32) as usize])
                    .collect();
                notes.push(Note {
                    title: format!("Note {}", index + 1),
                    tags,
                });
                rows.push(Row::Note(index));
            }
        }
        Self { rows, notes }
    }
}

/// The date of day `day`, counting from 1 January.
fn date(day: u32) -> String {
    let (month, of_month) = (day / 28 % 12, day % 28 + 1);
    format!("{} {of_month}", MONTHS[month as usize])
}

fn tag(t: &Tokens) -> Tw {
    tw().px(1.5)
        .py(0.5)
        .rounded_full()
        .border(1.0)
        .border_color(t.border)
        .text(t.muted_foreground)
}

/// ```html
/// <div class="px-4 pt-4 pb-1 text-muted-foreground">March 3</div>
/// <div class="flex flex-col gap-1 px-4 py-2 border-b">
///   <span>Note 12</span>
///   <div class="flex flex-wrap gap-1"><span class="rounded-full border px-1.5 py-0.5 text-muted-foreground">taffy</span> …</div>
/// </div>
/// ```
fn row(s: &mut B<'_, '_>, t: &Tokens, state: &State, row: Row) {
    match row {
        Row::Day(day) => s.text(
            tw().px(4.0).pt(4.0).pb(1.0).text(t.muted_foreground),
            date(day),
        ),
        Row::Note(index) => {
            let note = &state.notes[index as usize];
            s.div(
                tw().flex()
                    .flex_col()
                    .gap(1.0)
                    .px(4.0)
                    .py(2.0)
                    .border_b(1.0)
                    .border_color(t.border),
                |s| {
                    s.text(tw(), note.title.as_str());
                    s.div(tw().flex().flex_wrap().gap(1.0), |s| {
                        for &name in &note.tags {
                            s.text(tag(t), name);
                        }
                    });
                },
            );
        }
    }
}

/// ```html
/// <div class="flex items-center gap-2"><span>10000 rows</span> <button>First</button> …</div>
/// ```
fn jumps(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    let list = b.id_of(Part::List);
    let rows = &state.rows;
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        b.text(
            tw().text(t.muted_foreground),
            format!("{} rows", rows.len()),
        );
        for (part, label, row) in [
            (Part::First, "First", rows[0]),
            (Part::Middle, "Middle", rows[rows.len() / 2]),
            (Part::Last, "Last", rows[rows.len() - 1]),
        ] {
            b.button(part, label)
                .on_response(move |_, response| {
                    if response.clicked() {
                        vlist::scroll_to(&response.ctx, list, row, Some(Align::Min));
                    }
                })
                .add();
        }
    });
}

/// ```html
/// <div class="flex flex-col gap-3 size-full p-4">
///   <div class="flex items-center gap-2">buttons</div>
///   <div class="grow min-h-0 overflow-y-auto rounded-md border bg-card">rows</div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    let t = common::tokens(b.ctx());
    b.div(tw().flex().flex_col().gap(3.0).size_full().p(4.0), |b| {
        jumps(b, state);
        b.virtual_list(
            Part::List,
            tw().grow()
                .min_h_0()
                .rounded_md()
                .border(1.0)
                .border_color(t.border)
                .bg(t.card),
            &state.rows,
        )
        .estimate_px(48.0)
        .items(move |s, entry| row(s, &t, state, entry));
    });
}

fn main() -> eframe::Result {
    common::run_app("long_list", State::default(), layout)
}
