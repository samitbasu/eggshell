//! The layout debug overlay: a hairline around every node, in red where the
//! node's content did not fit, the viewport (green) and reserved gutter
//! (tint) of every scroll container, a label for the node under the pointer, and a
//! frame-timing readout in the corner farthest from the pointer.

use core::time::Duration;
use std::fmt::Write as _;

use egui::epaint::StrokeKind;
use egui::{
    Align, Align2, Color32, Context, FontId, Id, Key, LayerId, Order, Painter, Pos2, Rect, Shape,
    Stroke, Vec2, ViewportId, vec2,
};
use taffy::Overflow;

use crate::draw::{PixelGrid, add, inset};
use crate::measure::Content;
use crate::nodes::Nodes;
use crate::pipeline::{Flow, PlacedNode};
use crate::prim::Prim;

/// Whether frames are drawn with the layout debug overlay.
///
/// The mode lives in the [`egui::Context`], so one key binding anywhere in
/// the application (see [`Debug::toggle_on`]) governs every
/// [`Placed::draw`](crate::Placed::draw) of the frame. [`Off`](Self::Off) costs
/// one context read per build, layout and draw.
///
/// With [`Outlines`](Self::Outlines), [`Placed::draw`](crate::Placed::draw)
/// outlines the border box of every node, anonymous ones and overlay roots
/// included, with a hairline of one physical pixel on the topmost layer. A
/// node whose content did not fit is outlined in [`Debug::OVERFLOW`], every
/// other in [`Debug::OUTLINE`]. The innermost node under the pointer is
/// labelled with its readable path and rect.
///
/// [`Outlines`](Self::Outlines) also draws a readout, once per frame however
/// many trees are drawn, in the corner of the window farthest from the
/// pointer; while there is no pointer it stays where it was (the top right
/// until the pointer is first seen). It wraps to fit a narrow window, and the
/// node label is drawn over it:
///
/// - `frame`: egui's `unstable_dt`, the measured time since the last pass, and
///   the frames per second averaged over the last 30 passes;
/// - `dt`: egui's `stable_dt` and `predicted_dt`, which decide how far an
///   animation advances per frame, so a slow-motion animation is explainable;
/// - `eggshell`: the time spent in [`LayoutState::build`](crate::LayoutState::build),
///   [`Built::layout`](crate::Built::layout) and [`Placed::draw`](crate::Placed::draw),
///   summed over every tree of the pass (the readout sees the draws before it);
///   under it, a row per native pop-out window, by its title, with the layout
///   and draw of that window, which the page's row leaves out (an embedded
///   window's are in the page's row);
/// - `repaint`: whether egui has been asked to repaint at once, as it is while
///   an animation is in progress, or is idle;
/// - `reports`, once there are any: each [`Report`](crate::Report) of the
///   context with its count.
///
/// None of this is measured, stored or drawn while the mode is
/// [`Off`](Self::Off).
///
/// A leaf's content did not fit when the size its text or measure reports, at
/// the width it was given, exceeds its content box on either axis. A container's
/// did not when an in-flow child extends past its padding box on an axis it
/// does not clip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Debug {
    /// No overlay.
    #[default]
    Off,
    /// Outline every node.
    Outlines,
}

impl Debug {
    /// The outline of a node whose content fit.
    pub const OUTLINE: Color32 = Color32::from_rgb(0x6c, 0x8e, 0xbf);

    /// The outline of a scroll container's viewport: where its content is
    /// clipped.
    pub const VIEWPORT: Color32 = Color32::from_rgb(0x3c, 0xb3, 0x71);

    /// The tint of the gutter a scroll container reserves for its bars.
    pub const GUTTER: Color32 = Color32::from_rgba_premultiplied(0x3c, 0x2a, 0x00, 0x60);

    /// The outline of a node whose content did not fit.
    pub const OVERFLOW: Color32 = Color32::from_rgb(0xff, 0x30, 0x30);

    /// The mode set on `ctx`, [`Off`](Self::Off) until one is.
    pub fn of(ctx: &Context) -> Self {
        ctx.data(|data| data.get_temp(mode_id()))
            .unwrap_or_default()
    }

    /// Sets the mode of every frame drawn in `ctx` from now on.
    pub fn set(ctx: &Context, mode: Self) {
        ctx.data_mut(|data| data.insert_temp(mode_id(), mode));
    }

    /// Swaps [`Off`](Self::Off) and [`Outlines`](Self::Outlines).
    pub fn toggle(ctx: &Context) {
        let flipped = match Self::of(ctx) {
            Self::Off => Self::Outlines,
            Self::Outlines => Self::Off,
        };
        Self::set(ctx, flipped);
    }

    /// [`toggle`](Self::toggle)s when `key` was pressed this pass, consuming it.
    pub fn toggle_on(ctx: &Context, key: Key) {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
            Self::toggle(ctx);
        }
    }
}

fn mode_id() -> Id {
    Id::NULL.with("eggshell-debug-mode")
}

/// The stages of a frame that the readout times.
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Build,
    Layout,
    Draw,
}

/// The time spent in each [`Phase`].
#[derive(Clone, Copy, Default)]
struct Times {
    build: Duration,
    layout: Duration,
    draw: Duration,
}

impl Times {
    fn add(&mut self, phase: Phase, elapsed: Duration) {
        match phase {
            Phase::Build => self.build += elapsed,
            Phase::Layout => self.layout += elapsed,
            Phase::Draw => self.draw += elapsed,
        }
    }
}

impl std::ops::Sub for Times {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            build: self.build.saturating_sub(other.build),
            layout: self.layout.saturating_sub(other.layout),
            draw: self.draw.saturating_sub(other.draw),
        }
    }
}

/// A native pop-out window's share of a pass's time.
#[derive(Clone)]
struct WindowSpent {
    id: Id,
    title: String,
    times: Times,
}

/// The time eggshell spent in one egui pass of a page.
#[derive(Clone, Default)]
struct Spent {
    pass: u64,
    /// Everything, the native windows' time included: their timers run
    /// inside the page's.
    total: Times,
    windows: Vec<WindowSpent>,
}

impl Spent {
    fn page(&self) -> Times {
        self.windows
            .iter()
            .fold(self.total, |page, window| page - window.times)
    }
}

/// What a [`Timer`] is timing, and for which page's pass.
struct Timing {
    ctx: Context,
    phase: Phase,
    page: ViewportId,
    window: Option<(Id, String)>,
    started: web_time::Instant,
}

/// Adds the time between its creation and its drop to the pass's total for
/// its phase. Does nothing while the overlay is off.
pub(crate) struct Timer(Option<Timing>);

impl Timer {
    pub(crate) fn start(ctx: &Context, phase: Phase) -> Self {
        Self::timing(ctx, phase, ctx.viewport_id(), None)
    }

    /// Times `phase` of the native pop-out `window` (its id and title) for
    /// the pass of the page `page`, which may be another viewport's.
    pub(crate) fn window(
        ctx: &Context,
        phase: Phase,
        page: ViewportId,
        (id, title): (Id, &str),
    ) -> Self {
        Self::timing(ctx, phase, page, Some((id, title)))
    }

    fn timing(ctx: &Context, phase: Phase, page: ViewportId, window: Option<(Id, &str)>) -> Self {
        Self((Debug::of(ctx) == Debug::Outlines).then(|| Timing {
            ctx: ctx.clone(),
            phase,
            page,
            window: window.map(|(id, title)| (id, title.to_owned())),
            started: web_time::Instant::now(),
        }))
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let Some(Timing {
            ctx,
            phase,
            page,
            window,
            started,
        }) = self.0.take()
        else {
            return;
        };
        let elapsed = started.elapsed();
        let pass = ctx.cumulative_pass_nr_for(page);
        ctx.data_mut(|data| {
            let spent = data.get_temp_mut_or_default::<Spent>(spent_id().with(page));
            if spent.pass != pass {
                *spent = Spent {
                    pass,
                    ..Spent::default()
                };
            }
            match window {
                None => spent.total.add(phase, elapsed),
                Some((id, title)) => {
                    match spent.windows.iter_mut().find(|window| window.id == id) {
                        Some(window) => window.times.add(phase, elapsed),
                        None => {
                            let mut times = Times::default();
                            times.add(phase, elapsed);
                            spent.windows.push(WindowSpent { id, title, times });
                        }
                    }
                }
            }
        });
    }
}

fn spent_id() -> Id {
    Id::NULL.with("eggshell-debug-spent")
}

fn frames_id() -> Id {
    Id::NULL.with("eggshell-debug-frames")
}

fn topmost_id() -> Id {
    Id::NULL.with("eggshell-debug-topmost")
}

fn corner_id() -> Id {
    Id::NULL.with("eggshell-debug-corner")
}

/// The frames averaged into the fps.
const FPS_WINDOW: usize = 30;

/// The `unstable_dt` of the last [`FPS_WINDOW`] passes, in seconds.
#[derive(Clone, Default)]
struct Frames {
    pass: u64,
    dts: std::collections::VecDeque<f32>,
}

impl Frames {
    fn fps(&self) -> f32 {
        let total: f32 = self.dts.iter().sum();
        if total > 0.0 {
            self.dts.len() as f32 / total
        } else {
            0.0
        }
    }
}

/// Records this pass's `unstable_dt` once, however often it is called, and
/// returns the averaged frames per second.
fn record_frame(ctx: &Context, dt: f32) -> f32 {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let frames = data.get_temp_mut_or_default::<Frames>(frames_id());
        if frames.pass != pass || frames.dts.is_empty() {
            frames.pass = pass;
            if frames.dts.len() == FPS_WINDOW {
                frames.dts.pop_front();
            }
            frames.dts.push_back(dt);
        }
        frames.fps()
    })
}

fn readout_text(ctx: &Context) -> String {
    let (unstable, stable, predicted) =
        ctx.input(|input| (input.unstable_dt, input.stable_dt, input.predicted_dt));
    let fps = record_frame(ctx, unstable);
    let pass = ctx.cumulative_pass_nr();
    let spent = ctx
        .data(|data| data.get_temp::<Spent>(spent_id().with(ctx.viewport_id())))
        .filter(|spent| spent.pass == pass)
        .unwrap_or_default();
    let page = spent.page();
    let ms = |seconds: f32| seconds * 1000.0;
    let ms_of = |duration: Duration| duration.as_secs_f32() * 1000.0;
    let mut text = format!(
        "frame    {:6.1} ms  {:6.1} fps\n\
         dt       stable {:5.1}  predicted {:5.1} ms\n\
         eggshell build {:5.2}  layout {:5.2}  draw {:5.2} ms",
        ms(unstable),
        fps,
        ms(stable),
        ms(predicted),
        ms_of(page.build),
        ms_of(page.layout),
        ms_of(page.draw),
    );
    for window in &spent.windows {
        let _ = write!(
            text,
            "\n  {:<TITLE_COLUMN$} layout {:5.2}  draw {:5.2} ms",
            title_column(&window.title),
            ms_of(window.times.layout),
            ms_of(window.times.draw),
        );
    }
    let repaint = if ctx.has_requested_repaint() {
        "requested"
    } else {
        "idle"
    };
    let _ = write!(text, "\nrepaint  {repaint}");
    text.push_str(&reports_text(ctx));
    text
}

/// The characters of a window's title column in the readout.
const TITLE_COLUMN: usize = 19;

/// `title` to fit the readout's title column: cut with an ellipsis when it
/// is longer, so the times of every row stay in line.
fn title_column(title: &str) -> String {
    if title.chars().count() <= TITLE_COLUMN {
        return title.to_owned();
    }
    let kept: String = title.chars().take(TITLE_COLUMN - 1).collect();
    format!("{kept}…")
}

/// The reports section of the readout: one line per report, or nothing
/// while there are none.
fn reports_text(ctx: &Context) -> String {
    let reports = crate::report::reports(ctx);
    let dropped = crate::report::dropped(ctx);
    if reports.is_empty() && dropped == 0 {
        return String::new();
    }
    let mut text = String::from("\nreports");
    for report in reports {
        let _ = write!(
            text,
            "\n  {:>4}x {:?}: {} (at {})",
            report.count, report.problem, report.message, report.at
        );
    }
    if dropped > 0 {
        let _ = write!(text, "\n  and {dropped} more not kept");
    }
    text
}

/// The readout and the hover label of a pass: one shape kept last on the
/// overlay's layer, so that no tree's outlines cover it and the label is
/// above the readout.
#[derive(Clone)]
struct Topmost {
    pass: u64,
    index: egui::layers::ShapeIdx,
    label: Option<Shape>,
}

/// Paints the readout and `label` (or the label an earlier tree of this pass
/// painted) after everything painted so far, replacing what an earlier tree
/// painted, so that the readout shows the draw time so far.
fn paint_topmost(ctx: &Context, painter: &Painter, label: Option<Shape>) {
    let pass = ctx.cumulative_pass_nr();
    let earlier = ctx
        .data(|data| data.get_temp::<Topmost>(topmost_id()))
        .filter(|earlier| earlier.pass == pass);
    if let Some(earlier) = &earlier {
        painter.set(earlier.index, Shape::Noop);
    }
    let label = label.or_else(|| earlier.and_then(|earlier| earlier.label));
    let shape = Shape::Vec(
        std::iter::once(readout(ctx, painter))
            .chain(label.clone())
            .collect(),
    );
    let index = painter.add(shape);
    ctx.data_mut(|data| data.insert_temp(topmost_id(), Topmost { pass, index, label }));
}

/// The readout, wrapped to the content rect and kept inside it, in the
/// corner [`readout_corner`] picks.
fn readout(ctx: &Context, painter: &Painter) -> Shape {
    let screen = ctx.content_rect().shrink(4.0);
    let margin = vec2(4.0, 2.0);
    let galley = painter.layout(
        readout_text(ctx),
        FontId::monospace(11.0),
        Color32::WHITE,
        (screen.width() - margin.x * 2.0).max(0.0),
    );
    let size = galley.size() + margin * 2.0;
    let aligned = readout_corner(ctx, screen).align_size_within_rect(size, screen);
    let rect = aligned.translate((screen.min - aligned.min).max(Vec2::ZERO));
    Shape::Vec(vec![
        Shape::rect_filled(rect, 2.0, Color32::from_black_alpha(220)),
        Shape::galley(rect.min + margin, galley, Color32::WHITE),
    ])
}

/// The corner of `screen` farthest from the pointer, or the last one picked
/// while there is no pointer: the top right until the pointer is seen.
fn readout_corner(ctx: &Context, screen: Rect) -> Align2 {
    let away = |pointer: f32, centre: f32| {
        if pointer < centre {
            Align::Max
        } else {
            Align::Min
        }
    };
    let farthest = ctx.pointer_hover_pos().map(|pointer| {
        Align2([
            away(pointer.x, screen.center().x),
            away(pointer.y, screen.center().y),
        ])
    });
    ctx.data_mut(|data| {
        let kept = data.get_temp_mut_or(corner_id(), Align2::RIGHT_TOP);
        if let Some(farthest) = farthest {
            *kept = farthest;
        }
        *kept
    })
}

/// What a leaf's content may exceed its box by before it counts as not
/// fitting: egui's galleys are a fraction wider than the width they wrap at.
const TOLERANCE: f32 = 0.5;

/// For each node, whether its content did not fit.
pub(crate) fn overflow(ctx: &Context, nodes: &Nodes, prims: &[Option<Prim>]) -> Vec<bool> {
    (0..nodes.len())
        .map(|index| overflows(ctx, nodes, prims, index))
        .collect()
}

fn overflows(ctx: &Context, nodes: &Nodes, prims: &[Option<Prim>], index: usize) -> bool {
    let Some(node) = nodes.node(index) else {
        return false;
    };
    if node.end == index + 1 {
        if node.flow == Flow::Hidden {
            return false;
        }
        let content = inset(node.rect(), add(node.border, node.padding));
        let content_size = taffy::Size {
            width: content.width().max(0.0),
            height: content.height().max(0.0),
        };
        let wanted = node.measured.or_else(|| {
            let prim = prims.get(index)?.as_ref()?;
            crate::pipeline::wanted(
                ctx,
                Content::Prim(prim),
                (&node.text, &node.style),
                content_size,
            )
        });
        return wanted.is_some_and(|wanted| {
            wanted.x > content.width() + TOLERANCE || wanted.y > content.height() + TOLERANCE
        });
    }
    let padding = inset(node.rect(), node.border);
    let clips = |overflow| overflow != Overflow::Visible;
    let (clips_x, clips_y) = (clips(node.overflow.x), clips(node.overflow.y));
    nodes
        .children(index)
        .filter_map(|child| nodes.node(child))
        .filter(|child| child.flow == Flow::Normal)
        .any(|child| {
            let rect = child.rect();
            (!clips_x
                && (rect.min.x < padding.min.x - TOLERANCE
                    || rect.max.x > padding.max.x + TOLERANCE))
                || (!clips_y
                    && (rect.min.y < padding.min.y - TOLERANCE
                        || rect.max.y > padding.max.y + TOLERANCE))
        })
}

/// The nodes that are drawn, with their indices: those not under a
/// `display: none`.
fn shown(nodes: &Nodes) -> impl Iterator<Item = (usize, &PlacedNode)> {
    let mut next = 0;
    std::iter::from_fn(move || {
        while let Some(node) = nodes.get(next) {
            let index = next;
            if node.flow == Flow::Hidden {
                next = node.end.max(index + 1);
            } else {
                next = index + 1;
                return Some((index, node));
            }
        }
        None
    })
}

/// Paints the overlay for one placed tree.
pub(crate) fn paint(ctx: &Context, nodes: &Nodes, overflows: &[bool]) {
    let grid = PixelGrid {
        pixels_per_point: ctx.pixels_per_point(),
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Debug, Id::NULL.with("eggshell-debug")));
    let width = grid.line_width(0.0);
    let overflows_at = |index: usize| overflows.get(index).copied().unwrap_or(false);
    for (index, node) in shown(nodes) {
        let color = if overflows_at(index) {
            Debug::OVERFLOW
        } else {
            Debug::OUTLINE
        };
        painter.add(Shape::rect_stroke(
            grid.rect(node.rect()),
            0.0,
            Stroke::new(width, color),
            StrokeKind::Inside,
        ));
    }
    for (_, node) in shown(nodes) {
        let (Some(viewport), Some(plan)) = (node.scroll_viewport(), node.scroll()) else {
            continue;
        };
        for region in crate::scroll::gutters(
            crate::draw::inset(node.rect(), node.border),
            plan.bars.gutter,
        ) {
            painter.rect_filled(grid.rect(region), 0.0, Debug::GUTTER);
        }
        painter.add(Shape::rect_stroke(
            grid.rect(viewport),
            0.0,
            Stroke::new(width, Debug::VIEWPORT),
            StrokeKind::Inside,
        ));
    }
    let label = ctx.pointer_hover_pos().and_then(|pointer| {
        let (index, node) = shown(nodes)
            .filter(|(_, node)| node.rect().contains(pointer))
            .last()?;
        Some(label(
            &painter,
            ctx.content_rect(),
            pointer,
            node,
            if overflows_at(index) {
                Fit::Overflows
            } else {
                Fit::Fits
            },
        ))
    });
    paint_topmost(ctx, &painter, label);
}

/// Whether a node's content fits it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    Fits,
    Overflows,
}

fn label(painter: &Painter, screen: Rect, pointer: Pos2, node: &PlacedNode, fit: Fit) -> Shape {
    let Rect { min, max } = node.rect();
    let mut text = format!(
        "{}\n{:.1}, {:.1} .. {:.1}, {:.1}  ({:.1} x {:.1})",
        node.path(),
        min.x,
        min.y,
        max.x,
        max.y,
        max.x - min.x,
        max.y - min.y
    );
    if let (Some(viewport), Some(plan)) = (node.scroll_viewport(), node.scroll()) {
        text.push_str(&format!(
            "\nscroll viewport {:.1} x {:.1}, gutter {:.1} x {:.1}",
            viewport.width(),
            viewport.height(),
            plan.bars.gutter.x,
            plan.bars.gutter.y
        ));
    }
    if fit == Fit::Overflows {
        text.push_str("\ncontent does not fit");
    }
    let galley = painter.layout_no_wrap(text, FontId::monospace(11.0), Color32::WHITE);
    let margin = vec2(4.0, 2.0);
    let size = galley.size() + margin * 2.0;
    let mut origin = pointer + vec2(12.0, 12.0);
    origin.x = origin.x.min(screen.max.x - size.x).max(screen.min.x);
    origin.y = origin.y.min(screen.max.y - size.y).max(screen.min.y);
    Shape::Vec(vec![
        Shape::rect_filled(
            Rect::from_min_size(origin, size),
            2.0,
            Color32::from_black_alpha(220),
        ),
        Shape::galley(origin + margin, galley, Color32::WHITE),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_title_is_cut_to_the_column_and_a_short_one_is_kept() {
        assert_eq!(title_column("Revenue"), "Revenue");
        let fits = "A".repeat(TITLE_COLUMN);
        assert_eq!(title_column(&fits), fits, "exactly the column is kept");
        let cut = title_column("Quarterly revenue by region");
        assert_eq!(cut, "Quarterly revenue …");
        assert_eq!(cut.chars().count(), TITLE_COLUMN);
    }

    #[test]
    fn every_window_rows_times_line_up_whatever_its_title() {
        let rows = ["Revenue", "Quarterly revenue by region and month"]
            .map(|title| format!("  {:<TITLE_COLUMN$} layout", title_column(title)));
        let at = rows.clone().map(|row| {
            row.split(" layout")
                .next()
                .map_or(0, |before| before.chars().count())
        });
        assert_eq!(at[0], at[1], "{rows:?}");
    }

    fn spent(ctx: &Context) -> Option<Spent> {
        ctx.data(|data| data.get_temp::<Spent>(spent_id().with(ctx.viewport_id())))
    }

    fn time_a_window(ctx: &Context) {
        drop(Timer::window(
            ctx,
            Phase::Draw,
            ctx.viewport_id(),
            (Id::NULL.with("pane"), "Pane"),
        ));
    }

    #[test]
    fn nothing_is_timed_while_the_overlay_is_off() {
        let ctx = Context::default();
        assert_eq!(Debug::of(&ctx), Debug::Off);
        drop(Timer::start(&ctx, Phase::Draw));
        time_a_window(&ctx);
        assert!(spent(&ctx).is_none());
        Debug::set(&ctx, Debug::Outlines);
        time_a_window(&ctx);
        let windows = spent(&ctx).map(|spent| spent.windows.len());
        assert_eq!(windows, Some(1), "precondition: timed while it's on");
    }

    #[test]
    fn the_pages_row_leaves_out_its_windows() {
        let ms = Duration::from_millis;
        let spent = Spent {
            pass: 0,
            total: Times {
                build: ms(1),
                layout: ms(4),
                draw: ms(9),
            },
            windows: vec![WindowSpent {
                id: Id::NULL.with("pane"),
                title: "Pane".to_owned(),
                times: Times {
                    build: Duration::ZERO,
                    layout: ms(1),
                    draw: ms(6),
                },
            }],
        };
        let page = spent.page();
        assert_eq!((page.build, page.layout, page.draw), (ms(1), ms(3), ms(3)));
    }
}
