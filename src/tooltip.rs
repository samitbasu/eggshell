//! Tooltips: hover intent, and the overlay that shows one.
//!
//! A trigger is a draw closure that says so ([`register`]); nothing is
//! declared for it at build. At the end of the draw the frame's triggers are
//! resolved against the pointer, the keyboard focus and the style's timing
//! ([`Tooltips::tick`], pure), and the next build declares one overlay per
//! tooltip that is open or still leaving ([`declare`]). The state is
//! `Kept::tooltips`, so it is per `LayoutState`.

use core::time::Duration;
use std::any::Any;
use std::hash::Hash;
use std::sync::Arc;

use egui::{Context, Id, Key, Painter, Pos2, Rect, Shape, Ui, Vec2, WidgetText, pos2};

use crate::builder::{Builder, Surface};
use crate::kept::Handle;
use crate::overlay::{Anchor, Band, DismissOn, Distance, Focus, Placement};
use crate::overlay_chain::OverlayChain;
use crate::overlay_input::Drag;
use crate::presence::Shown;
use crate::tw::tw;

/// What a tooltip shows.
#[derive(Clone)]
pub(crate) enum Content {
    Text(WidgetText),
    /// A [`Built`], typed by the application it is built for, which the
    /// state it is kept in is not.
    Built(Arc<dyn Any + Send + Sync>),
}

type BuildFn<A> = dyn Fn(&mut Builder<'_, '_, A>) + Send + Sync;

struct Built<A>(Box<BuildFn<A>>);

/// Builds a [`Content::Built`] for the application `A`. Only a caller that
/// knows `A: 'static` can make one, so a frame holds the one its triggers
/// left, and declaring the tooltips asks nothing of `A`.
pub(crate) type BuildBuilt<A> = fn(&mut Builder<'_, '_, A>, &(dyn Any + Send + Sync));

fn build_built<A: 'static>(b: &mut Builder<'_, '_, A>, built: &(dyn Any + Send + Sync)) {
    if let Some(built) = built.downcast_ref::<Built<A>>() {
        (built.0)(b);
    }
}

impl Content {
    pub(crate) fn built<A: 'static>(
        build: impl Fn(&mut Builder<'_, '_, A>) + Send + Sync + 'static,
    ) -> Self {
        Self::Built(Arc::new(Built::<A>(Box::new(build))))
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
    /// nothing where it does not [show](Self::shows_in), and for an
    /// application other than `b`'s.
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
                    description: accesskit_is_on(b.ctx())
                        .then(|| b.text_of(trigger, |b| content.build(b))),
                    content,
                }
            }
        }
    }

    /// The tooltip showing `text`.
    pub(crate) fn text(text: WidgetText) -> Self {
        Self {
            description: Some(text.text().to_owned()),
            content: Content::Text(text),
        }
    }
}

/// egui has no public query for this; its root node is made only while
/// accesskit is on.
fn accesskit_is_on(ctx: &Context) -> bool {
    ctx.has_accesskit_node(egui::accesskit_root_id())
}

/// Why a trigger wants its tooltip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// The pointer is on it; the tooltip waits for the delay.
    Pointer,
    /// It has keyboard focus; the tooltip is shown at once.
    Focus,
}

/// A trigger drawn this frame that wants its tooltip.
#[derive(Clone)]
pub(crate) struct Trigger {
    pub(crate) id: Id,
    pub(crate) content: Content,
    pub(crate) cause: Cause,
}

/// What closes a tooltip and keeps it closed until its trigger is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interruption {
    Press,
    Escape,
    Drag,
    Blur,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Timing {
    pub(crate) delay: Duration,
    pub(crate) grace: Duration,
}

#[derive(Clone)]
struct Present {
    trigger: Id,
    content: Content,
    open: bool,
}

#[derive(Clone, Copy, Debug)]
struct Waiting {
    trigger: Id,
    since: Duration,
}

/// Ticks in which focus restored to a trigger may show up in the frame's
/// triggers: egui applies a requested focus a pass later.
const RESTORE_TICKS: u8 = 3;

#[derive(Default)]
pub(crate) struct Tooltips {
    present: Vec<Present>,
    waiting: Option<Waiting>,
    latched: Option<Id>,
    /// A trigger that focus was just restored to, and how many ticks to wait
    /// for the frame's triggers to show it.
    restoring: Option<(Id, u8)>,
    closed_at: Option<Duration>,
}

impl Tooltips {
    /// Focus was given back to `target` by an overlay closing. That is focus
    /// staying put, not arriving: a tooltip that Esc closed stays closed
    /// until the trigger is left and returned to.
    pub(crate) fn focus_restored(&mut self, target: Id) {
        self.restoring = Some((target, RESTORE_TICKS));
    }

    /// The triggers whose tooltip is open, in order.
    fn open_triggers(&self) -> Vec<Id> {
        self.present
            .iter()
            .filter(|present| present.open)
            .map(|present| present.trigger)
            .collect()
    }

    fn is_open(&self) -> bool {
        self.present.iter().any(|present| present.open)
    }

    fn in_grace(&self, now: Duration, grace: Duration) -> bool {
        self.is_open()
            || self
                .closed_at
                .is_some_and(|at| now.saturating_sub(at) <= grace)
    }

    fn close_all(&mut self, now: Duration) {
        if self.is_open() {
            self.closed_at = Some(now);
        }
        for present in &mut self.present {
            present.open = false;
        }
    }

    fn open(&mut self, trigger: &Trigger, now: Duration) {
        self.close_all(now);
        self.closed_at = None;
        match self
            .present
            .iter_mut()
            .find(|present| present.trigger == trigger.id)
        {
            Some(present) => {
                present.content.clone_from(&trigger.content);
                present.open = true;
            }
            None => self.present.push(Present {
                trigger: trigger.id,
                content: trigger.content.clone(),
                open: true,
            }),
        }
    }

    /// Moves the intent to `now`, given what the frame's triggers want and
    /// whether something interrupted. Returns how long until the delay of
    /// the tooltip being waited for runs out, if one is.
    pub(crate) fn tick(
        &mut self,
        now: Duration,
        timing: Timing,
        wanted: Option<&Trigger>,
        interruption: Option<Interruption>,
    ) -> Option<Duration> {
        if interruption.is_some() {
            self.latched = wanted.map(|trigger| trigger.id);
            self.waiting = None;
            self.close_all(now);
            return None;
        }
        if let Some((target, left)) = self.restoring {
            match wanted.map(|trigger| trigger.id) {
                Some(id) if id == target => {
                    self.latched = Some(target);
                    self.restoring = None;
                }
                None if left > 1 => self.restoring = Some((target, left - 1)),
                _ => self.restoring = None,
            }
        }
        if self.latched != wanted.map(|trigger| trigger.id) {
            self.latched = None;
        }
        let Some(trigger) = wanted.filter(|_| self.latched.is_none()) else {
            self.waiting = None;
            self.close_all(now);
            return None;
        };
        if self
            .present
            .iter()
            .any(|present| present.open && present.trigger == trigger.id)
        {
            self.open(trigger, now);
            self.waiting = None;
            return None;
        }
        let since = match self.waiting {
            Some(waiting) if waiting.trigger == trigger.id => waiting.since,
            _ => {
                self.waiting = Some(Waiting {
                    trigger: trigger.id,
                    since: now,
                });
                now
            }
        };
        let delay = match trigger.cause {
            Cause::Focus => Duration::ZERO,
            Cause::Pointer if self.in_grace(now, timing.grace) => Duration::ZERO,
            Cause::Pointer => timing.delay,
        };
        let waited = now.saturating_sub(since);
        if waited >= delay {
            self.open(trigger, now);
            self.waiting = None;
            None
        } else {
            Some(delay - waited)
        }
    }
}

fn requests_id() -> Id {
    crate::key::root_id().with("tooltip-triggers")
}

/// Starts the frame's collection of triggers.
pub(crate) fn begin(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(requests_id(), Vec::<Trigger>::new()));
}

/// Says that the node of the draw closure `ui` is a trigger for `tip`.
/// `response` is the one the node's hover and focus are read from.
pub(crate) fn register(ui: &Ui, response: &egui::Response, tip: &Tooltip) {
    use crate::draw::NodeUi as _;
    let id = ui.node_id();
    let wants = |cause| {
        let trigger = Trigger {
            id,
            content: tip.content.clone(),
            cause,
        };
        ui.data_mut(|data| {
            data.get_temp_mut_or_default::<Vec<Trigger>>(requests_id())
                .push(trigger);
        });
    };
    if response.has_focus() && crate::overlay_input::keyboard_was_last(ui.ctx()) {
        wants(Cause::Focus);
    }
    if response.contains_pointer() {
        wants(Cause::Pointer);
    }
    if let Some(description) = &tip.description {
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_description(description.as_str());
        });
    }
}

/// The triggers this frame, resolved: a pointer over a trigger beats focus
/// on another, and the one drawn last (the deepest) wins.
fn wanted(triggers: Vec<Trigger>) -> Option<Trigger> {
    let pointer = triggers
        .iter()
        .rfind(|trigger| trigger.cause == Cause::Pointer);
    pointer.or_else(|| triggers.iter().next_back()).cloned()
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

/// Ends the frame's collection: moves the state to this frame.
pub(crate) fn end(ctx: &Context, kept: &Handle, interruption: Option<Interruption>) {
    let triggers = ctx
        .data_mut(|data| data.remove_temp::<Vec<Trigger>>(requests_id()))
        .unwrap_or_default();
    let style = crate::Style::of(ctx);
    let timing = Timing {
        delay: style.tooltip.delay,
        grace: style.tooltip.grace,
    };
    let now = Duration::from_secs_f64(ctx.input(|input| input.time).max(0.0));
    let wanted = wanted(triggers);
    let (wait, changed) = kept.with(|kept| {
        let before = kept.tooltips.open_triggers();
        let wait = kept
            .tooltips
            .tick(now, timing, wanted.as_ref(), interruption);
        (wait, kept.tooltips.open_triggers() != before)
    });
    if let Some(wait) = wait {
        ctx.request_repaint_after(wait);
    }
    // The state moves at the end of the draw and the overlay is declared by
    // the next build, which a reactive app (eframe, the browser) only runs
    // when asked: without input, a tooltip that just opened would never show.
    if changed {
        ctx.request_repaint();
    }
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
    let style = crate::Style::of(b.ctx());
    let present = b.kept().with(|kept| kept.tooltips.present.clone());
    for tip in present {
        let slot = b.slot(&Tip(tip.trigger));
        let shows = tip.content.shows_in(b);
        let kept = b.kept().with(|kept| {
            let leaving = kept.presences.contains(slot.id);
            if !shows || (!tip.open && !leaving) {
                kept.tooltips
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
        let text = style.role(&tooltip.text).clone();
        let colors = tooltip.popup.paint.states.rest;
        let classes = tw()
            .flex()
            .flex_col()
            .max_w_px(tooltip.max_width)
            .px(tooltip.padding.x / 4.0)
            .py(tooltip.padding.y / 4.0)
            .border(tooltip.popup.border_width)
            .border_color(colors.border)
            .bg(colors.fill)
            .text(colors.text)
            .rounded_px(f32::from(tooltip.radius))
            .role(text);
        let classes = match tooltip.popup.paint.shadow {
            Some(shadow) => classes.shadow(shadow),
            None => classes,
        };
        let border = Vec2::splat(tooltip.popup.border_width);
        OverlayChain::new(b.reborrow(), slot)
            .band(Band::Tooltip)
            .anchor(Anchor::Node(trigger))
            .placement(Placement::tooltip().gap(Distance::new(gap)))
            .focus(Focus::None)
            .dismiss_on(DismissOn::Never)
            .shown(if tip.open { Shown::Open } else { Shown::Closed })
            .children(|s| {
                s.div_id("Box", classes, |s| {
                    tip.content.build(s);
                    if let Some(arrow) = arrow {
                        s.paint(
                            tw().absolute().inset_px(0.0),
                            arrow_painter(trigger, arrow.size, colors.fill, border),
                        );
                    }
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
    let x = anchor.center().x.clamp(
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

#[cfg(test)]
mod tests {
    use super::*;

    const TIMING: Timing = Timing {
        delay: Duration::from_millis(500),
        grace: Duration::from_millis(300),
    };

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn trigger(name: &str, cause: Cause) -> Trigger {
        Trigger {
            id: Id::NULL.with(name),
            content: Content::Text(name.into()),
            cause,
        }
    }

    fn id_of(trigger: Option<Trigger>) -> Option<Id> {
        trigger.map(|trigger| trigger.id)
    }

    fn open_ids(tooltips: &Tooltips) -> Vec<Id> {
        tooltips
            .present
            .iter()
            .filter(|present| present.open)
            .map(|present| present.trigger)
            .collect()
    }

    #[test]
    fn the_pointer_waits_for_the_delay_and_reports_what_is_left() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        assert_eq!(
            tooltips.tick(ms(1000), TIMING, Some(&a), None),
            Some(ms(500))
        );
        assert_eq!(
            tooltips.tick(ms(1200), TIMING, Some(&a), None),
            Some(ms(300))
        );
        assert!(open_ids(&tooltips).is_empty());
        assert_eq!(tooltips.tick(ms(1500), TIMING, Some(&a), None), None);
        assert_eq!(open_ids(&tooltips), [a.id]);
    }

    #[test]
    fn leaving_before_the_delay_forgets_the_wait() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(400), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(450), TIMING, Some(&a), None),
            Some(ms(500))
        );
    }

    #[test]
    fn focus_restored_to_the_trigger_is_not_an_arrival() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Focus);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(50), TIMING, None, Some(Interruption::Escape));
        tooltips.focus_restored(a.id);
        for at in [100, 150, 200] {
            tooltips.tick(ms(at), TIMING, Some(&a), None);
            assert!(open_ids(&tooltips).is_empty(), "open at {at}");
        }
        tooltips.tick(ms(250), TIMING, None, None);
        tooltips.tick(ms(300), TIMING, Some(&a), None);
        assert_eq!(open_ids(&tooltips), [a.id], "a fresh arrival opens it");
    }

    #[test]
    fn focus_opens_at_once() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Focus);
        assert_eq!(tooltips.tick(ms(0), TIMING, Some(&a), None), None);
        assert_eq!(open_ids(&tooltips), [a.id]);
    }

    #[test]
    fn another_trigger_opens_at_once_within_the_grace_and_not_after_it() {
        let mut tooltips = Tooltips::default();
        let (a, b) = (trigger("a", Cause::Pointer), trigger("b", Cause::Pointer));
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(600), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(850), TIMING, Some(&b), None),
            None,
            "grace"
        );
        assert_eq!(open_ids(&tooltips), [b.id]);
        tooltips.tick(ms(900), TIMING, None, None);
        assert_eq!(
            tooltips.tick(ms(1300), TIMING, Some(&a), None),
            Some(ms(500)),
            "grace is over"
        );
    }

    #[test]
    fn moving_straight_to_another_trigger_swaps_which_is_open() {
        let mut tooltips = Tooltips::default();
        let (a, b) = (trigger("a", Cause::Pointer), trigger("b", Cause::Pointer));
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(550), TIMING, Some(&b), None);
        assert_eq!(open_ids(&tooltips), [b.id]);
        assert_eq!(tooltips.present.len(), 2, "the first is leaving");
    }

    #[test]
    fn an_interruption_closes_and_keeps_it_closed_until_the_trigger_is_left() {
        let mut tooltips = Tooltips::default();
        let a = trigger("a", Cause::Pointer);
        tooltips.tick(ms(0), TIMING, Some(&a), None);
        tooltips.tick(ms(500), TIMING, Some(&a), None);
        tooltips.tick(ms(600), TIMING, Some(&a), Some(Interruption::Press));
        assert!(open_ids(&tooltips).is_empty());
        assert_eq!(tooltips.tick(ms(2000), TIMING, Some(&a), None), None);
        assert!(open_ids(&tooltips).is_empty(), "still latched");
        tooltips.tick(ms(2100), TIMING, None, None);
        assert!(tooltips.tick(ms(2200), TIMING, Some(&a), None).is_some());
    }

    #[test]
    fn a_pointer_over_one_trigger_beats_focus_on_another_and_the_deepest_wins() {
        let focus = trigger("focus", Cause::Focus);
        let outer = trigger("outer", Cause::Pointer);
        let inner = trigger("inner", Cause::Pointer);
        assert_eq!(
            id_of(wanted(vec![focus.clone(), outer, inner.clone()])),
            Some(inner.id)
        );
        assert_eq!(id_of(wanted(vec![focus.clone()])), Some(focus.id));
        assert_eq!(id_of(wanted(vec![])), None);
    }
}
