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

use crate::overlay::Side;
use crate::tw::{Opacity, Progress};

/// How a node may move on its own: by sliding over a layout change, or as part
/// of an overlay's entrance or exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Motion {
    /// A change of its layout position is painted at once.
    Snaps,
    /// A change of its layout position is slid over.
    Slides,
    /// The node is the box (or the scrim) of an overlay that enters or leaves
    /// with motion, which moves and fades it from frame to frame. Judged on
    /// the motion alone: the paint offset it adds to its layout may not jump.
    /// Where layout puts the node is its anchor's, which may move at once.
    Presents,
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
    /// How far the ancestors' paint offsets move the node: the part of
    /// `painted_rect` the node did not move itself. An ancestor that slides
    /// is judged by its own slide; the nodes inside it follow it.
    pub ancestors: Vec2,
    /// How far the scroll containers around the node have moved it from where
    /// an unscrolled layout puts it: minus the sum of their offsets. Layout
    /// and paint both include it, so it is motion the node did not make: a
    /// node that slides is not judged on the part of its movement scrolling
    /// accounts for.
    pub scrolled: Vec2,
    /// The node's opacity multiplied with its ancestors'.
    pub opacity: Opacity,
    /// Whether the node slides when layout moves it.
    pub motion: Motion,
    /// The slide the node is on, for a node that [`Slides`](Motion::Slides).
    pub slide: Option<Slide>,
}

/// One floating or modal overlay as one frame placed it, for the checks that
/// judge placement ([`OffScreen`](ViolationKind::OffScreen),
/// [`Reflip`](ViolationKind::Reflip)). Overlays whose anchor cannot be seen
/// are not drawn and are not recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayFrame {
    /// The overlay root's egui id: the id of its [`NodeFrame`].
    pub id: Id,
    /// The overlay root's readable key path.
    pub path: String,
    /// The part of the window the overlay may be in.
    pub bounds: Rect,
    /// The anchor's rect.
    pub anchor: Rect,
    /// The size the overlay measured at before the room clamped it.
    pub natural: Vec2,
    /// The side of its anchor it was placed on; a modal has none.
    pub side: Option<Side>,
}

/// What the draw pass records for one pass of the context.
#[derive(Clone, Debug, Default)]
pub(crate) struct RecordedPass {
    pub(crate) pass: u64,
    pub(crate) nodes: Vec<NodeFrame>,
    /// The layout rect of every identified node of the pass, whether or not
    /// it was painted.
    pub(crate) laid_out: Vec<(String, Rect)>,
    /// The identified nodes the pass left out because they were outside the
    /// viewport of a scroll container. Not painting them is not a gap.
    pub(crate) culled: Vec<Id>,
    pub(crate) overlays: Vec<OverlayFrame>,
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
pub(crate) fn store(ctx: &Context, recorded: RecordedPass) {
    ctx.data_mut(|data| {
        let history = data.get_temp_mut_or_default::<History>(history_id());
        match history.passes.last_mut() {
            Some(last) if last.pass == recorded.pass => {
                last.nodes.extend(recorded.nodes);
                last.laid_out.extend(recorded.laid_out);
                last.culled.extend(recorded.culled);
                last.overlays.extend(recorded.overlays);
            }
            _ => history.passes.push(recorded),
        }
    });
}

/// The layout rect of the identified node `path` in the last recorded pass,
/// whether or not the pass painted it: a node that is scrolled out of a
/// scroll container's viewport is laid out but not painted.
pub fn laid_out_rect(ctx: &Context, path: &str) -> Option<Rect> {
    ctx.data(|data| {
        let history = data.get_temp::<History>(history_id())?;
        let last = history.passes.last()?;
        last.laid_out
            .iter()
            .find(|(candidate, _)| candidate == path)
            .map(|(_, rect)| *rect)
    })
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
    /// An overlay was painted outside the bounds that can hold it.
    OffScreen {
        /// Where the overlay was painted.
        rect: Rect,
        /// The bounds it should be inside.
        bounds: Rect,
    },
    /// An overlay changed side with nothing that decides the side changed.
    Reflip {
        /// The side in the previous frame.
        before: Side,
        /// The side in this one.
        after: Side,
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
            ViolationKind::OffScreen { rect, bounds } => write!(
                f,
                "off screen: overlay {path} is painted at {rect:?} in frame {frame}, outside the bounds {bounds:?} that can hold it"
            ),
            ViolationKind::Reflip { before, after } => write!(
                f,
                "reflip: overlay {path} went from side {before:?} in frame {} to {after:?} in frame {frame}, though its anchor, natural size and bounds did not change",
                frame - 1
            ),
        }
    }
}

/// The frames recorded over a run, and the tolerances to check them with.
#[derive(Clone, Debug)]
pub struct Timeline {
    frames: Vec<Vec<NodeFrame>>,
    /// Per frame, the nodes left out as outside a viewport.
    culled: Vec<HashSet<Id>>,
    /// Per frame, the overlays that were placed.
    overlays: Vec<Vec<OverlayFrame>>,
    max_step: Points,
    slide_tolerance: Points,
    max_opacity_step: Opacity,
    settle_window: usize,
    allowed_nodes: Vec<NodeRef>,
    allowed_frames: HashSet<usize>,
}

/// How far a layout rect may move between frames and still be "unchanged".
const LAYOUT_EPSILON: f32 = 0.01;

/// How far outside its bounds an overlay may be painted: a rect is rounded to
/// physical pixels when it is drawn.
const BOUNDS_EPSILON: f32 = 0.5;

/// Whether `rect` is outside `bounds` on an axis along which they could hold
/// it: along an axis the rect is larger than the bounds, `place` starts it at
/// their edge and it cannot be inside.
pub fn leaves_bounds(rect: Rect, bounds: Rect) -> bool {
    (0..2).any(|axis| {
        let (lo, hi) = (rect.min[axis], rect.max[axis]);
        let (min, max) = (bounds.min[axis], bounds.max[axis]);
        hi - lo <= max - min + BOUNDS_EPSILON
            && (lo < min - BOUNDS_EPSILON || hi > max + BOUNDS_EPSILON)
    })
}

fn moved(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_elem() > LAYOUT_EPSILON
        || (a.max - b.max).abs().max_elem() > LAYOUT_EPSILON
}

impl Timeline {
    /// Everything [`record`] has collected on `ctx` so far, one frame per
    /// drawn pass.
    pub fn recorded(ctx: &Context) -> Self {
        let passes = ctx.data(|data| {
            data.get_temp::<History>(history_id())
                .map(|history| history.passes)
                .unwrap_or_default()
        });
        let culled = passes
            .iter()
            .map(|pass| pass.culled.iter().copied().collect())
            .collect();
        let overlays = passes.iter().map(|pass| pass.overlays.clone()).collect();
        Self {
            culled,
            overlays,
            ..Self::from_frames(passes.into_iter().map(|pass| pass.nodes).collect())
        }
    }

    fn from_frames(frames: Vec<Vec<NodeFrame>>) -> Self {
        Self {
            culled: vec![HashSet::new(); frames.len()],
            overlays: vec![Vec::new(); frames.len()],
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
    ///   own paint offset changed by no more than the step: a layout change
    ///   without `animate_layout` is an instant move on purpose, and what an
    ///   ancestor's slide moves it by is the ancestor's to keep continuous.
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
                let moved_by_ancestors = now.ancestors - prev.ancestors;
                let moved_by_scrolling = now.scrolled - prev.scrolled;
                let corners = |rect: Rect| [rect.min, rect.max];
                let jumps = (0..2).any(|c| {
                    let p = corners(now.painted_rect)[c] - corners(prev.painted_rect)[c] - carried;
                    let l = corners(now.layout_rect)[c] - corners(prev.layout_rect)[c];
                    let own_offset_change: Vec2 = p - l - moved_by_ancestors;
                    let jumped = p.length() > step;
                    match now.motion {
                        Motion::Slides => (p - moved_by_scrolling).length() > step,
                        Motion::Presents => own_offset_change.length() > step,
                        Motion::Snaps => jumped && own_offset_change.length() > step,
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

    /// Nodes absent for exactly one frame, unless the frame left them out as
    /// outside a scroll container's viewport, and nodes whose opacity differs
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
                    None if self.culled[k].contains(id) => continue,
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

    /// Overlays painted outside the bounds that can hold them (the window cut
    /// to the overlay's boundary), in the frames that painted them.
    ///
    /// Judged per axis: along an axis the overlay is larger than the bounds
    /// they cannot hold it, and it starts at their edge (`place`), so only
    /// the axes it fits are checked. A box that is mid-entrance moves on
    /// purpose, starting displaced from its place, so it is judged where
    /// layout put it. Exempt: allowed nodes and frames.
    pub fn off_screens(&self) -> Vec<Violation> {
        let mut found = Vec::new();
        let frames = self.by_id();
        for (k, overlays) in self.overlays.iter().enumerate() {
            if self.allowed_frames.contains(&k) {
                continue;
            }
            for overlay in overlays {
                let Some(node) = frames[k].get(&overlay.id) else {
                    continue;
                };
                if self.allowed_nodes.iter().any(|allowed| allowed.names(node)) {
                    continue;
                }
                let rect = match node.motion {
                    Motion::Presents => node.layout_rect,
                    Motion::Snaps | Motion::Slides => node.painted_rect,
                };
                let bounds = overlay.bounds;
                if leaves_bounds(rect, bounds) {
                    found.push(Violation {
                        path: overlay.path.clone(),
                        frame: k,
                        kind: ViolationKind::OffScreen { rect, bounds },
                    });
                }
            }
        }
        found.sort_by(|a, b| (a.frame, &a.path).cmp(&(b.frame, &b.path)));
        found
    }

    /// Overlays whose side changed between two frames in which nothing that
    /// decides the side did: the anchor's rect, the natural size and the
    /// bounds. The choice of side depends on those alone, so a change
    /// without one is an oscillation. Exempt: allowed nodes and frames.
    pub fn reflips(&self) -> Vec<Violation> {
        let mut found = Vec::new();
        let frames = self.by_id();
        for (k, pair) in self
            .overlays
            .windows(2)
            .enumerate()
            .map(|(i, w)| (i + 1, w))
        {
            if self.allowed_frames.contains(&k) {
                continue;
            }
            for now in &pair[1] {
                let Some(prev) = pair[0].iter().find(|prev| prev.id == now.id) else {
                    continue;
                };
                let (Some(before), Some(after)) = (prev.side, now.side) else {
                    continue;
                };
                if before == after
                    || moved(prev.anchor, now.anchor)
                    || moved(prev.bounds, now.bounds)
                    || (prev.natural - now.natural).abs().max_elem() > LAYOUT_EPSILON
                {
                    continue;
                }
                if frames[k]
                    .get(&now.id)
                    .is_some_and(|node| self.allowed_nodes.iter().any(|a| a.names(node)))
                {
                    continue;
                }
                found.push(Violation {
                    path: now.path.clone(),
                    frame: k,
                    kind: ViolationKind::Reflip { before, after },
                });
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
            self.off_screens(),
            self.reflips(),
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

    /// Panics unless [`off_screens`](Self::off_screens) is empty.
    #[track_caller]
    pub fn assert_on_screen(&self) {
        report(&self.off_screens());
    }

    /// Panics unless [`reflips`](Self::reflips) is empty.
    #[track_caller]
    pub fn assert_no_reflips(&self) {
        report(&self.reflips());
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
            ancestors: Vec2::ZERO,
            scrolled: Vec2::ZERO,
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

    const WINDOW: Rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 300.0));

    /// A frame with one overlay painted at `rect`, placed on `side` of an
    /// anchor at `anchor_x`.
    fn overlay_frame(rect: Rect, side: Side, anchor_x: f32) -> (Vec<NodeFrame>, Vec<OverlayFrame>) {
        let node = NodeFrame {
            id: Id::NULL,
            path: "/menu".to_owned(),
            layout_rect: rect,
            painted_rect: rect,
            ancestors: Vec2::ZERO,
            scrolled: Vec2::ZERO,
            opacity: Opacity::new(1.0),
            motion: Motion::Snaps,
            slide: None,
        };
        let overlay = OverlayFrame {
            id: Id::NULL,
            path: "/menu".to_owned(),
            bounds: WINDOW,
            anchor: Rect::from_min_size(egui::pos2(anchor_x, 100.0), Vec2::splat(20.0)),
            natural: rect.size(),
            side: Some(side),
        };
        (vec![node], vec![overlay])
    }

    fn overlay_timeline(frames: Vec<(Vec<NodeFrame>, Vec<OverlayFrame>)>) -> Timeline {
        let (nodes, overlays) = frames.into_iter().unzip();
        Timeline {
            overlays,
            ..Timeline::from_frames(nodes)
        }
    }

    fn at(x: f32, y: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), Vec2::new(80.0, 60.0))
    }

    #[test]
    fn an_overlay_painted_past_the_window_edge_is_off_screen() {
        let timeline = overlay_timeline(vec![
            overlay_frame(at(100.0, 130.0), Side::Bottom, 100.0),
            overlay_frame(at(340.0, 130.0), Side::Bottom, 100.0),
        ]);
        let found = timeline.off_screens();
        assert!(
            matches!(&found[..], [v] if v.frame == 1 && matches!(v.kind, ViolationKind::OffScreen { .. })),
            "{found:?}"
        );
        assert!(
            found[0]
                .to_string()
                .starts_with("off screen: overlay /menu")
        );
    }

    #[test]
    fn an_overlay_inside_its_bounds_or_too_large_for_them_is_on_screen() {
        let large = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(80.0, 500.0));
        overlay_timeline(vec![
            overlay_frame(at(0.0, 0.0), Side::Bottom, 100.0),
            overlay_frame(at(320.0, 240.0), Side::Bottom, 100.0),
            overlay_frame(large, Side::Bottom, 100.0),
        ])
        .assert_on_screen();
    }

    #[test]
    fn a_box_mid_entrance_is_judged_where_layout_put_it() {
        let (mut nodes, overlays) = overlay_frame(at(320.0, 240.0), Side::Bottom, 100.0);
        nodes[0].motion = Motion::Presents;
        nodes[0].painted_rect = at(326.0, 246.0);
        overlay_timeline(vec![(nodes, overlays)]).assert_on_screen();
    }

    #[test]
    fn a_side_that_changes_while_nothing_that_decides_it_does_is_a_reflip() {
        let timeline = overlay_timeline(vec![
            overlay_frame(at(100.0, 130.0), Side::Bottom, 100.0),
            overlay_frame(at(100.0, 40.0), Side::Top, 100.0),
        ]);
        let found = timeline.reflips();
        assert!(
            matches!(&found[..], [v] if v.frame == 1 && v.kind == ViolationKind::Reflip { before: Side::Bottom, after: Side::Top }),
            "{found:?}"
        );
    }

    #[test]
    fn a_side_that_changes_with_the_anchor_or_the_size_is_not_a_reflip() {
        let (nodes, mut resized) = overlay_frame(at(100.0, 40.0), Side::Top, 100.0);
        resized[0].natural = Vec2::new(80.0, 90.0);
        overlay_timeline(vec![
            overlay_frame(at(100.0, 130.0), Side::Bottom, 100.0),
            overlay_frame(at(100.0, 40.0), Side::Top, 140.0),
            (nodes.clone(), resized),
            overlay_frame(at(100.0, 130.0), Side::Bottom, 140.0),
        ])
        .assert_no_reflips();
        let (_, mut narrowed) = overlay_frame(at(100.0, 130.0), Side::Bottom, 140.0);
        narrowed[0].bounds = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(400.0, 200.0));
        overlay_timeline(vec![
            overlay_frame(at(100.0, 40.0), Side::Top, 140.0),
            (nodes, narrowed),
        ])
        .assert_no_reflips();
    }

    fn presenting(layout_x: f32, offset_x: f32) -> Vec<NodeFrame> {
        let layout = Rect::from_min_size(egui::pos2(layout_x, 0.0), Vec2::splat(10.0));
        vec![NodeFrame {
            id: Id::NULL,
            path: "/tip".to_owned(),
            layout_rect: layout,
            painted_rect: layout.translate(Vec2::new(offset_x, 0.0)),
            ancestors: Vec2::ZERO,
            scrolled: Vec2::ZERO,
            opacity: Opacity::new(1.0),
            motion: Motion::Presents,
            slide: None,
        }]
    }

    #[test]
    fn a_presented_box_follows_its_anchor_when_layout_moves_it_at_once() {
        Timeline::from_frames(vec![presenting(0.0, 4.0), presenting(300.0, 4.0)])
            .assert_no_teleports();
    }

    #[test]
    fn a_presented_box_may_not_jump_by_its_own_motion() {
        let found =
            Timeline::from_frames(vec![presenting(0.0, 0.0), presenting(0.0, 100.0)]).teleports();
        assert!(matches!(&found[..], [v] if v.frame == 1), "{found:?}");
    }

    #[test]
    fn a_slide_that_advances_is_on_its_path() {
        Timeline::from_frames(vec![frame(0.0), frame(0.5), frame(1.0), frame(1.0)])
            .assert_slides_stay_on_path();
    }
}
