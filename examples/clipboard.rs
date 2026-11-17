//! The clipboard, read on demand: a Paste menu item and a Paste button, both
//! the one `Command::paste`, and the Ctrl/Cmd+V chord.
//!
//! Copy some text or an image in another program, then paste three ways: the
//! chord, the Edit menu's Paste, and the Paste button. The readout says what
//! arrived and which way: `chord` is the paste event the window system sent,
//! `click` is a read of the clipboard. Copy text and an image together to see
//! that the text wins. A failed read shows as an error line instead.
//!
//! Native only: the browser's clipboard read is asynchronous and a build for
//! the web answers `Unavailable`.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Chord, ClipboardError, Command, Commands, Paste, Via, tw};
use egui::{Color32, ColorImage, Key, Modifiers};

/// The parts the layout names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Page,
    Menu,
    Button,
    Readout,
    Via,
    Thumbnail,
    Error,
}

/// The command the menu item and the button run.
#[derive(Clone, Copy, Debug, Hash)]
pub enum Cmd {
    Paste,
}

/// What arrived, and the way it came.
pub struct Arrival {
    pub paste: Paste,
    pub via: Via,
}

#[derive(Default)]
pub struct State {
    pub arrived: Option<Arrival>,
    pub error: Option<ClipboardError>,
}

/// The longest side of the thumbnail, in cells.
const THUMBNAIL: usize = 32;

/// One cell of the thumbnail is this many points.
const CELL: f32 = 4.0;

fn commands() -> Commands<State> {
    Commands::new().add(
        Cmd::Paste,
        Command::paste("Paste", |state: &mut State, paste, via| {
            state.arrived = Some(Arrival { paste, via });
            state.error = None;
        })
        .shortcut(const { Chord::new(Modifiers::COMMAND, Key::V).expect("not reserved") })
        .on_error(|state, error| state.error = Some(error)),
    )
}

/// What the readout says arrived.
pub fn describe(paste: &Paste) -> String {
    match paste {
        Paste::Text(text) => format!("text, {} characters: {text}", text.chars().count()),
        Paste::Image(image) => format!("image, {} x {}", image.width(), image.height()),
        Paste::Nothing => "nothing: the clipboard is empty".to_owned(),
    }
}

/// The way the paste came.
pub fn way(via: Via) -> &'static str {
    match via {
        Via::Event => "chord (the paste event)",
        Via::Read => "click (a read of the clipboard)",
    }
}

/// `image` sampled down to at most [`THUMBNAIL`] cells on a side.
fn thumbnail(image: &ColorImage) -> (usize, usize, Vec<Color32>) {
    let step = image.width().max(image.height()).div_ceil(THUMBNAIL).max(1);
    let (w, h) = (image.width().div_ceil(step), image.height().div_ceil(step));
    let cells = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| image.pixels[y * step * image.width() + x * step])
        .collect();
    (w, h, cells)
}

/// ```html
/// <div class="flex flex-col gap-3 p-4">
///   <div class="flex gap-2"> <button>Edit</button> <button>Paste</button> </div>
///   <p>Arrived: text, 5 characters: hello</p>
///   <p>Via: click (a read of the clipboard)</p>
///   <p class="text-destructive">the clipboard is not available</p>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.commands(&commands(), state);
    let t = common::tokens(b.ctx());
    b.div_id(Part::Page, tw().flex().flex_col().gap(3.0).p(4.0), |s| {
        s.div(tw().flex().gap(2.0).items_center(), |s| {
            s.dropdown_menu(Part::Menu, "Edit")
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .items(|m| m.command(Cmd::Paste));
            s.button(Part::Button, "Paste")
                .variant(eggshell_ui_shadcn::variant::OUTLINE)
                .command(Cmd::Paste);
        });
        match &state.arrived {
            None => {
                s.span(
                    Part::Readout,
                    tw().text(t.muted_foreground),
                    "Nothing pasted yet.",
                );
            }
            Some(Arrival { paste, via }) => {
                s.span(Part::Readout, tw(), format!("Arrived: {}", describe(paste)));
                s.span(Part::Via, tw(), format!("Via: {}", way(*via)));
                if let Paste::Image(image) = paste {
                    let (w, h, cells) = thumbnail(image);
                    s.custom(
                        Part::Thumbnail,
                        tw().w_px(w as f32 * CELL).h_px(h as f32 * CELL),
                    )
                    .ui(move |_, ui| {
                        let origin = ui.max_rect().min;
                        for (at, colour) in cells.iter().enumerate() {
                            let (x, y) = ((at % w) as f32, (at / w) as f32);
                            let cell = egui::Rect::from_min_size(
                                origin + egui::vec2(x * CELL, y * CELL),
                                egui::vec2(CELL, CELL),
                            );
                            ui.painter().rect_filled(cell, 0.0, *colour);
                        }
                    });
                }
            }
        }
        if let Some(error) = &state.error {
            s.span(Part::Error, tw().text(t.destructive), error.to_string());
        }
    });
}

fn main() -> eframe::Result {
    common::run_app("clipboard", State::default(), layout)
}
