//! Port of egui_taffy's "Overflow demo": four columns of labels in bordered,
//! padded nodes of a fixed height, one for each `overflow-y`: visible, clip,
//! hidden and scroll.
//!
//! After the demo in egui_taffy (https://github.com/PPakalns/egui_taffy, MIT),
//! rebuilt with eggshell: the first three are classes (`overflow-visible`,
//! `overflow-clip`, `overflow-hidden`) and the last a scroll container, whose
//! padding scrolls with its content and whose borders stay put. The original
//! sizes the columns by their labels; here each is 140 points wide. The count is a slider in the same tree.

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::scroll::Axis;
use eggshell_ui::{Builder, Tw, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How a column treats what does not fit it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Overflow {
    Visible,
    Clip,
    Hidden,
    Scroll,
}

impl Overflow {
    const ALL: [Self; 4] = [Self::Visible, Self::Clip, Self::Hidden, Self::Scroll];

    fn classes(self, base: Tw) -> Tw {
        match self {
            Self::Visible => base.overflow_y_visible(),
            Self::Clip => base.overflow_y_clip(),
            Self::Hidden => base.overflow_y_hidden(),
            Self::Scroll => base,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Count,
    Column(Overflow),
}

pub struct State {
    pub count: usize,
}

impl Default for State {
    fn default() -> Self {
        Self { count: 20 }
    }
}

/// The box every column shares: 200 tall, padded by 6, with borders of 20 and
/// 40 points so that where the content is clipped is visible.
///
/// ```html
/// <div class="flex flex-col h-[200px] w-[140px] p-1.5 border-l-[20px] border-r-[40px]
///             border-t-[20px] border-b-[40px] border-border bg-muted">…</div>
/// ```
fn column(b: &B<'_, '_>) -> Tw {
    let t = common::tokens(b.ctx());
    tw().flex()
        .flex_col()
        .h_px(200.0)
        .w_px(140.0)
        .p(1.5)
        .border_l(20.0)
        .border_r(40.0)
        .border_t(20.0)
        .border_b(40.0)
        .border_color(t.border)
        .bg(t.muted)
        .text(t.foreground)
}

/// ```html
/// <div class="flex flex-col gap-4 p-4">
///   <h2>UI nodes with non-zero borders and paddings with overflow-y</h2>
///   <div class="flex items-center gap-2">Count <input type="range" min="0" max="50"></div>
///   <div class="flex items-center gap-4"> four columns </div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    let t = common::tokens(b.ctx());
    let count = state.count;
    b.div(
        tw().flex().flex_col().gap(4.0).p(4.0).text(t.foreground),
        |b| {
            b.text(
                tw().role(common::sized(16.0)),
                "UI nodes with non-zero borders and paddings with overflow-y",
            );
            b.div(tw().flex().items_center().gap(2.0), |b| {
                b.text(tw(), "Count");
                b.slider(Part::Count, count as f64, 0.0..=50.0)
                    .named("Count")
                    .classes(tw().w_px(240.0))
                    .on_change(|app: &mut State, value| app.count = value.round() as usize);
                b.text(tw().text(t.muted_foreground), count.to_string());
            });
            b.div(tw().flex().items_center().gap(4.0), |b| {
                for overflow in Overflow::ALL {
                    let classes = overflow.classes(column(b));
                    let label = format!("{overflow:?}");
                    match overflow {
                        Overflow::Scroll => {
                            b.scroll(Part::Column(overflow), classes)
                                .axis(Axis::Y)
                                .children(|s| {
                                    s.div(tw().flex().flex_col(), |s| {
                                        for _ in 0..count {
                                            s.text(tw(), label.clone());
                                        }
                                    });
                                });
                        }
                        _ => {
                            b.div_id(Part::Column(overflow), classes, |s| {
                                for _ in 0..count {
                                    s.text(tw(), label.clone());
                                }
                            });
                        }
                    }
                }
            });
        },
    );
}

fn main() -> eframe::Result {
    common::run_app("overflow", State::default(), layout)
}
