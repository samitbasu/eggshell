//! A colour-swatch picker: a popover whose swatches the arrow keys move
//! between as they are laid out (`Nav::Grid`).
//!
//! Click the trigger, or press Enter or Space on it. The popover opens with
//! focus on the current colour. The arrows move to the nearest swatch in
//! their direction and stop at the edges: Left from the grid's first column
//! crosses the rule to "No colour". Home and End go to the first and last
//! swatch, and Enter or Space picks the focused one. Click away or press Esc
//! to close.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::overlay::{Anchor, Distance, Nav, Placement};
use eggshell_ui::{Builder, Scope, tw};
use egui::{Color32, Role};

/// The grid's columns.
pub const COLUMNS: usize = 4;

/// The size of a swatch.
const SWATCH: f32 = 24.0;

/// The colours of the grid, in reading order: Tailwind's 500 shades.
pub const COLOURS: [(&str, Color32); 12] = [
    ("Red", Color32::from_rgb(0xef, 0x44, 0x44)),
    ("Orange", Color32::from_rgb(0xf9, 0x73, 0x16)),
    ("Amber", Color32::from_rgb(0xf5, 0x9e, 0x0b)),
    ("Yellow", Color32::from_rgb(0xea, 0xb3, 0x08)),
    ("Lime", Color32::from_rgb(0x84, 0xcc, 0x16)),
    ("Green", Color32::from_rgb(0x22, 0xc5, 0x5e)),
    ("Teal", Color32::from_rgb(0x14, 0xb8, 0xa6)),
    ("Sky", Color32::from_rgb(0x0e, 0xa5, 0xe9)),
    ("Blue", Color32::from_rgb(0x3b, 0x82, 0xf6)),
    ("Violet", Color32::from_rgb(0x8b, 0x5c, 0xf6)),
    ("Pink", Color32::from_rgb(0xec, 0x48, 0x99)),
    ("Stone", Color32::from_rgb(0x78, 0x71, 0x6c)),
];

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Trigger,
    Picker,
    Box,
    Rule,
    Grid,
    Row(usize),
    Swatch(Choice),
}

/// What the picker can choose.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum Choice {
    #[default]
    NoColour,
    /// An index into [`COLOURS`].
    Colour(usize),
}

impl Choice {
    pub fn name(self) -> &'static str {
        match self {
            Self::NoColour => "No colour",
            Self::Colour(at) => COLOURS[at].0,
        }
    }

    fn colour(self) -> Option<Color32> {
        match self {
            Self::NoColour => None,
            Self::Colour(at) => Some(COLOURS[at].1),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub choice: Choice,
    pub open: bool,
}

impl State {
    pub fn with(choice: Choice) -> Self {
        Self {
            choice,
            open: false,
        }
    }
}

/// ```html
/// <div class="flex items-center gap-3 p-4">
///   <button>Colour</button>
///   <span>Blue</span>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.div_id(
        Part::Page,
        tw().flex().items_center().gap(3.0).p(4.0),
        |s| {
            s.button(Part::Trigger, "Colour")
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .on_click(|state: &mut State| state.open = !state.open);
            s.text(tw(), state.choice.name());
            if state.open {
                picker(s, state.choice);
            }
        },
    );
}

/// ```html
/// <div role="grid" class="flex gap-2 rounded-md border bg-popover p-2">
///   <div role="row"> <div role="gridcell">No colour</div> </div>
///   <div class="w-px self-stretch bg-border"></div>
///   <div class="flex flex-col gap-1.5">
///     <div role="row" class="flex gap-1.5"> four gridcells </div> ×3
///   </div>
/// </div>
/// ```
fn picker(s: &mut Scope<'_, '_, State>, current: Choice) {
    let anchor = s.id_of(Part::Trigger);
    let t = common::tokens(s.ctx());
    let popup = common::popup_box(s);
    s.overlay(Part::Picker)
        .anchor(Anchor::Node(anchor))
        .placement(Placement::below().gap(Distance::new(4.0)))
        .nav(Nav::Grid)
        .role(Role::Grid)
        .named("Colour")
        .on_dismiss(|state: &mut State| state.open = false)
        .children(|s| {
            s.div_id(Part::Box, popup.flex_row().gap(2.0).p(2.0), |s| {
                row(s, 0, |s| swatch(s, Choice::NoColour, current));
                s.div_id(
                    Part::Rule,
                    tw().w_px(1.0).self_stretch().bg(t.border),
                    |_| {},
                );
                s.div_id(Part::Grid, tw().flex().flex_col().gap(1.5), |s| {
                    for (at, colours) in COLOURS.chunks(COLUMNS).enumerate() {
                        row(s, at + 1, |s| {
                            for offset in 0..colours.len() {
                                swatch(s, Choice::Colour(at * COLUMNS + offset), current);
                            }
                        });
                    }
                });
            });
        });
}

fn row(s: &mut Scope<'_, '_, State>, at: usize, cells: impl FnOnce(&mut Scope<'_, '_, State>)) {
    s.div_id(
        Part::Row(at),
        tw().flex().gap(1.5).sense_hover().focusable_none(),
        cells,
    )
    .role(Role::Row);
}

/// ```html
/// <div role="gridcell" class="size-6 rounded-md border focus-visible:ring-2"></div>
/// ```
fn swatch(s: &mut Scope<'_, '_, State>, choice: Choice, current: Choice) {
    let t = common::tokens(s.ctx());
    let fill = choice.colour().unwrap_or(t.background);
    let chosen = choice == current;
    let border = if chosen { t.foreground } else { t.border };
    let cell = s
        .div_id(
            Part::Swatch(choice),
            tw().size_px(SWATCH)
                .rounded_md()
                .bg(fill)
                .border(if chosen { 2.0 } else { 1.0 })
                .border_color(border)
                .sense_click()
                .focus(|p| p.ring(2.0, t.ring)),
            |s| {
                if choice == Choice::NoColour {
                    s.paint(tw().size_full(), no_colour(t.destructive));
                }
            },
        )
        .role(Role::GridCell)
        .named(choice.name());
    let cell = if chosen { cell.focus_on_open() } else { cell };
    cell.on_click(move |state: &mut State| {
        state.choice = choice;
        state.open = false;
    });
}

/// A diagonal stroke across the swatch: no colour.
fn no_colour(colour: Color32) -> impl Fn(&egui::Painter, egui::Rect, Color32) {
    move |painter, rect, _| {
        let rect = rect.shrink(4.0);
        painter.line_segment(
            [rect.left_bottom(), rect.right_top()],
            egui::Stroke::new(1.5, colour),
        );
    }
}

fn main() -> eframe::Result {
    common::run_app("swatch_picker", State::default(), layout)
}
