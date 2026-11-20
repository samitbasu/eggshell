//! A style's rules for the parts of its widgets: the classes a part gets, by
//! what it is and what state it is in, written by layers that win in order.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::contract::{Size, Variant};
use crate::{Aria, Tw};

/// TODO(prose): the anatomy of a widget: the nodes a [`Sheet`] styles.
///
/// ```
/// use eggshell_ui::sheet::Part;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// enum ChipPart {
///     Root,
///     Label,
/// }
///
/// impl Part for ChipPart {
///     const SOURCE: &'static str = "badge.tsx";
///
///     fn slot(self) -> &'static str {
///         match self {
///             Self::Root => "badge",
///             Self::Label => "badge-label",
///         }
///     }
/// }
/// ```
pub trait Part: Copy + Eq + Ord + Debug + Send + Sync + 'static {
    /// TODO(prose): the shadcn component the widget's look comes from.
    const SOURCE: &'static str;

    /// TODO(prose): the part's `data-slot`, which rules match and the part's node declares.
    fn slot(self) -> &'static str;
}

/// TODO(prose): who wrote a rule, which decides whose wins. A later layer's classes win per property over an earlier one's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// TODO(prose): the widgets' own looks.
    Defaults,
    /// TODO(prose): a design system's preset.
    Preset,
    /// TODO(prose): the application's.
    App,
    /// TODO(prose): a [`Builder::styled`](crate::Builder::styled) scope's.
    Scope,
    /// TODO(prose): the rules of the containers above a node.
    Ancestor,
    /// TODO(prose): the classes given where the widget is built.
    CallSite,
    /// TODO(prose): an `important` rule of a container above.
    Important,
}

/// TODO(prose): one fact about a node that a rule can ask for.
///
/// ```
/// use eggshell_ui::Aria;
/// use eggshell_ui::contract::Variant;
/// use eggshell_ui::sheet::Key;
///
/// let keys = [Key::Variant(Variant::DEFAULT), Key::Aria(Aria::Invalid)];
/// assert_eq!(keys.len(), 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// TODO(prose): the node's variant.
    Variant(Variant),
    /// TODO(prose): the node's size.
    Size(Size),
    /// TODO(prose): an attribute, as `data-[orientation=vertical]`.
    Attr(&'static str, &'static str),
    /// TODO(prose): an ARIA state.
    Aria(Aria),
    /// TODO(prose): in a rule: the node says no such attribute.
    NotAttr(&'static str, &'static str),
    /// TODO(prose): in a rule: the node says no such ARIA state.
    NotAria(Aria),
}

/// TODO(prose): the keys a rule needs: it applies to a node whose keys include all of them.
///
/// ```
/// use eggshell_ui::Aria;
/// use eggshell_ui::contract::{Size, Variant};
/// use eggshell_ui::sheet::When;
///
/// let invalid_small = When::always().size(Size("sm")).aria(Aria::Invalid);
/// let primary = When::always().variant(Variant("primary"));
/// assert_ne!(invalid_small, primary);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct When {
    keys: [Option<Key>; 4],
}

impl When {
    /// TODO(prose): no keys: every node of the part.
    pub const fn always() -> Self {
        Self { keys: [None; 4] }
    }

    /// TODO(prose): and the node has `variant`.
    #[must_use]
    pub fn variant(self, variant: Variant) -> Self {
        self.and(Key::Variant(variant))
    }

    /// TODO(prose): and the node is `size`.
    #[must_use]
    pub fn size(self, size: Size) -> Self {
        self.and(Key::Size(size))
    }

    /// TODO(prose): and the node says the attribute `name` is `value`.
    #[must_use]
    pub fn attr(self, name: &'static str, value: &'static str) -> Self {
        self.and(Key::Attr(name, value))
    }

    /// TODO(prose): and the node says `state`.
    #[must_use]
    pub fn aria(self, state: Aria) -> Self {
        self.and(Key::Aria(state))
    }

    /// TODO(prose): and the node does not say the attribute `name` is `value`: `not-data-[name=value]`.
    #[must_use]
    pub fn not_attr(self, name: &'static str, value: &'static str) -> Self {
        self.and(Key::NotAttr(name, value))
    }

    /// TODO(prose): and the node does not say `state`: `not-aria-*`.
    #[must_use]
    pub fn not_aria(self, state: Aria) -> Self {
        self.and(Key::NotAria(state))
    }

    /// TODO(prose): and the node has `key`.
    #[must_use]
    pub fn and(mut self, key: Key) -> Self {
        match self.keys.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => *slot = Some(key),
            None => {
                // TODO(#279): report a rule that asks for more than four keys
            }
        }
        self
    }

    fn named(&self) -> impl Iterator<Item = Key> + '_ {
        self.keys.iter().flatten().copied()
    }

    fn holds(&self, keys: &[Key]) -> bool {
        self.named().all(|key| match key {
            Key::NotAttr(name, value) => !keys.contains(&Key::Attr(name, value)),
            Key::NotAria(state) => !keys.contains(&Key::Aria(state)),
            key => keys.contains(&key),
        })
    }
}

/// What a part is named by: its widget's parts, and its slot.
type PartKey = (TypeId, &'static str);

type Body = Arc<dyn Fn(Tw) -> Tw + Send + Sync>;

#[derive(Clone)]
struct Stored {
    when: When,
    layer: Layer,
    replace: bool,
    body: Body,
}

impl Debug for Stored {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored")
            .field("when", &self.when)
            .field("layer", &self.layer)
            .field("replace", &self.replace)
            .finish_non_exhaustive()
    }
}

fn next_version() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// TODO(prose): the rules of a style: for each part of each widget, the classes it gets when its keys hold.
///
/// ```
/// use eggshell_ui::contract::Variant;
/// use eggshell_ui::sheet::{Key, Part, Sheet, When};
/// use eggshell_ui::tw;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// struct Chip;
///
/// impl Part for Chip {
///     const SOURCE: &'static str = "badge.tsx";
///
///     fn slot(self) -> &'static str {
///         "badge"
///     }
/// }
///
/// let mut sheet = Sheet::defaults();
/// sheet
///     .rule(Chip, When::always(), |t| t.px(2.0).rounded_md())
///     .rule(Chip, When::always().variant(Variant("wide")), |t| t.px(4.0));
/// let wide = sheet.resolve(Chip, &[Key::Variant(Variant("wide"))]);
/// assert_eq!(wide.style(), tw().px(4.0).style());
/// ```
#[derive(Clone, Debug)]
pub struct Sheet {
    layer: Layer,
    rules: Arc<HashMap<PartKey, Vec<Stored>>>,
    /// Changes with every edit, so that the installed style's every-frame comparison stays cheap.
    version: u64,
}

impl Default for Sheet {
    fn default() -> Self {
        Self::defaults()
    }
}

impl PartialEq for Sheet {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version && self.layer == other.layer
    }
}

impl Sheet {
    /// TODO(prose): a sheet with no rules, written at the layer of the widgets' defaults.
    pub fn defaults() -> Self {
        Self {
            layer: Layer::Defaults,
            rules: Arc::default(),
            version: 0,
        }
    }

    /// TODO(prose): the rules written from here on are a preset's.
    pub fn as_preset(&mut self) -> &mut Self {
        self.layer = Layer::Preset;
        self
    }

    /// TODO(prose): the rules written from here on are the application's.
    pub fn as_app(&mut self) -> &mut Self {
        self.layer = Layer::App;
        self
    }

    pub(crate) fn as_scope(&mut self) -> &mut Self {
        self.layer = Layer::Scope;
        self
    }

    /// TODO(prose): the layer the rules written now are in.
    pub fn layer(&self) -> Layer {
        self.layer
    }

    /// TODO(prose): adds the classes `body` writes for `part` while `when` holds, at this sheet's layer, after the rules it has.
    pub fn rule<P: Part>(
        &mut self,
        part: P,
        when: When,
        body: impl Fn(Tw) -> Tw + Send + Sync + 'static,
    ) -> &mut Self {
        self.add(part, when, false, body)
    }

    /// TODO(prose): [`rule`](Self::rule), but the part's classes start empty under it: every rule below that holds with it is dropped.
    ///
    /// ```
    /// use eggshell_ui::contract::Variant;
    /// use eggshell_ui::sheet::{Key, Part, Sheet, When};
    /// use eggshell_ui::tw;
    ///
    /// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// struct Chip;
    ///
    /// impl Part for Chip {
    ///     const SOURCE: &'static str = "badge.tsx";
    ///
    ///     fn slot(self) -> &'static str {
    ///         "badge"
    ///     }
    /// }
    ///
    /// let mut sheet = Sheet::defaults();
    /// sheet.rule(Chip, When::always(), |t| t.px(2.0).gap(1.0));
    /// sheet.as_app().replace(Chip, When::always(), |t| t.px(4.0));
    /// let chip = sheet.resolve(Chip, &[]);
    /// assert_eq!(chip.style(), tw().px(4.0).style());
    /// ```
    pub fn replace<P: Part>(
        &mut self,
        part: P,
        when: When,
        body: impl Fn(Tw) -> Tw + Send + Sync + 'static,
    ) -> &mut Self {
        self.add(part, when, true, body)
    }

    fn add<P: Part>(
        &mut self,
        part: P,
        when: When,
        replace: bool,
        body: impl Fn(Tw) -> Tw + Send + Sync + 'static,
    ) -> &mut Self {
        let layer = self.layer;
        Arc::make_mut(&mut self.rules)
            .entry(part_key(part))
            .or_default()
            .push(Stored {
                when,
                layer,
                replace,
                body: Arc::new(body),
            });
        self.version = next_version();
        self
    }

    /// TODO(prose): the classes of `part` for a node with `keys`: the rules that hold, the lowest layer's first, each over the last. The node's `data-slot` is the part's.
    ///
    /// A builder keeps what it resolved, so this runs each rule's body: for a call outside a build, such as a test's.
    pub fn resolve<P: Part>(&self, part: P, keys: &[Key]) -> Tw {
        self.resolved(part, part_key(part), keys)
    }

    fn resolved<P: Part>(&self, part: P, at: PartKey, keys: &[Key]) -> Tw {
        let mut holding: Vec<&Stored> = self
            .rules
            .get(&at)
            .into_iter()
            .flatten()
            .filter(|rule| rule.when.holds(keys))
            .collect();
        holding.sort_by_key(|rule| rule.layer);
        let from = holding.iter().rposition(|rule| rule.replace).unwrap_or(0);
        holding
            .iter()
            .skip(from)
            .fold(Tw::blank(), |classes, rule| {
                classes.merge((rule.body)(Tw::blank()).staged(rule.layer))
            })
            .slot(part.slot())
    }

    /// TODO(prose): the variants and sizes the sheet's rules name for the parts of `P`'s widget.
    pub fn keys<P: Part>(&self) -> impl Iterator<Item = Key> + use<P> {
        let widget = TypeId::of::<P>();
        let mut named = Vec::new();
        let rules = self
            .rules
            .iter()
            .filter(|(at, _)| at.0 == widget)
            .flat_map(|(_, rules)| rules);
        for key in rules.flat_map(|rule| rule.when.named()) {
            if matches!(key, Key::Variant(_) | Key::Size(_)) && !named.contains(&key) {
                named.push(key);
            }
        }
        named.into_iter()
    }

    /// The first variant or size in `keys` that `part`'s rules leave out while naming others of its kind.
    pub(crate) fn unnamed<P: Part>(&self, part: P, keys: &[Key]) -> Option<Key> {
        let named: Vec<Key> = self
            .rules
            .get(&part_key(part))
            .into_iter()
            .flatten()
            .flat_map(|rule| rule.when.named())
            .collect();
        let same_kind = |a: &Key, b: &Key| {
            matches!(
                (a, b),
                (Key::Variant(_), Key::Variant(_)) | (Key::Size(_), Key::Size(_))
            )
        };
        keys.iter()
            .find(|key| !named.contains(key) && named.iter().any(|known| same_kind(known, key)))
            .copied()
    }
}

/// The classes resolved for a part, by the keys they were for.
type Resolved = Vec<(Box<[Key]>, Tw)>;

/// What a build has resolved, by sheet version. A [`Tw`] is not `Send`, so
/// the sheet, which a style shares between threads, cannot keep it.
#[derive(Default)]
pub(crate) struct Memo {
    sheets: HashMap<u64, HashMap<PartKey, Resolved>>,
}

impl Memo {
    /// The most sheets kept: scopes that edit theirs make a new one each frame.
    const SHEETS: usize = 16;

    /// [`Sheet::resolve`], and whether it was resolved now rather than found.
    pub(crate) fn resolve<P: Part>(&mut self, sheet: &Sheet, part: P, keys: &[Key]) -> (Tw, bool) {
        if self.sheets.len() >= Self::SHEETS && !self.sheets.contains_key(&sheet.version) {
            self.sheets.clear();
        }
        let at = part_key(part);
        let known = self
            .sheets
            .entry(sheet.version)
            .or_default()
            .entry(at)
            .or_default();
        if let Some((_, resolved)) = known.iter().find(|(known, _)| **known == *keys) {
            return (resolved.clone(), false);
        }
        let resolved = sheet.resolved(part, at, keys);
        known.push((keys.into(), resolved.clone()));
        (resolved, true)
    }
}

fn part_key<P: Part>(part: P) -> PartKey {
    (TypeId::of::<P>(), part.slot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tw;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Test {
        Root,
        Icon,
    }

    impl Part for Test {
        const SOURCE: &'static str = "test.tsx";

        fn slot(self) -> &'static str {
            match self {
                Self::Root => "test",
                Self::Icon => "test-icon",
            }
        }
    }

    const WIDE: Variant = Variant("wide");
    const NARROW: Variant = Variant("narrow");

    fn sheet() -> Sheet {
        let mut sheet = Sheet::defaults();
        sheet
            .rule(Test::Root, When::always(), |t| t.px(2.0).gap(1.0))
            .rule(Test::Root, When::always().variant(WIDE), |t| t.px(4.0))
            .rule(Test::Root, When::always().variant(NARROW), |t| t.px(1.0))
            .rule(
                Test::Root,
                When::always().variant(WIDE).aria(Aria::Invalid),
                |t| t.gap(3.0),
            )
            .rule(Test::Icon, When::always(), |t| t.size(4.0));
        sheet
    }

    #[test]
    fn the_rules_that_hold_are_merged_in_order_later_winning_per_property() {
        let sheet = sheet();
        let wide = sheet.resolve(Test::Root, &[Key::Variant(WIDE)]);
        assert_eq!(wide.style(), tw().px(4.0).gap(1.0).style());
        let invalid = sheet.resolve(Test::Root, &[Key::Variant(WIDE), Key::Aria(Aria::Invalid)]);
        assert_eq!(invalid.style(), tw().px(4.0).gap(3.0).style());
    }

    #[test]
    fn a_negated_key_holds_while_the_node_lacks_the_state() {
        let mut sheet = Sheet::defaults();
        sheet
            .rule(Test::Root, When::always().not_aria(Aria::Disabled), |t| {
                t.px(2.0)
            })
            .rule(Test::Root, When::always().not_attr("side", "top"), |t| {
                t.py(3.0)
            });
        let free = sheet.resolve(Test::Root, &[]);
        assert_eq!(free.style(), tw().px(2.0).py(3.0).style());
        let keys = [Key::Aria(Aria::Disabled), Key::Attr("side", "top")];
        assert_eq!(sheet.resolve(Test::Root, &keys).style(), tw().style());
    }

    #[test]
    fn a_part_is_resolved_from_its_own_rules_only() {
        let icon = sheet().resolve(Test::Icon, &[Key::Variant(WIDE)]);
        assert_eq!(icon.style(), tw().size(4.0).style());
    }

    #[test]
    fn a_later_layer_wins_whatever_the_order_the_rules_were_written_in() {
        let mut sheet = Sheet::defaults();
        sheet
            .as_app()
            .rule(Test::Root, When::always(), |t| t.px(8.0));
        sheet.layer = Layer::Defaults;
        sheet.rule(Test::Root, When::always(), |t| t.px(2.0).gap(1.0));
        let root = sheet.resolve(Test::Root, &[]);
        assert_eq!(root.style(), tw().px(8.0).gap(1.0).style());
    }

    #[test]
    fn replace_drops_what_is_below_it_and_only_where_it_holds() {
        let mut sheet = sheet();
        sheet
            .as_app()
            .replace(Test::Root, When::always().variant(WIDE), |t| t.py(5.0));
        let wide = sheet.resolve(Test::Root, &[Key::Variant(WIDE)]);
        assert_eq!(wide.style(), tw().py(5.0).style());
        let narrow = sheet.resolve(Test::Root, &[Key::Variant(NARROW)]);
        assert_eq!(narrow.style(), tw().px(1.0).gap(1.0).style());
    }

    #[test]
    fn an_edit_makes_the_sheet_unequal_and_a_clone_equal() {
        let mut sheet = sheet();
        let clone = sheet.clone();
        assert_eq!(sheet, clone);
        sheet.rule(Test::Root, When::always(), |t| t.px(9.0));
        assert_ne!(sheet, clone);
        assert_eq!(
            clone.resolve(Test::Root, &[]).style(),
            tw().px(2.0).gap(1.0).style(),
            "the clone keeps its rules"
        );
    }

    #[test]
    fn a_variant_the_sheet_leaves_out_is_found_when_it_names_others() {
        let sheet = sheet();
        let missing = Key::Variant(Variant("missing"));
        assert_eq!(sheet.unnamed(Test::Root, &[missing]), Some(missing));
        assert_eq!(sheet.unnamed(Test::Root, &[Key::Variant(WIDE)]), None);
        assert_eq!(
            sheet.unnamed(Test::Icon, &[missing]),
            None,
            "a part with no variants takes any"
        );
    }

    #[test]
    fn the_keys_a_part_names_are_its_variants_and_sizes() {
        let keys: Vec<Key> = sheet().keys::<Test>().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Key::Variant(WIDE)) && keys.contains(&Key::Variant(NARROW)));
    }
}
