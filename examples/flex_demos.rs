//! Port of egui_flex's "flex_demos": align, grow, basis, justify, nesting and
//! the alignment of an item's content, each in a flex container laid out in
//! the direction the controls choose.
//!
//! After the example in hello_egui (https://github.com/lucasmerlin/hello_egui,
//! MIT) by Lucas Meurer, rebuilt with bullseye rather than copied: taffy does
//! the flexbox, so `Flex`, `FlexItem` and its two-pass sizing are classes
//! here. The controls are bullseye widgets in the same tree (a segmented
//! control, a stock egui checkbox and a select), and the labels are text boxes
//! that egui_flex draws with `Label`.

#[path = "common/mod.rs"]
mod common;

use bullseye::prim::select::{self, Options};
use bullseye::widget::Checkbox;
use bullseye::{Builder, Tw, none, tw};
use egui::{Role, WidgetInfo};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// The direction of the demos' flex containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    /// `base` as a flex container in this direction.
    fn flex(self, base: Tw) -> Tw {
        match self {
            Self::Horizontal => base.flex().flex_row(),
            Self::Vertical => base.flex().flex_col(),
        }
    }
}

/// The values of `align-self`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

pub const ALIGNS: [Align; 4] = [Align::Start, Align::Center, Align::End, Align::Stretch];

impl Align {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Center => "Center",
            Self::End => "End",
            Self::Stretch => "Stretch",
        }
    }

    fn apply(self, item: Tw) -> Tw {
        match self {
            Self::Start => item.self_start(),
            Self::Center => item.self_center(),
            Self::End => item.self_end(),
            Self::Stretch => item.self_stretch(),
        }
    }
}

/// The values of `justify-content`.
/// A class applied to a container.
type Class = fn(Tw) -> Tw;

const JUSTIFIES: [(&str, Class); 6] = [
    ("Start", Tw::justify_start),
    ("Center", Tw::justify_center),
    ("End", Tw::justify_end),
    ("SpaceBetween", Tw::justify_between),
    ("SpaceAround", Tw::justify_around),
    ("SpaceEvenly", Tw::justify_evenly),
];

/// Where an item's content sits in the item: its name, and the classes that
/// put one child there, by justifying along the row and aligning across it.
type Anchor = (&'static str, Class);

pub const ANCHORS: [Anchor; 9] = [
    ("Left top", |t| t.justify_start().items_start()),
    ("Center top", |t| t.justify_center().items_start()),
    ("Right top", |t| t.justify_end().items_start()),
    ("Left center", |t| t.justify_start().items_center()),
    ("Center center", |t| t.justify_center().items_center()),
    ("Right center", |t| t.justify_end().items_center()),
    ("Left bottom", |t| t.justify_start().items_end()),
    ("Center bottom", |t| t.justify_center().items_end()),
    ("Right bottom", |t| t.justify_end().items_end()),
];

/// The identified nodes the tests name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Direction,
    Grow,
    Content,
    Aligned(Align),
    Growing(u8),
    Based(u8),
    SelfContent(Align),
}

pub struct State {
    pub direction: Direction,
    pub grow: bool,
    /// An index into [`ANCHORS`].
    pub anchor: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            direction: Direction::Horizontal,
            grow: false,
            anchor: 0,
        }
    }
}

/// The classes of a framed box, `Frame::group`.
///
/// ```html
/// <div class="p-1.5 border border-border rounded-md">…</div>
/// ```
fn framed<A>(b: &Builder<'_, '_, A>, base: Tw) -> Tw {
    base.p(1.5)
        .border(1.0)
        .border_color(common::tokens(b.ctx()).border)
        .rounded_md()
}

/// What `grow_items` gives every item of a container.
fn spread(state: &State, base: Tw) -> Tw {
    if state.grow { base.grow() } else { base }
}

/// A framed text box.
fn label<A>(b: &mut Builder<'_, '_, A>, class: Tw, text: &str) {
    b.div(framed(b, class), |b| b.text(tw(), text));
}

/// ```html
/// <div class="flex flex-row gap-2 …">
///   <h3 class="grow-0 text-[20px]">Align</h3>
///   <div class="self-start p-1.5 border … rounded-md grow">Start</div> <!-- Center, End, Stretch -->
///   <div class="p-1.5 border … rounded-md grow">Some bigger item<br>with some<br>more lines</div>
/// </div>
/// ```
fn align_group(b: &mut B<'_, '_>, state: &State) {
    for align in ALIGNS {
        b.div_id(
            Part::Aligned(align),
            align.apply(spread(state, framed(b, tw()))),
            none,
            |b| b.text(tw(), align.name()),
        );
    }
    label(
        b,
        spread(state, tw()),
        "Some bigger item\nwith some\nmore lines",
    );
}

/// ```html
/// <div class="flex flex-row gap-2 size-full">
///   <h3 class="grow-0">Grow</h3>
///   <div class="grow-0 … ">0</div> <div class="grow-1 …">1</div> <!-- 2, 3 -->
/// </div>
/// ```
fn grow_group(b: &mut B<'_, '_>) {
    for factor in 0..4u8 {
        b.div_id(
            Part::Growing(factor),
            framed(b, tw()).grow_by(f32::from(factor)),
            none,
            |b| b.text(tw(), factor.to_string()),
        );
    }
}

/// ```html
/// <div class="flex flex-row gap-2">
///   <h3 class="grow-0">Basis</h3>
///   <div class="basis-[0px] … ">0</div> <!-- 50, 100, 200 -->
/// </div>
/// ```
fn basis_group(b: &mut B<'_, '_>, state: &State) {
    for (index, basis) in [0.0, 50.0, 100.0, 200.0].into_iter().enumerate() {
        let index = u8::try_from(index).expect("four bases");
        b.div_id(
            Part::Based(index),
            spread(state, framed(b, tw())).basis_px(basis),
            none,
            |b| b.text(tw(), format!("{basis}")),
        );
    }
}

/// ```html
/// <div class="grow p-1.5 border … rounded-md flex flex-col items-stretch gap-2">
///   <div class="flex flex-row justify-start … p-1.5 border … rounded-md"><span>Start</span><span>two</span><span>three</span></div>
///   <!-- Center, End, SpaceBetween, SpaceAround, SpaceEvenly -->
/// </div>
/// ```
fn justify_group(b: &mut B<'_, '_>, state: &State) {
    let direction = state.direction;
    b.div(
        framed(b, direction.opposite().flex(tw()))
            .gap(2.0)
            .items_stretch()
            .grow(),
        |b| {
            for (name, justify) in JUSTIFIES {
                b.div(
                    spread(state, justify(framed(b, direction.flex(tw())).gap(2.0))),
                    |b| {
                        for text in [name, "two", "three"] {
                            b.text(tw(), text);
                        }
                    },
                );
            }
        },
    );
}

/// ```html
/// <div class="flex flex-row gap-2 …">
///   <h3>Nested</h3>
///   <div class="flex flex-col gap-2 p-1.5 border … rounded-md">
///     <div class="…">one</div>
///     <div class="flex flex-row gap-2 p-1.5 border … rounded-md"><div>two</div><div>three</div></div>
///   </div>
///   <div class="flex flex-col gap-2 p-1.5 border … rounded-md"><div>one</div><div>two</div><div>three</div></div>
/// </div>
/// ```
fn nested_group(b: &mut B<'_, '_>, state: &State) {
    let (demo, main) = (state.direction, state.direction.opposite());
    b.div(spread(state, framed(b, main.flex(tw())).gap(2.0)), |b| {
        label(b, spread(state, tw()), "one");
        b.div(spread(state, framed(b, demo.flex(tw())).gap(2.0)), |b| {
            label(b, spread(state, tw()), "two");
            label(b, spread(state, tw()), "three");
        });
    });
    b.div(spread(state, framed(b, main.flex(tw())).gap(2.0)), |b| {
        for text in ["one", "two", "three"] {
            label(b, spread(state, tw()), text);
        }
    });
}

/// ```html
/// <div class="flex flex-row gap-2 …">
///   <h3>Align Self Content</h3>
///   <select class="…">Left top …</select>
///   <div class="self-start flex justify-start items-start p-1.5 border … rounded-md">Start</div> <!-- Center, End, Stretch -->
///   <div class="…">Some bigger item<br>with some<br>more lines</div>
/// </div>
/// ```
fn content_group(b: &mut B<'_, '_>, state: &State) {
    let options = Options::new(ANCHORS.map(|(name, _)| name));
    let measured = options.clone();
    b.leaf(
        Part::Content,
        tw(),
        move |m| select::measure(m, &measured),
        move |state: &mut State, ui| {
            let response = select::show(ui, &mut state.anchor, &options);
            response.widget_info(|| {
                WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), "Align self content")
            });
        },
    );
    let (_, place) = ANCHORS[state.anchor];
    for align in ALIGNS {
        b.div_id(
            Part::SelfContent(align),
            align.apply(spread(state, place(framed(b, tw()).flex()))),
            none,
            |b| b.text(tw(), align.name()),
        );
    }
    label(
        b,
        spread(state, tw()),
        "Some bigger item\nwith some\nmore lines",
    );
}

/// One group of the demo: a frame with a heading and the group's items in a
/// flex container in the demo direction.
///
/// ```html
/// <div class="flex flex-col grow p-1.5 border border-border rounded-md">
///   <div class="flex flex-row gap-2 grow"><h3 class="grow-0 text-[20px]">Align</h3>…</div>
/// </div>
/// ```
fn group<'f>(b: &mut B<'_, 'f>, state: &State, title: &str, items: impl FnOnce(&mut B<'_, 'f>)) {
    b.div(framed(b, tw()).flex().flex_col().grow(), |b| {
        b.div(
            state
                .direction
                .flex(tw())
                .gap(2.0)
                .grow()
                .self_stretch()
                .items_start(),
            |b| {
                b.text(tw().grow_0().role(common::sized(20.0)), title);
                items(b);
            },
        );
    });
}

/// The controls: the demo direction, and whether the items grow.
///
/// ```html
/// <div class="flex items-center gap-2">
///   <span>Demo direction:</span>
///   <div class="segmented">Horizontal Vertical</div>
///   <label><input type="checkbox"> Grow</label>
/// </div>
/// ```
fn header(b: &mut B<'_, '_>, state: &State) {
    b.div(tw().flex().items_center().gap(2.0), |b| {
        b.text(tw(), "Demo direction:");
        common::choice(
            b,
            Part::Direction,
            state.direction,
            &[
                (Direction::Horizontal, "Horizontal"),
                (Direction::Vertical, "Vertical"),
            ],
            |s| &mut s.direction,
        );
        b.widget(
            Part::Grow,
            tw(),
            Checkbox::new("Grow"),
            |state, ui, grow| {
                grow.show(ui, &mut state.grow);
            },
        );
    });
}

/// ```html
/// <div class="flex flex-col gap-2 p-2 size-full">
///   <div class="flex items-center gap-2">…controls…</div>
///   <div class="flex flex-col flex-wrap content-start gap-2"> <!-- opposite of the demo direction -->
///     …the six groups…
///   </div>
/// </div>
/// ```
pub fn layout(b: &mut B<'_, '_>, state: &State) {
    b.div(tw().flex().flex_col().gap(2.0).p(2.0).size_full(), |b| {
        header(b, state);
        b.div(
            state
                .direction
                .opposite()
                .flex(tw())
                .flex_wrap()
                .content_start()
                .gap(2.0)
                .grow(),
            |b| {
                group(b, state, "Align", |b| align_group(b, state));
                group(b, state, "Grow", grow_group);
                group(b, state, "Basis", |b| basis_group(b, state));
                group(b, state, "Justify Content", |b| justify_group(b, state));
                group(b, state, "Nested", |b| nested_group(b, state));
                group(b, state, "Align Self Content", |b| content_group(b, state));
            },
        );
    });
}

fn main() -> eframe::Result {
    common::run_app("flex_demos", State::default(), layout)
}
