//! Randomised user sessions over the frame-continuity checker. proptest
//! generates a sequence of actions, the harness runs it frame by frame, and no
//! frame may disagree with its neighbours; a failing sequence is shrunk to a
//! minimal one.
//!
//! A small number of cases runs by default; `PROPTEST_CASES` (or
//! `cargo xtask fuzz`) runs more.

use eggshell_ui::testing::{Points, Timeline};
use egui::{Key, Pos2, Vec2, vec2};
use egui_kittest::Harness;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestError, TestRng, TestRunner};

use crate::support::{self, Rig, click_node, hover_node};
use crate::sweep::{Flaw, Grid, IDLE, NARROWEST, WIDEST, grid_harness, set_width};

#[path = "../../examples/todo.rs"]
#[allow(dead_code, reason = "the example's main is not run here")]
mod todo;

#[path = "../../examples/dnd_gallery.rs"]
#[allow(dead_code, reason = "the example's main is not run here")]
mod gallery;

const DEFAULT_CASES: u32 = 32;
/// How far a dragged window edge moves per frame.
const EDGE_STEP: f32 = 12.0;
/// A panel in a column of `Grid::MOST_PANELS` moves `MOST_PANELS - 1` times
/// the edge step per frame as the column resizes.
const MAX_GRID_STEP: f32 = 100.0;
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

/// A run's recording, and the frames that may jump for a reason that is not
/// a slide: a panel's size is not animated (#62).
struct Session {
    first_pass: u64,
    resized_instantly: Vec<usize>,
}

impl Session {
    fn start(ctx: &egui::Context) -> Self {
        eggshell_ui::testing::record(ctx);
        Self {
            first_pass: ctx.cumulative_pass_nr(),
            resized_instantly: Vec::new(),
        }
    }

    /// The index of the frame drawn next.
    fn next_frame(&self, ctx: &egui::Context) -> usize {
        (ctx.cumulative_pass_nr() - self.first_pass) as usize
    }

    fn timeline(self, ctx: &egui::Context) -> Timeline {
        let timeline = Timeline::recorded(ctx);
        assert_eq!(
            timeline.len(),
            self.next_frame(ctx),
            "one recorded frame per pass"
        );
        self.resized_instantly
            .into_iter()
            .fold(timeline, Timeline::allow_jump_into_frame)
    }
}

/// Moves the window edge from `*width` to `target` the way a drag does: one
/// bounded step per frame.
fn drag_edge<A>(
    harness: &mut Harness<'_, Rig<A>>,
    width: &mut f32,
    target: f32,
    mut set: impl FnMut(&mut Harness<'_, Rig<A>>, f32),
) {
    while *width != target {
        *width += (target - *width).clamp(-EDGE_STEP, EDGE_STEP);
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
                drag_edge(&mut harness, &mut width, target as f32, set_width);
            }
            GridAction::SetCols(cols) => {
                if harness.state().app.cols != cols {
                    session
                        .resized_instantly
                        .push(session.next_frame(&harness.ctx));
                }
                harness.state_mut().app.cols = cols;
                harness.step();
            }
            GridAction::Idle(count) => harness.run_steps(count),
            GridAction::AddPanel(index) => {
                harness.state_mut().app.add(index);
                harness.step();
            }
            GridAction::RemovePanel(index) => {
                harness.state_mut().app.remove(index);
                harness.step();
            }
        }
    }
    harness.run_steps(IDLE);
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
    let session = Session::start(&harness.ctx);
    let mut width = 520.0;
    for action in actions {
        let rows = harness.state().app.items.len();
        let key_at = |harness: &Harness<'_, Rig<todo::State>>, i: usize| {
            harness.state().app.items[i % rows].key
        };
        match *action {
            TodoAction::SetWidth(target) => {
                drag_edge(&mut harness, &mut width, target as f32, |h, w| {
                    h.set_size(Vec2::new(w, 320.0));
                });
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
                // The frame that shows the result is part of the click: a move
                // drawn in the same frame as a resize is not a slide.
                harness.step();
            }
        }
    }
    harness.run_steps(IDLE);
    verdict(&session.timeline(&harness.ctx))
}

/// The gallery's window: wide enough for three columns, short enough to
/// scroll.
const GALLERY_HEIGHT: f32 = 400.0;
const GALLERY_WIDTH: f32 = 520.0;
/// How far the pointer moves per frame, in points: a quick flick.
const POINTER_STEP: f32 = 40.0;

#[derive(Clone, Debug)]
enum DragAction {
    /// Presses on a panel and moves far enough to lift it.
    Lift(usize),
    /// Moves the held panel to the slot of another one, crossing rows.
    MoveTo(usize),
    Escape,
    ReleaseInside,
    ReleaseOutside,
    /// Changes the window's width. Not while a panel is held: a reorder the
    /// resize itself causes coincides with a resize frame, where nodes are
    /// placed at once (#80), so the checker rightly finds a snap. The held
    /// case is `a_resize_under_a_held_panel_keeps_the_drag`.
    Resize(u32),
    SetCols(u32),
    /// Swaps the live image for the chip, or back. Not while a panel is held:
    /// the app's own look change then lasts one frame when the drag ends in
    /// the next, which the checker rightly reports as an opacity blip.
    ToggleImage,
    /// A wheel event over the middle of the gallery, of `notches` times a
    /// notch's worth of points (negative scrolls down), then two frames of
    /// egui smoothing it. Also while a panel is held.
    Wheel(i32),
    /// The application scrolls the gallery by `points`, in one frame. Also
    /// while a panel is held.
    ScrollBy(i32),
    Idle(usize),
}

/// How far one wheel notch scrolls, in points.
const NOTCH: f32 = 40.0;
/// The most frames a scroll may take to come to rest.
const SCROLL_SETTLE_LIMIT: usize = 240;

fn drag_action() -> impl Strategy<Value = DragAction> {
    let panel = 0usize..Grid::MOST_PANELS;
    prop_oneof![
        3 => panel.clone().prop_map(DragAction::Lift),
        6 => panel.prop_map(DragAction::MoveTo),
        2 => Just(DragAction::Escape),
        2 => Just(DragAction::ReleaseInside),
        2 => Just(DragAction::ReleaseOutside),
        2 => (NARROWEST as u32..=WIDEST as u32).prop_map(DragAction::Resize),
        2 => (1u32..=3).prop_map(DragAction::SetCols),
        1 => Just(DragAction::ToggleImage),
        4 => (-8i32..=8).prop_map(DragAction::Wheel),
        2 => (-600i32..=600).prop_map(DragAction::ScrollBy),
        2 => (1usize..=8).prop_map(DragAction::Idle),
    ]
}

fn drag_session() -> impl Strategy<Value = Vec<DragAction>> {
    prop::collection::vec(drag_action(), 1..=14)
}

/// A user's hand on the gallery: where the pointer is and whether the button
/// is down.
struct Hand<'a> {
    harness: Harness<'a, support::FrameRig<gallery::State>>,
    at: Pos2,
    down: bool,
    width: f32,
}

impl Hand<'_> {
    fn panel_centre(&self, index: usize) -> Option<Pos2> {
        let keys: Vec<u32> = self
            .harness
            .state()
            .app
            .panels
            .iter()
            .map(|p| p.0.0)
            .collect();
        let key = keys[index % keys.len()];
        let path = format!("{}/PanelKey({key})/\"face\"", gallery::GRID);
        let rect = support::last_drawn(&self.harness.ctx, &path)?.layout_rect;
        let visible = egui::Rect::from_min_max(
            Pos2::new(0.0, 8.0 + 20.0),
            Pos2::new(self.width, GALLERY_HEIGHT - 8.0 - 20.0),
        );
        visible.contains(rect.center()).then(|| rect.center())
    }

    /// Steps until the gallery has stopped scrolling: egui eases a wheel
    /// event in over about half a second, and the session is over when
    /// the view is.
    fn settle_scrolling(&mut self) {
        let Some(page) = support::last_drawn(&self.harness.ctx, "/\"page\"") else {
            return;
        };
        let offset = |hand: &Self| {
            eggshell_ui::scroll::viewport(&hand.harness.ctx, page.id).map(|v| v.offset())
        };
        let mut still = 0;
        for _ in 0..SCROLL_SETTLE_LIMIT {
            let before = offset(self);
            self.harness.step();
            still = if offset(self) == before { still + 1 } else { 0 };
            if still == 4 {
                return;
            }
        }
        panic!("the gallery is still scrolling after {SCROLL_SETTLE_LIMIT} frames");
    }

    /// Moves the pointer to `to` the way a hand does: a bounded step a frame.
    fn glide(&mut self, to: Pos2) {
        while self.at != to {
            let to_go = to - self.at;
            let step = to_go * (POINTER_STEP / to_go.length()).min(1.0);
            self.at += step;
            self.harness.hover_at(self.at);
            self.harness.step();
        }
    }

    fn act(&mut self, action: &DragAction, session: &mut Session) {
        match *action {
            DragAction::Lift(index) if !self.down => {
                if let Some(centre) = self.panel_centre(index) {
                    self.glide(centre);
                    self.harness.drag_at(centre);
                    self.harness.step();
                    self.down = true;
                    self.glide(centre + vec2(0.0, 6.0));
                    self.glide(centre + vec2(0.0, 12.0));
                }
            }
            DragAction::MoveTo(index) if self.down => {
                if let Some(centre) = self.panel_centre(index) {
                    self.glide(centre);
                }
            }
            DragAction::Escape => {
                self.harness.key_press(Key::Escape);
                self.harness.step();
            }
            DragAction::ReleaseInside if self.down => {
                self.harness.drop_at(self.at);
                self.harness.step();
                self.down = false;
            }
            DragAction::ReleaseOutside if self.down => {
                self.glide(Pos2::new(self.width + 30.0, self.at.y));
                self.harness.drop_at(self.at);
                self.harness.step();
                self.down = false;
            }
            DragAction::Resize(target) if !self.down => {
                let target = target as f32;
                while self.width != target {
                    self.width += (target - self.width).clamp(-EDGE_STEP, EDGE_STEP);
                    self.harness.set_size(vec2(self.width, GALLERY_HEIGHT));
                    self.harness.step();
                }
            }
            DragAction::SetCols(cols) => {
                if self.harness.state().app.cols != cols as usize {
                    session
                        .resized_instantly
                        .push(session.next_frame(&self.harness.ctx));
                }
                self.harness.state_mut().app.cols = cols as usize;
                self.harness.step();
            }
            DragAction::ToggleImage if !self.down => {
                let app = &mut self.harness.state_mut().app;
                app.image = match app.image {
                    gallery::Image::Live => gallery::Image::Chip,
                    gallery::Image::Chip => gallery::Image::Live,
                };
                self.harness.step();
            }
            DragAction::Wheel(notches) => {
                let middle = Pos2::new(self.width / 2.0, GALLERY_HEIGHT / 2.0);
                if !self.down {
                    self.glide(middle);
                }
                self.harness.hover_at(self.at);
                self.harness.step();
                self.harness.event(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, notches as f32 * NOTCH),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                });
                self.harness.run_steps(2);
            }
            DragAction::ScrollBy(points) => {
                if let Some(page) = support::last_drawn(&self.harness.ctx, "/\"page\"") {
                    eggshell_ui::scroll::scroll_by(
                        &self.harness.ctx,
                        page.id,
                        vec2(0.0, points as f32),
                    );
                }
                self.harness.step();
            }
            DragAction::Idle(count) => self.harness.run_steps(count),
            _ => {}
        }
    }
}

fn run_drags(actions: &[DragAction]) -> Result<(), TestCaseError> {
    run_drags_ending(actions).map(|_| ())
}

/// A hand on a fresh gallery with as many panels as the grid fuzz keeps, so
/// that the checker's step for a resize is the same: a panel moves
/// `panels - 1` times the edge step per frame as the column resizes.
fn gallery_hand() -> (Hand<'static>, Session) {
    let mut state = gallery::State::default();
    state.panels.truncate(Grid::MOST_PANELS);
    let mut harness = support::frame_harness(
        vec2(GALLERY_WIDTH, GALLERY_HEIGHT),
        crate::sweep::common::base_text(egui::Theme::Light),
        state,
        |ui, app, layout| gallery::window(ui, app, layout, &mut eggshell_ui::LayoutState::new()),
    );
    harness.run_steps(4);
    let session = Session::start(&harness.ctx);
    harness.step();
    let hand = Hand {
        harness,
        at: Pos2::new(GALLERY_WIDTH / 2.0, GALLERY_HEIGHT / 2.0),
        down: false,
        width: GALLERY_WIDTH,
    };
    (hand, session)
}

/// The gallery's panels, in order, after the session.
fn run_drags_ending(actions: &[DragAction]) -> Result<Vec<u32>, TestCaseError> {
    let (mut hand, mut session) = gallery_hand();
    for action in actions {
        hand.act(action, &mut session);
    }
    if hand.down {
        hand.harness.drop_at(hand.at);
    }
    hand.settle_scrolling();
    hand.harness.run_steps(IDLE);
    verdict(
        &session
            .timeline(&hand.harness.ctx)
            .max_step(Points::new(MAX_GRID_STEP)),
    )?;
    Ok(hand
        .harness
        .state()
        .app
        .panels
        .iter()
        .map(|p| p.0.0)
        .collect())
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn a_dragged_gallery_stays_continuous_under_any_session(actions in drag_session()) {
        run_drags(&actions)?;
    }

    #[test]
    fn a_gallery_grid_stays_continuous_under_any_session(actions in grid_session()) {
        run_grid(Flaw::None, &actions)?;
    }

    #[test]
    fn the_todo_list_stays_continuous_under_any_session(actions in todo_session()) {
        run_todo(&actions)?;
    }
}

/// The hand of the drag fuzzer really lifts, moves and drops.
#[test]
fn the_drag_fuzzer_reorders_when_the_session_says_so() {
    let moved = run_drags_ending(&[
        DragAction::Lift(0),
        DragAction::MoveTo(5),
        DragAction::ReleaseInside,
    ])
    .expect("continuous");
    assert_ne!(moved[..6], [0, 1, 2, 3, 4, 5], "{moved:?}");
    let escaped = run_drags_ending(&[
        DragAction::Lift(0),
        DragAction::MoveTo(5),
        DragAction::Escape,
        DragAction::ReleaseInside,
    ])
    .expect("continuous");
    assert_eq!(escaped[..6], [0, 1, 2, 3, 4, 5], "{escaped:?}");
    let outside = run_drags_ending(&[
        DragAction::Lift(0),
        DragAction::MoveTo(5),
        DragAction::ReleaseOutside,
    ])
    .expect("continuous");
    assert_eq!(outside[..6], [0, 1, 2, 3, 4, 5], "{outside:?}");
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

/// The scroll actions of the drag fuzzer really scroll the gallery, with a
/// panel held or not.
#[test]
fn the_drag_fuzzer_scrolls_when_the_session_says_so() {
    let offset = |hand: &Hand<'_>| {
        let page = support::last_drawn(&hand.harness.ctx, "/\"page\"").expect("drawn");
        eggshell_ui::scroll::viewport(&hand.harness.ctx, page.id)
            .expect("laid out")
            .offset()
            .y
    };
    for actions in [
        vec![DragAction::Wheel(-6)],
        vec![DragAction::ScrollBy(300)],
        vec![
            DragAction::Lift(0),
            DragAction::Wheel(-6),
            DragAction::ReleaseInside,
        ],
    ] {
        let (mut hand, mut session) = gallery_hand();
        assert_eq!(offset(&hand), 0.0, "precondition: at the top");
        for action in &actions {
            hand.act(action, &mut session);
        }
        hand.settle_scrolling();
        assert!(
            offset(&hand) > 100.0,
            "{actions:?} scrolled to {}",
            offset(&hand)
        );
    }
}
