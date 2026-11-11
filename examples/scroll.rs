//! Scroll containers: vertical, horizontal, both ways, and nested ones on the
//! same axis and on crossed axes.
//!
//! Each pane is `b.scroll(key, classes).axis(..).children(..)`. The wheel
//! scrolls the innermost container under the pointer that can still move, then
//! the one around it. The nested pane holds a horizontal carousel and a
//! vertical list inside a vertical page: the carousel takes horizontal
//! gestures and leaves vertical ones to the page. Every container reserves a
//! gutter for its bar and draws the bar only when its content overflows; one
//! pane gives its bar a style of its own. The buttons above the first
//! pane use the programmatic API.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::scroll::{self, Axis, EdgeFade};
use eggshell_ui::{Builder, Opacity, Tw, tw};
use eggshell_ui_shadcn::Tokens;
use egui::{Align, Rect, vec2};

type B<'b, 'f> = Builder<'b, 'f, State>;

const ROWS: u32 = 40;
const CARDS: u32 = 14;
const COLUMNS: u32 = 12;
const SHOWN_ROW: u32 = 30;

/// The scroll containers and the nodes the buttons name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Vertical,
    Horizontal,
    Both,
    Styled,
    Nested,
    Carousel,
    List,
    Row(u32),
    Card(u32),
    Cell(u32, u32),
    Top,
    Jump,
    Down,
}

#[derive(Default)]
pub struct State {
    pub clicked: Option<u32>,
}

/// A titled column that takes its share of the width.
///
/// ```html
/// <div class="flex flex-col gap-2 grow basis-0 min-w-0 min-h-0">
///   <h2 class="text-muted-foreground">title</h2>
///   <div class="flex flex-col grow min-h-0 rounded-md border bg-card">…</div>
/// </div>
/// ```
fn pane(b: &mut B<'_, '_>, title: &str, body: impl FnOnce(&mut B<'_, '_>)) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .gap(2.0)
            .grow()
            .basis_px(0.0)
            .min_w_0()
            .min_h_0(),
        |b| {
            b.text(tw().text(t.muted_foreground), title);
            b.div(
                tw().flex()
                    .flex_col()
                    .grow()
                    .min_h_0()
                    .rounded_md()
                    .border(1.0)
                    .border_color(t.border)
                    .bg(t.card),
                body,
            );
        },
    );
}

fn item(t: &Tokens) -> Tw {
    tw().flex()
        .items_center()
        .px(3.0)
        .py(2.0)
        .rounded_md()
        .bg(t.muted)
        .text(t.foreground)
}

/// ```html
/// <div class="grow min-h-0 overflow-y-auto p-2">
///   <div class="flex flex-col gap-1">  <!-- rows: px-3 py-2 rounded-md bg-muted -->
/// </div>
/// ```
fn vertical(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Vertical, tw().grow().min_h_0().p(2.0))
        .children(|s| {
            s.div(tw().flex().flex_col().gap(1.0), |s| {
                for n in 0..ROWS {
                    s.div_id(Part::Row(n), item(&t).sense_click(), |s| {
                        s.text(tw(), format!("Row {n}"));
                    })
                    .on_click(move |app: &mut State| app.clicked = Some(n));
                }
            });
        });
}

/// ```html
/// <div class="grow overflow-x-auto p-2">
///   <div class="flex gap-2 h-full">  <!-- cards: w-[120px] shrink-0 -->
/// </div>
/// ```
fn horizontal(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Horizontal, tw().grow().p(2.0))
        .axis(Axis::X)
        .children(|s| {
            s.div(tw().flex().gap(2.0).h_full(), |s| {
                for n in 0..CARDS {
                    s.div_id(
                        Part::Card(n),
                        item(&t).w_px(120.0).shrink_0().justify_center(),
                        |s| s.text(tw(), format!("Card {n}")),
                    );
                }
            });
        });
}

/// ```html
/// <div class="grow overflow-auto p-2">
///   <div class="flex flex-col gap-1">  <!-- 24 rows of 12 cells, w-24 shrink-0 -->
/// </div>
/// ```
fn both(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    b.scroll(Part::Both, tw().grow().p(2.0))
        .axis(Axis::Both)
        .children(|s| {
            s.div(tw().flex().flex_col().gap(1.0), |s| {
                for row in 0..24 {
                    s.div_id(Part::Row(row), tw().flex().gap(1.0), |s| {
                        for column in 0..COLUMNS {
                            s.div_id(
                                Part::Cell(row, column),
                                item(&t).w_px(96.0).shrink_0().justify_center(),
                                |s| s.text(tw(), format!("{row},{column}")),
                            );
                        }
                    });
                }
            });
        });
}

/// A vertical list with a bar of its own: wider, with a coloured thumb and a
/// track. The container reserves the style's width, whatever it is.
///
/// ```html
/// <div class="grow min-h-0 overflow-y-auto p-2" style="scrollbar-width: 16px; scrollbar-color: primary muted">…</div>
/// ```
fn styled(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    let mut bar = eggshell_ui::Style::of(b.ctx()).scroll;
    bar.width = 16.0;
    bar.thumb = t.primary;
    bar.thumb_hover = t.primary.gamma_multiply(0.8);
    bar.thumb_active = t.primary.gamma_multiply(0.6);
    bar.track = t.muted;
    b.scroll(Part::Styled, tw().grow().min_h_0().p(2.0))
        .scrollbar_style(bar)
        .children(|s| {
            s.div(tw().flex().flex_col().gap(1.0), |s| {
                for n in 0..ROWS {
                    s.div_id(Part::Row(n), item(&t), |s| {
                        s.text(tw(), format!("Row {n}"));
                    });
                }
            });
        });
}

/// A vertical page with a horizontal carousel and a vertical list in it.
///
/// ```html
/// <div class="grow min-h-0 overflow-y-auto p-2">
///   <p>…</p> <div class="h-[88px] overflow-x-auto">carousel</div> <p>…</p>
///   <div class="h-[120px] overflow-y-auto">list</div> <p>…</p>
/// </div>
/// ```
fn nested(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    let note = |b: &mut B<'_, '_>, text: String| {
        b.text(tw().text(t.muted_foreground).py(2.0), text);
    };
    b.scroll(Part::Nested, tw().grow().min_h_0().p(2.0))
        .children(|s| {
            note(
                s,
                "A horizontal carousel in a vertical page: horizontal gestures move it.".into(),
            );
            s.scroll(
                Part::Carousel,
                tw().h_px(88.0).rounded_md().bg(t.muted).p(2.0),
            )
            .axis(Axis::X)
            .fade_edges(EdgeFade::new(Opacity::new(0.5), 20.0))
            .children(|s| {
                s.div(tw().flex().gap(2.0).h_full(), |s| {
                    for n in 0..CARDS {
                        s.div_id(
                            Part::Card(n),
                            tw().flex()
                                .items_center()
                                .justify_center()
                                .w_px(100.0)
                                .shrink_0()
                                .rounded_md()
                                .bg(t.background),
                            |s| s.text(tw(), format!("Slide {n}")),
                        );
                    }
                });
            });
            note(
                s,
                "A vertical list in a vertical page: the list moves first.".into(),
            );
            s.scroll(Part::List, tw().h_px(120.0).rounded_md().bg(t.muted).p(2.0))
                .children(|s| {
                    s.div(tw().flex().flex_col().gap(1.0), |s| {
                        for n in 0..ROWS {
                            s.div_id(Part::Row(n), item(&t).bg(t.background), |s| {
                                s.text(tw(), format!("Inner row {n}"));
                            });
                        }
                    });
                });
            for n in 0..10 {
                note(s, format!("Paragraph {n} of the page."));
            }
        });
}

/// Buttons that scroll the vertical pane from outside it.
///
/// ```html
/// <div class="flex items-center gap-2"><span>Vertical pane:</span> <button>Top</button> …</div>
/// ```
fn jumps(b: &mut B<'_, '_>) {
    let t = common::tokens(b.ctx());
    let pane = b.id_of(Part::Vertical);
    let mut row = None;
    b.scope(Part::Vertical, |s| {
        row = Some(s.id_of(Part::Row(SHOWN_ROW)))
    });
    let row = row.expect("scope ran");
    b.div(tw().flex().gap(2.0).items_center(), |b| {
        b.text(tw().text(t.muted_foreground), "Vertical pane:");
        b.button(Part::Top, "Top")
            .on_response(move |_, response| {
                if response.clicked() {
                    let top = Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0));
                    scroll::scroll_to_rect(&response.ctx, pane, top, Some(Align::Min));
                }
            })
            .add();
        b.button(Part::Jump, format!("Show row {SHOWN_ROW}"))
            .on_response(move |_, response| {
                if response.clicked() {
                    scroll::scroll_into_view(&response.ctx, row, Some(Align::Center));
                }
            })
            .add();
        b.button(Part::Down, "Down 200")
            .on_response(move |_, response| {
                if response.clicked() {
                    scroll::scroll_by(&response.ctx, pane, vec2(0.0, 200.0));
                }
            })
            .add();
    });
}

/// ```html
/// <div class="flex flex-col gap-3 size-full p-4">
///   <div class="flex items-center gap-2">buttons</div>
///   <div class="flex gap-4 grow min-h-0">four panes</div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    b.div(
        tw().flex()
            .flex_col()
            .gap(3.0)
            .size_full()
            .p(4.0)
            .text(t.foreground),
        |b| {
            jumps(b);
            b.text(
                tw().text(t.muted_foreground),
                match state.clicked {
                    Some(n) => format!("Clicked row {n}"),
                    None => "Click a row".to_owned(),
                },
            );
            b.div(tw().flex().gap(4.0).grow().min_h_0(), |b| {
                pane(b, "Vertical", vertical);
                pane(b, "Horizontal", horizontal);
                pane(b, "Both", both);
                pane(b, "Custom bar style", styled);
                pane(b, "Nested", nested);
            });
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("scroll", State::default(), layout)
}
