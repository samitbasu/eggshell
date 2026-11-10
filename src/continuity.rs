//! Frame-continuity checking: record where every identified node is painted
//! each frame, then check that no frame disagrees with its neighbours.
//!
//! Recording is switched on per context with [`record`]. Once on, the draw
//! pass appends one [`NodeFrame`] per identified node it paints to the
//! context's history; [`Timeline::recorded`] reads the history back and its
//! `assert_*` methods check the invariants.
//!
//! ```no_run
//! use eggshell_ui::testing::{Timeline, record};
//!
//! let ctx = egui::Context::default();
//! record(&ctx);
//! // ... run frames that draw eggshell views ...
//! Timeline::recorded(&ctx).assert_continuous();
//! ```

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};

use egui::{Context, Id, Rect, Vec2};

use crate::tw::{Opacity, Progress};

/// Whether a node declared `animate_layout`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// A change of its layout position is painted at once.
    Snaps,
    /// A change of its layout position is slid over.
    Slides,
}

/// Names one slide: the same for every frame of it, and different for the
/// next slide of the same node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlideId(u64);

impl SlideId {
    pub(crate) fn new(began: f64) -> Self {
        Self(began.to_bits())
    }
}

/// The slide a new slide interrupted, as the retarget found it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interrupted {
    /// The interrupted slide's start offset.
    pub start: Vec2,
    /// Its progress in the frame that interrupted it.
    pub progress: Progress,
}

/// Where a sliding node is on its slide in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slide {
    /// Which slide this is.
    pub id: SlideId,
    /// How far from its layout the node was painted when the slide began.
    pub start: Vec2,
    /// How far along the slide the node is, eased.
    pub progress: Progress,
    /// The slide this one retargeted, if it began by interrupting one.
    pub interrupted: Option<Interrupted>,
    /// How far from its layout the node is painted by itself: the slide plus
    /// its own state translate.
    pub offset: Vec2,
}

/// One identified node as one frame drew it.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeFrame {
    /// The node's egui id.
    pub id: Id,
    /// The readable key path, as in [`PlacedNode::path`](crate::PlacedNode::path).
    pub path: String,
    /// The border box layout gave the node.
    pub layout_rect: Rect,
    /// The layout rect moved by every paint offset: the node's own state
    /// translate and slide, and those of its ancestors. A node lifted for a
    /// drag, and its subtree, are recorded at their layout rects: where input
    /// carries a node is not motion, and the ancestors' slides do not move
    /// it. It does not slide, only snap.
    pub painted_rect: Rect,
    /// The node's opacity multiplied with its ancestors'.
    pub opacity: Opacity,
    /// Whether the node slides when layout moves it.
    pub motion: Motion,
    /// The slide the node is on, for a node that [`Slides`](Motion::Slides).
    pub slide: Option<Slide>,
}

/// What the draw pass records for one pass of the context.
#[derive(Clone, Debug, Default)]
pub(crate) struct RecordedPass {
    pub(crate) pass: u64,
    pub(crate) nodes: Vec<NodeFrame>,
}

#[derive(Clone, Default)]
struct History {
    on: bool,
    passes: Vec<RecordedPass>,
}

fn history_id() -> Id {
    crate::key::root_id().with("continuity-history")
}

/// Starts recording the nodes drawn on `ctx`, discarding what was recorded.
pub fn record(ctx: &Context) {
    ctx.data_mut(|data| {
        *data.get_temp_mut_or_default::<History>(history_id()) = History {
            on: true,
            passes: Vec::new(),
        };
    });
}

/// Whether [`record`] is on for `ctx`. The draw pass asks once per frame.
pub(crate) fn recording(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<History>(history_id()).is_some_and(|h| h.on))
}

/// Adds the nodes one draw pass painted to the history.
pub(crate) fn store(ctx: &Context, nodes: Vec<NodeFrame>) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let history = data.get_temp_mut_or_default::<History>(history_id());
        match history.passes.last_mut() {
            Some(last) if last.pass == pass => last.nodes.extend(nodes),
            _ => history.passes.push(RecordedPass { pass, nodes }),
        }
    });
}

/// A distance in points.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Points(f32);

impl Points {
    /// `value` points, at least zero.
    pub fn new(value: f32) -> Self {
        Self(value.max(0.0))
    }
}

impl From<Points> for f32 {
    fn from(points: Points) -> f32 {
        points.0
    }
}

/// A node an [`allow_jump`](Timeline::allow_jump) names.
#[derive(Clone, Debug)]
pub enum NodeRef {
    /// The node at this path and everything below it.
    Path(String),
    /// The node with this id.
    Id(Id),
}

impl From<&str> for NodeRef {
    fn from(path: &str) -> Self {
        Self::Path(path.to_owned())
    }
}

impl From<Id> for NodeRef {
    fn from(id: Id) -> Self {
        Self::Id(id)
    }
}

impl NodeRef {
    fn names(&self, node: &NodeFrame) -> bool {
        match self {
            Self::Path(path) => {
                node.path == *path
                    || node
                        .path
                        .strip_prefix(path.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            }
            Self::Id(id) => node.id == *id,
        }
    }
}

/// What a [`Violation`] found.
#[derive(Clone, Debug, PartialEq)]
pub enum ViolationKind {
    /// The painted rect jumped between two frames.
    Teleport {
        /// The painted rect in the previous frame.
        before: Rect,
        /// The painted rect in this one.
        after: Rect,
    },
    /// Present in the frames before and after, absent in this one.
    Gap,
    /// Opacity differed for this one frame only.
    OpacityBlip {
        /// The opacity in the previous frame.
        before: f32,
        /// The opacity in this one.
        during: f32,
        /// The opacity in the next.
        after: f32,
    },
    /// A sliding node was painted off its slide's path between two frames of
    /// the one slide: its offset moved by something other than the slide's
    /// progress, a snap or a jump.
    OffPath {
        /// How far the slide's progress moved the offset.
        expected: Vec2,
        /// How far the offset moved.
        actual: Vec2,
    },
    /// A slide went backwards: its progress fell.
    Regressed {
        /// The progress in the previous frame.
        before: Progress,
        /// The progress in this one.
        after: Progress,
    },
    /// Painted more than once in one frame.
    DoublePaint {
        /// How many times.
        times: usize,
    },
    /// Layout was still changing in the last frames of the run.
    Unsettled {
        /// The layout rect in the previous frame.
        before: Rect,
        /// The layout rect in this one.
        after: Rect,
    },
}

/// One broken invariant: which node, in which frame (an index into the
/// timeline), and what happened.
#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    /// The readable key path of the node.
    pub path: String,
    /// The index of the frame the violation is found in.
    pub frame: usize,
    /// What was found.
    pub kind: ViolationKind,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { path, frame, kind } = self;
        match kind {
            ViolationKind::Teleport { before, after } => write!(
                f,
                "teleport: {path} was painted at {before:?} in frame {} and at {after:?} in frame {frame}",
                frame - 1
            ),
            ViolationKind::Gap => write!(
                f,
                "gap: {path} is painted in frames {} and {} but not in frame {frame}",
                frame - 1,
                frame + 1
            ),
            ViolationKind::OpacityBlip {
                before,
                during,
                after,
            } => write!(
                f,
                "blip: {path} has opacity {before} in frame {}, {during} in frame {frame} and {after} in frame {}",
                frame - 1,
                frame + 1
            ),
            ViolationKind::OffPath { expected, actual } => write!(
                f,
                "off path: {path} slid from frame {} to frame {frame} so that its offset should move by {expected:?} but moved by {actual:?}",
                frame - 1
            ),
            ViolationKind::Regressed { before, after } => write!(
                f,
                "regressed: the slide of {path} went from progress {} in frame {} to {} in frame {frame}",
                f32::from(*before),
                frame - 1,
                f32::from(*after)
            ),
            ViolationKind::DoublePaint { times } => {
                write!(
                    f,
                    "double paint: {path} was painted {times} times in frame {frame}"
                )
            }
            ViolationKind::Unsettled { before, after } => write!(
                f,
                "unsettled: layout of {path} changed from {before:?} to {after:?} in frame {frame}, within the last frames of the run"
            ),
        }
    }
}

/// The frames recorded over a run, and the tolerances to check them with.
#[derive(Clone, Debug)]
pub struct Timeline {
    frames: Vec<Vec<NodeFrame>>,
    max_step: Points,
    slide_tolerance: Points,
    max_opacity_step: Opacity,
    settle_window: usize,
    allowed_nodes: Vec<NodeRef>,
    allowed_frames: HashSet<usize>,
}

/// How far a layout rect may move between frames and still be "unchanged".
const LAYOUT_EPSILON: f32 = 0.01;

impl Timeline {
    /// Everything [`record`] has collected on `ctx` so far, one frame per
    /// drawn pass.
    pub fn recorded(ctx: &Context) -> Self {
        let passes = ctx.data(|data| {
            data.get_temp::<History>(history_id())
                .map(|history| history.passes)
                .unwrap_or_default()
        });
        Self::from_frames(passes.into_iter().map(|pass| pass.nodes).collect())
    }

    fn from_frames(frames: Vec<Vec<NodeFrame>>) -> Self {
        Self {
            frames,
            max_step: Points::new(40.0),
            slide_tolerance: Points::new(4.0),
            max_opacity_step: Opacity::new(0.5),
            settle_window: 4,
            allowed_nodes: Vec::new(),
            allowed_frames: HashSet::new(),
        }
    }

    /// The number of recorded frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether no frame was recorded.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The nodes drawn in frame `index`.
    pub fn frame(&self, index: usize) -> &[NodeFrame] {
        &self.frames[index]
    }

    /// The largest painted move between consecutive frames that is not a
    /// jump. Default 40 points.
    pub fn max_step(mut self, step: Points) -> Self {
        self.max_step = step;
        self
    }

    /// How far a sliding node's paint offset may move in a frame beyond what
    /// the slide's progress moves it, for its own state translate to ease
    /// meanwhile. Default 4 points.
    pub fn slide_tolerance(mut self, tolerance: Points) -> Self {
        self.slide_tolerance = tolerance;
        self
    }

    /// The opacity a node may differ by for a single frame. Default 0.5: an in-flight fade may reverse, a node may not flash to or from nothing.
    pub fn max_opacity_blip(mut self, step: Opacity) -> Self {
        self.max_opacity_step = step;
        self
    }

    /// The number of final frames whose layout must be identical for the run
    /// to have [settled](Self::settles). Default 4.
    pub fn settle_window(mut self, frames: usize) -> Self {
        self.settle_window = frames;
        self
    }

    /// Declares that `node` (a path, which covers the nodes below it, or an
    /// id) jumps on purpose.
    pub fn allow_jump(mut self, node: impl Into<NodeRef>) -> Self {
        self.allowed_nodes.push(node.into());
        self
    }

    /// Declares that anything may jump into frame `index`.
    pub fn allow_jump_into_frame(mut self, index: usize) -> Self {
        self.allowed_frames.insert(index);
        self
    }

    fn by_id(&self) -> Vec<HashMap<Id, &NodeFrame>> {
        self.frames
            .iter()
            .map(|frame| frame.iter().map(|n| (n.id, n)).collect())
            .collect()
    }

    /// Nodes whose painted rect moved by more than [`max_step`](Self::max_step)
    /// between consecutive frames.
    ///
    /// Per corner, a node is judged on how far its painted rect moved (`p`)
    /// and how much of that its layout rect accounts for (`l`, so the paint
    /// offset changed by `p - l`):
    /// - a node with `animate_layout` must stay continuous: `p` within the step;
    /// - any other node may also move as far as layout moved it, provided its
    ///   paint offset changed by no more than the step: a layout change
    ///   without `animate_layout` is an instant move on purpose.
    ///
    /// Exempt: nodes first drawn in the frame, allowed nodes and frames, and
    /// frames in the middle of a slide, which [`slides`](Self::slides) judges
    /// by progress instead.
    pub fn teleports(&self) -> Vec<Violation> {
        let step = f32::from(self.max_step);
        let mut found = Vec::new();
        let frames = self.by_id();
        for (k, pair) in frames.windows(2).enumerate().map(|(i, w)| (i + 1, w)) {
            let (before, after) = (&pair[0], &pair[1]);
            if self.allowed_frames.contains(&k) {
                continue;
            }
            for (id, now) in after {
                let Some(prev) = before.get(id) else { continue };
                if self.allowed_nodes.iter().any(|allowed| allowed.names(now)) {
                    continue;
                }
                if continues_slide(prev, now) {
                    continue;
                }
                let carried = carried_by_retarget(prev, now);
                let corners = |rect: Rect| [rect.min, rect.max];
                let jumps = (0..2).any(|c| {
                    let p = corners(now.painted_rect)[c] - corners(prev.painted_rect)[c] - carried;
                    let l = corners(now.layout_rect)[c] - corners(prev.layout_rect)[c];
                    let offset_change: Vec2 = p - l;
                    let jumped = p.length() > step;
                    match now.motion {
                        Motion::Slides => jumped,
                        Motion::Snaps => jumped && offset_change.length() > step,
                    }
                });
                if jumps {
                    found.push(Violation {
                        path: now.path.clone(),
                        frame: k,
                        kind: ViolationKind::Teleport {
                            before: prev.painted_rect,
                            after: now.painted_rect,
                        },
                    });
                }
            }
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Sliding nodes that left their slide between two frames of it. Within
    /// one slide, the node's paint offset from layout runs along the segment
    /// from the slide's start offset to zero:
    /// - progress never falls;
    /// - the offset moves by `-start * (progress change)`, within the
    ///   [tolerance](Self::slide_tolerance). A node that snaps to the end, jumps
    ///   back or leaves the path moves by something else, however long the
    ///   slide is.
    ///
    /// The frame a slide starts is judged by [`teleports`](Self::teleports): a
    /// retargeted slide starts from where the node was painted, so it may have
    /// moved on only as far as the slide it interrupted did. Exempt:
    /// allowed nodes and frames.
    pub fn slides(&self) -> Vec<Violation> {
        let tolerance = f32::from(self.slide_tolerance);
        let mut found = Vec::new();
        let frames = self.by_id();
        for (k, pair) in frames.windows(2).enumerate().map(|(i, w)| (i + 1, w)) {
            let (before, after) = (&pair[0], &pair[1]);
            if self.allowed_frames.contains(&k) {
                continue;
            }
            for (id, now) in after {
                let Some(prev) = before.get(id) else { continue };
                if self.allowed_nodes.iter().any(|allowed| allowed.names(now)) {
                    continue;
                }
                let (Some(was), Some(is)) = (prev.slide, now.slide) else {
                    continue;
                };
                if !continues_slide(prev, now) {
                    continue;
                }
                let kind = if is.progress < was.progress {
                    ViolationKind::Regressed {
                        before: was.progress,
                        after: is.progress,
                    }
                } else {
                    let expected = -is.start * (f32::from(is.progress) - f32::from(was.progress));
                    let actual = is.offset - was.offset;
                    if (actual - expected).length() <= tolerance {
                        continue;
                    }
                    ViolationKind::OffPath { expected, actual }
                };
                found.push(Violation {
                    path: now.path.clone(),
                    frame: k,
                    kind,
                });
            }
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Nodes absent for exactly one frame, and nodes whose opacity differs
    /// from both neighbouring frames, which agree with each other, by more
    /// than [`max_opacity_blip`](Self::max_opacity_blip).
    pub fn gaps(&self) -> Vec<Violation> {
        let step = f32::from(self.max_opacity_step);
        let by_id = self.by_id();
        let mut found = Vec::new();
        for k in 1..self.frames.len().saturating_sub(1) {
            let (prev, now, next) = (&by_id[k - 1], &by_id[k], &by_id[k + 1]);
            for (id, before) in prev {
                let Some(after) = next.get(id) else { continue };
                let kind = match now.get(id) {
                    None => ViolationKind::Gap,
                    Some(during) => {
                        let (a, d, z) = (
                            f32::from(before.opacity),
                            f32::from(during.opacity),
                            f32::from(after.opacity),
                        );
                        let agree = (a - z).abs() <= step;
                        let departs = (d - a).abs() > step && (d - z).abs() > step;
                        if !(agree && departs) {
                            continue;
                        }
                        ViolationKind::OpacityBlip {
                            before: a,
                            during: d,
                            after: z,
                        }
                    }
                };
                found.push(Violation {
                    path: before.path.clone(),
                    frame: k,
                    kind,
                });
            }
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Ids painted more than once in a frame.
    pub fn double_paints(&self) -> Vec<Violation> {
        let mut found = Vec::new();
        for (k, frame) in self.frames.iter().enumerate() {
            let mut counts: HashMap<Id, (usize, &str)> = HashMap::new();
            for node in frame {
                counts.entry(node.id).or_insert((0, &node.path)).0 += 1;
            }
            found.extend(counts.into_values().filter(|&(times, _)| times > 1).map(
                |(times, path)| Violation {
                    path: path.to_owned(),
                    frame: k,
                    kind: ViolationKind::DoublePaint { times },
                },
            ));
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Nodes whose layout rect still changed in the last
    /// [`settle_window`](Self::settle_window) frames. The run must end with
    /// the input held still for that long; a shorter run is not judged.
    pub fn settles(&self) -> Vec<Violation> {
        let window = self.settle_window;
        let len = self.frames.len();
        if window < 2 || len < window {
            return Vec::new();
        }
        let mut found = Vec::new();
        let frames = self.by_id();
        for k in (len - window + 1)..len {
            let (before, after) = (&frames[k - 1], &frames[k]);
            for (id, now) in after {
                let Some(prev) = before.get(id) else { continue };
                let moved = |a: Rect, b: Rect| {
                    (a.min - b.min).abs().max_elem() > LAYOUT_EPSILON
                        || (a.max - b.max).abs().max_elem() > LAYOUT_EPSILON
                };
                if moved(prev.layout_rect, now.layout_rect) {
                    found.push(Violation {
                        path: now.path.clone(),
                        frame: k,
                        kind: ViolationKind::Unsettled {
                            before: prev.layout_rect,
                            after: now.layout_rect,
                        },
                    });
                }
            }
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Every violation of every invariant, with the defaults or the
    /// tolerances set.
    pub fn violations(&self) -> Vec<Violation> {
        [
            self.teleports(),
            self.slides(),
            self.gaps(),
            self.double_paints(),
            self.settles(),
        ]
        .concat()
    }

    /// Panics unless [`teleports`](Self::teleports) is empty.
    #[track_caller]
    pub fn assert_no_teleports(&self) {
        report(&self.teleports());
    }

    /// Panics unless [`slides`](Self::slides) is empty.
    #[track_caller]
    pub fn assert_slides_stay_on_path(&self) {
        report(&self.slides());
    }

    /// Panics unless [`gaps`](Self::gaps) is empty.
    #[track_caller]
    pub fn assert_no_gaps(&self) {
        report(&self.gaps());
    }

    /// Panics unless [`double_paints`](Self::double_paints) is empty.
    #[track_caller]
    pub fn assert_no_double_paints(&self) {
        report(&self.double_paints());
    }

    /// Panics unless [`settles`](Self::settles) is empty.
    #[track_caller]
    pub fn assert_settles(&self) {
        report(&self.settles());
    }

    /// Runs every check. Panics with one line per violation.
    #[track_caller]
    pub fn assert_continuous(&self) {
        assert!(
            !self.is_empty(),
            "nothing was recorded: call testing::record(ctx) before the frames run"
        );
        report(&self.violations());
    }
}

/// How far a retarget carried the painted position along the slide it
/// interrupted since the previous frame: a slide that starts from where its
/// predecessor is painted moves on by that much in its first frame.
fn carried_by_retarget(prev: &NodeFrame, now: &NodeFrame) -> Vec2 {
    match (prev.slide, now.slide.and_then(|slide| slide.interrupted)) {
        (Some(was), Some(interrupted)) if was.start == interrupted.start => {
            -interrupted.start * (f32::from(interrupted.progress) - f32::from(was.progress))
        }
        _ => Vec2::ZERO,
    }
}

/// Whether `now` is the same slide as `prev`, a frame on.
fn continues_slide(prev: &NodeFrame, now: &NodeFrame) -> bool {
    matches!((prev.slide, now.slide), (Some(was), Some(is)) if was.id == is.id)
}

#[track_caller]
fn report(violations: &[Violation]) {
    if violations.is_empty() {
        return;
    }
    let mut message = format!("{} continuity violation(s):", violations.len());
    for violation in violations {
        write!(message, "\n  {violation}").expect("writing to a String cannot fail");
    }
    panic!("{message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(progress: f32) -> Vec<NodeFrame> {
        let start = Vec2::new(-100.0, 0.0);
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(10.0));
        vec![NodeFrame {
            id: Id::NULL,
            path: "/node".to_owned(),
            layout_rect: rect,
            painted_rect: rect.translate(start * (1.0 - progress)),
            opacity: Opacity::new(1.0),
            motion: Motion::Slides,
            slide: Some(Slide {
                id: SlideId::new(1.0),
                start,
                progress: Progress::new(progress),
                interrupted: None,
                offset: start * (1.0 - progress),
            }),
        }]
    }

    #[test]
    fn a_slide_whose_progress_falls_is_regressed() {
        let timeline = Timeline::from_frames(vec![frame(0.0), frame(0.5), frame(0.25)]);
        let found = timeline.slides();
        assert!(
            matches!(&found[..], [v] if v.frame == 2 && matches!(v.kind, ViolationKind::Regressed { .. })),
            "{found:?}"
        );
        assert_eq!(
            found[0].to_string(),
            "regressed: the slide of /node went from progress 0.5 in frame 1 to 0.25 in frame 2"
        );
    }

    #[test]
    fn a_slide_that_advances_is_on_its_path() {
        Timeline::from_frames(vec![frame(0.0), frame(0.5), frame(1.0), frame(1.0)])
            .assert_slides_stay_on_path();
    }
}
