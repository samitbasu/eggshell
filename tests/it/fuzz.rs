//! Randomised user sessions over the frame-continuity checker. proptest
//! generates a sequence of actions, the harness runs it frame by frame, and no
//! frame may disagree with its neighbours; a failing sequence is shrunk to a
//! minimal one.
//!
//! A small number of cases runs by default; `PROPTEST_CASES` (or
//! `cargo xtask fuzz`) runs more.

use core::ops::Range;

use eggshell_ui::testing::{Points, Timeline};
use egui::{Vec2, vec2};
use egui_kittest::Harness;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestError, TestRng, TestRunner};

use crate::support::{self, Rig, click_node, hover_node};
use crate::sweep::{Flaw, Grid, IDLE, NARROWEST, WIDEST, grid_harness, set_width};

#[path = "../../examples/todo.rs"]
#[allow(dead_code, reason = "the example's main is not run here")]
mod todo;

const DEFAULT_CASES: u32 = 32;
/// How far a dragged window edge moves per frame.
const EDGE_STEP: f32 = 12.0;
/// A panel in a column of `Grid::MOST_PANELS` moves `MOST_PANELS - 1` times
/// the edge step per frame as the column resizes.
const MAX_GRID_STEP: f32 = 100.0;
/// Frames an `animate_layout` of 150 ms slides for at 60 frames a second,
/// with one to spare.
const SLIDE_FRAMES: usize = 10;

fn config() -> Config {
    let mut config = Config::default();
    if std::env::var_os("PROPTEST_CASES").is_none() {
        config.cases = DEFAULT_CASES;
    }
    config
}

fn verdict(timeline: &Timeline) -> Result<(), TestCaseError> {
    let violations = timeline.violations();
    if violations.is_empty() {
        return Ok(());
    }
    let report: Vec<String> = violations.iter().take(5).map(ToString::to_string).collect();
    Err(TestCaseError::fail(format!(
        "{} continuity violation(s), first:\n{}",
        violations.len(),
        report.join("\n")
    )))
}

/// The frames of a run that may jump, and why.
struct Session {
    first_pass: u64,
    slides: Vec<Range<usize>>,
    allowed: Vec<usize>,
}

impl Session {
    fn start(ctx: &egui::Context) -> Self {
        eggshell_ui::testing::record(ctx);
        Self {
            first_pass: ctx.cumulative_pass_nr(),
            slides: Vec::new(),
            allowed: Vec::new(),
        }
    }

    /// The index of the frame drawn next.
    fn next_frame(&self, ctx: &egui::Context) -> usize {
        (ctx.cumulative_pass_nr() - self.first_pass) as usize
    }

    /// An action that moves layout, so nodes with `animate_layout` slide for
    /// the next `SLIDE_FRAMES`.
    fn moved_layout(&mut self, ctx: &egui::Context) {
        let from = self.next_frame(ctx);
        self.slides.push(from..from + SLIDE_FRAMES);
    }

    /// A frame of a window resize: every node is placed at once, which cuts
    /// short a slide still in flight.
    fn resized(&mut self, ctx: &egui::Context) {
        let frame = self.next_frame(ctx);
        if self.slides.iter().any(|slide| slide.contains(&frame)) {
            self.allowed.push(frame);
        }
    }

    /// A slide's distance is bounded by nothing but the layout, so one frame
    /// of a long slide moves further than the step that tells a jump from a
    /// slide. Every frame of a slide may therefore jump; the checker is
    /// blind to a snap in these frames, which layer 1's fixtures cover.
    fn allow_slide_frames(&mut self) {
        self.allowed.extend(self.slides.iter().cloned().flatten());
    }

    fn timeline(self, ctx: &egui::Context) -> Timeline {
        let timeline = Timeline::recorded(ctx);
        assert_eq!(
            timeline.len(),
            self.next_frame(ctx),
            "one recorded frame per pass"
        );
        self.allowed
            .into_iter()
            .fold(timeline, Timeline::allow_jump_into_frame)
    }
}

/// Moves the window edge from `*width` to `target` the way a drag does: one
/// bounded step per frame.
fn drag_edge<A>(
    harness: &mut Harness<'_, Rig<A>>,
    session: &mut Session,
    width: &mut f32,
    target: f32,
    mut set: impl FnMut(&mut Harness<'_, Rig<A>>, f32),
) {
    while *width != target {
        *width += (target - *width).clamp(-EDGE_STEP, EDGE_STEP);
        session.resized(&harness.ctx);
        set(harness, *width);
        harness.step();
    }
}

fn window_width() -> impl Strategy<Value = u32> {
    NARROWEST as u32..=WIDEST as u32
}

#[derive(Clone, Debug)]
enum GridAction {
    SetWidth(u32),
    SetCols(u32),
    Idle(usize),
    AddPanel(usize),
    RemovePanel(usize),
}

fn grid_action() -> impl Strategy<Value = GridAction> {
    prop_oneof![
        window_width().prop_map(GridAction::SetWidth),
        (1u32..=3).prop_map(GridAction::SetCols),
        (1usize..=8).prop_map(GridAction::Idle),
        (0..Grid::MOST_PANELS).prop_map(GridAction::AddPanel),
        (0..Grid::MOST_PANELS).prop_map(GridAction::RemovePanel),
    ]
}

fn grid_session() -> impl Strategy<Value = Vec<GridAction>> {
    prop::collection::vec(grid_action(), 1..=10)
}

fn run_grid(flaw: Flaw, actions: &[GridAction]) -> Result<(), TestCaseError> {
    let mut harness = grid_harness(flaw);
    let mut session = Session::start(&harness.ctx);
    let mut width = harness.state().app.width;
    for action in actions {
        match *action {
            GridAction::SetWidth(target) => {
                drag_edge(
                    &mut harness,
                    &mut session,
                    &mut width,
                    target as f32,
                    set_width,
                );
            }
            GridAction::SetCols(cols) => {
                if harness.state().app.cols != cols {
                    // A panel's size is not animated (#62): a column change
                    // repaints it at its new size at once.
                    session.allowed.push(session.next_frame(&harness.ctx));
                    session.moved_layout(&harness.ctx);
                }
                harness.state_mut().app.cols = cols;
                harness.step();
            }
            GridAction::Idle(count) => harness.run_steps(count),
            GridAction::AddPanel(index) => {
                session.moved_layout(&harness.ctx);
                harness.state_mut().app.add(index);
                harness.step();
            }
            GridAction::RemovePanel(index) => {
                session.moved_layout(&harness.ctx);
                harness.state_mut().app.remove(index);
                harness.step();
            }
        }
    }
    harness.run_steps(IDLE);
    session.allow_slide_frames();
    verdict(
        &session
            .timeline(&harness.ctx)
            .max_step(Points::new(MAX_GRID_STEP)),
    )
}

#[derive(Clone, Debug)]
enum TodoAction {
    ClickUp(usize),
    ClickDown(usize),
    Remove(usize),
    Hover(usize),
    SetWidth(u32),
    Idle(usize),
}

fn todo_action() -> impl Strategy<Value = TodoAction> {
    let row = 0usize..4;
    prop_oneof![
        row.clone().prop_map(TodoAction::ClickUp),
        row.clone().prop_map(TodoAction::ClickDown),
        row.clone().prop_map(TodoAction::Remove),
        row.prop_map(TodoAction::Hover),
        window_width().prop_map(TodoAction::SetWidth),
        (1usize..=8).prop_map(TodoAction::Idle),
    ]
}

fn todo_session() -> impl Strategy<Value = Vec<TodoAction>> {
    prop::collection::vec(todo_action(), 1..=12)
}

fn run_todo(actions: &[TodoAction]) -> Result<(), TestCaseError> {
    let mut harness = support::harness(
        vec2(520.0, 320.0),
        crate::sweep::common::base_text(egui::Theme::Light),
        todo::State::default(),
        todo::layout,
    );
    let mut session = Session::start(&harness.ctx);
    let mut width = 520.0;
    for action in actions {
        let rows = harness.state().app.items.len();
        let key_at = |harness: &Harness<'_, Rig<todo::State>>, i: usize| {
            harness.state().app.items[i % rows].key
        };
        match *action {
            TodoAction::SetWidth(target) => {
                drag_edge(
                    &mut harness,
                    &mut session,
                    &mut width,
                    target as f32,
                    |h, w| {
                        h.set_size(Vec2::new(w, 320.0));
                    },
                );
            }
            TodoAction::Idle(count) => harness.run_steps(count),
            _ if rows == 0 => {}
            TodoAction::Hover(i) => {
                let key = key_at(&harness, i);
                hover_node(&mut harness, &format!("/{key:?}"));
            }
            TodoAction::ClickUp(i) | TodoAction::ClickDown(i) | TodoAction::Remove(i) => {
                let key = key_at(&harness, i);
                let button = match action {
                    TodoAction::ClickUp(_) => "up",
                    TodoAction::ClickDown(_) => "down",
                    _ => "remove",
                };
                click_node(&mut harness, &format!("/{key:?}/{button:?}"));
                session.moved_layout(&harness.ctx);
            }
        }
    }
    harness.run_steps(IDLE);
    verdict(&session.timeline(&harness.ctx))
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn a_gallery_grid_stays_continuous_under_any_session(actions in grid_session()) {
        run_grid(Flaw::None, &actions)?;
    }

    #[test]
    fn the_todo_list_stays_continuous_under_any_session(actions in todo_session()) {
        run_todo(&actions)?;
    }
}

/// The fuzzer is only worth running if it finds a bug that is there.
#[test]
fn the_fuzzer_finds_a_planted_bug_and_shrinks_it() {
    let mut runner = TestRunner::new_with_rng(
        Config {
            cases: 256,
            failure_persistence: None,
            ..Config::default()
        },
        TestRng::deterministic_rng(RngAlgorithm::ChaCha),
    );
    let outcome = runner.run(&grid_session(), |actions| {
        run_grid(Flaw::OffsetFlip, &actions)
    });
    let Err(TestError::Fail(reason, minimal)) = outcome else {
        panic!("the planted offset flip went unnoticed: {outcome:?}")
    };
    eprintln!("shrunk to {minimal:?}: {reason}");
    assert!(
        matches!(minimal[..], [GridAction::SetWidth(700)]),
        "not minimal: {minimal:?}"
    );
}
