//! Rules a node declares for the nodes built under it: [`Tw::descendants`]
//! and [`Tw::children`], and what they match on.

use std::ops::Not;
use std::rc::Rc;

use super::{BoxTw, Tw};
use crate::pipeline::NodeKind;

/// An ARIA state a node can say of itself and a [`Match`] can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aria {
    /// TODO(prose): `aria-busy`.
    Busy,
    /// TODO(prose): `aria-checked`.
    Checked,
    /// TODO(prose): `aria-disabled`.
    Disabled,
    /// TODO(prose): `aria-expanded`.
    Expanded,
    /// TODO(prose): `aria-invalid`.
    Invalid,
    /// TODO(prose): `aria-pressed`.
    Pressed,
    /// TODO(prose): `aria-required`.
    Required,
    /// TODO(prose): `aria-selected`.
    Selected,
}

impl Aria {
    fn bit(self) -> u16 {
        1 << self as u16
    }
}

static NO_FACTS: Facts = Facts {
    slot: None,
    attrs: Vec::new(),
    aria: 0,
};

/// What a node says of itself, for the rules that match it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Facts {
    slot: Option<&'static str>,
    attrs: Vec<(&'static str, &'static str)>,
    aria: u16,
}

impl Facts {
    pub(crate) fn slot(&mut self, slot: &'static str) {
        self.slot = Some(slot);
    }

    pub(crate) fn attr(&mut self, name: &'static str, value: &'static str) {
        match self.attrs.iter_mut().find(|(known, _)| *known == name) {
            Some(known) => known.1 = value,
            None => self.attrs.push((name, value)),
        }
    }

    pub(crate) fn aria(&mut self, state: Aria) {
        self.aria |= state.bit();
    }

    /// These facts with `later`'s over them: its slot, its attributes by
    /// name, and every ARIA state either says.
    pub(crate) fn merge(mut self, later: Self) -> Self {
        self.slot = later.slot.or(self.slot);
        for (name, value) in later.attrs {
            self.attr(name, value);
        }
        self.aria |= later.aria;
        self
    }
}

/// What a node says of itself and the rules it declares.
#[derive(Clone, Debug, Default)]
pub(crate) struct Ruled {
    facts: Facts,
    rules: Vec<Rule>,
    has: Vec<HasRule>,
}

impl Ruled {
    pub(crate) fn merge(mut self, later: Self) -> Self {
        self.facts = self.facts.merge(later.facts);
        self.rules.extend(later.rules);
        self.has.extend(later.has);
        self
    }
}

/// A rule on a node about what it holds: the classes `body` when a child
/// matches `when`.
#[derive(Clone, Debug)]
pub(crate) struct HasRule {
    when: Match,
    body: Tw,
}

/// What the parent of a node needs to know of it, for its `has` rules.
#[derive(Clone, Debug)]
pub(crate) struct Observed {
    pub(crate) kind: NodeKind,
    pub(crate) facts: Facts,
    pub(crate) focused: bool,
}

/// What a node says of itself.
impl Tw {
    pub(crate) fn facts(&self) -> &Facts {
        self.rules.as_ref().map_or(&NO_FACTS, |ruled| &ruled.facts)
    }

    /// `self` with the classes of its `has` rules that match a child of
    /// `seen`, over its own; `None` if none match.
    pub(crate) fn with_has<'a>(
        &self,
        mut seen: impl Iterator<Item = &'a Observed> + Clone,
    ) -> Option<Self> {
        let ruled = self.rules.as_ref().filter(|ruled| !ruled.has.is_empty())?;
        let mut applied: Option<Self> = None;
        for rule in &ruled.has {
            if seen
                .clone()
                .any(|child| rule.when.matches(child.kind, &child.facts, child.focused))
            {
                applied = Some(
                    applied
                        .unwrap_or_else(|| self.clone())
                        .merge(rule.body.clone()),
                );
            }
        }
        let _ = &mut seen;
        applied
    }
}

/// What a node is, for [`Match`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Glyph,
    Text,
}

impl Kind {
    fn of(kind: NodeKind) -> Option<Self> {
        match kind {
            NodeKind::Glyph | NodeKind::Icon => Some(Self::Glyph),
            NodeKind::Text | NodeKind::Span => Some(Self::Text),
            _ => None,
        }
    }
}

/// Which nodes a rule applies to. Every part given must hold; `!` takes the
/// whole back.
///
/// TODO(prose): the nodes a rule applies to.
///
/// ```
/// use eggshell_ui::{Match, tw};
/// let rule = tw().descendants(Match::slot("icon").attr("side", "start"), |t| t.size(4.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    kind: Option<Kind>,
    slot: Option<&'static str>,
    attr: Option<(&'static str, &'static str)>,
    aria: Option<Aria>,
    focus: bool,
    negated: bool,
}

impl Match {
    /// Every node.
    pub const fn any() -> Self {
        Self {
            kind: None,
            slot: None,
            attr: None,
            aria: None,
            focus: false,
            negated: false,
        }
    }

    /// An icon: CSS's `svg`.
    pub const fn glyph() -> Self {
        Self {
            kind: Some(Kind::Glyph),
            ..Self::any()
        }
    }

    /// A run of text.
    pub const fn text() -> Self {
        Self {
            kind: Some(Kind::Text),
            ..Self::any()
        }
    }

    /// A node with this [`slot`](Tw::slot): CSS's `[data-slot=...]`.
    pub const fn slot(slot: &'static str) -> Self {
        Self {
            slot: Some(slot),
            ..Self::any()
        }
    }

    /// A node with this [`aria`](Tw::aria) state.
    pub const fn aria(state: Aria) -> Self {
        Self {
            aria: Some(state),
            ..Self::any()
        }
    }

    /// TODO(prose): a node that has keyboard focus, or has a widget inside it
    /// that does, as of the last frame, so it is one frame late. Only a
    /// [`has`](Tw::has) rule can see it.
    pub const fn focus_visible() -> Self {
        Self {
            focus: true,
            ..Self::any()
        }
    }

    /// Also a node with this [`attr`](Tw::attr).
    #[must_use]
    pub const fn attr(mut self, name: &'static str, value: &'static str) -> Self {
        self.attr = Some((name, value));
        self
    }

    fn matches(&self, kind: NodeKind, facts: &Facts, focused: bool) -> bool {
        let holds = self.kind.is_none_or(|want| Kind::of(kind) == Some(want))
            && self.slot.is_none_or(|want| facts.slot == Some(want))
            && self.attr.is_none_or(|(name, value)| {
                facts
                    .attrs
                    .iter()
                    .any(|&(known, has)| known == name && has == value)
            })
            && self.aria.is_none_or(|state| facts.aria & state.bit() != 0)
            && (focused || !self.focus);
        holds != self.negated
    }
}

impl Not for Match {
    type Output = Self;

    fn not(mut self) -> Self {
        self.negated = !self.negated;
        self
    }
}

/// How far down from the node that declares it a rule reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    Children,
    Descendants,
}

/// A rule a node declares: the classes `body` for the nodes under it that
/// `when` matches.
#[derive(Clone, Debug)]
pub(crate) struct Rule {
    reach: Reach,
    when: Match,
    body: Tw,
}

impl Rule {
    pub(crate) fn new(reach: Reach, when: Match, body: Tw) -> Self {
        Self { reach, when, body }
    }
}

/// The rules of the containers above a point of the build, the nearest
/// first.
#[derive(Debug)]
pub(crate) struct Link {
    above: Option<Rc<Link>>,
    rules: Vec<Rule>,
    /// The nesting depth of the declaring container's children.
    depth: u32,
}

/// The rules in force at a point of the build, and how deep it is.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rules {
    chain: Option<Rc<Link>>,
    depth: u32,
    /// Whether the container the nodes are built in has `has` rules, and so
    /// needs to know what they are.
    pub(crate) observed: bool,
}

impl Rules {
    pub(crate) const NONE: Self = Self {
        chain: None,
        depth: 0,
        observed: false,
    };

    /// The rules for the children of a container with `classes`.
    pub(crate) fn within(&self, classes: &Tw) -> Self {
        let depth = self.depth.saturating_add(1);
        let declared = classes
            .rules
            .as_ref()
            .filter(|ruled| !ruled.rules.is_empty());
        let chain = match declared {
            None => self.chain.clone(),
            Some(ruled) => Some(Rc::new(Link {
                above: self.chain.clone(),
                rules: ruled.rules.clone(),
                depth,
            })),
        };
        let observed = classes
            .rules
            .as_ref()
            .is_some_and(|ruled| !ruled.has.is_empty());
        Self {
            chain,
            depth,
            observed,
        }
    }

    /// Nothing above reaches what is built with the result.
    pub(crate) fn cut(&self) -> Self {
        Self::NONE
    }

    /// `own` as the rules above give it its classes: the rules that match,
    /// the farthest container's first, under `own`; the `important` ones
    /// over it.
    pub(crate) fn apply(&self, own: Tw, kind: NodeKind) -> Tw {
        let Some(chain) = &self.chain else {
            return own;
        };
        let mut links = Vec::new();
        let mut at = Some(chain);
        while let Some(link) = at {
            links.push(link);
            at = link.above.as_ref();
        }
        let facts = own.rules.as_ref().map_or(&NO_FACTS, |ruled| &ruled.facts);
        let matching: Vec<&Rule> = links
            .iter()
            .rev()
            .flat_map(|link| {
                let direct = link.depth == self.depth;
                link.rules.iter().filter(move |rule| {
                    (direct || rule.reach == Reach::Descendants)
                        && rule.when.matches(kind, facts, false)
                })
            })
            .collect();
        let mut under = Tw::blank();
        for rule in matching.iter().filter(|rule| !rule.body.important) {
            under = under.merge(rule.body.clone());
        }
        let mut applied = under.merge(own.clone());
        for rule in matching.iter().filter(|rule| rule.body.important) {
            applied = applied.merge(rule.body.clone());
        }
        applied
    }
}

impl Tw {
    /// TODO(prose): names the node, as `data-slot`.
    ///
    /// ```
    /// use eggshell_ui::tw;
    /// let group = tw().slot("input-group");
    /// ```
    #[must_use]
    pub fn slot(mut self, slot: &'static str) -> Self {
        self.ruled().facts.slot(slot);
        self
    }

    /// TODO(prose): an attribute the node says of itself, as `data-*`.
    ///
    /// ```
    /// use eggshell_ui::tw;
    /// let small = tw().attr("size", "sm");
    /// ```
    #[must_use]
    pub fn attr(mut self, name: &'static str, value: &'static str) -> Self {
        self.ruled().facts.attr(name, value);
        self
    }

    /// TODO(prose): an ARIA state the node says of itself.
    ///
    /// ```
    /// use eggshell_ui::{Aria, tw};
    /// let invalid = tw().aria(Aria::Invalid);
    /// ```
    #[must_use]
    pub fn aria(mut self, state: Aria) -> Self {
        self.ruled().facts.aria(state);
        self
    }

    /// TODO(prose): `body`'s classes for the nodes under this one that
    /// `when` matches, at any depth: Tailwind's `[&_svg]:size-4`. The
    /// node's own classes win over them property by property.
    ///
    /// ```
    /// use eggshell_ui::{Match, tw};
    /// let button = tw().flex().descendants(Match::glyph(), |t| t.size(4.0).shrink_0());
    /// ```
    #[must_use]
    pub fn descendants(self, when: Match, body: impl FnOnce(Tw) -> Tw) -> Self {
        self.rule(Reach::Descendants, when, body)
    }

    /// TODO(prose): [`descendants`](Self::descendants) for the node's own
    /// children: Tailwind's `[&>svg]:size-3`.
    ///
    /// ```
    /// use eggshell_ui::{Match, tw};
    /// let button = tw().flex().children(Match::glyph(), |t| t.size(3.0).important());
    /// ```
    #[must_use]
    pub fn children(self, when: Match, body: impl FnOnce(Tw) -> Tw) -> Self {
        self.rule(Reach::Children, when, body)
    }

    /// TODO(prose): `body`'s box classes for this node while one of its
    /// children matches `when`: Tailwind's `has-*`. The classes win over the
    /// node's own, as a variant's do, and cannot touch text or behavior.
    ///
    /// ```
    /// use eggshell_ui::{Aria, Match, tw};
    /// let field = tw().has(Match::aria(Aria::Invalid), |b| b.px(2.0));
    /// ```
    #[must_use]
    pub fn has(mut self, when: Match, body: impl FnOnce(BoxTw) -> BoxTw) -> Self {
        let body = body(BoxTw::blank()).0;
        self.ruled().has.push(HasRule { when, body });
        self
    }

    /// TODO(prose): makes the rule it ends win over the classes of the nodes
    /// it applies to, as Tailwind's `!`.
    #[must_use]
    pub fn important(mut self) -> Self {
        self.important = true;
        self
    }

    fn rule(mut self, reach: Reach, when: Match, body: impl FnOnce(Tw) -> Tw) -> Self {
        self.ruled()
            .rules
            .push(Rule::new(reach, when, body(Tw::blank())));
        self
    }

    fn ruled(&mut self) -> &mut Ruled {
        self.rules.get_or_insert_with(Box::default)
    }
}
