//! A pane grid that follows the room it has: container queries by regions.
//!
//! The grid is a region, so its closure learns the width layout gave it, in the
//! same frame, and lays out as many columns as fit. Each pane's title bar is a
//! region too, and picks the first of three forms whose max-content width
//! fits: the full title with its subtitle and labelled buttons, the title
//! with icon buttons, or icons alone. Resize the window to watch both change.
//! The buttons are placeholders.

#[path = "common/mod.rs"]
mod common;

use common::glyphs::{self, Glyph};
use eggshell_ui::owned::Face;
use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, Width, tw};
use eggshell_ui_shadcn::{size, variant};
use egui::Vec2;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

struct Pane {
    title: &'static str,
    subtitle: &'static str,
    body: &'static str,
}

const PANES: [Pane; 8] = [
    Pane {
        title: "Revenue",
        subtitle: "Updated 3 min ago",
        body: "Quarterly revenue by region.",
    },
    Pane {
        title: "Latency",
        subtitle: "Last hour",
        body: "Median and tail latency of the API.",
    },
    Pane {
        title: "Signups",
        subtitle: "Updated 1 min ago",
        body: "New accounts per day.",
    },
    Pane {
        title: "Errors",
        subtitle: "Last 24 hours",
        body: "Failed requests by endpoint.",
    },
    Pane {
        title: "Throughput",
        subtitle: "Updated 5 min ago",
        body: "Requests handled per second.",
    },
    Pane {
        title: "Queue depth",
        subtitle: "Live",
        body: "Jobs waiting for a worker.",
    },
    Pane {
        title: "Uptime",
        subtitle: "Last 30 days",
        body: "Availability of each service.",
    },
    Pane {
        title: "Spend",
        subtitle: "This month",
        body: "Cloud cost against budget.",
    },
];

/// The narrowest a pane gets before the grid drops a column.
const MIN_PANE_WIDTH: f32 = 200.0;
/// The grid's gap, and the padding around it, in spacing units.
const GAP_UNITS: f32 = 3.0;
const UNIT: f32 = 4.0;
const PANE_HEIGHT: f32 = 140.0;
const TITLE_BAR_HEIGHT: f32 = 40.0;

/// How a pane's title bar is drawn: from the most to the least it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Title, subtitle and labelled buttons.
    Full,
    /// Title and icon buttons.
    Compact,
    /// A pane icon and icon buttons.
    Icons,
}

/// The columns of the grid in `room`: as many as keep a pane from being
/// narrower than [`MIN_PANE_WIDTH`], and at least one.
pub fn columns(room: Vec2) -> u16 {
    let gap = GAP_UNITS * UNIT;
    let inner = room.x - 2.0 * gap;
    (((inner + gap) / (MIN_PANE_WIDTH + gap)).floor() as u16).max(1)
}

/// A pane's title bar in `form`.
///
/// ```html
/// <div class="flex items-center gap-2 size-full px-3">
///   <span>Revenue</span> <span class="text-muted-foreground">Updated 3 min ago</span>
///   <span class="grow"></span>
///   <button class="ghost">+ Add</button> <button class="ghost">× Close</button>
/// </div>
/// ```
fn title_bar<A: 'static>(b: &mut B<'_, '_, A>, pane: &Pane, form: Form) {
    let muted = common::tokens(b.ctx()).muted_foreground;
    let (add, close, pane_icon) = (
        glyphs::icon(b.ctx(), Glyph::Plus),
        glyphs::icon(b.ctx(), Glyph::Cross),
        glyphs::icon(b.ctx(), Glyph::Monitor),
    );
    b.div(
        tw().flex().items_center().gap(2.0).size_full().px(3.0),
        |b| {
            match form {
                Form::Full => {
                    b.text(tw().truncate(), pane.title);
                    b.text(tw().text(muted).truncate(), pane.subtitle);
                }
                Form::Compact => b.text(tw().truncate(), pane.title),
                Form::Icons => b.glyph(tw().size_px(16.0).shrink_0(), pane_icon),
            }
            b.div(tw().grow(), |_| {});
            match form {
                Form::Full => {
                    b.button("add", "Add")
                        .icon(add)
                        .variant(variant::GHOST)
                        .add();
                    b.button("close", "Close")
                        .icon(close)
                        .variant(variant::GHOST)
                        .add();
                }
                Form::Compact | Form::Icons => {
                    b.button("add", Face::Icon(add))
                        .named("Add")
                        .variant(variant::GHOST)
                        .size(size::ICON)
                        .add();
                    b.button("close", Face::Icon(close))
                        .named("Close")
                        .variant(variant::GHOST)
                        .size(size::ICON)
                        .add();
                }
            }
        },
    );
}

/// The richest form of the title bar that fits `room`.
fn form_for<A: 'static>(b: &mut B<'_, '_, A>, pane: &Pane, room: Vec2) -> Form {
    [Form::Full, Form::Compact]
        .into_iter()
        .find(|&form| b.max_content_width(|b| title_bar(b, pane, form)) <= Width::new(room.x))
        .unwrap_or(Form::Icons)
}

/// ```html
/// <section class="flex flex-col h-[140px] bg-card border border-border rounded-lg text-card-foreground">
///   <header class="w-full h-10">…title bar…</header>
///   <p class="p-3 text-muted-foreground">Quarterly revenue by region.</p>
/// </section>
/// ```
fn pane<A: 'static>(b: &mut B<'_, '_, A>, index: usize) {
    let t = common::tokens(b.ctx());
    let pane = &PANES[index];
    b.div_id(
        index,
        tw().flex()
            .flex_col()
            .h_px(PANE_HEIGHT)
            .bg(t.card)
            .text(t.card_foreground)
            .border(1.0)
            .border_color(t.border)
            .rounded_lg(),
        |b| {
            b.region(
                "title",
                tw().w_full().h_px(TITLE_BAR_HEIGHT).shrink_0(),
                move |b, room| {
                    let form = form_for(b, &PANES[index], room);
                    title_bar(b, &PANES[index], form);
                },
            );
            b.text(tw().p(3.0).text(t.muted_foreground), pane.body);
        },
    );
}

/// ```html
/// <div class="size-full overflow-y-auto">
///   <div class="grid grid-cols-3 gap-3 p-3"> <!-- as many columns as fit -->
///     <section>…</section> <!-- one per pane -->
///   </div>
/// </div>
/// ```
pub fn layout<A: 'static>(b: &mut B<'_, '_, A>, _: &()) {
    b.region("grid", tw().size_full(), |b, room| {
        b.scroll("panes", tw().size_full())
            .axis(Axis::Y)
            .children(|s| {
                s.div(
                    tw().grid()
                        .grid_cols(columns(room))
                        .gap(GAP_UNITS)
                        .p(GAP_UNITS),
                    |b| {
                        for index in 0..PANES.len() {
                            pane(b, index);
                        }
                    },
                );
            });
    });
}

fn main() -> eframe::Result {
    common::run("responsive", (), layout)
}
