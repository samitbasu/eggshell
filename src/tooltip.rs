//! Tooltips: hover intent, and the overlay that shows one.
//!
//! A trigger is a draw closure that says so ([`register`]); nothing is
//! declared for it at build. At the end of the draw the frame's triggers are
//! resolved against the pointer, the keyboard focus and the style's timing
//! ([`Intent::tick`], pure), and the next build declares one overlay per
//! tooltip that is open or still leaving ([`declare`]). The state is
//! `Kept::tooltips`, so it is per `LayoutState`.

use core::time::Duration;
use std::hash::Hash;
use std::sync::Arc;

use egui::{Context, Id, Key, Painter, Pos2, Rect, Shape, Ui, Vec2, WidgetText, pos2};

use crate::NodeUi as _;
use crate::builder::{Builder, Surface};
use crate::finite::clamp;
use crate::hover::Quiet;
use crate::intent::{Cause, FocusOpens, Intent, Interruption, Skip, Timing, Trigger, wanted};
use crate::kept::Handle;
use crate::overlay::{Anchor, Band, DismissOn, Distance, Focus, Placement};
use crate::overlay_chain::OverlayChain;
use crate::overlay_input::Drag;
use crate::owned::TooltipPart;
use crate::presence::Shown;
use crate::tw::tw;

/// What a tooltip shows.
#[derive(Clone)]
pub(crate) enum Content {
    Text(WidgetText),
    /// Built over `()`: the content takes no input, so it is built the same
    /// for any application, on the page or in a mount.
    Built(Arc<BuildFn>),
}

pub(crate) type BuildFn = dyn Fn(&mut Builder<'_, '_, ()>) + Send + Sync;

/// Builds a [`Content::Built`] with a builder of `A`. Only a caller that
/// knows `A: 'static` can name one, so a frame holds the one its triggers
/// left, and declaring the tooltips asks nothing of `A`.
pub(crate) type BuildBuilt<A> = fn(&mut Builder<'_, '_, A>, &BuildFn);

pub(crate) fn build_built<A: 'static>(b: &mut Builder<'_, '_, A>, build: &BuildFn) {
    b.unit(|b| build(b));
}

impl Content {
    pub(crate) fn built(build: impl Fn(&mut Builder<'_, '_, ()>) + Send + Sync + 'static) -> Self {
        Self::Built(Arc::new(build))
    }

    /// Whether `b` can build this. Built content cannot in a frame where no
    /// trigger of built content was built, as when a leaving tooltip's
    /// trigger is gone and was the only one.
    fn shows_in<A>(&self, b: &Builder<'_, '_, A>) -> bool {
        match self {
            Self::Text(_) => true,
            Self::Built(_) => b.build_built().is_some(),
        }
    }

    /// Adds what this shows to `b`, drawn on a tooltip. Built content adds
    /// nothing where it does not [show](Self::shows_in).
    fn build<A>(&self, b: &mut Builder<'_, '_, A>) {
        let build_built = b.build_built();
        b.on_surface(Surface::Tooltip, |b| match self {
            Self::Text(text) => b.rich(tw(), text.clone()),
            Self::Built(built) => {
                if let Some(build_built) = build_built {
                    build_built(b, &**built);
                }
            }
        });
    }
}

/// A trigger's tooltip: what it shows, and its text, which describes the
/// trigger to assistive technology.
#[derive(Clone)]
pub(crate) struct Tooltip {
    content: Content,
    /// `None` where nothing reads it.
    description: Option<String>,
    press: Press,
}

/// What a press on a tooltip's trigger does to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Press {
    /// Closes it, as any press does.
    Closes,
    /// Shows it at once and keeps it until the next press elsewhere: a
    /// refused control's reason, which a touch screen reaches no other way.
    Shows,
}

impl Tooltip {
    /// The tooltip showing `content`, for a trigger built by `b`. While
    /// accesskit is on, a built content's text is found by building it where
    /// nothing shows it, under `trigger`, so two triggers' contents do not
    /// share keys.
    pub(crate) fn new<A: 'static>(
        b: &mut Builder<'_, '_, A>,
        trigger: impl Hash + std::fmt::Debug,
        content: Content,
    ) -> Self {
        match content {
            Content::Text(text) => Self::text(text),
            Content::Built(_) => {
                b.set_build_built(build_built::<A>);
                Self {
                    description: b.description_of(trigger, |b| content.build(b)),
                    content,
                    press: Press::Closes,
                }
            }
        }
    }

    /// The tooltip showing `text`, which says nothing its trigger's
    /// accessible name does not.
    pub(crate) fn undescribed(text: WidgetText) -> Self {
        Self {
            description: None,
            content: Content::Text(text),
            press: Press::Closes,
        }
    }

    /// The tooltip showing `text`.
    pub(crate) fn text(text: WidgetText) -> Self {
        Self {
            description: Some(text.text().to_owned()),
            content: Content::Text(text),
            press: Press::Closes,
        }
    }

    /// The reason a control is refused: a tooltip that a press on the
    /// control shows.
    pub(crate) fn refusal(reason: WidgetText) -> Self {
        Self {
            press: Press::Shows,
            ..Self::text(reason)
        }
    }
}

/// What an open tooltip shows, and the style its trigger was built in,
/// which the tooltip is built and drawn with.
#[derive(Clone)]
pub(crate) struct Shows {
    content: Content,
    style: Arc<crate::Style>,
}

pub(crate) type Tooltips = Intent<Shows>;

/// Starts the frame's collection of triggers.
pub(crate) fn begin(ctx: &Context, kept: &Handle) {
    kept.viewport(ctx.viewport_id(), |here| here.tooltips.requests.clear());
}

/// Says that the node of the draw closure `ui` is a trigger for `tip`.
/// `response` is the one the node's hover and focus are read from.
pub(crate) fn register(ui: &Ui, response: &egui::Response, tip: &Tooltip) {
    let focus = response.has_focus().then_some(Cause::Focus);
    let pointer = over(ui, response).then_some(Cause::Pointer);
    let pressed = pressed(ui, response, tip);
    request(ui, tip, [focus, pointer, pressed], response.rect);
    describes(ui, response, tip);
}

/// Says that the node of the draw closure `ui` is the one trigger for `tip`
/// of everything inside it: the pointer anywhere over it, or keyboard focus
/// on any widget inside it, wants the tooltip, which is anchored to the node.
pub(crate) fn register_whole(ui: &Ui, tip: &Tooltip) {
    let response = ui.node_response();
    let focus = ui.has_focus_within().then_some(Cause::Focus);
    let pointer = over(ui, &response).then_some(Cause::Pointer);
    let pressed = pressed(ui, &response, tip);
    request(ui, tip, [focus, pointer, pressed], response.rect);
    describes(ui, &response, tip);
}

/// [`Cause::Pressed`] when the pointer was pressed on `response`'s widget
/// this frame and `tip` shows on a press. A disabled widget senses no click,
/// so the press is read from the pointer.
fn pressed(ui: &Ui, response: &egui::Response, tip: &Tooltip) -> Option<Cause> {
    let pressed = ui.input(|input| input.pointer.primary_pressed());
    (tip.press == Press::Shows && pressed && over(ui, response)).then_some(Cause::Pressed)
}

/// Whether the pointer is on `response`'s widget, inside its rect: egui also
/// hovers a widget the pointer is only near, within its interact radius.
fn over(ui: &Ui, response: &egui::Response) -> bool {
    response.contains_pointer()
        && ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|at| response.interact_rect.contains(at))
}

/// Describes `response`'s widget to assistive technology by `tip`'s text,
/// without making it a trigger.
pub(crate) fn describes(ui: &Ui, response: &egui::Response, tip: &Tooltip) {
    if let Some(description) = &tip.description {
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_description(description.as_str());
        });
    }
}

/// Pushes a trigger at `rect` for each of `causes`; focus counts only after
/// a key.
fn request(ui: &Ui, tip: &Tooltip, causes: [Option<Cause>; 3], rect: Rect) {
    let id = crate::draw::node_id_or_report(ui, "a tooltip");
    let style = ui.node_style();
    let wants = |cause| {
        let Some(id) = id else { return };
        let trigger = Trigger {
            id,
            payload: Shows {
                content: tip.content.clone(),
                style: style.clone(),
            },
            cause,
            rect,
        };
        if let Some(shell) = crate::shell::shell_or_report(ui, "a tooltip") {
            shell.kept.viewport(ui.ctx().viewport_id(), |here| {
                here.tooltips.requests.push(trigger)
            });
        }
    };
    let keyboard = crate::overlay_input::keyboard_was_last(ui.ctx());
    for cause in causes.into_iter().flatten() {
        if cause != Cause::Focus || keyboard {
            wants(cause);
        }
    }
}

/// What the frame's input says of interruptions, read before the library
/// takes the keys it answers.
pub(crate) fn interruption(ctx: &Context, drag: Drag) -> Option<Interruption> {
    ctx.input(|input| {
        if input.pointer.any_pressed() {
            Some(Interruption::Press)
        } else if input.key_pressed(Key::Escape) {
            Some(Interruption::Escape)
        } else if !input.focused {
            Some(Interruption::Blur)
        } else {
            None
        }
    })
    .or((drag == Drag::Live).then_some(Interruption::Drag))
}

/// Ends the frame's collection: moves the state to this frame. A trigger
/// inside a hover card's trigger, or around one the same cause wants, yields
/// to the card ([`Quiet`]).
pub(crate) fn end(
    ctx: &Context,
    kept: &Handle,
    interruption: Option<Interruption>,
    quiet: &Quiet<'_>,
) {
    let triggers: Vec<_> = kept
        .viewport(ctx.viewport_id(), |here| {
            std::mem::take(&mut here.tooltips.requests)
        })
        .into_iter()
        .filter(|trigger| !quiet.holds(trigger.id))
        .collect();
    let style = crate::Shell::new(ctx, kept).style();
    let timing = Timing {
        open: style.tooltip.delay,
        close: Duration::ZERO,
        skip: Skip::Grace(style.tooltip.grace),
        focus: FocusOpens::AtOnce,
    };
    let now = crate::intent::now(ctx);
    let wanted = wanted(triggers).filter(|trigger| !quiet.outranks(trigger.id, trigger.cause));
    let focused = ctx.memory(|memory| memory.focused());
    let (wait, changed) = kept.viewport(ctx.viewport_id(), |here| {
        let before = here.tooltips.open_triggers();
        here.tooltips.follow_focus(focused);
        let wait = here
            .tooltips
            .tick(now, timing, wanted.as_ref(), interruption);
        (wait, here.tooltips.open_triggers() != before)
    });
    crate::intent::repaint(ctx, wait, changed);
}

#[derive(Clone, Copy, Hash)]
struct Tip(Id);

impl std::fmt::Debug for Tip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tooltip({:?})", self.0)
    }
}

/// Declares the overlay of every tooltip that is open or still leaving.
pub(crate) fn declare<'f, A>(b: &mut Builder<'_, 'f, A>) {
    let present = b
        .kept()
        .viewport(b.viewport(), |here| here.tooltips.present.clone());
    for tip in present {
        let slot = b.slot(&Tip(tip.trigger));
        let Shows { content, style } = &tip.payload;
        let shows = content.shows_in(b);
        let viewport = b.viewport();
        let kept = b.kept().with(|kept| {
            let leaving = kept.presences.contains(slot.id);
            if !shows || (!tip.open && !leaving) {
                kept.viewport(viewport)
                    .tooltips
                    .present
                    .retain(|present| present.trigger != tip.trigger);
            }
            shows && (tip.open || leaving)
        });
        if !kept {
            continue;
        }
        let tooltip = &style.tooltip;
        let arrow = tooltip.arrow;
        let gap = f32::from(tooltip.offset) + arrow.map_or(0.0, |arrow| arrow.size);
        let trigger = tip.trigger;
        b.with_style(style, |b| {
            let classes = b.part(TooltipPart::Content, &[], tw());
            let resolved = b.resolved_part(TooltipPart::Content, &[]);
            let fill = resolved
                .paint
                .background
                .unwrap_or(egui::Color32::TRANSPARENT);
            let border = Vec2::splat(resolved.style().border.left.into_raw().value());
            OverlayChain::new(b.reborrow(), slot)
                .band(Band::Tooltip)
                .anchor(Anchor::Node(trigger))
                .placement(Placement::tooltip().gap(Distance::new(gap)))
                .focus(Focus::None)
                .dismiss_on(DismissOn::Never)
                .shown(if tip.open { Shown::Open } else { Shown::Closed })
                .children(|s| {
                    s.div_id("Box", classes, |s| {
                        content.build(s);
                        if let Some(arrow) = arrow {
                            s.paint(
                                tw().absolute().inset_px(0.0),
                                arrow_painter(trigger, arrow.size, fill, border),
                            );
                        }
                    });
                });
        });
    }
}

/// How far the arrow's centre is inside the box's edge: shadcn's
/// `translate-y-[calc(-50%_-_2px)]` of a square that starts at the edge.
const ARROW_SINK: f32 = 2.0;

/// Paints the arrow that points from the tooltip's box to its `trigger`.
///
/// Not generic over the app, unlike a draw closure written inside
/// [`declare`]: a closure's type carries the type parameters of the function
/// it is written in, so a `'static` one there would demand `A: 'static` of
/// every layout.
fn arrow_painter(
    trigger: Id,
    size: f32,
    fill: egui::Color32,
    border: Vec2,
) -> impl Fn(&Painter, Rect, egui::Color32) + 'static {
    move |painter, padding_box, _| {
        paint_arrow(painter, padding_box.expand2(border), trigger, size, fill);
    }
}

fn paint_arrow(painter: &Painter, rect: Rect, trigger: Id, size: f32, fill: egui::Color32) {
    let Some(anchor) = painter
        .ctx()
        .read_response(trigger)
        .map(|response| response.rect)
    else {
        return;
    };
    let reach = size * std::f32::consts::FRAC_1_SQRT_2;
    let above = rect.center().y < anchor.center().y;
    let (edge, toward) = if above {
        (rect.max.y, 1.0)
    } else {
        (rect.min.y, -1.0)
    };
    let x = clamp(
        anchor.center().x,
        rect.min.x + reach,
        (rect.max.x - reach).max(rect.min.x + reach),
    );
    let centre: Pos2 = pos2(x, edge - toward * ARROW_SINK);
    let points = vec![
        pos2(centre.x - reach, centre.y),
        pos2(centre.x, centre.y - reach),
        pos2(centre.x + reach, centre.y),
        pos2(centre.x, centre.y + reach),
    ];
    painter.add(Shape::convex_polygon(points, fill, egui::Stroke::NONE));
}
