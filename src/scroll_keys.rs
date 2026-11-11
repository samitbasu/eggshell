//! Scrolling scroll containers with the keyboard, as a browser does, and
//! following the keyboard focus into view.
//!
//! The keys act on one container: the innermost that holds the keyboard
//! focus, or, with nothing focused, the one last pressed in. Not the one
//! under the pointer. They are read after the frame's widgets have been
//! drawn, so a widget that uses a key (a slider, a stepper, a list) has
//! consumed it already; a text field keeps all of them. The scroll goes
//! through [`scroll_by`], so the layout and egui agree on the offset.
//!
//! A key never chains: at the edge of the container the key is spent.

use egui::{Context, EventFilter, Id, Key, Vec2};

use crate::focus;
use crate::pipeline::PlacedNode;
use crate::scroll::{scroll_by, scroll_into_view};

/// The container's page keys leave this many lines of the last page in view.
const PAGE_OVERLAP_LINES: f32 = 1.0;

/// The scroll container at `nodes[index]` and where it sits in its frame.
struct Target<'a> {
    node: &'a PlacedNode,
    id: Id,
}

impl Target<'_> {
    /// One line: the container's own line height, so text scrolls by whole
    /// lines of the text it holds.
    fn line(&self) -> f32 {
        self.node.text().spec.line_height
    }

    fn view(&self) -> Vec2 {
        self.node
            .scroll_viewport()
            .map_or(Vec2::ZERO, |rect| rect.size())
    }

    /// A page along `view`: the viewport less the overlap.
    fn page(&self, view: f32) -> f32 {
        (view - PAGE_OVERLAP_LINES * self.line()).max(self.line())
    }
}

/// The container the keys act on: the innermost holding the focus, else, if
/// nothing has it, the one last pressed in.
fn target<'a>(ctx: &Context, nodes: &'a [PlacedNode]) -> Option<Target<'a>> {
    let scrollers = || {
        nodes
            .iter()
            .filter(|node| node.scroll.is_some())
            .filter_map(|node| {
                Some(Target {
                    node,
                    id: node.id()?,
                })
            })
    };
    if ctx.memory(|memory| memory.focused()).is_some() {
        return scrollers()
            .filter(|target| focus::focus_within(ctx, target.id))
            .next_back();
    }
    let pressed = pressed_in(ctx)?;
    scrollers().find(|target| target.id == pressed)
}

#[derive(Clone, Copy, Default)]
struct Pressed {
    pass: u64,
    scroller: Option<Id>,
}

fn pressed_key() -> Id {
    crate::key::root_id().with("scroll-pressed-in")
}

fn pressed_in(ctx: &Context) -> Option<Id> {
    ctx.data(|data| data.get_temp::<Pressed>(pressed_key()))?
        .scroller
}

/// Remembers the container a press landed in, the innermost whose viewport
/// holds it, or that it landed in none.
pub(crate) fn note_press(ctx: &Context, nodes: &[PlacedNode]) {
    let Some(at) = ctx.input(|input| {
        input
            .pointer
            .any_pressed()
            .then(|| input.pointer.interact_pos())
            .flatten()
    }) else {
        return;
    };
    let pass = ctx.cumulative_pass_nr();
    let hit = nodes
        .iter()
        .filter(|node| node.scroll_viewport().is_some_and(|view| view.contains(at)))
        .filter_map(PlacedNode::id)
        .next_back();
    ctx.data_mut(|data| {
        let kept = data.get_temp::<Pressed>(pressed_key()).unwrap_or_default();
        // Several trees may be drawn in a pass: one that holds the press wins.
        if hit.is_some() || kept.pass != pass || kept.scroller.is_none() {
            data.insert_temp(
                pressed_key(),
                Pressed {
                    pass,
                    scroller: hit,
                },
            );
        }
    });
}

/// Keeps the focused widget's arrow keys from moving the focus when they
/// scroll the container around it. Run before the widgets are drawn: one that
/// holds the arrows itself sets its own filter after.
pub(crate) fn hold_arrows(ctx: &Context, nodes: &[PlacedNode]) {
    let Some(focused) = ctx.memory(|memory| memory.focused()) else {
        return;
    };
    if ctx.text_edit_focused() {
        return;
    }
    let Some(target) = target(ctx, nodes) else {
        return;
    };
    let enabled = target.node.scroll.as_ref().map(|plan| plan.axis.enabled());
    if let Some(enabled) = enabled {
        let filter = EventFilter {
            horizontal_arrows: enabled.x,
            vertical_arrows: enabled.y,
            ..EventFilter::default()
        };
        ctx.memory_mut(|memory| memory.set_focus_lock_filter(focused, filter));
    }
}

/// What a key does to the container, as a function of its geometry.
#[derive(Clone, Copy)]
enum Move {
    Line(Axis, Sign),
    Page(Sign),
    Edge(Sign),
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy)]
enum Sign {
    Back,
    Forward,
}

impl Sign {
    fn of(self, amount: f32) -> f32 {
        match self {
            Self::Back => -amount,
            Self::Forward => amount,
        }
    }
}

/// Whether a key is Shift's, and who may take it.
#[derive(Clone, Copy, PartialEq)]
enum Chord {
    Plain,
    Shifted,
}

/// Whether the key goes to a focused control first, which acts on it.
#[derive(Clone, Copy, PartialEq)]
enum Claim {
    /// Only the container, or nothing, being focused lets it scroll.
    Contested,
    Free,
}

const BINDINGS: [(Key, Chord, Claim, Move); 10] = [
    (
        Key::ArrowUp,
        Chord::Plain,
        Claim::Free,
        Move::Line(Axis::Y, Sign::Back),
    ),
    (
        Key::ArrowDown,
        Chord::Plain,
        Claim::Free,
        Move::Line(Axis::Y, Sign::Forward),
    ),
    (
        Key::ArrowLeft,
        Chord::Plain,
        Claim::Free,
        Move::Line(Axis::X, Sign::Back),
    ),
    (
        Key::ArrowRight,
        Chord::Plain,
        Claim::Free,
        Move::Line(Axis::X, Sign::Forward),
    ),
    (
        Key::PageUp,
        Chord::Plain,
        Claim::Free,
        Move::Page(Sign::Back),
    ),
    (
        Key::PageDown,
        Chord::Plain,
        Claim::Free,
        Move::Page(Sign::Forward),
    ),
    (Key::Home, Chord::Plain, Claim::Free, Move::Edge(Sign::Back)),
    (
        Key::End,
        Chord::Plain,
        Claim::Free,
        Move::Edge(Sign::Forward),
    ),
    (
        Key::Space,
        Chord::Shifted,
        Claim::Contested,
        Move::Page(Sign::Back),
    ),
    (
        Key::Space,
        Chord::Plain,
        Claim::Contested,
        Move::Page(Sign::Forward),
    ),
];

/// Takes the presses of `key` with `chord` out of the frame's input and
/// counts them.
fn take(ctx: &Context, key: Key, chord: Chord) -> usize {
    ctx.input_mut(|input| {
        let mut taken = 0;
        input.events.retain(|event| {
            let matches = matches!(event, egui::Event::Key { key: pressed, pressed: true, modifiers, .. }
                if *pressed == key
                    && !modifiers.alt
                    && !modifiers.command
                    && !modifiers.ctrl
                    && modifiers.shift == (chord == Chord::Shifted));
            taken += usize::from(matches);
            !matches
        });
        taken
    })
}

/// Scrolls the container the keys pressed this frame act on.
pub(crate) fn apply_keys(ctx: &Context, nodes: &[PlacedNode]) {
    if ctx.text_edit_focused() {
        return;
    }
    let Some(target) = target(ctx, nodes) else {
        return;
    };
    let Some(plan) = target.node.scroll.as_ref() else {
        return;
    };
    let enabled = plan.axis.enabled();
    let primary = if enabled.y { Axis::Y } else { Axis::X };
    let focused = ctx.memory(|memory| memory.focused());
    let uncontested = focused.is_none_or(|id| id == target.id);
    let view = target.view();
    let along = |axis: Axis, vector: Vec2| match axis {
        Axis::X => vector.x,
        Axis::Y => vector.y,
    };
    let vector = |axis: Axis, amount: f32| match axis {
        Axis::X => Vec2::new(amount, 0.0),
        Axis::Y => Vec2::new(0.0, amount),
    };
    let mut by = Vec2::ZERO;
    for (key, chord, claim, step) in BINDINGS {
        let axis = match step {
            Move::Line(axis, _) => axis,
            Move::Page(_) | Move::Edge(_) => primary,
        };
        let scrolls = match axis {
            Axis::X => enabled.x,
            Axis::Y => enabled.y,
        };
        if !scrolls || (claim == Claim::Contested && !uncontested) {
            continue;
        }
        let presses = take(ctx, key, chord);
        if presses == 0 {
            continue;
        }
        let amount = match step {
            Move::Line(_, sign) => sign.of(target.line()),
            Move::Page(sign) => sign.of(target.page(along(axis, view))),
            Move::Edge(Sign::Back) => -along(axis, plan.offset),
            Move::Edge(Sign::Forward) => {
                along(axis, (plan.content - view).max(Vec2::ZERO) - plan.offset)
            }
        };
        by += vector(axis, amount * presses as f32);
    }
    if by != Vec2::ZERO {
        scroll_by(ctx, target.id, by);
    }
}

#[derive(Clone, Copy, Default)]
struct Followed(Option<Id>);

fn followed_key() -> Id {
    crate::key::root_id().with("scroll-followed-focus")
}

/// Scrolls the node that took the keyboard focus into view, unless a press
/// did: a widget under the pointer is where it is wanted. `subtrees` has the
/// focusable widgets of each identified node of the frame, the innermost
/// first.
pub(crate) fn follow_focus(ctx: &Context, subtrees: &[(Id, Box<[Id]>)]) {
    let now = ctx.memory(|memory| memory.focused());
    let before = ctx.data_mut(|data| {
        std::mem::replace(
            data.get_temp_mut_or_default::<Followed>(followed_key()),
            Followed(now),
        )
    });
    let pressing = ctx.input(|input| input.pointer.any_pressed());
    let Some(focused) = now.filter(|&now| before.0 != Some(now) && !pressing) else {
        return;
    };
    let holder = subtrees
        .iter()
        .find(|(node, widgets)| *node == focused || widgets.contains(&focused));
    if let Some((node, _)) = holder {
        scroll_into_view(ctx, *node, None);
    }
}
