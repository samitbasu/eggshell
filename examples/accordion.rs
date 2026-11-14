//! An accordion: three sections, one open at a time, over a list.
//!
//! Click a header, or focus it and press Enter or Space, to open its section
//! and close the open one; the chevron turns with the section. The sections
//! open and close by their height over 200 ms, ease-out, and the list below
//! moves with them: layout is what animates, so nothing is scaled and no text
//! stretches. Toggle a section while it moves and it turns round from where it
//! is.
//!
//! F12 toggles the layout debug overlay and its frame-timing readout.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::overlay::Shown;
use eggshell_ui::{Builder, tw};
use egui::{Color32, Role};

#[derive(Clone, Copy, Debug, Hash)]
pub enum Part {
    Page,
    Accordion,
    Item(usize),
    Head(usize),
    Body(usize),
    List,
    Row(usize),
}

pub const SECTIONS: [(&str, &str); 3] = [
    (
        "Is it accessible?",
        "Yes. Every box is a real node, so the accessibility tree is the layout tree.",
    ),
    (
        "Is it animated?",
        "Yes. A section is as tall as its content wants times how far it has opened, so what follows it moves with it.",
    ),
    (
        "Does it stretch the text?",
        "No. The content keeps its own size and is clipped, so text never reflows while a section moves.",
    ),
];

pub const ROWS: usize = 4;

#[derive(Default)]
pub struct State {
    pub open: Option<usize>,
}

impl State {
    pub fn toggle(&mut self, section: usize) {
        self.open = if self.open == Some(section) {
            None
        } else {
            Some(section)
        };
    }
}

/// ```html
/// <div class="flex w-[360px] flex-col gap-4 p-4">
///   <div class="flex flex-col"> <!-- per section -->
///     <button>Title</button>
///     <div class="overflow-hidden"> <p class="px-2 pb-3">Body</p> </div>
///   </div>
///   <div class="flex flex-col gap-1"> <div>Row 1</div> … </div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut Builder<'_, 'f, State>, state: &'f State) {
    b.div_id(
        Part::Page,
        tw().flex().flex_col().gap(4.0).w_px(360.0).p(4.0),
        |s| {
            s.div_id(Part::Accordion, tw().flex().flex_col(), |s| {
                for (index, (title, body)) in SECTIONS.into_iter().enumerate() {
                    s.div_id(Part::Item(index), tw().flex().flex_col(), |s| {
                        let open = state.open == Some(index);
                        let muted = eggshell_ui_shadcn::tokens(s.ctx())
                            .map_or(Color32::GRAY, |tokens| tokens.muted_foreground);
                        s.collapsible(Part::Body(index), Shown::from(open))
                            .content(tw().px(2.0).pb(3.0))
                            .children_with_header(
                                |s, progress| {
                                    s.div_id(
                                        Part::Head(index),
                                        tw().sense_click()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px(2.0)
                                            .py(3.0),
                                        |s| {
                                            s.text(tw(), title);
                                            s.paint(
                                                tw().size_px(16.0).shrink_0(),
                                                common::chevron(muted, 180.0 * f32::from(progress)),
                                            );
                                        },
                                    )
                                    .role(Role::Button)
                                    .named(title)
                                    .expanded(open)
                                    .on_click(move |state: &mut State| state.toggle(index));
                                },
                                |s| s.text(tw(), body),
                            );
                    });
                }
            });
            s.div_id(Part::List, tw().flex().flex_col().gap(1.0), |s| {
                for row in 0..ROWS {
                    s.div_id(Part::Row(row), tw().px(2.0).py(1.0), |s| {
                        s.text(tw(), format!("Row {}", row + 1));
                    });
                }
            });
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("accordion", State::default(), layout)
}
