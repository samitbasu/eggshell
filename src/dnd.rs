//! Drag and drop, inside the app and from outside it.
//!
//! A node that is a **source** ([`NodeHandle::drag_source`]) carries a payload
//! of any type. A node that is a **target** ([`NodeHandle::drop_target`])
//! takes the payloads of one type. Files dragged in from the OS are a
//! payload too, [`Files`], and a target takes them the same way. A source
//! with a [`drag_handle`](NodeHandle::drag_handle) lifts only from the
//! handle, and is still what moves.
//!
//! ```
//! use eggshell_ui::{Builder, Drop, DragImage, tw};
//!
//! #[derive(Clone, Copy, Debug, Hash)]
//! struct Card(u32);
//!
//! #[derive(Default)]
//! struct App { placed: Vec<(u32, egui::Vec2)> }
//!
//! fn view(b: &mut Builder<'_, '_, App>, app: &App) {
//!     b.div_id(Card(1), tw().w_px(80.0).h_px(24.0), |_| {})
//!         .drag_source(Card(1))
//!         .drag_image(DragImage::Live);
//!     b.div_id("canvas", tw().w_px(200.0).h_px(200.0), |_| {})
//!         .drop_target::<Card>(|app, drop: Drop<Card>| {
//!             app.placed.push((drop.payload.0, drop.position.unwrap_or_default()));
//!         });
//! }
//! ```
//!
//! # One gesture, resolved once
//!
//! The gesture lives in the egui context, so the build, the draw and the
//! application all read the one state. Which target a drag is over is decided
//! once per frame, in [`Placed::draw`](crate::Placed::draw), over **this
//! frame's final layout rects**, not egui's previous-frame hit rects: after a
//! live reorder, a node that has just moved still claims the pointer in
//! egui's, and the order thrashes. Of the targets that accept the payload and
//! hold the pointer, the innermost wins (the last in paint order). The same
//! answer paints `drag_over`, is delivered on release, and is what
//! [`Dragging::over`] reads at the next build.
//!
//! A gesture belongs to the [`Placed`](crate::Placed) whose source started
//! it, and ends there. Targets in another `Placed` on the same context do not
//! see it; files dragged in from outside are resolved by each `Placed` over
//! its own targets.
//!
//! # Previewing a reorder
//!
//! The application's data does not change until the drop. While the pointer
//! is over a target, [`Dragging::index_hint`] says where the payload would
//! land among the target's items, and the view builds that order; the layout
//! slides to it. Cancelling needs no undo.
//!
//! # Outside drags
//!
//! eframe on winit 0.30 gives no pointer position during an OS drag (some
//! platforms give one at the drop). The library never guesses: a target is
//! `drag_over` only while pointer events arrive during the drag. A drop with
//! no current position goes to the first target that declares
//! [`drop_fallback`](NodeHandle::drop_fallback), with `position` and
//! `index_hint` unset. When eframe moves to winit 0.31, whose drag events
//! carry positions, the same pointer events begin to arrive and targets
//! begin to be `drag_over`; applications do not change.

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Context, DragAndDrop, DroppedFileHandle, Event, Id, Pos2, Rect, Vec2, vec2};

use crate::draw::{Lift, children_of};
use crate::key::root_id;
use crate::overlay_input::Drag;
use crate::pipeline::{Flow, PlacedNode};
use crate::roots::{PlacedRoot, root_at, root_of, stacked_top_down};
use crate::tw::Sensing;

type Erased = Arc<dyn Any + Send + Sync>;

/// How a drag source looks while it is dragged.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DragImage {
    /// The node itself, painted at the pointer on a layer above everything,
    /// unclipped. It stays live: whatever it animates keeps animating. Its
    /// slot keeps its layout.
    #[default]
    Live,
    /// Nothing follows the pointer. The node stays in its slot, where its
    /// [`drag_lifted`](crate::Tw::drag_lifted) look applies.
    ///
    /// A [`drag_preview`](crate::NodeHandle::drag_preview), when the source
    /// has one, is the image whatever this says.
    None,
}

/// What a target receives when a payload is dropped on it.
#[derive(Debug)]
pub struct Drop<T> {
    /// What was dragged.
    pub payload: Arc<T>,
    /// Where the pointer was, from the top-left of the target. `None` for a
    /// drop from outside the app on a platform that gave no position.
    pub position: Option<Vec2>,
    /// Where the payload lands among the target's items: its direct children
    /// that are drag sources or hold one, the dragged one left out. See
    /// [`insertion_index`]. `None` when the target has no items or the
    /// position is unknown.
    pub index_hint: Option<usize>,
}

/// Why a drag ended without a drop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The user pressed Esc.
    Esc,
    /// The button was released over no target that accepts the payload.
    Outside,
    /// The window lost focus; a release outside the window may never arrive.
    FocusLost,
    /// The source node left the tree during the drag.
    SourceGone,
}

/// How a drag ended. Delivered once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragEnd {
    /// A target took the payload.
    Dropped,
    /// Nothing took it.
    Cancelled(Cause),
}

/// One file of an outside drag: hovering over the window or dropped on it.
#[derive(Clone, Debug)]
pub struct File {
    path: Option<PathBuf>,
    mime: String,
    handle: Option<DroppedFileHandle>,
}

impl File {
    /// The path of the file. Native only for a hovering file; on the web a
    /// dropped file has just its name, and a hovering one has none.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file's extension, lowercased.
    pub fn extension(&self) -> Option<String> {
        let extension = self.path.as_deref()?.extension()?;
        Some(extension.to_string_lossy().to_lowercase())
    }

    /// The mime type, where the platform gives one (the web does, while the
    /// file hovers; native does not).
    pub fn mime(&self) -> &str {
        &self.mime
    }

    /// The dropped file, to read its contents. `None` while it hovers.
    pub fn handle(&self) -> Option<&DroppedFileHandle> {
        self.handle.as_ref()
    }
}

/// The payload of a drag from outside the app: the files being dragged.
#[derive(Clone, Debug, Default)]
pub struct Files(Vec<File>);

impl std::ops::Deref for Files {
    type Target = [File];

    fn deref(&self) -> &[File] {
        &self.0
    }
}

/// What the build knows of the drag in progress, for the view to show a gap
/// or a placeholder. Read with [`Builder::dragging`](crate::Builder::dragging).
///
/// Where the pointer is over lags the pointer by a frame: it is the answer of
/// the last frame drawn.
#[derive(Debug)]
pub struct Dragging<T> {
    /// What is dragged.
    pub payload: Arc<T>,
    /// The source node of an in-app drag; `None` for files from outside.
    pub source: Option<Id>,
    /// The target the payload is over, if any.
    pub over: Option<Over>,
}

impl<T> Dragging<T> {
    /// Where the payload would land among the items of the target it is
    /// over. See [`Drop::index_hint`].
    pub fn index_hint(&self) -> Option<usize> {
        self.over?.index_hint
    }

    /// [`index_hint`](Self::index_hint) if the target `target` is the one the
    /// payload is over: of nested targets, only the innermost that takes the
    /// payload is.
    pub fn index_hint_in(&self, target: Id) -> Option<usize> {
        self.over.filter(|over| over.target == target)?.index_hint
    }
}

/// The target a drag is over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Over {
    /// The target node.
    pub target: Id,
    /// The pointer, from the top-left of the target.
    pub position: Vec2,
    /// See [`Drop::index_hint`].
    pub index_hint: Option<usize>,
}

/// Where a payload dropped at `pos` goes among items laid out in reading
/// order: the index of the first item that is after the pointer.
///
/// An item is after the pointer when it starts below `pos`, or when it is on
/// the pointer's row and the pointer has not passed its middle. The middle is
/// the horizontal one, except for an item as wide as all the items together
/// (a list, one column of a grid): that item's row is itself, and its
/// vertical middle decides. `rects` are the items' final layout rects, the
/// dragged one left out, in order.
pub fn insertion_index(rects: &[Rect], pos: Pos2) -> usize {
    let extent = rects
        .iter()
        .fold(Rect::NOTHING, |all, rect| all.union(*rect));
    rects
        .iter()
        .position(|rect| {
            let passed = if rect.width() >= extent.width() - SAME_COLUMN {
                pos.y >= rect.center().y
            } else {
                pos.x >= rect.center().x
            };
            pos.y < rect.min.y || (pos.y <= rect.max.y && !passed)
        })
        .unwrap_or(rects.len())
}

/// How much narrower than all the items together an item may be and still
/// count as filling a row, in points.
const SAME_COLUMN: f32 = 0.5;

/// How close to a viewport edge a drag starts to scroll it, in points.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct EdgeBand(f32);

impl EdgeBand {
    /// A band of `points`, at least zero.
    pub fn new(points: f32) -> Self {
        Self(points.max(0.0))
    }
}

impl From<EdgeBand> for f32 {
    fn from(band: EdgeBand) -> f32 {
        band.0
    }
}

/// How fast a drag at the very edge scrolls, in points per second.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ScrollSpeed(f32);

impl ScrollSpeed {
    /// `points_per_second`, at least zero.
    pub fn new(points_per_second: f32) -> Self {
        Self(points_per_second.max(0.0))
    }
}

impl From<ScrollSpeed> for f32 {
    fn from(speed: ScrollSpeed) -> f32 {
        speed.0
    }
}

/// How a drag near the edge of a viewport asks it to scroll. A container
/// (egui's `ScrollArea` today, a native one in M5) applies the velocity; this
/// is the drag's side of it: the policy of band and speed, for in-app drags
/// and outside ones alike. See [`drag_pointer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeScroll {
    /// The depth of the band along each edge.
    pub band: EdgeBand,
    /// The speed at the edge itself; it falls linearly to zero across the
    /// band.
    pub speed: ScrollSpeed,
}

impl Default for EdgeScroll {
    fn default() -> Self {
        Self {
            band: EdgeBand::new(48.0),
            speed: ScrollSpeed::new(600.0),
        }
    }
}

impl EdgeScroll {
    /// The scroll velocity in points per second for a pointer at `pointer`
    /// over `viewport`: positive y scrolls down, positive x scrolls right.
    pub fn velocity(&self, viewport: Rect, pointer: Pos2) -> Vec2 {
        let (band, speed) = (f32::from(self.band), f32::from(self.speed));
        let along = |at: f32, min: f32, max: f32| {
            if band == 0.0 {
                return 0.0;
            }
            let near_min = ((min + band - at) / band).clamp(0.0, 1.0);
            let near_max = ((at - (max - band)) / band).clamp(0.0, 1.0);
            speed * (near_max - near_min)
        };
        vec2(
            along(pointer.x, viewport.min.x, viewport.max.x),
            along(pointer.y, viewport.min.y, viewport.max.y),
        )
    }
}

/// What a node declares of itself in a drag.
#[derive(Debug, Default)]
pub(crate) struct NodeDrag {
    pub(crate) source: Option<Source>,
    pub(crate) image: DragImage,
    /// The source declared a drag preview, which is its image.
    pub(crate) previewed: bool,
    pub(crate) drops: Vec<usize>,
    pub(crate) fallback: bool,
}

/// A drag source: what it carries, and where a drag that lifts it starts.
#[derive(Debug)]
pub(crate) struct Source {
    pub(crate) payload: Erased,
    pub(crate) activator: Activator,
    /// What the node sensed before it became a source, which it goes back
    /// to when a handle takes its drags.
    pub(crate) declared: Sensing,
}

/// Where a drag that lifts a source starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Activator {
    /// Anywhere on the source.
    Itself,
    /// Only on the handle, the node with this id in the source's subtree.
    Handle(Id),
}

/// The source a drag starting on the node at `index` lifts: the node itself
/// if it is a source with no handle, or the source whose handle it is.
pub(crate) fn lifted_by(nodes: &[PlacedNode], index: usize) -> Option<usize> {
    let activator = |at: usize| {
        let source = nodes[at].drag.as_ref()?.source.as_ref()?;
        Some(source.activator)
    };
    if activator(index) == Some(Activator::Itself) {
        return Some(index);
    }
    let handle = Activator::Handle(nodes[index].id()?);
    (0..index)
        .rev()
        .filter(|&above| nodes[above].end > index)
        .find(|&above| activator(above) == Some(handle))
}

/// Shows the pointer what a press would do: `Grab` over what starts a drag
/// while none is live, `Grabbing` for the whole of a live one.
pub(crate) fn show_grab(
    ctx: &Context,
    nodes: &[PlacedNode],
    index: usize,
    response: &egui::Response,
) {
    if response.hovered() && live_gesture(ctx).is_none() && lifted_by(nodes, index).is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// The closures a frame declared for drags, for the draw to deliver.
pub(crate) struct Decls<A> {
    drops: Vec<Option<DropDecl<A>>>,
    ends: Vec<Option<EndDecl<A>>>,
}

impl<A> Default for Decls<A> {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            ends: Vec::new(),
        }
    }
}

impl<A: 'static> Decls<A> {
    /// Declares a drop handler and returns its index.
    pub(crate) fn add_drop<T: Any + Send + Sync>(
        &mut self,
        accept: impl Fn(&T) -> bool + 'static,
        handler: impl FnOnce(&mut A, Drop<T>) + 'static,
    ) -> usize {
        self.drops.push(Some(DropDecl {
            accepts: Box::new(move |payload| payload.downcast_ref::<T>().is_some_and(&accept)),
            deliver: Box::new(move |app, payload, landing| {
                if let Ok(payload) = payload.downcast::<T>() {
                    handler(
                        app,
                        Drop {
                            payload,
                            position: landing.position,
                            index_hint: landing.index_hint,
                        },
                    );
                }
            }),
        }));
        self.drops.len() - 1
    }

    pub(crate) fn add_end<T: Any + Send + Sync>(
        &mut self,
        handler: impl FnOnce(&mut A, Arc<T>, DragEnd) + 'static,
    ) {
        self.ends.push(Some(EndDecl {
            matches: |payload| payload.is::<T>(),
            deliver: Box::new(move |app, payload, end| {
                if let Ok(payload) = payload.downcast::<T>() {
                    handler(app, payload, end);
                }
            }),
        }));
    }
}

impl<A> Decls<A> {
    fn is_empty(&self) -> bool {
        self.drops.is_empty() && self.ends.is_empty()
    }

    fn accepts(&self, node: &PlacedNode, payload: &Erased) -> bool {
        node.drag.as_ref().is_some_and(|drag| {
            drag.drops.iter().any(|&decl| {
                self.drops[decl]
                    .as_ref()
                    .is_some_and(|decl| (decl.accepts)(&**payload))
            })
        })
    }

    fn drop_on(&mut self, node: &PlacedNode, payload: &Erased, app: &mut A, landing: Landing) {
        let Some(drag) = &node.drag else { return };
        let taking = drag.drops.iter().copied().find(|&decl| {
            self.drops[decl]
                .as_ref()
                .is_some_and(|decl| (decl.accepts)(&**payload))
        });
        if let Some(decl) = taking.and_then(|decl| self.drops[decl].take()) {
            (decl.deliver)(app, payload.clone(), landing);
        }
    }

    fn end(&mut self, app: &mut A, payload: &Erased, end: DragEnd) {
        for slot in &mut self.ends {
            if slot.as_ref().is_some_and(|decl| (decl.matches)(&**payload))
                && let Some(decl) = slot.take()
            {
                (decl.deliver)(app, payload.clone(), end);
            }
        }
    }
}

/// A handler of one outcome for a payload.
type Delivery<A, Outcome> = Box<dyn FnOnce(&mut A, Erased, Outcome)>;

type Accepts = Box<dyn Fn(&(dyn Any + Send + Sync)) -> bool>;

struct DropDecl<A> {
    accepts: Accepts,
    deliver: Delivery<A, Landing>,
}

type Matches = fn(&(dyn Any + Send + Sync)) -> bool;

struct EndDecl<A> {
    matches: Matches,
    deliver: Delivery<A, DragEnd>,
}

#[derive(Clone, Copy)]
struct Landing {
    position: Option<Vec2>,
    index_hint: Option<usize>,
}

/// The egui payload of an in-app drag. egui clears it on Esc and, at the end
/// of the pass, on release, which is the cancel signal and the cursor the
/// library wants; the payload itself is in the [`Gesture`].
struct Lifted;

/// An in-app drag, kept in the egui context.
#[derive(Clone)]
struct Gesture {
    scope: Id,
    source: Id,
    payload: Erased,
    /// The pointer's offset from the source's top-left when it was lifted.
    grab: Vec2,
    /// Where the pointer last was, for the frames in which it is not
    /// reported (it has left the window with the button held).
    last: Pos2,
    /// The last pass the gesture's scope drew.
    touched: u64,
}

/// What the last draw found the drag over.
#[derive(Clone, Copy)]
struct Hovering {
    pass: u64,
    over: Option<Over>,
}

/// Whether an outside drag has a position, as of a pass.
#[derive(Clone, Copy)]
struct Positioned {
    pass: u64,
    dragging: bool,
    known: bool,
}

fn key(name: &str) -> Id {
    root_id().with(("dnd", name))
}

/// What the pointer and window did this pass.
struct Input {
    released: bool,
    interact: Option<Pos2>,
    latest: Option<Pos2>,
    press_origin: Option<Pos2>,
    moved: bool,
    focus_lost: bool,
    hovered: Option<Files>,
    dropped: Option<Files>,
}

impl Input {
    fn read(ctx: &Context) -> Self {
        ctx.input(|i| Self {
            released: i.pointer.any_released(),
            interact: i.pointer.interact_pos(),
            latest: i.pointer.latest_pos(),
            press_origin: i.pointer.press_origin(),
            moved: i.events.iter().any(|e| matches!(e, Event::PointerMoved(_))),
            focus_lost: focus_lost(&i.events),
            hovered: (!i.raw.hovered_files.is_empty()).then(|| {
                Files(
                    i.raw
                        .hovered_files
                        .iter()
                        .map(|file| File {
                            path: file.path.clone(),
                            mime: file.mime.clone(),
                            handle: None,
                        })
                        .collect(),
                )
            }),
            dropped: (!i.raw.dropped_files.is_empty()).then(|| {
                Files(
                    i.raw
                        .dropped_files
                        .iter()
                        .map(|file| File {
                            path: Some(file.path().to_owned()),
                            mime: String::new(),
                            handle: Some(file.clone()),
                        })
                        .collect(),
                )
            }),
        })
    }
}

fn focus_lost(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::WindowFocused(false)))
}

/// The in-app drag in progress this pass: its payload is held and this pass
/// is not its last.
fn live_gesture(ctx: &Context) -> Option<Gesture> {
    let gesture = ctx.data(|data| data.get_temp::<Gesture>(key("gesture")))?;
    let (released, lost) = ctx.input(|i| (i.pointer.any_released(), focus_lost(&i.events)));
    (DragAndDrop::has_payload_of_type::<Lifted>(ctx) && !released && !lost).then_some(gesture)
}

/// Whether an in-app drag is live this pass.
pub(crate) fn drag_state(ctx: &Context) -> Drag {
    if live_gesture(ctx).is_some() {
        Drag::Live
    } else {
        Drag::Idle
    }
}

fn fresh(ctx: &Context, pass: u64) -> bool {
    ctx.cumulative_pass_nr().saturating_sub(pass) <= 1
}

/// What the build reads: see [`Builder::dragging`](crate::Builder::dragging).
pub(crate) fn dragging<T: Any + Send + Sync>(ctx: &Context) -> Option<Dragging<T>> {
    let (payload, source): (Erased, Option<Id>) = match live_gesture(ctx) {
        Some(gesture) => (gesture.payload, Some(gesture.source)),
        None => (Arc::new(Input::read(ctx).hovered?), None),
    };
    let over = ctx
        .data(|data| data.get_temp::<Hovering>(key("hovering")))
        .filter(|hovering| fresh(ctx, hovering.pass))
        .and_then(|hovering| hovering.over);
    Some(Dragging {
        payload: payload.downcast().ok()?,
        source,
        over,
    })
}

/// The source of the in-app drag in progress, as the build sees it.
pub(crate) fn lifted_source(ctx: &Context) -> Option<Id> {
    live_gesture(ctx).map(|gesture| gesture.source)
}

/// The id of the node a source's [`drag_preview`](crate::NodeHandle::drag_preview)
/// is built under: derived from the source's, so it is stable and unique.
pub(crate) fn preview_id(source: Id) -> Id {
    source.with("eggshell-drag-preview")
}

/// How far into the source the pointer was when it was lifted, from the
/// source's top-left; zero when nothing is lifted. A
/// [`drag_preview`](crate::NodeHandle::drag_preview) is anchored so that its
/// top-left is this far behind the pointer, as the live image is: a preview
/// that should sit at the pointer itself offsets its content by it.
pub fn drag_grab(ctx: &Context) -> Vec2 {
    live_gesture(ctx).map_or(Vec2::ZERO, |gesture| gesture.grab)
}

/// Where the pointer is while a drag is live and its position is known: an
/// in-app drag, or an outside one that has had pointer events. The input of
/// [`EdgeScroll::velocity`], after the frame is drawn.
pub fn drag_pointer(ctx: &Context) -> Option<Pos2> {
    if live_gesture(ctx).is_some() {
        return ctx.input(|i| i.pointer.interact_pos());
    }
    let positioned = ctx.data(|data| data.get_temp::<Positioned>(key("positioned")))?;
    if fresh(ctx, positioned.pass) && positioned.dragging && positioned.known {
        ctx.input(|i| i.pointer.latest_pos())
    } else {
        None
    }
}

/// Whether an outside drag has a position this pass. It does once pointer
/// events have arrived since the files appeared, or in the pass they drop;
/// a pointer position from before the drag entered is not one.
fn outside_position_known(ctx: &Context, pass: u64, input: &Input) -> bool {
    let key = key("positioned");
    let dragging = input.hovered.is_some() || input.dropped.is_some();
    let previous = ctx.data(|data| data.get_temp::<Positioned>(key));
    if let Some(done) = previous.filter(|previous| previous.pass == pass) {
        return done.known;
    }
    let was_dragging =
        previous.is_some_and(|previous| previous.dragging && fresh(ctx, previous.pass));
    let known = dragging
        && (input.moved || (was_dragging && previous.is_some_and(|previous| previous.known)));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            Positioned {
                pass,
                dragging,
                known,
            },
        );
    });
    known
}

/// The targets of a frame in stacking order, with the part of each the
/// pointer can reach: its final layout rect, cut by the clips above it. The
/// page's come first, then each overlay's from the bottom of the stack up, so
/// that the last to contain the pointer is the topmost.
fn targets(nodes: &[PlacedNode], roots: &[PlacedRoot], clip: Rect) -> Vec<(usize, Rect)> {
    fn walk(nodes: &[PlacedNode], index: usize, clip: Rect, found: &mut Vec<(usize, Rect)>) {
        let node = &nodes[index];
        if node.flow == Flow::Hidden || node.is_inert() {
            return;
        }
        if node
            .drag
            .as_ref()
            .is_some_and(|drag| !drag.drops.is_empty())
        {
            found.push((index, node.rect().intersect(clip)));
        }
        let inner = node.clip_for_children(clip);
        let mut children = children_of(nodes, index);
        children.sort_by_key(|&child| nodes[child].z());
        for child in children {
            walk(nodes, child, inner, found);
        }
    }
    let mut found = Vec::new();
    walk(nodes, 0, clip, &mut found);
    for root in stacked_top_down(roots).into_iter().rev() {
        walk(nodes, roots[root].first, clip, &mut found);
    }
    found
}

/// Where the payload would land among the items of the target at `index`: its
/// direct children that are drag sources or hold one, the one that holds the
/// dragged source left out.
fn item_hint(nodes: &[PlacedNode], index: usize, leaving: Option<Id>, pos: Pos2) -> Option<usize> {
    let holds = |child: usize, wanted: &dyn Fn(&PlacedNode) -> bool| {
        nodes[child..nodes[child].end].iter().any(wanted)
    };
    let rects: Vec<Rect> = children_of(nodes, index)
        .into_iter()
        .filter(|&child| nodes[child].flow != Flow::Hidden)
        .filter(|&child| holds(child, &PlacedNode::is_drag_source))
        .filter(|&child| leaving.is_none_or(|id| !holds(child, &|node| node.id() == Some(id))))
        .map(|child| nodes[child].rect())
        .collect();
    (!rects.is_empty()).then(|| insertion_index(&rects, pos))
}

/// The target a payload is over, and the targets that would take it.
struct Resolved {
    accepting: Vec<(usize, Rect)>,
    over: Option<Hit>,
}

struct Hit {
    index: usize,
    position: Vec2,
    index_hint: Option<usize>,
}

fn resolve<A>(
    nodes: &[PlacedNode],
    roots: &[PlacedRoot],
    decls: &Decls<A>,
    payload: &Erased,
    pointer: Option<Pos2>,
    leaving: Option<Id>,
    clip: Rect,
) -> Resolved {
    let accepting: Vec<(usize, Rect)> = targets(nodes, roots, clip)
        .into_iter()
        .filter(|&(index, _)| decls.accepts(&nodes[index], payload))
        .collect();
    let over = pointer.and_then(|pos| {
        let under = root_at(roots, nodes, pos);
        let (index, _) = accepting
            .iter()
            .rfind(|&&(index, reach)| root_of(roots, index) == under && reach.contains(pos))?;
        Some(Hit {
            index: *index,
            position: pos - nodes[*index].rect().min,
            index_hint: item_hint(nodes, *index, leaving, pos),
        })
    });
    Resolved { accepting, over }
}

/// How a frame's drag ends, once it is painted.
enum Ending {
    Nothing,
    /// The button was released.
    Released {
        gesture: Gesture,
        hit: Option<Hit>,
    },
    /// Files were dropped from outside.
    Dropped {
        files: Erased,
        landing: Option<(usize, Landing)>,
    },
}

/// What a drag makes of a node, for its paint variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Stakes {
    pub(crate) accepts: bool,
    pub(crate) over: bool,
    pub(crate) lifted: bool,
}

/// The drag, as one draw of a [`Placed`](crate::Placed) sees it.
pub(crate) struct Draw {
    pub(crate) lift: Option<Lift>,
    accepting: Vec<bool>,
    over: Option<usize>,
    lifted: Option<usize>,
}

/// What is left to do once the frame is painted.
pub(crate) struct Finish {
    ending: Ending,
}

impl Draw {
    fn idle() -> Self {
        Self {
            lift: None,
            accepting: Vec::new(),
            over: None,
            lifted: None,
        }
    }

    /// Whether the lifted node, or the node painted as its image, is among
    /// the nodes `within`.
    pub(crate) fn involves(&self, within: &std::ops::Range<usize>) -> bool {
        self.lifted.is_some_and(|index| within.contains(&index))
            || self.lift.is_some_and(|lift| within.contains(&lift.index))
    }

    pub(crate) fn stakes(&self, index: usize) -> Stakes {
        Stakes {
            accepts: self.accepting.get(index).copied().unwrap_or(false),
            over: self.over == Some(index),
            lifted: self.lifted == Some(index),
        }
    }
}

/// The laid-out frame a drag is resolved against.
#[derive(Clone, Copy)]
pub(crate) struct Tree<'a> {
    pub(crate) nodes: &'a [PlacedNode],
    pub(crate) by_id: &'a HashMap<Id, usize>,
    pub(crate) roots: &'a [PlacedRoot],
}

/// Starts a draw: begins a gesture a source started, ends one that was
/// cancelled, and resolves what the drag is over.
pub(crate) fn begin<A>(
    ctx: &Context,
    scope: Id,
    tree: Tree<'_>,
    clip: Rect,
    decls: &mut Decls<A>,
    app: &mut A,
) -> (Draw, Finish) {
    let Tree {
        nodes,
        by_id,
        roots,
    } = tree;
    let pass = ctx.cumulative_pass_nr();
    let input = Input::read(ctx);
    let positioned = outside_position_known(ctx, pass, &input);
    let mut gesture = ctx.data(|data| data.get_temp::<Gesture>(key("gesture")));
    let outside = input.dropped.is_some() || input.hovered.is_some();
    if gesture.is_none() && decls.is_empty() && !outside {
        return Finish::nothing();
    }
    if gesture
        .as_ref()
        .is_some_and(|g| g.scope != scope && g.touched + 1 < pass)
    {
        // Its scope is no longer drawn, so nothing will ever end it, and
        // nothing of this scope can deliver the end.
        clear_gesture(ctx);
        gesture = None;
    }
    if gesture.is_none() {
        gesture = lift(ctx, scope, nodes, by_id, &input, pass);
    }
    let mut gesture = gesture.filter(|gesture| gesture.scope == scope);
    if let Some(held) = &gesture {
        let source_here = by_id
            .get(&held.source)
            .is_some_and(|&at| nodes[at].is_drag_source());
        let cause = if input.focus_lost {
            Some(Cause::FocusLost)
        } else if !DragAndDrop::has_payload_of_type::<Lifted>(ctx) && !input.released {
            Some(Cause::Esc)
        } else if !source_here || held.touched + 1 < pass {
            Some(Cause::SourceGone)
        } else {
            None
        };
        if let Some(cause) = cause {
            ctx.stop_dragging();
            end_gesture(ctx, held, DragEnd::Cancelled(cause), decls, app);
            gesture = None;
        }
    }
    if let Some(held) = &mut gesture {
        held.touched = pass;
        held.last = input.interact.unwrap_or(held.last);
        ctx.data_mut(|data| data.insert_temp(key("gesture"), held.clone()));
    }

    let (payload, pointer, leaving): (Erased, Option<Pos2>, Option<Id>) =
        match (&gesture, &input.dropped, &input.hovered) {
            (Some(held), ..) => (held.payload.clone(), input.interact, Some(held.source)),
            (None, Some(files), _) | (None, None, Some(files)) => (
                Arc::new(files.clone()),
                input.latest.filter(|_| positioned),
                None,
            ),
            // Another tree's drag, or none: what this tree would write is not
            // its to change. A record nobody refreshes goes stale after a pass.
            _ => return Finish::nothing(),
        };
    let found = resolve(nodes, roots, decls, &payload, pointer, leaving, clip);

    let releasing = gesture.is_some() && input.released;
    let dropping = gesture.is_none() && input.dropped.is_some();
    if releasing || dropping {
        ctx.data_mut(|data| data.remove::<Hovering>(key("hovering")));
        let ending = match gesture {
            Some(gesture) => Ending::Released {
                gesture,
                hit: found.over,
            },
            None => Ending::Dropped {
                landing: landing_of(nodes, &found, positioned),
                files: payload,
            },
        };
        return (Draw::idle(), Finish { ending });
    }

    let over = found.over.as_ref().map(|hit| Over {
        target: nodes[hit.index].id().expect("a target is identified"),
        position: hit.position,
        index_hint: hit.index_hint,
    });
    // Several trees resolve outside files in one pass; only one with a target
    // that takes them may say where they would land.
    if gesture.is_some() || !found.accepting.is_empty() {
        ctx.data_mut(|data| data.insert_temp(key("hovering"), Hovering { pass, over }));
    }

    let mut draw = Draw::idle();
    draw.accepting = vec![false; nodes.len()];
    for &(index, _) in &found.accepting {
        draw.accepting[index] = true;
    }
    draw.over = found.over.as_ref().map(|hit| hit.index);
    draw.lifted = gesture
        .as_ref()
        .and_then(|held| by_id.get(&held.source).copied());
    if let Some(held) = &gesture
        && let Some(&at) = by_id.get(&held.source)
    {
        let pointer = pointer.unwrap_or(held.last);
        let image = nodes[at]
            .drag
            .as_ref()
            .map(|drag| (&drag.image, drag.previewed));
        let carried = match image {
            Some((_, true)) => by_id.get(&preview_id(held.source)).copied(),
            Some((DragImage::Live, false)) => Some(at),
            Some((DragImage::None, false)) | None => None,
        };
        draw.lift = carried.map(|index| Lift {
            index,
            min: pointer - held.grab,
        });
    }
    (
        draw,
        Finish {
            ending: Ending::Nothing,
        },
    )
}

/// The target a drop from outside lands on, and where: under the pointer if
/// the platform gave one, else the topmost accepting target that declared
/// itself the fallback.
fn landing_of(
    nodes: &[PlacedNode],
    found: &Resolved,
    positioned: bool,
) -> Option<(usize, Landing)> {
    if positioned {
        let hit = found.over.as_ref()?;
        return Some((
            hit.index,
            Landing {
                position: Some(hit.position),
                index_hint: hit.index_hint,
            },
        ));
    }
    let (index, _) = found.accepting.iter().rfind(|(index, _)| {
        nodes[*index]
            .drag
            .as_ref()
            .is_some_and(|drag| drag.fallback)
    })?;
    Some((
        *index,
        Landing {
            position: None,
            index_hint: None,
        },
    ))
}

/// Begins the gesture of the source the pointer has just started dragging.
fn lift(
    ctx: &Context,
    scope: Id,
    nodes: &[PlacedNode],
    by_id: &HashMap<Id, usize>,
    input: &Input,
    pass: u64,
) -> Option<Gesture> {
    let started = ctx.drag_started_id()?;
    let at_source = lifted_by(nodes, *by_id.get(&started)?)?;
    let node = &nodes[at_source];
    let payload = node.drag.as_ref()?.source.as_ref()?.payload.clone();
    let at = input.press_origin.or(input.interact)?;
    DragAndDrop::set_payload(ctx, Lifted);
    Some(Gesture {
        scope,
        source: node.id()?,
        payload,
        grab: at - node.rect().min,
        last: at,
        touched: pass,
    })
}

fn clear_gesture(ctx: &Context) {
    ctx.data_mut(|data| {
        data.remove::<Gesture>(key("gesture"));
        data.remove::<Hovering>(key("hovering"));
    });
    DragAndDrop::clear_payload(ctx);
}

fn end_gesture<A>(
    ctx: &Context,
    gesture: &Gesture,
    end: DragEnd,
    decls: &mut Decls<A>,
    app: &mut A,
) {
    clear_gesture(ctx);
    decls.end(app, &gesture.payload, end);
}

impl Finish {
    fn nothing() -> (Draw, Self) {
        (
            Draw::idle(),
            Self {
                ending: Ending::Nothing,
            },
        )
    }

    /// Ends the draw: delivers a drop and ends the gesture.
    pub(crate) fn finish<A>(
        self,
        ctx: &Context,
        nodes: &[PlacedNode],
        decls: &mut Decls<A>,
        app: &mut A,
    ) {
        if live_gesture(ctx).is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        match self.ending {
            Ending::Nothing => {}
            Ending::Released { gesture, hit } => {
                let end = match hit {
                    Some(hit) => {
                        let landing = Landing {
                            position: Some(hit.position),
                            index_hint: hit.index_hint,
                        };
                        decls.drop_on(&nodes[hit.index], &gesture.payload, app, landing);
                        DragEnd::Dropped
                    }
                    None => DragEnd::Cancelled(Cause::Outside),
                };
                end_gesture(ctx, &gesture, end, decls, app);
            }
            Ending::Dropped { files, landing } => {
                if let Some((index, landing)) = landing {
                    decls.drop_on(&nodes[index], &files, app, landing);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), vec2(10.0, 10.0))
    }

    fn grid(columns: usize, count: usize) -> Vec<Rect> {
        (0..count)
            .map(|i| rect(10.0 * (i % columns) as f32, 10.0 * (i / columns) as f32))
            .collect()
    }

    #[test]
    fn a_wide_list_is_split_at_each_rows_vertical_middle() {
        let rows: Vec<Rect> = (0..3)
            .map(|i| Rect::from_min_size(Pos2::new(0.0, 12.0 * i as f32), vec2(100.0, 10.0)))
            .collect();
        let at = |x, y| insertion_index(&rows, Pos2::new(x, y));
        assert_eq!(at(5.0, 2.0), 0, "top half of the first row: before it");
        assert_eq!(at(95.0, 2.0), 0, "the pointer's x does not matter");
        assert_eq!(at(5.0, 8.0), 1, "bottom half: after it");
        assert_eq!(at(95.0, 8.0), 1);
        assert_eq!(
            at(5.0, 11.0),
            1,
            "the gap between rows reads as the next row"
        );
        assert_eq!(at(5.0, 100.0), 3, "below everything: the end");
        assert_eq!(at(5.0, -5.0), 0, "above everything: the start");
    }

    #[test]
    fn a_grid_is_split_at_each_cells_horizontal_middle() {
        let cells = grid(3, 7);
        let at = |x, y| insertion_index(&cells, Pos2::new(x, y));
        assert_eq!(at(14.0, 4.0), 1, "left of the middle of the second cell");
        assert_eq!(at(16.0, 4.0), 2, "right of it");
        assert_eq!(at(29.5, 4.0), 3, "end of the row: the start of the next");
        assert_eq!(at(1.0, 10.5), 3, "across the row boundary");
        assert_eq!(at(14.0, 14.0), 4);
        assert_eq!(at(2.0, 25.0), 6, "left of the lone cell of the last row");
        assert_eq!(at(9.0, 25.0), 7, "right of it");
        assert_eq!(at(9.0, 40.0), 7);
    }

    #[test]
    fn the_hint_is_a_fixed_point_of_its_own_preview() {
        let slots = grid(3, 6);
        let pointer = Pos2::new(14.0, 14.0);
        let hint_with_gap_at = |gap: usize| {
            let others: Vec<Rect> = (0..5)
                .map(|item| slots[if item < gap { item } else { item + 1 }])
                .collect();
            insertion_index(&others, pointer)
        };
        for start in 0..6 {
            let (mut gap, mut settled_after) = (start, 0);
            for step in 1..=3 {
                let next = hint_with_gap_at(gap);
                if next != gap {
                    settled_after = step;
                }
                gap = next;
            }
            assert!(
                settled_after <= 1,
                "from {start}: still moving after {settled_after}"
            );
            assert_eq!(
                hint_with_gap_at(gap),
                gap,
                "from {start}: not a fixed point"
            );
        }
    }

    #[test]
    fn edge_scroll_is_proportional_to_the_depth_in_the_band() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(200.0, 400.0));
        let scroll = EdgeScroll::default();
        let at = |x, y| scroll.velocity(view, Pos2::new(x, y));
        assert_eq!(at(100.0, 200.0), Vec2::ZERO);
        assert_eq!(at(100.0, 400.0), vec2(0.0, 600.0));
        assert_eq!(at(100.0, 0.0), vec2(0.0, -600.0));
        assert_eq!(at(100.0, 376.0), vec2(0.0, 300.0));
        assert_eq!(at(0.0, 200.0), vec2(-600.0, 0.0));
        assert_eq!(
            EdgeScroll {
                band: EdgeBand::new(0.0),
                ..scroll
            }
            .velocity(view, Pos2::new(0.0, 0.0)),
            Vec2::ZERO,
            "no band, no scrolling"
        );
    }

    mod resolving {
        use super::super::*;
        use crate::testing::Headless;
        use crate::{Builder, Placed, tw};

        struct Card;
        struct Tag;

        fn erased<T: Any + Send + Sync>(payload: T) -> Erased {
            Arc::new(payload)
        }

        fn placed(view: impl for<'f> FnOnce(&mut Builder<'_, 'f, ()>, &'f ())) -> Placed<()> {
            Headless::new(vec2(400.0, 400.0)).layout(&(), view)
        }

        fn over<T: Any + Send + Sync>(
            placed: &Placed<()>,
            payload: T,
            at: Pos2,
            leaving: Option<Id>,
        ) -> Option<(String, Option<usize>)> {
            let found = resolve(
                &placed.nodes,
                &placed.roots,
                &placed.drag,
                &erased(payload),
                Some(at),
                leaving,
                Rect::EVERYTHING,
            );
            let hit = found.over?;
            Some((placed.nodes[hit.index].path().to_owned(), hit.index_hint))
        }

        fn nested(b: &mut Builder<'_, '_, ()>, outer_takes_tags: bool) {
            b.div_id("outer", tw().w_px(300.0).h_px(300.0).p(10.0), |b| {
                b.div_id("inner", tw().w_px(100.0).h_px(100.0), |_| {})
                    .drop_target::<Card>(|_, _| {});
            })
            .drop_target::<Card>(|_, _| {})
            .drop_target_if::<Tag>(move |_| outer_takes_tags, |_, _| {});
        }

        #[test]
        fn of_nested_targets_the_innermost_that_takes_the_payload_wins() {
            let placed = placed(|b, _| nested(b, true));
            let path = |at| over(&placed, Card, at, None).map(|hit| hit.0);
            assert_eq!(
                path(Pos2::new(50.0, 50.0)).as_deref(),
                Some(r#"/"outer"/"inner""#)
            );
            assert_eq!(
                path(Pos2::new(250.0, 250.0)).as_deref(),
                Some(r#"/"outer""#)
            );
            assert_eq!(path(Pos2::new(350.0, 350.0)), None, "outside both");
        }

        #[test]
        fn an_inner_target_that_refuses_the_payload_does_not_claim_the_pointer() {
            let placed = placed(|b, _| nested(b, true));
            let hit = over(&placed, Tag, Pos2::new(50.0, 50.0), None);
            assert_eq!(hit.map(|hit| hit.0).as_deref(), Some(r#"/"outer""#));
            let placed = self::placed(|b, _| nested(b, false));
            assert_eq!(over(&placed, Tag, Pos2::new(50.0, 50.0), None), None);
        }

        #[test]
        fn a_target_is_only_reachable_inside_the_clips_above_it() {
            let placed = placed(|b, _| {
                b.div_id(
                    "window",
                    tw().w_px(100.0).h_px(100.0).overflow_hidden(),
                    |b| {
                        b.div_id("tall", tw().w_px(100.0).h_px(300.0), |_| {})
                            .drop_target::<Card>(|_, _| {});
                    },
                );
            });
            assert!(over(&placed, Card, Pos2::new(50.0, 50.0), None).is_some());
            assert_eq!(
                over(&placed, Card, Pos2::new(50.0, 200.0), None),
                None,
                "the part of the target the window clips away"
            );
        }

        fn popover_over_the_zone(
            nav: crate::overlay::Nav,
            band: crate::overlay::Band,
        ) -> Placed<()> {
            placed(|b, _| {
                b.div_id("zone", tw().w_px(100.0).h_px(100.0), |_| {})
                    .drop_target::<Card>(|_, _| {});
                b.overlay("pop")
                    .anchor(crate::overlay::Anchor::Point(Pos2::new(50.0, 50.0)))
                    .nav(nav)
                    .band(band)
                    .children(|s| {
                        s.div_id("box", tw().w_px(100.0).h_px(100.0), |s| {
                            s.div_id("inner", tw().w_px(40.0).h_px(40.0), |_| {})
                                .drop_target::<Card>(|_, _| {});
                        });
                    });
            })
        }

        const ON_BOTH: Pos2 = Pos2::new(95.0, 95.0);
        const ONLY_ZONE: Pos2 = Pos2::new(25.0, 25.0);

        fn path_over(placed: &Placed<()>, at: Pos2) -> Option<String> {
            over(placed, Card, at, None).map(|hit| hit.0)
        }

        #[test]
        fn a_page_target_under_a_popover_is_unreachable() {
            let placed =
                popover_over_the_zone(crate::overlay::Nav::None, crate::overlay::Band::Popup);
            assert!(
                placed
                    .at("zone")
                    .node()
                    .expect("a node")
                    .rect()
                    .contains(ON_BOTH)
                    && placed.overlay("pop").rect().contains(ON_BOTH),
                "precondition: the popover covers part of the zone"
            );
            assert_eq!(path_over(&placed, ON_BOTH), None);
            assert_eq!(path_over(&placed, ONLY_ZONE).as_deref(), Some(r#"/"zone""#));
        }

        #[test]
        fn a_target_inside_a_popover_is_reachable() {
            let placed =
                popover_over_the_zone(crate::overlay::Nav::None, crate::overlay::Band::Popup);
            let inner = placed.overlay("pop").rect().min + vec2(10.0, 10.0);
            assert!(
                placed
                    .at("zone")
                    .node()
                    .expect("a node")
                    .rect()
                    .contains(inner),
                "precondition: the target is over the zone"
            );
            assert_eq!(
                path_over(&placed, inner).as_deref(),
                Some(r#"/"pop"/"box"/"inner""#)
            );
        }

        #[test]
        fn a_swallowing_overlay_covers_every_page_target() {
            let placed =
                popover_over_the_zone(crate::overlay::Nav::Menu, crate::overlay::Band::Popup);
            assert_eq!(path_over(&placed, ONLY_ZONE), None, "even outside its box");
            let inner = placed.overlay("pop").rect().min + vec2(10.0, 10.0);
            assert!(path_over(&placed, inner).is_some(), "its own are reachable");
        }

        #[test]
        fn a_tooltip_leaves_the_page_reachable() {
            let placed =
                popover_over_the_zone(crate::overlay::Nav::None, crate::overlay::Band::Tooltip);
            assert_eq!(path_over(&placed, ON_BOTH).as_deref(), Some(r#"/"zone""#));
        }

        #[test]
        fn of_two_overlays_with_targets_the_one_opened_later_wins() {
            let placed = placed(|b, _| {
                for (key, at) in [("first", 0.0), ("second", 20.0)] {
                    b.overlay(key)
                        .anchor(crate::overlay::Anchor::Point(Pos2::new(at, at)))
                        .children(|s| {
                            s.div_id("box", tw().w_px(100.0).h_px(100.0), |_| {})
                                .drop_target::<Card>(|_, _| {});
                        });
                }
            });
            assert_eq!(
                path_over(&placed, Pos2::new(50.0, 50.0)).as_deref(),
                Some(r#"/"second"/"box""#)
            );
            assert_eq!(
                path_over(&placed, Pos2::new(10.0, 10.0)).as_deref(),
                Some(r#"/"first"/"box""#)
            );
        }

        #[test]
        fn the_index_hint_counts_sources_and_leaves_the_dragged_one_out() {
            let placed = placed(|b, _| {
                b.div_id("list", tw().flex().flex_col().w_px(100.0), |b| {
                    for (key, source) in [("a", true), ("heading", false), ("b", true), ("c", true)]
                    {
                        let node = b.div_id(key, tw().w_px(100.0).h_px(10.0), |_| {});
                        if source {
                            node.drag_source(key);
                        }
                    }
                })
                .drop_target::<&'static str>(|_, _| {});
            });
            let rects: Vec<(String, Rect)> = placed
                .nodes()
                .map(|n| (n.path().to_owned(), n.rect()))
                .collect();
            let at = |path: &str, fraction: f32| {
                let r = rects.iter().find(|(p, _)| p == path).unwrap().1;
                Pos2::new(r.center().x, r.min.y + r.height() * fraction)
            };
            let hint = |pos, leaving| over(&placed, "x", pos, leaving).and_then(|hit| hit.1);
            assert_eq!(
                hint(at(r#"/"list"/"b""#, 0.2), None),
                Some(1),
                "before b: the heading is no item"
            );
            assert_eq!(hint(at(r#"/"list"/"b""#, 0.8), None), Some(2), "after b");
            let leaving = placed
                .nodes()
                .find(|n| n.path() == r#"/"list"/"a""#)
                .and_then(PlacedNode::id);
            assert_eq!(
                hint(at(r#"/"list"/"b""#, 0.2), leaving),
                Some(0),
                "a is the dragged one, so b is the first item"
            );
        }
    }
}
