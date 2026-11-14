//! What a frame costs: build, layout and draw of plain trees at 100, 1,000
//! and 5,000 nodes (#133), a virtual list's frame at 1,000 to 100,000 rows,
//! and the js-framework-benchmark operations on a virtual list (#54).
//!
//! `cargo bench --bench frame`; one group with `cargo bench --bench frame --
//! list`. Each frame is a whole egui pass, as an application runs it, so the
//! times include egui's own per-pass work. The allocations a frame makes are
//! printed before the timings.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use eggshell_ui::{Builder, LayoutState, scroll, tw};
use egui::{Context, Id, RawInput, Rect, pos2, vec2};

/// Counts allocations, for the allocations-per-frame figures.
struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: forwarded as given.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded as given.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: forwarded as given.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const SCREEN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

/// A context whose fonts are loaded.
fn context() -> Context {
    let ctx = Context::default();
    ctx.run_ui(input(), |_| {}).textures_delta.clear();
    ctx
}

fn input() -> RawInput {
    RawInput {
        screen_rect: Some(SCREEN),
        ..RawInput::default()
    }
}

/// How much of a frame runs.
#[derive(Clone, Copy, Debug)]
enum Phase {
    Build,
    Layout,
    Draw,
}

impl Phase {
    const ALL: [Self; 3] = [Self::Build, Self::Layout, Self::Draw];

    fn name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Layout => "build+layout",
            Self::Draw => "build+layout+draw",
        }
    }
}

/// One egui pass in which `view` is built, laid out and drawn, as far as
/// `phase` says.
fn frame<A>(
    ctx: &Context,
    state: &mut LayoutState,
    app: &mut A,
    phase: Phase,
    view: impl for<'f> Fn(&mut Builder<'_, 'f, A>, &'f A),
) {
    let mut output = ctx.run_ui(input(), |ui| {
        let built = state.build(ui.ctx(), |b| view(b, &*app));
        match phase {
            Phase::Build => {
                black_box(built);
            }
            Phase::Layout => {
                black_box(built.layout(ui.max_rect()));
            }
            Phase::Draw => built.layout(ui.max_rect()).draw(app, ui),
        }
    });
    output.textures_delta.clear();
}

/// The words of a cell or a paragraph.
const WORDS: [&str; 8] = [
    "layout", "measure", "draw", "scroll", "region", "overlay", "focus", "row",
];

fn words(seed: usize, count: usize) -> String {
    (0..count)
        .map(|at| WORDS[(seed + at) % WORDS.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// #133's trees, `nodes` nodes each.
#[derive(Clone, Copy, Debug)]
enum Tree {
    /// An eight-column grid of `div > text`.
    Grid,
    /// A column of wrapping paragraphs, `div > text` each.
    Text,
}

impl Tree {
    fn name(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Text => "text",
        }
    }

    fn build(self, b: &mut Builder<'_, '_, ()>, nodes: usize) {
        let cells = nodes / 2;
        match self {
            Self::Grid => b.div(tw().grid().grid_cols(8).gap(1.0).p(2.0), |b| {
                for at in 0..cells {
                    b.div(tw().px(1.0), |b| b.text(tw(), WORDS[at % WORDS.len()]));
                }
            }),
            Self::Text => b.div(tw().flex().flex_col().gap(1.0).p(2.0), |b| {
                for at in 0..cells {
                    b.div(tw(), |b| b.text(tw(), words(at, 6 + at % 20)));
                }
            }),
        }
    }
}

const TREE_SIZES: [usize; 3] = [100, 1_000, 5_000];

fn trees(c: &mut Criterion) {
    let ctx = context();
    for tree in [Tree::Grid, Tree::Text] {
        for nodes in TREE_SIZES {
            let mut state = LayoutState::new();
            let view = |b: &mut Builder<'_, '_, ()>, _: &()| tree.build(b, nodes);
            frame(&ctx, &mut state, &mut (), Phase::Draw, view);
            let before = ALLOCATIONS.load(Ordering::Relaxed);
            frame(&ctx, &mut state, &mut (), Phase::Draw, view);
            let made = ALLOCATIONS.load(Ordering::Relaxed) - before;
            println!("{}/{nodes}: {made} allocations a frame", tree.name());
        }
    }
    for tree in [Tree::Grid, Tree::Text] {
        let mut group = c.benchmark_group(tree.name());
        group.sample_size(30);
        for phase in Phase::ALL {
            for nodes in TREE_SIZES {
                let mut state = LayoutState::new();
                let view = |b: &mut Builder<'_, '_, ()>, _: &()| tree.build(b, nodes);
                frame(&ctx, &mut state, &mut (), phase, view);
                group.bench_function(BenchmarkId::new(phase.name(), nodes), |bench| {
                    bench.iter(|| frame(&ctx, &mut state, &mut (), phase, view));
                });
            }
        }
        group.finish();
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct RowId(u32);

/// How the list's rows are sized.
#[derive(Clone, Copy, Debug)]
enum Rows {
    /// One line each, given a fixed height.
    Fixed,
    /// Paragraphs of one to four lines, measured.
    Measured,
}

impl Rows {
    fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Measured => "variable",
        }
    }
}

/// A list of labelled rows, as the js-framework-benchmark keeps them.
struct Table {
    rows: Vec<RowId>,
    labels: Vec<String>,
    sizing: Rows,
    list: Cell<Option<Id>>,
}

impl Table {
    fn new(count: u32, sizing: Rows) -> Self {
        let labels = (0..count as usize)
            .map(|at| match sizing {
                Rows::Fixed => format!("row {at}"),
                Rows::Measured => words(at, 4 + (at % 4) * 24),
            })
            .collect();
        Self {
            rows: (0..count).map(RowId).collect(),
            labels,
            sizing,
            list: Cell::new(None),
        }
    }
}

fn table<'f>(b: &mut Builder<'_, 'f, Table>, table: &'f Table) {
    table.list.set(Some(b.id_of("list")));
    let list = b.virtual_list("list", tw().size_full(), &table.rows);
    let list = match table.sizing {
        Rows::Fixed => list.fixed_px(20.0),
        Rows::Measured => list.estimate_px(40.0),
    };
    list.items(|s, row| s.text(tw().px(2.0), table.labels[row.0 as usize].as_str()));
}

/// A table laid out in its own state, scrolled to its middle.
struct Fixture {
    ctx: Context,
    state: LayoutState,
    table: Table,
}

impl Fixture {
    fn new(count: u32, sizing: Rows) -> Self {
        let mut fixture = Self {
            ctx: context(),
            state: LayoutState::new(),
            table: Table::new(count, sizing),
        };
        fixture.frame();
        let list = fixture.table.list.get().expect("the list was built");
        let middle = scroll::viewport(&fixture.ctx, list)
            .expect("the list was laid out")
            .content
            .height()
            / 2.0;
        scroll::scroll_by(&fixture.ctx, list, vec2(0.0, middle));
        for _ in 0..8 {
            fixture.frame();
        }
        fixture
    }

    fn frame(&mut self) {
        frame(
            &self.ctx,
            &mut self.state,
            &mut self.table,
            Phase::Draw,
            table,
        );
    }
}

fn list(c: &mut Criterion) {
    let mut group = c.benchmark_group("list");
    group.sample_size(30);
    for sizing in [Rows::Fixed, Rows::Measured] {
        for count in [1_000, 10_000, 100_000] {
            let mut fixture = Fixture::new(count, sizing);
            group.bench_function(BenchmarkId::new(sizing.name(), count), |bench| {
                bench.iter(|| fixture.frame());
            });
        }
    }
    group.finish();
}

/// The js-framework-benchmark operations, each followed by the frame that
/// shows it, on a measured list scrolled to its middle.
fn operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("operations");
    group.sample_size(30);
    for count in [1_000, 10_000] {
        group.bench_function(BenchmarkId::new("create", count), |bench| {
            bench.iter_batched(
                || {
                    (
                        context(),
                        LayoutState::new(),
                        Table::new(count, Rows::Measured),
                    )
                },
                |(ctx, mut state, mut table)| {
                    frame(&ctx, &mut state, &mut table, Phase::Draw, self::table);
                },
                BatchSize::PerIteration,
            );
        });
        let fixture = RefCell::new(Fixture::new(count, Rows::Measured));
        let every_tenth = |fixture: &mut Fixture, suffix: &str| {
            for label in fixture.table.labels.iter_mut().step_by(10) {
                let stem = label.trim_end_matches(" !!!").len();
                label.truncate(stem);
                label.push_str(suffix);
            }
            fixture.frame();
        };
        group.bench_function(BenchmarkId::new("update every 10th", count), |bench| {
            bench.iter_batched(
                || every_tenth(&mut fixture.borrow_mut(), ""),
                |()| every_tenth(&mut fixture.borrow_mut(), " !!!"),
                BatchSize::PerIteration,
            );
        });
        let swap = |fixture: &mut Fixture| {
            let last = fixture.table.rows.len() - 2;
            fixture.table.rows.swap(1, last);
            fixture.frame();
        };
        group.bench_function(BenchmarkId::new("swap rows", count), |bench| {
            bench.iter_batched(
                || swap(&mut fixture.borrow_mut()),
                |()| swap(&mut fixture.borrow_mut()),
                BatchSize::PerIteration,
            );
        });
        let removed = count as usize / 2;
        group.bench_function(BenchmarkId::new("remove row", count), |bench| {
            bench.iter_batched(
                || {
                    let mut fixture = fixture.borrow_mut();
                    if fixture.table.rows[removed] != RowId(removed as u32) {
                        fixture.table.rows.insert(removed, RowId(removed as u32));
                    }
                    fixture.frame();
                },
                |()| {
                    let mut fixture = fixture.borrow_mut();
                    fixture.table.rows.remove(removed);
                    fixture.frame();
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, trees, list, operations);
criterion_main!(benches);
