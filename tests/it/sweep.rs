//! Sweeps over the frame-continuity checker: a parameter steps frame after
//! frame, as a user drags a window edge, and no frame may disagree with its
//! neighbours.

use core::time::Duration;

use bullseye::testing::Timeline;
use bullseye::{Builder, tw};
use egui::{Vec2, vec2};
use egui_kittest::Harness;

#[path = "../../examples/common/mod.rs"]
pub mod common;

use crate::support::{self, Rig, sweep};

pub const WINDOW_HEIGHT: f32 = 600.0;
pub const NARROWEST: f32 = 240.0;
pub const WIDEST: f32 = 1200.0;
const SLIDE: Duration = Duration::from_millis(150);
/// Frames after a sweep: a slide at 60 frames a second plus the settle window.
pub const IDLE: usize = 16;

/// The planted flaw of a [`Grid`], for proving that the checks find bugs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flaw {
    None,
    /// The hovered panel is painted 120 points aside in windows at least 700
    /// wide: a paint offset that flips at a threshold.
    OffsetFlip,
}

/// A wrapping grid of equal square panels, `1 / cols` of the window wide.
pub struct Grid {
    pub cols: u32,
    pub panels: Vec<u32>,
    pub width: f32,
    pub flaw: Flaw,
    next: u32,
}

impl Grid {
    pub fn new(flaw: Flaw) -> Self {
        Self {
            cols: 3,
            panels: (0..6).collect(),
            width: 600.0,
            flaw,
            next: 6,
        }
    }

    pub const MOST_PANELS: usize = 8;

    pub fn add(&mut self, index: usize) {
        if self.panels.len() >= Self::MOST_PANELS {
            return;
        }
        self.panels.insert(index.min(self.panels.len()), self.next);
        self.next += 1;
    }

    pub fn remove(&mut self, index: usize) {
        if !self.panels.is_empty() {
            self.panels.remove(index % self.panels.len());
        }
    }
}

fn grid_view(b: &mut Builder<'_, '_, Grid>, grid: &Grid) {
    let tokens = common::tokens(b.ctx());
    let aside = match grid.flaw {
        Flaw::OffsetFlip if grid.width >= 700.0 => 120.0,
        _ => 0.0,
    };
    b.div_id("grid", tw().flex().flex_wrap().w_full(), |b| {
        for &panel in &grid.panels {
            b.div_id(
                panel,
                tw().w_frac(1.0, grid.cols as f32)
                    .aspect_square()
                    .bg(tokens.muted)
                    .hover(|p| p.translate(aside, 0.0))
                    .animate_layout(SLIDE),
                |_| {},
            );
        }
    });
}

pub fn grid_harness(flaw: Flaw) -> Harness<'static, Rig<Grid>> {
    let grid = Grid::new(flaw);
    let mut harness = support::harness_stepped(
        vec2(grid.width, WINDOW_HEIGHT),
        1.0 / 60.0,
        common::base_text(egui::Theme::Light),
        grid,
        grid_view,
    );
    harness.hover_at(egui::pos2(10.0, 10.0));
    harness.run_steps(2);
    harness
}

pub fn set_width(harness: &mut Harness<'_, Rig<Grid>>, width: f32) {
    harness.state_mut().app.width = width;
    harness.set_size(Vec2::new(width, WINDOW_HEIGHT));
}

fn widths(from: f32, to: f32, step: f32) -> Vec<f32> {
    let count = ((to - from).abs() / step) as usize;
    (0..=count)
        .map(|k| from + (to - from).signum() * step * k as f32)
        .collect()
}

fn grid_sweep(
    cols: u32,
    widths: impl IntoIterator<Item = f32>,
    idle: usize,
) -> (Harness<'static, Rig<Grid>>, Timeline) {
    let mut harness = grid_harness(Flaw::None);
    harness.state_mut().app.cols = cols;
    let timeline = sweep(
        &mut harness,
        widths,
        |h, _, width| set_width(h, width),
        idle,
    );
    (harness, timeline)
}

#[test]
fn dragging_the_window_edge_is_continuous_at_every_column_count() {
    for cols in 1..=3 {
        let there = widths(NARROWEST, WIDEST, 3.0);
        let back = widths(WIDEST, NARROWEST, 3.0);
        let (_, timeline) = grid_sweep(cols, there.into_iter().chain(back), IDLE);
        assert!(timeline.len() > 250, "precondition: a long sweep");
        timeline.assert_continuous();
    }
}

#[test]
fn switching_columns_mid_resize_is_continuous() {
    let mut harness = grid_harness(Flaw::None);
    let steps: Vec<(f32, u32)> = widths(NARROWEST, WIDEST, 5.0)
        .into_iter()
        .enumerate()
        .map(|(k, width)| (width, [3, 3, 3, 2, 1, 1, 2, 3, 1][k % 9]))
        .collect();
    let mut switches = Vec::new();
    let timeline = sweep(
        &mut harness,
        steps,
        |harness, frame, (width, cols)| {
            if harness.state().app.cols != cols {
                switches.push(frame);
            }
            harness.state_mut().app.cols = cols;
            set_width(harness, width);
        },
        IDLE,
    );
    assert!(switches.len() > 20, "precondition: columns switch often");
    // A panel's size is not animated (#62), so a column change repaints it at
    // its new size at once, and a frame whose window was resized places every
    // node at once.
    switches
        .into_iter()
        .fold(timeline, Timeline::allow_jump_into_frame)
        .assert_continuous();
}
