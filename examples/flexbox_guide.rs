//! A tour of flexbox after "A Complete Guide to Flexbox" on CSS-Tricks.
//!
//! Rebuilt with eggshell (taffy places, egui draws) from the guide's
//! descriptions: perfect centering, a fluid six-item grid, and one demo per
//! container and item property over a row of navigation links. The selector is
//! a list of clickable nodes and a segmented control inside the tree.
//!
//! Taffy has no `order` property: the "order" demo builds the children in the
//! desired order instead. The guide's responsive navigation and three-column
//! layout belong to M7.
//! Guide: https://css-tricks.com/snippets/css/a-guide-to-flexbox/

#[path = "common/mod.rs"]
mod common;

use common::{mono, sized};
use eggshell_ui::owned::Cell;
use eggshell_ui::{Builder, Tw, tw};
use egui::Color32;

type B<'b, 'f, A> = Builder<'b, 'f, A>;

/// A class applied to the container of a demo.
type Class = fn(Tw) -> Tw;

/// A class applied to the link at an index.
type ItemClass = fn(Tw, usize) -> Tw;

/// Builds a demo for the selected value.
type Build = Box<dyn Fn(&mut B<'_, '_, State>, usize)>;

/// One demo: a title, the values the selector offers, and how to build it for one.
pub struct Demo {
    pub title: &'static str,
    pub choices: Vec<&'static str>,
    build: Build,
}

impl Demo {
    fn fixed(title: &'static str, layout: fn(&mut B<'_, '_, State>)) -> Self {
        Self {
            title,
            choices: Vec::new(),
            build: Box::new(move |b, _| layout(b)),
        }
    }

    fn choosing<T: Copy + 'static>(
        title: &'static str,
        table: &'static [(&'static str, T)],
        layout: fn(&mut B<'_, '_, State>, T),
    ) -> Self {
        Self {
            title,
            choices: table.iter().map(|(name, _)| *name).collect(),
            build: Box::new(move |b, choice| layout(b, table[choice].1)),
        }
    }
}

/// The demos, the selected one and its selected value.
pub struct State {
    pub demos: Vec<Demo>,
    pub demo: usize,
    pub choice: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            demos: vec![
                Demo::fixed("Perfect centering", centering),
                Demo::fixed("Six items, evenly distributed", six_items),
                Demo::choosing("flex-direction", &DIRECTIONS, flex_direction),
                Demo::choosing("flex-wrap", &WRAPS, flex_wrap),
                Demo::choosing("gap", &GAPS, gap),
                Demo::choosing("justify-content", &JUSTIFIES, justify_content),
                Demo::choosing("align-items", &ALIGN_ITEMS, align_items),
                Demo::choosing("align-content", &ALIGN_CONTENT, align_content),
                Demo::choosing("flex-grow", &GROWS, flex_grow),
                Demo::choosing("flex-shrink", &SHRINKS, flex_shrink),
                Demo::choosing("flex-basis", &BASES, flex_basis),
                Demo::choosing("align-self", &ALIGN_SELVES, align_self),
                Demo::choosing("order", &ORDERS, order),
            ],
            demo: 0,
            choice: 0,
        }
    }
}

impl State {
    /// The state with the demo called `title` showing its `choice`th value.
    pub fn showing(title: &str, choice: usize) -> Self {
        let state = Self::default();
        let demo = state
            .demos
            .iter()
            .position(|demo| demo.title == title)
            .expect("the guide has this demo");
        Self {
            demo,
            choice,
            ..state
        }
    }
}

/// The demos as a column of rows, the selected one pressed, and the values
/// of that demo as a segmented control. Choosing a demo shows its first value.
///
/// ```html
/// <aside class="flex flex-col gap-3 p-3 w-[240px]">
///   <div class="flex flex-col gap-0.5">
///     <span class="px-2.5 py-1 rounded bg-[#aac4e8]">Perfect centering</span>
///     <span class="px-2.5 py-1 rounded hover:bg-[#e4e4e4]">flex-direction</span>
///     <!-- one per demo -->
///   </div>
///   <div class="flex flex-wrap gap-1 p-0.5 border border-[#b0b0b0] rounded-md">…</div>
/// </aside>
/// ```
fn controls(b: &mut B<'_, '_, State>, state: &State) {
    b.div(tw().flex().flex_col().gap(0.5), |b| {
        let t = common::tokens(b.ctx());
        for (index, demo) in state.demos.iter().enumerate() {
            let cell = if index == state.demo {
                Cell::Pressed
            } else {
                Cell::Released
            };
            b.span(
                demo.title,
                common::menu_item(&t, cell).sense_click(),
                demo.title,
            )
            .on_click(move |state| (state.demo, state.choice) = (index, 0));
        }
    });
    let choices = &state.demos[state.demo].choices;
    if !choices.is_empty() {
        let options: Vec<_> = choices.iter().copied().enumerate().collect();
        common::choice(b, "choices", state.choice, &options, |s| &mut s.choice);
    }
}

/// The selected demo under its title.
///
/// ```html
/// <div class="flex flex-col gap-4 p-4 size-full">
///   <h2 class="font-mono text-[14px]/[17.5px]">flex-direction: row</h2>
///   <!-- the demo -->
/// </div>
/// ```
pub fn guide(b: &mut B<'_, '_, State>, state: &State) {
    let demo = &state.demos[state.demo];
    let title = match demo.choices.get(state.choice) {
        Some(choice) => format!("{}: {choice}", demo.title),
        None => demo.title.to_owned(),
    };
    b.div(tw().flex().flex_col().gap(4.0).p(4.0).size_full(), |b| {
        b.text(tw().role(mono(14.0)), title);
        (demo.build)(b, state.choice);
    });
}

const LINKS: [&str; 5] = ["Home", "About", "Pricing", "Blog", "Contact"];

/// ```html
/// <nav class="flex gap-2 p-2 bg-[#dcdcdc] border border-[#a0a0a0] rounded-md"> ... </nav>
/// ```
fn bar() -> Tw {
    tw().flex()
        .gap(2.0)
        .p(2.0)
        .bg(Color32::LIGHT_GRAY)
        .border(1.0)
        .border_color(Color32::GRAY)
        .rounded_md()
}

/// ```html
/// <a class="px-3 py-2 bg-[#00008b] text-white rounded">Home</a>
/// ```
fn link() -> Tw {
    tw().px(3.0)
        .py(2.0)
        .bg(Color32::DARK_BLUE)
        .text(Color32::WHITE)
        .rounded()
}

/// ```html
/// <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
/// ```
fn fixed_link() -> Tw {
    link().w_px(100.0).flex_none().text_center()
}

fn links<A>(b: &mut B<'_, '_, A>, style: impl Fn(usize) -> Tw) {
    for (index, label) in LINKS.into_iter().enumerate() {
        b.div(style(index), |b| b.text(tw(), label));
    }
}

/// A child centred on both axes by auto margins.
///
/// ```html
/// <div class="flex h-[300px] bg-[#dcdcdc] border border-[#a0a0a0] rounded-md">
///   <div class="m-auto size-[100px] bg-[#00008b] text-white text-center">m-auto</div>
/// </div>
/// ```
pub fn centering<A>(b: &mut B<'_, '_, A>) {
    b.div(bar().h_px(300.0), |b| {
        b.div(
            tw().m_auto()
                .size_px(100.0)
                .bg(Color32::DARK_BLUE)
                .text(Color32::WHITE)
                .text_center(),
            |b| b.text(tw(), "m-auto"),
        );
    });
}

/// Six fixed-size items that spread and wrap as the window resizes.
///
/// ```html
/// <div class="flex flex-row flex-wrap justify-around gap-2 p-2 bg-[#dcdcdc] border border-[#a0a0a0] rounded-md">
///   <div class="size-[100px] my-2 bg-[#00008b] text-white text-center">1</div>
///   <!-- 2 to 6 -->
/// </div>
/// ```
pub fn six_items<A>(b: &mut B<'_, '_, A>) {
    b.div(bar().flex_row().flex_wrap().justify_around(), |b| {
        for number in 1..=6 {
            b.div(
                tw().size_px(100.0)
                    .my(2.0)
                    .bg(Color32::DARK_BLUE)
                    .text(Color32::WHITE)
                    .text_center(),
                |b| b.text(tw(), number.to_string()),
            );
        }
    });
}

static DIRECTIONS: [(&str, Class); 4] = [
    ("row", |t| t.flex_row()),
    ("row-reverse", |t| t.flex_row_reverse()),
    ("column", |t| t.flex_col()),
    ("column-reverse", |t| t.flex_col_reverse()),
];

/// ```html
/// <nav class="flex gap-2 p-2 ... items-start h-[260px] flex-row"> <!-- or flex-row-reverse, flex-col, flex-col-reverse -->
///   <a class="px-3 py-2 bg-[#00008b] text-white rounded">Home</a>
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn flex_direction<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().items_start().h_px(260.0)), |b| {
        links(b, |_| link())
    });
}

static WRAPS: [(&str, Class); 3] = [
    ("nowrap", |t| t.flex_nowrap()),
    ("wrap", |t| t.flex_wrap()),
    ("wrap-reverse", |t| t.flex_wrap_reverse()),
];

/// ```html
/// <nav class="flex gap-2 p-2 ... items-start w-[340px] flex-nowrap"> <!-- or flex-wrap, flex-wrap-reverse -->
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn flex_wrap<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().items_start().w_px(340.0).h_px(200.0)), |b| {
        links(b, |_| fixed_link())
    });
}

static GAPS: [(&str, Class); 4] = [
    ("gap-0", |t| t.gap(0.0)),
    ("gap-2", |t| t.gap(2.0)),
    ("gap-6", |t| t.gap(6.0)),
    ("gap-x-8 gap-y-1", |t| t.gap_x(8.0).gap_y(1.0)),
];

/// ```html
/// <nav class="flex flex-wrap items-start p-2 ... w-[340px] gap-0"> <!-- or gap-2, gap-6, gap-x-8 gap-y-1 -->
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn gap<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().flex_wrap().items_start().w_px(340.0)), |b| {
        links(b, |_| fixed_link())
    });
}

static JUSTIFIES: [(&str, Class); 6] = [
    ("flex-start", |t| t.justify_start()),
    ("flex-end", |t| t.justify_end()),
    ("center", |t| t.justify_center()),
    ("space-between", |t| t.justify_between()),
    ("space-around", |t| t.justify_around()),
    ("space-evenly", |t| t.justify_evenly()),
];

/// ```html
/// <nav class="flex gap-2 p-2 ... items-start justify-start"> <!-- or justify-end, justify-center, justify-between, justify-around, justify-evenly -->
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn justify_content<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().items_start()), |b| links(b, |_| fixed_link()));
}

static ALIGN_ITEMS: [(&str, Class); 5] = [
    ("stretch", |t| t.items_stretch()),
    ("flex-start", |t| t.items_start()),
    ("flex-end", |t| t.items_end()),
    ("center", |t| t.items_center()),
    ("baseline", |t| t.items_baseline()),
];

const LINK_TEXT_SIZES: [f32; 5] = [14.0, 20.0, 16.0, 28.0, 16.0];

/// Links of different heights in a bar taller than all of them.
///
/// ```html
/// <nav class="flex gap-2 p-2 ... h-[140px] items-stretch"> <!-- or items-start, items-end, items-center, items-baseline -->
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded text-[14px]/[17.5px]">Home</a>
///   <!-- About 20px, Pricing 16px, Blog 28px, Contact 16px -->
/// </nav>
/// ```
pub fn align_items<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().h_px(140.0)), |b| {
        links(b, |index| fixed_link().role(sized(LINK_TEXT_SIZES[index])))
    });
}

static ALIGN_CONTENT: [(&str, Class); 7] = [
    ("flex-start", |t| t.content_start()),
    ("flex-end", |t| t.content_end()),
    ("center", |t| t.content_center()),
    ("space-between", |t| t.content_between()),
    ("space-around", |t| t.content_around()),
    ("space-evenly", |t| t.content_evenly()),
    ("stretch", |t| t.content_stretch()),
];

/// Two lines of links in a bar taller than both.
///
/// ```html
/// <nav class="flex flex-wrap gap-2 p-2 ... w-[340px] h-[300px] content-start"> <!-- or content-end, content-center, content-between, content-around, content-evenly, content-stretch -->
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn align_content<A>(b: &mut B<'_, '_, A>, class: Class) {
    b.div(class(bar().flex_wrap().w_px(340.0).h_px(300.0)), |b| {
        links(b, |_| fixed_link())
    });
}

static GROWS: [(&str, ItemClass); 3] = [
    ("grow-0 (default)", |t, _| t),
    ("equal: grow", |t, _| t.grow()),
    ("doubled: third grow-2", |t, index| {
        t.grow_by(if index == 2 { 2.0 } else { 1.0 })
    }),
];

/// ```html
/// <nav class="flex gap-2 p-2 ... items-start">
///   <a class="px-3 py-2 bg-[#00008b] text-white rounded grow">Home</a> <!-- grow-0 | grow | grow-2 on the third -->
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn flex_grow<A>(b: &mut B<'_, '_, A>, class: ItemClass) {
    b.div(bar().items_start(), |b| {
        links(b, |index| class(link().text_center(), index))
    });
}

static SHRINKS: [(&str, ItemClass); 3] = [
    ("shrink (default)", |t, _| t),
    (
        "first shrink-0",
        |t, index| {
            if index == 0 { t.shrink_0() } else { t }
        },
    ),
    ("all shrink-0", |t, _| t.shrink_0()),
];

/// Five 100 point links in a bar too narrow for them.
///
/// ```html
/// <nav class="flex gap-2 p-2 ... items-start w-[460px] overflow-hidden">
///   <a class="px-3 py-2 w-[100px] text-center bg-[#00008b] text-white rounded shrink-0">Home</a> <!-- shrink | shrink-0 on the first | shrink-0 on all -->
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn flex_shrink<A>(b: &mut B<'_, '_, A>, class: ItemClass) {
    b.div(bar().items_start().w_px(460.0).overflow_hidden(), |b| {
        links(b, |index| class(link().w_px(100.0).text_center(), index))
    });
}

static BASES: [(&str, ItemClass); 3] = [
    ("auto", |t, _| t),
    ("all basis-[120px]", |t, _| t.basis_px(120.0)),
    ("first basis-1/2", |t, index| {
        if index == 0 {
            t.basis_frac(1.0, 2.0)
        } else {
            t
        }
    }),
];

/// ```html
/// <nav class="flex gap-2 p-2 ... items-start">
///   <a class="px-3 py-2 basis-[120px] text-center bg-[#00008b] text-white rounded">Home</a> <!-- basis-auto | basis-[120px] on all | basis-1/2 on the first -->
///   <!-- About, Pricing, Blog, Contact -->
/// </nav>
/// ```
pub fn flex_basis<A>(b: &mut B<'_, '_, A>, class: ItemClass) {
    b.div(bar().items_start(), |b| {
        links(b, |index| class(link().text_center(), index))
    });
}

static ALIGN_SELVES: [(&str, ItemClass); 5] = [
    ("auto", |t, _| t),
    ("third self-start", |t, index| {
        if index == 2 { t.self_start() } else { t }
    }),
    (
        "third self-end",
        |t, index| {
            if index == 2 { t.self_end() } else { t }
        },
    ),
    ("third self-center", |t, index| {
        if index == 2 { t.self_center() } else { t }
    }),
    ("third self-stretch", |t, index| {
        if index == 2 { t.self_stretch() } else { t }
    }),
];

/// One link overriding the bar's align-items.
///
/// ```html
/// <nav class="flex gap-2 p-2 ... h-[140px] items-center">
///   <a class="px-3 py-2 w-[100px] flex-none text-center bg-[#00008b] text-white rounded">Home</a>
///   <a class="... self-end">Pricing</a> <!-- self-auto | self-start | self-end | self-center | self-stretch on the third -->
///   <!-- About, Blog, Contact -->
/// </nav>
/// ```
pub fn align_self<A>(b: &mut B<'_, '_, A>, class: ItemClass) {
    b.div(bar().items_center().h_px(140.0), |b| {
        links(b, |index| class(fixed_link(), index))
    });
}

static ORDERS: [(&str, [usize; 5]); 2] = [
    ("source order", [0, 1, 2, 3, 4]),
    ("children built in the desired order", [2, 0, 3, 4, 1]),
];

/// Taffy has no `order`: reordering means building the children in order.
///
/// ```html
/// <nav class="flex gap-2 p-2 ... items-start">
///   <a class="px-3 py-2 bg-[#00008b] text-white rounded">Pricing</a> <!-- listed in the desired order -->
///   <!-- Home, Blog, Contact, About -->
/// </nav>
/// ```
pub fn order<A>(b: &mut B<'_, '_, A>, order: [usize; 5]) {
    b.text(
        tw(),
        "Taffy has no `order` property: build the children in the order to show.",
    );
    b.div(bar().items_start(), |b| {
        for index in order {
            b.div(link(), |b| b.text(tw(), LINKS[index]));
        }
    });
}

/// The selector beside the guide.
pub fn layout(b: &mut B<'_, '_, State>, state: &State) {
    common::with_controls(b, |b| controls(b, state), |b| guide(b, state));
}

fn main() -> eframe::Result {
    common::run_app("flexbox_guide", State::default(), layout)
}
