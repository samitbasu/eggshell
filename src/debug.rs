//! The layout debug overlay: a hairline around every node, in red where the
//! node's content did not fit, the viewport (green) and reserved gutter
//! (tint) of every scroll container, a label for the node under the pointer, and a
//! frame-timing readout in the top-right corner.

use core::time::Duration;

use egui::epaint::StrokeKind;
use egui::{
    Align2, Color32, Context, FontId, Id, Key, LayerId, Order, Painter, Pos2, Rect, Shape, Stroke,
    vec2,
};
use taffy::Overflow;

use crate::draw::{PixelGrid, add, inset};
use crate::pipeline::{Flow, PlacedNode};

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
/// [`Outlines`](Self::Outlines) also draws a readout in the top-right corner,
/// once per frame however many trees are drawn:
///
/// - `frame`: egui's `unstable_dt`, the measured time since the last pass, and
///   the frames per second averaged over the last 30 passes;
/// - `dt`: egui's `stable_dt` and `predicted_dt`, which decide how far an
///   animation advances per frame, so a slow-motion animation is explainable;
/// - `eggshell`: the time spent in [`LayoutState::build`](crate::LayoutState::build),
///   [`Built::layout`](crate::Built::layout) and [`Placed::draw`](crate::Placed::draw),
///   summed over every tree of the pass (the readout sees the draws before it);
/// - `repaint`: whether egui has been asked to repaint at once, as it is while
///   an animation is in progress, or is idle.
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

/// The time eggshell spent in each [`Phase`] of one egui pass.
#[derive(Clone, Copy, Default)]
struct Spent {
    pass: u64,
    build: Duration,
    layout: Duration,
    draw: Duration,
}

/// Adds the time between its creation and its drop to the pass's total for
/// its phase. Does nothing while the overlay is off.
pub(crate) struct Timer(Option<(Context, Phase, web_time::Instant)>);

impl Timer {
    pub(crate) fn start(ctx: &Context, phase: Phase) -> Self {
        Self(
            (Debug::of(ctx) == Debug::Outlines)
                .then(|| (ctx.clone(), phase, web_time::Instant::now())),
        )
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let Some((ctx, phase, started)) = self.0.take() else {
            return;
        };
        let elapsed = started.elapsed();
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|data| {
            let spent = data.get_temp_mut_or_default::<Spent>(spent_id());
            if spent.pass != pass {
                *spent = Spent {
                    pass,
                    ..Spent::default()
                };
            }
            match phase {
                Phase::Build => spent.build += elapsed,
                Phase::Layout => spent.layout += elapsed,
                Phase::Draw => spent.draw += elapsed,
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

fn readout_id() -> Id {
    Id::NULL.with("eggshell-debug-readout")
}

/// The frames averaged into the fps.
const FPS_WINDOW: usize = 30;

/// The `unstable_dt` of the last [`FPS_WINDOW`] passes, in seconds.
#[derive(Clone, Default)]
struct Frames {
    pass: u64,
    dts: Vec<f32>,
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
                frames.dts.remove(0);
            }
            frames.dts.push(dt);
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
        .data(|data| data.get_temp::<Spent>(spent_id()))
        .filter(|spent| spent.pass == pass)
        .unwrap_or_default();
    let ms = |seconds: f32| seconds * 1000.0;
    let ms_of = |duration: Duration| duration.as_secs_f32() * 1000.0;
    format!(
        "frame    {:6.1} ms  {:6.1} fps\n\
         dt       stable {:5.1}  predicted {:5.1} ms\n\
         eggshell build {:5.2}  layout {:5.2}  draw {:5.2} ms\n\
         repaint  {}",
        ms(unstable),
        fps,
        ms(stable),
        ms(predicted),
        ms_of(spent.build),
        ms_of(spent.layout),
        ms_of(spent.draw),
        if ctx.has_requested_repaint() {
            "requested"
        } else {
            "idle"
        },
    )
}

/// Paints the readout, replacing the one an earlier tree of this pass painted
/// so that it shows the draw time so far.
fn paint_readout(ctx: &Context, painter: &Painter) {
    let galley = painter.layout_no_wrap(readout_text(ctx), FontId::monospace(11.0), Color32::WHITE);
    let margin = vec2(4.0, 2.0);
    let size = galley.size() + margin * 2.0;
    let screen = ctx.content_rect();
    let rect = Align2::RIGHT_TOP.align_size_within_rect(size, screen.shrink(4.0));
    let shape = Shape::Vec(vec![
        Shape::rect_filled(rect, 2.0, Color32::from_black_alpha(220)),
        Shape::galley(rect.min + margin, galley, Color32::WHITE),
    ]);
    let pass = ctx.cumulative_pass_nr();
    let earlier = ctx
        .data(|data| data.get_temp::<(u64, egui::layers::ShapeIdx)>(readout_id()))
        .filter(|&(earlier, _)| earlier == pass);
    match earlier {
        Some((_, index)) => painter.set(index, shape),
        None => {
            let index = painter.add(shape);
            ctx.data_mut(|data| data.insert_temp(readout_id(), (pass, index)));
        }
    }
}

/// What a leaf's content may exceed its box by before it counts as not
/// fitting: egui's galleys are a fraction wider than the width they wrap at.
const TOLERANCE: f32 = 0.5;

/// Whether the content of `nodes[index]` did not fit.
pub(crate) fn overflows(nodes: &[PlacedNode], index: usize) -> bool {
    let node = &nodes[index];
    if node.end == index + 1 {
        return node.measured.is_some_and(|measured| {
            let content = inset(node.rect(), add(node.border, node.padding));
            measured.x > content.width() + TOLERANCE || measured.y > content.height() + TOLERANCE
        });
    }
    let padding = inset(node.rect(), node.border);
    let clips = |overflow| overflow != Overflow::Visible;
    let (clips_x, clips_y) = (clips(node.overflow.x), clips(node.overflow.y));
    children(nodes, index)
        .filter(|&child| nodes[child].flow == Flow::Normal)
        .any(|child| {
            let rect = nodes[child].rect();
            (!clips_x
                && (rect.min.x < padding.min.x - TOLERANCE
                    || rect.max.x > padding.max.x + TOLERANCE))
                || (!clips_y
                    && (rect.min.y < padding.min.y - TOLERANCE
                        || rect.max.y > padding.max.y + TOLERANCE))
        })
}

fn children(nodes: &[PlacedNode], index: usize) -> impl Iterator<Item = usize> {
    let end = nodes[index].end;
    std::iter::successors((index + 1 < end).then_some(index + 1), move |&child| {
        Some(nodes[child].end).filter(|&next| next < end)
    })
}

/// The indices of the nodes that are drawn: those not under a `display: none`.
fn shown(nodes: &[PlacedNode]) -> impl Iterator<Item = usize> {
    let mut next = 0;
    std::iter::from_fn(move || {
        while next < nodes.len() {
            let index = next;
            if nodes[index].flow == Flow::Hidden {
                next = nodes[index].end;
            } else {
                next += 1;
                return Some(index);
            }
        }
        None
    })
}

/// Paints the overlay for one placed tree.
pub(crate) fn paint(ctx: &Context, nodes: &[PlacedNode]) {
    let grid = PixelGrid {
        pixels_per_point: ctx.pixels_per_point(),
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Debug, Id::NULL.with("eggshell-debug")));
    let width = grid.line_width(0.0);
    for index in shown(nodes) {
        let color = if overflows(nodes, index) {
            Debug::OVERFLOW
        } else {
            Debug::OUTLINE
        };
        painter.add(Shape::rect_stroke(
            grid.rect(nodes[index].rect()),
            0.0,
            Stroke::new(width, color),
            StrokeKind::Inside,
        ));
    }
    for index in shown(nodes) {
        let node = &nodes[index];
        let (Some(viewport), Some(plan)) = (node.scroll_viewport(), node.scroll.as_ref()) else {
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
    if let Some(pointer) = ctx.pointer_hover_pos()
        && let Some(index) = shown(nodes)
            .filter(|&i| nodes[i].rect().contains(pointer))
            .last()
    {
        label(&painter, ctx.content_rect(), pointer, nodes, index);
    }
    paint_readout(ctx, &painter);
}

fn label(painter: &Painter, screen: Rect, pointer: Pos2, nodes: &[PlacedNode], index: usize) {
    let node = &nodes[index];
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
    if let (Some(viewport), Some(plan)) = (node.scroll_viewport(), node.scroll.as_ref()) {
        text.push_str(&format!(
            "\nscroll viewport {:.1} x {:.1}, gutter {:.1} x {:.1}",
            viewport.width(),
            viewport.height(),
            plan.bars.gutter.x,
            plan.bars.gutter.y
        ));
    }
    if overflows(nodes, index) {
        text.push_str("\ncontent does not fit");
    }
    let galley = painter.layout_no_wrap(text, FontId::monospace(11.0), Color32::WHITE);
    let margin = vec2(4.0, 2.0);
    let size = galley.size() + margin * 2.0;
    let mut origin = pointer + vec2(12.0, 12.0);
    origin.x = origin.x.min(screen.max.x - size.x).max(screen.min.x);
    origin.y = origin.y.min(screen.max.y - size.y).max(screen.min.y);
    painter.rect_filled(
        Rect::from_min_size(origin, size),
        2.0,
        Color32::from_black_alpha(220),
    );
    painter.galley(origin + margin, galley, Color32::WHITE);
}
