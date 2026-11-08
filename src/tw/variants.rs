//! Interaction classes: what a node senses, groups, and the paint-only state
//! variants (`hover:`, `active:`, `group-hover:`) with their transition.

use core::time::Duration;

use egui::{Color32, Sense, Shadow};

use super::{BoxPaint, Opacity, Progress, Ring, Sides, Tw};
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

/// Whether a node is the `group` that its descendants' `group_hover` follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Grouping {
    #[default]
    Ungrouped,
    Group,
}

/// The paint a state variant changes: colours, opacity, underline, ring and
/// shadow. It has no layout or font-size methods on purpose, so a state
/// change never needs another layout pass.
///
/// Fields a variant leaves unset keep the base paint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Paint {
    background: Option<Color32>,
    text: Option<Color32>,
    border: Option<Color32>,
    ring: Option<Ring>,
    shadow: Option<Shadow>,
    opacity: Option<Opacity>,
    decoration: Option<TextDecoration>,
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

    pub(crate) fn sets_text_color(&self) -> bool {
        self.text.is_some()
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
            shadow: self.shadow.filter(|_| switched).or(look.paint.shadow),
            opacity: self.opacity.map_or(look.paint.opacity, |to| {
                Opacity::new(progress.lerp(look.paint.opacity.into(), to.into()))
            }),
            ..look.paint
        };
        Look {
            paint,
            text: blend(Some(look.text), self.text).unwrap_or(look.text),
            decoration: self
                .decoration
                .filter(|_| switched)
                .unwrap_or(look.decoration),
        }
    }
}

/// The three states a node can be styled for, in the order they override
/// one another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum State {
    GroupHover,
    Hover,
    Active,
}

impl State {
    pub(crate) const ALL: [Self; 3] = [Self::GroupHover, Self::Hover, Self::Active];
}

/// How far each state has taken hold.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Presence {
    pub group_hover: Progress,
    pub hover: Progress,
    pub active: Progress,
}

impl Presence {
    fn of(self, state: State) -> Progress {
        match state {
            State::GroupHover => self.group_hover,
            State::Hover => self.hover,
            State::Active => self.active,
        }
    }
}

/// What a node is painted with, besides its geometry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    pub paint: BoxPaint,
    pub text: Color32,
    pub decoration: TextDecoration,
}

/// A node's state variants and how they transition.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Variants {
    hover: Option<Paint>,
    active: Option<Paint>,
    group_hover: Option<Paint>,
    pub(crate) transition: Option<Duration>,
}

impl Variants {
    pub(crate) fn get(&self, state: State) -> Option<&Paint> {
        match state {
            State::GroupHover => self.group_hover.as_ref(),
            State::Hover => self.hover.as_ref(),
            State::Active => self.active.as_ref(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        State::ALL.iter().all(|&state| self.get(state).is_none())
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
    /// variants follow. The node must sense something, and hovering any
    /// sensing descendant counts as hovering the group, as in CSS.
    pub fn group(mut self) -> Self {
        self.grouping = Grouping::Group;
        self
    }

    /// `hover:`. The pointer is over the node's border box and nothing
    /// sensing is above it. Implies [`sense_hover`](Self::sense_hover).
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

    /// `transition duration-*`: animates the node's variant changes over
    /// `duration` instead of switching.
    pub fn transition(mut self, duration: Duration) -> Self {
        self.variants.transition = Some(duration);
        self
    }

    /// Whether the classes declare any sense, group or variant, which only an
    /// identified node can carry.
    pub(crate) fn is_interactive(&self) -> bool {
        !self.sensing.is_none()
            || self.grouping == Grouping::Group
            || !self.variants.is_empty()
            || self.variants.transition.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tw::tw;

    fn look() -> Look {
        Look {
            paint: BoxPaint {
                background: Some(Color32::from_rgb(0, 0, 0)),
                ..BoxPaint::default()
            },
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
    fn plain_classes_are_not_interactive() {
        assert!(!tw().bg(Color32::RED).is_interactive());
        assert!(tw().sense_hover().is_interactive());
        assert!(tw().group().is_interactive());
        assert!(tw().transition(Duration::from_millis(1)).is_interactive());
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
            active: Progress::START,
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
