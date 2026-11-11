//! Interaction classes: what a node senses, groups, and the paint-only state
//! variants (`hover:`, `active:`, `focus:`, `focus-within:`, `group-hover:`)
//! with their transition.

use core::time::Duration;

use egui::{Color32, Sense, Shadow, Vec2};

use super::{BoxPaint, Easing, Opacity, Progress, Ring, RingPlacement, Sides, Tw};
use crate::text::TextDecoration;

/// What a node senses. Egui's own [`Sense`] cannot say "nothing declared":
/// its hover-only sense is the empty set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Sensing {
    #[default]
    None,
    Hover,
    Click,
    Drag,
    ClickDrag,
}

impl Sensing {
    fn clicks(self) -> bool {
        matches!(self, Self::Click | Self::ClickDrag)
    }

    fn drags(self) -> bool {
        matches!(self, Self::Drag | Self::ClickDrag)
    }

    pub(crate) fn or(self, other: Self) -> Self {
        match (
            self.clicks() || other.clicks(),
            self.drags() || other.drags(),
        ) {
            (true, true) => Self::ClickDrag,
            (true, false) => Self::Click,
            (false, true) => Self::Drag,
            (false, false) if self == Self::Hover || other == Self::Hover => Self::Hover,
            (false, false) => Self::None,
        }
    }

    /// Enough to be pressed: a node that only hovers, or senses nothing,
    /// becomes clickable.
    fn pressable(self) -> Self {
        match self {
            Self::None | Self::Hover => Self::Click,
            other => other,
        }
    }

    pub(crate) fn is_none(self) -> bool {
        self == Self::None
    }
}

impl From<Sensing> for Sense {
    fn from(sensing: Sensing) -> Sense {
        match sensing {
            Sensing::None | Sensing::Hover => Sense::hover(),
            Sensing::Click => Sense::click(),
            Sensing::Drag => Sense::drag(),
            Sensing::ClickDrag => Sense::click_and_drag(),
        }
    }
}

/// Whether the pointer can reach a node painted with a background or border,
/// or goes through it to what lies beneath, as CSS `pointer-events` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PointerEvents {
    #[default]
    Auto,
    None,
}

/// Whether a sensing node takes part in keyboard navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Focusability {
    #[default]
    Tab,
    Skip,
}

impl Focusability {
    pub(crate) fn merge(self, other: Self) -> Self {
        if self == Self::Skip || other == Self::Skip {
            Self::Skip
        } else {
            Self::Tab
        }
    }

    /// The egui sense for `sensing`, focusable unless the node opts out.
    pub(crate) fn sense(self, sensing: Sensing) -> Sense {
        let sense = Sense::from(sensing);
        match self {
            Self::Tab => sense,
            Self::Skip => sense - Sense::FOCUSABLE,
        }
    }
}

/// Whether a node is the `group` that its descendants' `group_hover` follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Grouping {
    #[default]
    Ungrouped,
    Group,
}

impl Grouping {
    pub(crate) fn merge(self, other: Self) -> Self {
        if self == Self::Group || other == Self::Group {
            Self::Group
        } else {
            Self::Ungrouped
        }
    }
}

/// The paint a state variant changes: colours, opacity, underline, ring,
/// shadow and translation. It has no layout or font-size methods on purpose,
/// so a state change never needs another layout pass.
///
/// Fields a variant leaves unset keep the base paint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Paint {
    background: Option<Color32>,
    text: Option<Color32>,
    border: Option<Color32>,
    ring: Option<Ring>,
    ring_placement: Option<RingPlacement>,
    shadow: Option<Shadow>,
    opacity: Option<Opacity>,
    decoration: Option<TextDecoration>,
    translate: Option<Vec2>,
}

impl Paint {
    /// `bg-<color>`.
    pub fn bg(mut self, color: impl Into<Color32>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// `text-<color>`. Descendants that inherit their text colour follow it.
    pub fn text(mut self, color: impl Into<Color32>) -> Self {
        self.text = Some(color.into());
        self
    }

    /// `border-<color>`: every side.
    pub fn border_color(mut self, color: impl Into<Color32>) -> Self {
        self.border = Some(color.into());
        self
    }

    /// `ring-<width>`.
    pub fn ring(mut self, width: f32, color: impl Into<Color32>) -> Self {
        self.ring = Some(Ring {
            width,
            color: color.into(),
        });
        self
    }

    /// `ring-inset`: the ring is drawn inside the border box.
    pub fn ring_inset(mut self) -> Self {
        self.ring_placement = Some(RingPlacement::Inside);
        self
    }

    /// `shadow-*`.
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// `opacity-*`.
    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// `underline`.
    pub fn underline(mut self) -> Self {
        self.decoration = Some(TextDecoration::Underline);
        self
    }

    /// `no-underline`.
    pub fn no_underline(mut self) -> Self {
        self.decoration = Some(TextDecoration::None);
        self
    }

    /// `translate-x-* translate-y-*`: moves the painting of the node and its
    /// subtree by `dx` and `dy` points. As with CSS transforms, layout and
    /// hit rects stay where they are, and the move eases with `transition`.
    pub fn translate(mut self, dx: f32, dy: f32) -> Self {
        self.translate = Some(Vec2::new(dx, dy));
        self
    }

    /// `translate-y-*`: [`translate`](Self::translate) down by `dy` points.
    pub fn translate_y(self, dy: f32) -> Self {
        self.translate(0.0, dy)
    }

    pub(crate) fn sets_text_color(&self) -> bool {
        self.text.is_some()
    }

    /// This paint with the properties `other` sets on top.
    fn merge(self, other: Self) -> Self {
        Self {
            background: other.background.or(self.background),
            text: other.text.or(self.text),
            border: other.border.or(self.border),
            ring: other.ring.or(self.ring),
            ring_placement: other.ring_placement.or(self.ring_placement),
            shadow: other.shadow.or(self.shadow),
            opacity: other.opacity.or(self.opacity),
            decoration: other.decoration.or(self.decoration),
            translate: other.translate.or(self.translate),
        }
    }

    /// `look` moved `progress` of the way to this paint. Colours and opacity
    /// blend; ring, shadow and underline switch at the halfway point.
    fn towards(&self, look: Look, progress: Progress) -> Look {
        if progress == Progress::START {
            return look;
        }
        let switched = progress >= Progress::new(0.5);
        let blend = |from: Option<Color32>, to: Option<Color32>| match (from, to) {
            (Some(from), Some(to)) => Some(from.lerp_to_gamma(to, progress.into())),
            (None, Some(to)) => Some(to.gamma_multiply(progress.into())),
            (from, None) => from,
        };
        let sides = look.paint.border_colors;
        let border = |side: Option<Color32>| blend(side, self.border);
        let paint = BoxPaint {
            background: blend(look.paint.background, self.background),
            border_colors: Sides {
                top: border(sides.top),
                right: border(sides.right),
                bottom: border(sides.bottom),
                left: border(sides.left),
            },
            ring: self.ring.filter(|_| switched).or(look.paint.ring),
            ring_placement: self
                .ring_placement
                .filter(|_| switched)
                .unwrap_or(look.paint.ring_placement),
            shadow: self.shadow.filter(|_| switched).or(look.paint.shadow),
            opacity: self.opacity.map_or(look.paint.opacity, |to| {
                Opacity::new(progress.lerp(look.paint.opacity.into(), to.into()))
            }),
            ..look.paint
        };
        Look {
            paint,
            translate: self.translate.map_or(look.translate, |to| {
                look.translate + (to - look.translate) * f32::from(progress)
            }),
            text: blend(Some(look.text), self.text).unwrap_or(look.text),
            decoration: self
                .decoration
                .filter(|_| switched)
                .unwrap_or(look.decoration),
        }
    }
}

/// The states a node can be styled for, in the order they override one
/// another (the order Tailwind emits them in).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum State {
    GroupHover,
    Hover,
    FocusWithin,
    Focus,
    Active,
    DragAccepts,
    DragOver,
    DragLifted,
}

impl State {
    pub(crate) const ALL: [Self; 8] = [
        Self::GroupHover,
        Self::Hover,
        Self::FocusWithin,
        Self::Focus,
        Self::Active,
        Self::DragAccepts,
        Self::DragOver,
        Self::DragLifted,
    ];
}

/// How far each state has taken hold.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Presence {
    pub group_hover: Progress,
    pub hover: Progress,
    pub focus_within: Progress,
    pub focus: Progress,
    pub active: Progress,
    pub drag_accepts: Progress,
    pub drag_over: Progress,
    pub drag_lifted: Progress,
}

impl Presence {
    /// No state has taken hold.
    #[cfg(test)]
    pub(crate) const AT_REST: Self = Self {
        group_hover: Progress::START,
        hover: Progress::START,
        focus_within: Progress::START,
        focus: Progress::START,
        active: Progress::START,
        drag_accepts: Progress::START,
        drag_over: Progress::START,
        drag_lifted: Progress::START,
    };

    fn of(self, state: State) -> Progress {
        match state {
            State::GroupHover => self.group_hover,
            State::Hover => self.hover,
            State::FocusWithin => self.focus_within,
            State::Focus => self.focus,
            State::Active => self.active,
            State::DragAccepts => self.drag_accepts,
            State::DragOver => self.drag_over,
            State::DragLifted => self.drag_lifted,
        }
    }
}

/// What a node is painted with, besides its geometry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    pub paint: BoxPaint,
    /// How far the node and its subtree are painted from their layout.
    pub translate: Vec2,
    pub text: Color32,
    pub decoration: TextDecoration,
}

/// A node's state variants and how they transition.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Variants {
    hover: Option<Paint>,
    active: Option<Paint>,
    group_hover: Option<Paint>,
    focus: Option<Paint>,
    focus_within: Option<Paint>,
    drag_accepts: Option<Paint>,
    drag_over: Option<Paint>,
    drag_lifted: Option<Paint>,
    pub(crate) transition: Option<Duration>,
    pub(crate) animate_layout: Option<Duration>,
    pub(crate) easing: Easing,
}

impl Variants {
    /// Whether a `focus:` look is set.
    pub(crate) fn has_focus(&self) -> bool {
        self.focus.is_some()
    }

    /// These variants with `other`'s on top, state by state.
    pub(crate) fn merge(self, other: Self) -> Self {
        let both = |ours: Option<Paint>, theirs: Option<Paint>| match (ours, theirs) {
            (Some(ours), Some(theirs)) => Some(ours.merge(theirs)),
            (ours, theirs) => theirs.or(ours),
        };
        Self {
            hover: both(self.hover, other.hover),
            active: both(self.active, other.active),
            group_hover: both(self.group_hover, other.group_hover),
            focus: both(self.focus, other.focus),
            focus_within: both(self.focus_within, other.focus_within),
            drag_accepts: both(self.drag_accepts, other.drag_accepts),
            drag_over: both(self.drag_over, other.drag_over),
            drag_lifted: both(self.drag_lifted, other.drag_lifted),
            transition: other.transition.or(self.transition),
            animate_layout: other.animate_layout.or(self.animate_layout),
            easing: if other.easing == Easing::Default {
                self.easing
            } else {
                other.easing
            },
        }
    }

    pub(crate) fn get(&self, state: State) -> Option<&Paint> {
        match state {
            State::GroupHover => self.group_hover.as_ref(),
            State::Hover => self.hover.as_ref(),
            State::FocusWithin => self.focus_within.as_ref(),
            State::Focus => self.focus.as_ref(),
            State::Active => self.active.as_ref(),
            State::DragAccepts => self.drag_accepts.as_ref(),
            State::DragOver => self.drag_over.as_ref(),
            State::DragLifted => self.drag_lifted.as_ref(),
        }
    }

    /// Whether any variant changes the text colour.
    pub(crate) fn sets_text_color(&self) -> bool {
        State::ALL
            .iter()
            .any(|&state| self.get(state).is_some_and(Paint::sets_text_color))
    }

    /// `base` with every variant applied at its [`Presence`].
    pub(crate) fn resolve(&self, base: Look, presence: Presence) -> Look {
        State::ALL.iter().fold(base, |look, &state| {
            self.get(state)
                .map_or(look, |paint| paint.towards(look, presence.of(state)))
        })
    }
}

impl Tw {
    /// `cursor-pointer`-like sense: the node reports clicks.
    pub fn sense_click(mut self) -> Self {
        self.sensing = self.sensing.or(Sensing::Click);
        self
    }

    /// The node reports drags.
    pub fn sense_drag(mut self) -> Self {
        self.sensing = self.sensing.or(Sensing::Drag);
        self
    }

    /// The node reports clicks and drags.
    pub fn sense_click_drag(mut self) -> Self {
        self.sensing = self.sensing.or(Sensing::ClickDrag);
        self
    }

    /// The node reports hover only.
    pub fn sense_hover(mut self) -> Self {
        self.sensing = self.sensing.or(Sensing::Hover);
        self
    }

    /// `group`: marks the node whose hover its descendants' `group_hover`
    /// variants follow. The node must sense something; it is hovered as
    /// [`hover`](Self::hover) defines.
    pub fn group(mut self) -> Self {
        self.grouping = Grouping::Group;
        self
    }

    /// `hover:`. The pointer is over the node's border box with no sensing
    /// node above it, or over a sensing descendant, as `:hover` is in CSS.
    /// Implies [`sense_hover`](Self::sense_hover).
    /// `style` receives the paint set so far, so calls accumulate.
    pub fn hover(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.hover = Some(style(self.variants.hover.unwrap_or_default()));
        self.sensing = self.sensing.or(Sensing::Hover);
        self
    }

    /// `active:`. The primary button is held on the node. Implies at least
    /// [`sense_click`](Self::sense_click).
    pub fn active(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.active = Some(style(self.variants.active.unwrap_or_default()));
        self.sensing = self.sensing.pressable();
        self
    }

    /// `group-hover:`. The nearest ancestor marked [`group`](Self::group) is
    /// hovered.
    pub fn group_hover(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.group_hover = Some(style(self.variants.group_hover.unwrap_or_default()));
        self
    }

    /// `focus-visible:`. The node has keyboard focus. Implies at least
    /// [`sense_click`](Self::sense_click), which makes the node a Tab stop
    /// (see [`focusable_none`](Self::focusable_none)). egui does not give
    /// focus to a node on a click, so this is `focus-visible`, not `focus`.
    pub fn focus(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.focus = Some(style(self.variants.focus.unwrap_or_default()));
        self.sensing = self.sensing.pressable();
        self
    }

    /// `focus-within:`. The node, or a widget its draw closure or a
    /// descendant's registered, has keyboard focus. Needs an identified node
    /// but no sense. Widgets that appear this frame count from the next one.
    pub fn focus_within(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.focus_within = Some(style(self.variants.focus_within.unwrap_or_default()));
        self
    }

    /// A drag that the node's [`drop_target`](crate::NodeHandle::drop_target)s
    /// would take is in progress, anywhere in the window: the drop zones of
    /// a drag light up. Known for in-app drags and, for files dragged in from
    /// outside, from the moment they hover over the window. Needs an
    /// identified node.
    pub fn drag_accepts(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.drag_accepts = Some(style(self.variants.drag_accepts.unwrap_or_default()));
        self
    }

    /// [`drag_accepts`](Self::drag_accepts), and the drag is over this
    /// target, so a release here drops here. It applies on top of
    /// `drag_accepts`. It stays off for files dragged in from outside while
    /// the platform gives no pointer position (winit 0.30): nothing is shown
    /// as targeted on a guess. Of nested targets, only the innermost one that
    /// takes the drag is over.
    pub fn drag_over(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.drag_over = Some(style(self.variants.drag_over.unwrap_or_default()));
        self
    }

    /// The node is the source of the drag in progress. With
    /// [`DragImage::Live`](crate::DragImage::Live) the node is the image that
    /// follows the pointer, and this is its look; with a [`drag_preview`](crate::NodeHandle::drag_preview) or no image it
    /// is the look of the node in its slot. The slot of a live image is not
    /// painted by the node, since the node has left it: style a node around
    /// it from [`Builder::dragging`](crate::Builder::dragging).
    pub fn drag_lifted(mut self, style: impl FnOnce(Paint) -> Paint) -> Self {
        self.variants.drag_lifted = Some(style(self.variants.drag_lifted.unwrap_or_default()));
        self
    }

    /// Leaves a sensing node out of keyboard navigation, like a splitter
    /// that only a pointer drags. It still senses clicks and drags, and
    /// `focus` variants never apply to it.
    pub fn focusable_none(mut self) -> Self {
        self.focusability = Focusability::Skip;
        self
    }

    /// `transition duration-*`: animates the node's variant changes over
    /// `duration` instead of switching, along Tailwind's default curve
    /// `cubic-bezier(0.4, 0, 0.2, 1)` unless an `ease_*` class picks another.
    ///
    /// ```html
    /// <button class="bg-primary hover:bg-primary/90 transition duration-150"></button>
    /// ```
    pub fn transition(mut self, duration: Duration) -> Self {
        self.variants.transition = Some(duration);
        self
    }

    /// Animates the node to its new place when layout moves it, over
    /// `duration`, along the `ease_*` curve (Tailwind's default unless one is
    /// picked). Layout and hit rects are final at once; only the painting of
    /// the node and its subtree catches up, from where it was last frame.
    ///
    /// A node's place is its position within its parent, so a child that
    /// opts in too does not animate again when only its parent moved. A node
    /// seen for the first time, and every node on a frame where the window
    /// was resized, is placed at once, unlike CSS view transitions. Moves
    /// only: a change of size is not animated. Needs an identified node.
    pub fn animate_layout(mut self, duration: Duration) -> Self {
        self.variants.animate_layout = Some(duration);
        self
    }

    /// `ease-linear`: the transition's timing function is `linear`.
    ///
    /// ```html
    /// <div class="transition duration-150 ease-linear"></div>
    /// ```
    pub fn ease_linear(self) -> Self {
        self.easing(Easing::Linear)
    }

    /// `ease-in`: the transition's timing function is
    /// `cubic-bezier(0.4, 0, 1, 1)`.
    ///
    /// ```html
    /// <div class="transition duration-150 ease-in"></div>
    /// ```
    pub fn ease_in(self) -> Self {
        self.easing(Easing::In)
    }

    /// `ease-out`: the transition's timing function is
    /// `cubic-bezier(0, 0, 0.2, 1)`.
    ///
    /// ```html
    /// <div class="transition duration-150 ease-out"></div>
    /// ```
    pub fn ease_out(self) -> Self {
        self.easing(Easing::Out)
    }

    /// `ease-in-out`: the transition's timing function is
    /// `cubic-bezier(0.4, 0, 0.2, 1)`.
    ///
    /// ```html
    /// <div class="transition duration-150 ease-in-out"></div>
    /// ```
    pub fn ease_in_out(self) -> Self {
        self.easing(Easing::InOut)
    }

    fn easing(mut self, easing: Easing) -> Self {
        self.variants.easing = easing;
        self
    }

    /// Whether the classes declare a sense, a group, a `hover` or `active`
    /// variant a transition or an animated layout, which only an identified node can carry.
    /// `group_hover` alone reads its group's state and needs no identity of
    /// its own, so an anonymous child can follow its parent's hover.
    pub(crate) fn needs_identity(&self) -> bool {
        !self.sensing.is_none()
            || self.grouping == Grouping::Group
            || self.variants.get(State::Hover).is_some()
            || self.variants.get(State::Active).is_some()
            || self.variants.get(State::Focus).is_some()
            || self.variants.get(State::FocusWithin).is_some()
            || self.variants.get(State::DragAccepts).is_some()
            || self.variants.get(State::DragOver).is_some()
            || self.variants.get(State::DragLifted).is_some()
            || self.variants.transition.is_some()
            || self.variants.animate_layout.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variants_ring_inset_merges_over_its_ring_and_switches_with_it() {
        let ring = Paint::default().ring(2.0, Color32::RED);
        let merged = ring.merge(Paint::default().ring_inset());
        assert_eq!(merged.ring, ring.ring, "the ring stays");
        assert_eq!(merged.ring_placement, Some(RingPlacement::Inside));
        let look = Look {
            paint: BoxPaint::default(),
            translate: Vec2::ZERO,
            text: Color32::BLACK,
            decoration: TextDecoration::None,
        };
        let shown = merged.towards(look, Progress::END);
        assert_eq!(shown.paint.ring_placement, RingPlacement::Inside);
        let hidden = merged.towards(look, Progress::START);
        assert_eq!(hidden.paint.ring_placement, RingPlacement::Outside);
    }
    use crate::tw::tw;

    fn look() -> Look {
        Look {
            paint: BoxPaint {
                background: Some(Color32::from_rgb(0, 0, 0)),
                ..BoxPaint::default()
            },
            translate: Vec2::ZERO,
            text: Color32::BLACK,
            decoration: TextDecoration::None,
        }
    }

    #[test]
    fn senses_combine() {
        assert_eq!(Sensing::Click.or(Sensing::Drag), Sensing::ClickDrag);
        assert_eq!(Sensing::Hover.or(Sensing::Click), Sensing::Click);
        assert_eq!(Sensing::None.or(Sensing::Hover), Sensing::Hover);
        assert_eq!(Sensing::None.or(Sensing::None), Sensing::None);
        assert_eq!(Sensing::Drag.pressable(), Sensing::Drag);
    }

    #[test]
    fn variants_imply_senses_and_leave_layout_alone() {
        assert_eq!(tw().sensing, Sensing::None);
        assert_eq!(tw().hover(|p| p.bg(Color32::RED)).sensing, Sensing::Hover);
        assert_eq!(tw().active(|p| p.bg(Color32::RED)).sensing, Sensing::Click);
        assert_eq!(
            tw().sense_drag().active(|p| p.underline()).sensing,
            Sensing::Drag
        );
        assert_eq!(
            tw().group_hover(|p| p.bg(Color32::RED)).sensing,
            Sensing::None
        );
        assert_eq!(
            *tw()
                .hover(|p| p.bg(Color32::RED))
                .transition(Duration::from_millis(100))
                .style(),
            *tw().style()
        );
    }

    #[test]
    fn focus_implies_a_focusable_sense_and_focus_within_none() {
        assert_eq!(tw().focus(|p| p.underline()).sensing, Sensing::Click);
        assert_eq!(tw().focus_within(|p| p.underline()).sensing, Sensing::None);
        assert!(tw().focus(|p| p.underline()).needs_identity());
        assert!(tw().focus_within(|p| p.underline()).needs_identity());
        assert_eq!(
            *tw().focus(|p| p.underline()).style(),
            *tw().style(),
            "focus never touches layout"
        );
    }

    #[test]
    fn opting_out_removes_only_the_focusable_flag() {
        let focusable = Focusability::Tab.sense(Sensing::Click);
        let skipped = Focusability::Skip.sense(Sensing::Click);
        assert!(focusable.is_focusable() && focusable.senses_click());
        assert!(!skipped.is_focusable() && skipped.senses_click());
        assert!(!Focusability::Tab.sense(Sensing::Hover).is_focusable());
        assert!(Focusability::Skip.sense(Sensing::Drag).senses_drag());
    }

    #[test]
    fn focus_overrides_hover_and_active_overrides_focus() {
        let variants = tw()
            .hover(|p| p.bg(Color32::RED))
            .focus(|p| p.bg(Color32::GREEN))
            .active(|p| p.bg(Color32::BLUE))
            .variants;
        let presence = |hover, focus, active| Presence {
            hover,
            focus,
            active,
            ..Presence::AT_REST
        };
        let background = |presence| variants.resolve(look(), presence).paint.background;
        let (on, off) = (Progress::END, Progress::START);
        assert_eq!(background(presence(on, on, off)), Some(Color32::GREEN));
        assert_eq!(background(presence(on, on, on)), Some(Color32::BLUE));
        assert_eq!(background(presence(on, off, off)), Some(Color32::RED));
    }

    #[test]
    fn hover_calls_accumulate() {
        let styled = tw()
            .hover(|p| p.bg(Color32::RED))
            .hover(|p| p.text(Color32::WHITE));
        assert_eq!(
            styled.variants.hover,
            Some(Paint::default().bg(Color32::RED).text(Color32::WHITE))
        );
    }

    #[test]
    fn drag_states_need_an_identity_and_leave_layout_alone() {
        let red = |p: Paint| p.bg(Color32::RED);
        for styled in [
            tw().drag_accepts(red),
            tw().drag_over(red),
            tw().drag_lifted(red),
        ] {
            assert!(styled.needs_identity());
            assert_eq!(*styled.style(), *tw().style());
            assert_eq!(styled.sensing, Sensing::None, "they sense nothing");
        }
    }

    #[test]
    fn drag_over_overrides_drag_accepts() {
        let variants = tw()
            .drag_accepts(|p| p.bg(Color32::RED))
            .drag_over(|p| p.bg(Color32::GREEN))
            .variants;
        let background = |presence| variants.resolve(look(), presence).paint.background;
        let accepting = Presence {
            drag_accepts: Progress::END,
            ..Presence::AT_REST
        };
        assert_eq!(background(accepting), Some(Color32::RED));
        let over = Presence {
            drag_over: Progress::END,
            ..accepting
        };
        assert_eq!(background(over), Some(Color32::GREEN));
    }

    #[test]
    fn plain_classes_are_not_interactive() {
        assert!(!tw().bg(Color32::RED).needs_identity());
        assert!(tw().sense_hover().needs_identity());
        assert!(tw().group().needs_identity());
        assert!(tw().transition(Duration::from_millis(1)).needs_identity());
        assert!(tw().hover(|p| p.bg(Color32::RED)).needs_identity());
        assert!(tw().active(|p| p.bg(Color32::RED)).needs_identity());
    }

    #[test]
    fn group_hover_alone_needs_no_identity() {
        assert!(!tw().group_hover(|p| p.bg(Color32::RED)).needs_identity());
    }

    #[test]
    fn paint_blends_towards_the_variant() {
        let to = Paint::default().bg(Color32::from_rgb(200, 100, 0));
        let at = |t| to.towards(look(), Progress::new(t)).paint.background;
        assert_eq!(at(0.0), look().paint.background);
        assert_eq!(at(1.0), Some(Color32::from_rgb(200, 100, 0)));
        let Some(mid) = at(0.5) else {
            panic!("blended background")
        };
        assert!(mid.r() > 0 && mid.r() < 200, "{mid:?}");
    }

    #[test]
    fn translate_moves_paint_in_proportion_and_leaves_layout_alone() {
        let to = Paint::default().translate(4.0, 2.0);
        let at = |t| to.towards(look(), Progress::new(t)).translate;
        assert_eq!(at(0.0), Vec2::ZERO);
        assert_eq!(at(0.5), Vec2::new(2.0, 1.0));
        assert_eq!(at(1.0), Vec2::new(4.0, 2.0));
        assert_eq!(
            Paint::default().translate_y(1.0),
            Paint::default().translate(0.0, 1.0)
        );
        let styled = tw().active(|p| p.translate_y(1.0));
        assert_eq!(*styled.style(), *tw().style());
        assert_eq!(styled.sensing, Sensing::Click);
    }

    #[test]
    fn a_background_the_base_lacks_fades_in() {
        let bare = Look {
            paint: BoxPaint::default(),
            ..look()
        };
        let to = Paint::default().bg(Color32::from_rgb(200, 100, 0));
        let half = to.towards(bare, Progress::new(0.5)).paint.background;
        assert!(half.is_some_and(|c| c.a() > 0 && c.a() < 255), "{half:?}");
    }

    #[test]
    fn later_states_override_earlier_ones() {
        let variants = tw()
            .group_hover(|p| p.bg(Color32::RED))
            .hover(|p| p.bg(Color32::GREEN))
            .variants;
        let on = Presence {
            group_hover: Progress::END,
            hover: Progress::END,
            ..Presence::AT_REST
        };
        assert_eq!(
            variants.resolve(look(), on).paint.background,
            Some(Color32::GREEN)
        );
        let group_only = Presence {
            hover: Progress::START,
            ..on
        };
        assert_eq!(
            variants.resolve(look(), group_only).paint.background,
            Some(Color32::RED)
        );
    }
}
