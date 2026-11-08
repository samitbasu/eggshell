//! The layout debug overlay: a hairline around every node, in red where the
//! node's content did not fit, and a label for the node under the pointer.

use egui::epaint::StrokeKind;
use egui::{
    Color32, Context, FontId, Id, Key, LayerId, Order, Painter, Pos2, Rect, Shape, Stroke, vec2,
};
use taffy::Overflow;

use crate::draw::{PixelGrid, add, inset};
use crate::pipeline::{Flow, PlacedNode};

/// Whether frames are drawn with the layout debug overlay.
///
/// The mode lives in the [`egui::Context`], so one key binding anywhere in
/// the application (see [`Debug::toggle_on`]) governs every
/// [`Placed::draw`](crate::Placed::draw) of the frame. [`Off`](Self::Off) costs
/// one context read per layout and per draw.
///
/// With [`Outlines`](Self::Outlines), [`Placed::draw`](crate::Placed::draw)
/// outlines the border box of every node, anonymous ones and overlay roots
/// included, with a hairline of one physical pixel on the topmost layer. A
/// node whose content did not fit is outlined in [`Debug::OVERFLOW`], every
/// other in [`Debug::OUTLINE`]. The innermost node under the pointer is
/// labelled with its readable path and rect.
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
    Id::NULL.with("bullseye-debug-mode")
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
    let painter = ctx.layer_painter(LayerId::new(Order::Debug, Id::NULL.with("bullseye-debug")));
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
    if let Some(pointer) = ctx.pointer_hover_pos()
        && let Some(index) = shown(nodes)
            .filter(|&i| nodes[i].rect().contains(pointer))
            .last()
    {
        label(&painter, ctx.content_rect(), pointer, nodes, index);
    }
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
