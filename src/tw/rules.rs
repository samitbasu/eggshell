//! Rules a node declares for the nodes built under it: [`Tw::descendants`]
//! and [`Tw::children`], and what they match on.

use std::ops::Not;

use super::{BoxTw, Tw};
use crate::pipeline::NodeKind;
use crate::sheet::Layer;

/// An ARIA state a node can say of itself and a [`Match`] can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Aria {
    /// TODO(prose): `aria-busy`.
    Busy,
    /// TODO(prose): `aria-checked`.
    Checked,
    /// TODO(prose): `aria-disabled`.
    Disabled,
    /// TODO(prose): `aria-expanded`.
    Expanded,
    /// TODO(prose): `aria-haspopup`.
    HasPopup,
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
    /// The slot the node says it fills, if any.
    pub(crate) fn said_slot(&self) -> Option<&'static str> {
        self.slot
    }

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
    /// Whether the rule these classes are the body of wins over a node's own.
    important: bool,
    /// The classes the node's sheet gave it, which the rules above go over
    /// and the node's own classes go over them.
    base: Option<Box<Tw>>,
}

impl Ruled {
    pub(crate) fn merge(mut self, later: Self) -> Self {
        self.facts = self.facts.merge(later.facts);
        self.rules.extend(later.rules);
        self.has.extend(later.has);
        self.important |= later.important;
        self.base = match (self.base, later.base) {
            (Some(own), Some(later)) => Some(Box::new(own.merge(*later))),
            (own, later) => later.or(own),
        };
        self
    }
}

/// A rule on a node about what it holds: the classes `body` when a node
/// `reach` below it matches `when`.
#[derive(Clone, Debug)]
pub(crate) struct HasRule {
    reach: Reach,
    when: Match,
    body: Tw,
}

/// What the containers above a node need to know of it, for their `has`
/// rules.
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

    /// `self` with the classes of its `has` rules that match what `seen`
    /// holds, over its own; `None` if none match. Each child's observations
    /// are its own first, then its descendants'.
    pub(crate) fn with_has(&self, seen: &[Vec<Observed>]) -> Option<Self> {
        let ruled = self.rules.as_ref().filter(|ruled| !ruled.has.is_empty())?;
        let mut applied: Option<Self> = None;
        for rule in &ruled.has {
            let matches = |node: &Observed| rule.when.matches(node.kind, &node.facts, node.focused);
            let held = match rule.reach {
                Reach::Children => seen.iter().filter_map(|child| child.first()).any(matches),
                Reach::Descendants => seen.iter().flatten().any(matches),
            };
            if held {
                applied = Some(
                    applied
                        .unwrap_or_else(|| self.clone())
                        .merge(rule.body.clone()),
                );
            }
        }
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

    /// TODO(prose): also a node with this [`slot`](Tw::slot).
    ///
    /// ```
    /// use eggshell_ui::{Match, tw};
    /// let row = tw().children(Match::any().with_slot("input-group"), |t| t.h(8.0));
    /// ```
    #[must_use]
    pub const fn with_slot(mut self, slot: &'static str) -> Self {
        self.slot = Some(slot);
        self
    }

    /// TODO(prose): also a node that has keyboard focus, or a widget inside it
    /// that does, as [`focus_visible`](Self::focus_visible) says.
    ///
    /// ```
    /// use eggshell_ui::{Match, tw};
    /// let group = tw().has(Match::slot("input-group-control").with_focus_visible(), |b| b.ring(3.0, egui::Color32::BLUE));
    /// ```
    #[must_use]
    pub const fn with_focus_visible(mut self) -> Self {
        self.focus = true;
        self
    }

    /// TODO(prose): also a node in this [`aria`](Tw::aria) state.
    ///
    /// ```
    /// use eggshell_ui::{Aria, Match, tw};
    /// let card = tw().descendants(Match::slot("checkbox").with_aria(Aria::Checked), |t| t.mt(0.25));
    /// ```
    #[must_use]
    pub const fn with_aria(mut self, state: Aria) -> Self {
        self.aria = Some(state);
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

/// The rules one container declares, and the link above it in the arena.
#[derive(Debug)]
pub(crate) struct Link {
    above: Option<u32>,
    rules: Vec<Rule>,
    /// The nesting depth of the declaring container's children.
    depth: u32,
}

/// The rules in force at a point of the build, as the nearest link of the
/// frame's arena, and how deep the point is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rules {
    top: Option<u32>,
    depth: u32,
    /// Whether a container the nodes are built in, at any depth, has `has`
    /// rules, and so needs to know what they are.
    pub(crate) observed: bool,
}

impl Rules {
    pub(crate) const NONE: Self = Self {
        top: None,
        depth: 0,
        observed: false,
    };

    /// The rules for the children of a container with `classes`; a
    /// container that declares some adds a link to `links`.
    #[inline(always)]
    pub(crate) fn within(&self, classes: &Tw, links: &mut Vec<Link>) -> Self {
        let depth = self.depth.saturating_add(1);
        match &classes.rules {
            None => Self {
                top: self.top,
                depth,
                observed: self.observed,
            },
            Some(ruled) => self.declared(ruled, depth, links),
        }
    }

    #[inline(never)]
    fn declared(&self, ruled: &Ruled, depth: u32, links: &mut Vec<Link>) -> Self {
        let observed = self.observed || !ruled.has.is_empty();
        if ruled.rules.is_empty() {
            return Self {
                top: self.top,
                depth,
                observed,
            };
        }
        let at = u32::try_from(links.len()).ok();
        links.push(Link {
            above: self.top,
            rules: ruled.rules.clone(),
            depth,
        });
        // TODO(#279): report a frame of more than 2^32 rules
        Self {
            top: at.or(self.top),
            depth,
            observed,
        }
    }

    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    /// The rules that match a node of `kind` with `facts`, the farthest
    /// container's first.
    fn matching<'a>(&self, links: &'a [Link], facts: &Facts, kind: NodeKind) -> Vec<&'a Rule> {
        let mut chain = Vec::new();
        let mut at = self.top;
        while let Some(link) = at.and_then(|at| links.get(at as usize)) {
            chain.push(link);
            at = link.above;
        }
        let depth = self.depth;
        chain
            .iter()
            .rev()
            .flat_map(|link| {
                let direct = link.depth == depth;
                link.rules.iter().filter(move |rule| {
                    (direct || rule.reach == Reach::Descendants)
                        && rule.when.matches(kind, facts, false)
                })
            })
            .collect()
    }

    /// `own` as the rules above give it its classes: the rules that match,
    /// the farthest container's first, under `own`; the `important` ones
    /// over it.
    #[inline(never)]
    pub(crate) fn apply(&self, links: &[Link], own: &Tw, kind: NodeKind) -> Tw {
        let matching = self.matching(links, own.facts(), kind);
        let mut under = Tw::blank();
        for rule in matching.iter().filter(|rule| !rule.body.is_important()) {
            under = under.merge(rule.body.clone().staged(Layer::Ancestor));
        }
        let mut applied = under.merge(own.clone());
        for rule in matching.iter().filter(|rule| rule.body.is_important()) {
            applied = applied.merge(rule.body.clone().staged(Layer::Important));
        }
        applied
    }

    /// [`apply`](Self::apply) for a node with a sheet's classes: the rules
    /// that match go over those and under the node's own.
    #[inline(never)]
    pub(crate) fn apply_layered(&self, links: &[Link], own: Tw, kind: NodeKind) -> Tw {
        let (sheet, own) = own.split_sheet();
        let sheet = sheet.map(|mut sheet| {
            sheet.sensing = sheet.sensing.looks_only();
            sheet
        });
        let flat;
        let facts = match &sheet {
            Some(sheet) => {
                flat = sheet.facts().clone().merge(own.facts().clone());
                &flat
            }
            None => own.facts(),
        };
        let matching = self.matching(links, facts, kind);
        let mut under = sheet.unwrap_or_else(Tw::blank);
        for rule in matching.iter().filter(|rule| !rule.body.is_important()) {
            under = under.merge(rule.body.clone().staged(Layer::Ancestor));
        }
        let mut applied = under.merge(own);
        for rule in matching.iter().filter(|rule| rule.body.is_important()) {
            applied = applied.merge(rule.body.clone().staged(Layer::Important));
        }
        applied.without_sheet()
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

    /// TODO(prose): `body`'s box classes for this node while a node built
    /// anywhere inside it matches `when`: Tailwind's `has-*`. The classes win
    /// over the node's own, as a variant's do, and cannot touch text or
    /// behavior.
    ///
    /// ```
    /// use eggshell_ui::{Aria, Match, tw};
    /// let field = tw().has(Match::aria(Aria::Invalid), |b| b.px(2.0));
    /// ```
    #[must_use]
    pub fn has(self, when: Match, body: impl FnOnce(BoxTw) -> BoxTw) -> Self {
        self.has_at(Reach::Descendants, when, body)
    }

    /// TODO(prose): [`has`](Self::has) for the node's own children:
    /// Tailwind's `has-[>*]`.
    ///
    /// ```
    /// use eggshell_ui::{Match, tw};
    /// let label = tw().has_child(Match::slot("field"), |b| b.border(1.0));
    /// ```
    #[must_use]
    pub fn has_child(self, when: Match, body: impl FnOnce(BoxTw) -> BoxTw) -> Self {
        self.has_at(Reach::Children, when, body)
    }

    fn has_at(mut self, reach: Reach, when: Match, body: impl FnOnce(BoxTw) -> BoxTw) -> Self {
        let body = body(BoxTw::blank()).0;
        self.ruled().has.push(HasRule { reach, when, body });
        self
    }

    /// TODO(prose): makes the rule it ends win over the classes of the nodes
    /// it applies to, as Tailwind's `!`.
    #[must_use]
    pub fn important(mut self) -> Self {
        self.ruled().important = true;
        self
    }

    /// Classes that say nothing, over the classes `sheet` gives a node: what
    /// is merged into them goes over `sheet`, and the rules of the containers
    /// above go between.
    pub(crate) fn over_sheet(sheet: Tw) -> Tw {
        let mut classes = Tw::blank();
        classes.ruled().base = Some(Box::new(sheet));
        classes
    }

    /// Whether the node's classes hold a sheet's, to go under its own.
    #[inline(always)]
    pub(crate) fn is_layered(&self) -> bool {
        self.rules
            .as_ref()
            .is_some_and(|ruled| ruled.base.is_some())
    }

    /// The sheet's classes of the node and its own.
    fn split_sheet(mut self) -> (Option<Tw>, Tw) {
        let sheet = self
            .rules
            .as_mut()
            .and_then(|ruled| ruled.base.take())
            .map(|sheet| *sheet);
        (sheet, self)
    }

    fn without_sheet(mut self) -> Tw {
        if let Some(ruled) = self.rules.as_mut() {
            ruled.base = None;
        }
        self
    }

    fn is_important(&self) -> bool {
        self.rules.as_ref().is_some_and(|ruled| ruled.important)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tw::tw;

    fn glyph_size(rules: &Rules, links: &[Link], own: Tw) -> taffy::Dimension {
        rules
            .apply_layered(links, own, NodeKind::Glyph)
            .style
            .size
            .width
    }

    #[test]
    fn rules_above_go_between_the_sheet_and_the_call_site() {
        let mut links = Vec::new();
        let above = tw().descendants(Match::glyph(), |t| t.size_px(12.0));
        let rules = Rules::NONE.within(&above, &mut links);
        let sheet = || tw().size_px(16.0).slot("icon");
        let width = |points| taffy::Dimension::length(points);

        let from_sheet = Tw::over_sheet(sheet());
        assert_eq!(glyph_size(&rules, &links, from_sheet), width(12.0));

        let called = Tw::over_sheet(sheet()).merge(tw().size_px(20.0));
        assert_eq!(glyph_size(&rules, &links, called), width(20.0));

        let nowhere = Tw::over_sheet(sheet());
        assert_eq!(glyph_size(&Rules::NONE, &[], nowhere), width(16.0));
    }

    #[test]
    fn rules_above_match_the_sheets_slot() {
        let mut links = Vec::new();
        let above = tw().children(Match::slot("icon"), |t| t.size_px(9.0));
        let rules = Rules::NONE.within(&above, &mut links);
        let matched = Tw::over_sheet(tw().size_px(16.0).slot("icon"));
        let plain = Tw::over_sheet(tw().size_px(16.0));
        let width = |points| taffy::Dimension::length(points);
        assert_eq!(glyph_size(&rules, &links, matched), width(9.0));
        assert_eq!(glyph_size(&rules, &links, plain), width(16.0));
    }

    #[test]
    fn the_applied_classes_keep_no_sheet() {
        let applied = Rules::NONE.apply_layered(&[], Tw::over_sheet(tw().px(1.0)), NodeKind::Div);
        assert!(!applied.is_layered());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn each_property_keeps_the_layer_that_won_it() {
        use crate::Property;
        let mut links = Vec::new();
        let above = tw()
            .descendants(Match::glyph(), |t| t.size_px(12.0))
            .descendants(Match::glyph(), |t| t.gap(2.0).important());
        let rules = Rules::NONE.within(&above, &mut links);
        let sheet = tw().size_px(16.0).m(1.0).gap(1.0).staged(Layer::Base);
        let own = Tw::over_sheet(sheet).merge(tw().p(3.0));
        let applied = rules.apply_layered(&links, own, NodeKind::Glyph);
        let layer = |property| applied.origin_of(property).map(|origin| origin.layer);
        assert_eq!(layer(Property::width), Some(Layer::Ancestor));
        assert_eq!(layer(Property::margin_left), Some(Layer::Base));
        assert_eq!(layer(Property::padding_left), Some(Layer::CallSite));
        assert_eq!(layer(Property::gap_width), Some(Layer::Important));
    }
}
