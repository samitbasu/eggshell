//! A component's classes as `Sheet` rules, written out as Rust (#443, the
//! `Sheet` note §8). Each class becomes a call on the rule of its part and
//! keys, a key of that rule, a rule over other nodes, or an unmapped comment,
//! and each carries its source line. The text targets the `Sheet` API of the
//! note; it is generated, not compiled, until that API lands.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::Row;
use super::classes::{Classifier, Fate, Paint, split_variants};
use super::source::{Origin, Sources};

/// How a class reached the sheet; the report counts these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A `Tw` method the class names.
    TwCall,
    /// A preset token: a colour, the theme radius, a text size.
    Token,
    /// An arbitrary value evaluated now: `rounded-[min(var(--radius-md),8px)]`.
    Constant,
    /// A `Tw` call that does what the class does, though `Tw` names it otherwise.
    Equivalent,
    /// A class with nothing to do in eggshell.
    NoOp,
    /// A key of the rule: `aria-invalid:`, `disabled:`.
    Key,
    /// Written into the dark half only: `dark:`.
    DarkHalf,
    /// A rule over other nodes: `[&_svg]:`, `*:`, `has-…:`.
    Rule,
    /// A state looked up at draw: `hover:`, `focus-visible:`.
    DrawVariant,
    /// Nothing yet.
    Unmapped,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::TwCall => "a `Tw` call",
            Self::Token => "a token",
            Self::Constant => "a constant evaluated at import",
            Self::Equivalent => "an equivalent `Tw` call",
            Self::NoOp => "nothing to do",
            Self::Key => "a key (`aria-*`, `disabled:`)",
            Self::DarkHalf => "the dark half (`dark:`)",
            Self::Rule => "a rule (`[&_svg]`, `*:`, `has-`)",
            Self::DrawVariant => "a draw-time variant (`hover:`)",
            Self::Unmapped => "unmapped",
        }
    }
}

/// A closure a call sits in: a draw-time state's paint, or a rule's body.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Wrap {
    /// `.hover(|p| p…)`.
    Paint(String),
    /// `.descendants(Match::glyph(), |t| t…)`, `.has(…, |b| b…)`.
    Rule { method: &'static str, when: String },
}

/// What a class sets, inside its closures.
#[derive(Clone, Debug, PartialEq)]
enum Leaf {
    Call(String),
    /// `ring-3`: the ring's width, combined with its colour per rule.
    RingWidth(f32),
    /// `ring-ring/50`: the ring's colour.
    RingColor(String),
}

/// One class, translated.
#[derive(Clone, Debug, PartialEq)]
pub struct Translated {
    dark: bool,
    /// The ARIA key of the rule it belongs to, as `Aria::Invalid`.
    key: Option<&'static str>,
    wraps: Vec<Wrap>,
    leaf: Leaf,
    important: bool,
    kinds: BTreeSet<Kind>,
}

/// `Aria` variants by their attribute.
const ARIA: [(&str, &str); 8] = [
    ("busy", "Aria::Busy"),
    ("checked", "Aria::Checked"),
    ("disabled", "Aria::Disabled"),
    ("expanded", "Aria::Expanded"),
    ("invalid", "Aria::Invalid"),
    ("pressed", "Aria::Pressed"),
    ("required", "Aria::Required"),
    ("selected", "Aria::Selected"),
];

fn aria(name: &str) -> Option<&'static str> {
    ARIA.iter().find(|(a, _)| *a == name).map(|(_, v)| *v)
}

/// A colour token as a Rust expression over the tokens `k`.
fn color(paint: &Paint) -> String {
    let base = match paint.token.as_str() {
        "transparent" => "Color32::TRANSPARENT".to_owned(),
        "white" => "Color32::WHITE".to_owned(),
        "black" => "Color32::BLACK".to_owned(),
        token => format!("k.{}", token.replace('-', "_")),
    };
    match paint.alpha {
        Some(a) => format!("{base}.gamma_multiply({a})"),
        None => base,
    }
}

/// The shadows the preset's tokens name.
const SHADOWS: [&str; 3] = ["xs", "sm", "md"];

/// `.h(9)` as `.h(9.0)`: `Tw` takes `f32`s.
fn floats(call: &str) -> String {
    match (call.find('('), call.rfind(')')) {
        (Some(open), Some(close)) if close == call.len() - 1 => {
            let arg = &call[open + 1..close];
            if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()) {
                format!("{}{arg}.0)", &call[..=open])
            } else {
                call.to_owned()
            }
        }
        _ => call.to_owned(),
    }
}

/// The match of a `[&…]`, `*` or `**` selector, with the method it scopes.
fn selector(variant: &str) -> Option<(&'static str, &'static str)> {
    Some(match variant {
        "[&_svg:not([class*='size-'])]" | "[&_svg]" => ("descendants", "Match::glyph()"),
        "[&>svg]" => ("children", "Match::glyph()"),
        "*" => ("children", "Match::any()"),
        "**" => ("descendants", "Match::any()"),
        _ => return None,
    })
}

/// The match of a `has-…` variant.
fn has(variant: &str) -> Option<String> {
    let rest = variant.strip_prefix("has-")?;
    if let Some(name) = rest.strip_prefix("aria-") {
        return aria(name).map(|a| format!("Match::any().aria({a})"));
    }
    let attr = rest.strip_prefix("data-[")?.strip_suffix(']')?;
    let (name, value) = attr.split_once('=')?;
    Some(format!("Match::any().attr({name:?}, {value:?})"))
}

/// A utility `Tw` spells otherwise, or that has nothing to do.
fn equivalent(utility: &str) -> Option<(Option<&'static str>, Kind)> {
    Some(match utility {
        "border" => (Some(".border(1.0)"), Kind::Equivalent),
        "inline-flex" => (Some(".flex()"), Kind::Equivalent),
        "transition-all" => (
            Some(".transition(Duration::from_millis(150))"),
            Kind::Equivalent,
        ),
        "pointer-events-none" => (Some(".pointer_events_none()"), Kind::Equivalent),
        "outline-none" | "outline-hidden" => (None, Kind::NoOp),
        _ => return None,
    })
}

impl Classifier<'_> {
    /// `class` as a piece of a rule, or why it is not one yet.
    pub fn translate(&self, class: &str) -> Result<Translated, String> {
        let important = class.ends_with('!');
        let (variants, utility) = split_variants(class.trim_end_matches('!'));
        let mut t = Translated {
            dark: false,
            key: None,
            wraps: Vec::new(),
            leaf: Leaf::Call(String::new()),
            important,
            kinds: BTreeSet::new(),
        };
        let mut variants = variants.into_iter().peekable();
        while let Some(variant) = variants.next() {
            if variant == "dark" {
                t.dark = true;
                t.kinds.insert(Kind::DarkHalf);
            } else if let Some(name) = variant
                .strip_prefix("aria-")
                .or((variant == "disabled").then_some("disabled"))
                && t.wraps.is_empty()
                && let Some(a) = aria(name)
            {
                t.key = Some(a);
                t.kinds.insert(Kind::Key);
            } else if let Some(method) = self.tw.variants.get(variant) {
                t.wraps.push(Wrap::Paint(method.clone()));
                t.kinds.insert(Kind::DrawVariant);
            } else if let Some((method, base)) = selector(variant) {
                let mut when = base.to_owned();
                if let Some(slot) = variants
                    .peek()
                    .and_then(|v| v.strip_prefix("data-[slot="))
                    .and_then(|v| v.strip_suffix(']'))
                {
                    when = format!("{when}.slot({slot:?})");
                    variants.next();
                }
                t.wraps.push(Wrap::Rule { method, when });
                t.kinds.insert(Kind::Rule);
            } else if let Some(when) = has(variant) {
                t.wraps.push(Wrap::Rule {
                    method: "has",
                    when,
                });
                t.kinds.insert(Kind::Rule);
            } else {
                return Err(format!("variant `{variant}:`"));
            }
        }
        if important && !t.wraps.iter().any(|w| matches!(w, Wrap::Rule { .. })) {
            return Err("`!` outside a rule".to_owned());
        }
        t.leaf = self.leaf(utility, &mut t.kinds)?;
        let in_has = t
            .wraps
            .iter()
            .any(|w| matches!(w, Wrap::Rule { method: "has", .. }));
        if in_has && matches!(&t.leaf, Leaf::Call(c) if c.starts_with(".text")) {
            return Err("`has-` sets box classes only, not inherited text".to_owned());
        }
        Ok(t)
    }

    fn leaf(&self, utility: &str, kinds: &mut BTreeSet<Kind>) -> Result<Leaf, String> {
        if let Some((call, kind)) = equivalent(utility) {
            kinds.insert(kind);
            return Ok(Leaf::Call(call.unwrap_or_default().to_owned()));
        }
        if let Some(width) = utility
            .strip_prefix("ring-")
            .and_then(|w| w.parse::<f32>().ok())
        {
            kinds.insert(Kind::TwCall);
            return Ok(Leaf::RingWidth(width));
        }
        if let Some(name) = utility.strip_prefix("shadow-") {
            return if SHADOWS.contains(&name) {
                kinds.insert(Kind::Token);
                Ok(Leaf::Call(format!(".shadow(k.shadow_{name})")))
            } else {
                Err(format!("no shadow token `shadow-{name}`"))
            };
        }
        if let Some(n) = utility
            .strip_prefix("opacity-")
            .and_then(|n| n.parse::<f32>().ok())
        {
            kinds.insert(Kind::TwCall);
            return Ok(Leaf::Call(format!(".opacity(Opacity::new({}))", n / 100.0)));
        }
        match self.utility(utility) {
            Fate::Tw { call } => {
                kinds.insert(Kind::TwCall);
                Ok(Leaf::Call(floats(&call)))
            }
            Fate::Token {
                paint: Some(paint), ..
            } => {
                kinds.insert(Kind::Token);
                let c = color(&paint);
                match paint.property {
                    "fill" => Ok(Leaf::Call(format!(".bg({c})"))),
                    "text" => Ok(Leaf::Call(format!(".text({c})"))),
                    "border" => Ok(Leaf::Call(format!(".border_color({c})"))),
                    "ring" => Ok(Leaf::RingColor(c)),
                    other => Err(format!("no `Tw` method for the {other}")),
                }
            }
            Fate::Token { value, paint: None } => {
                if let Some(points) = self.radius.rounded(utility) {
                    kinds.insert(if utility.contains('[') {
                        Kind::Constant
                    } else {
                        Kind::Token
                    });
                    return Ok(Leaf::Call(format!(".rounded_px({points:?})")));
                }
                if let Some((size, line)) = super::classes::text_size(utility) {
                    kinds.insert(Kind::Token);
                    return Ok(Leaf::Call(format!(".text_size({size:?}, {line:?})")));
                }
                Err(format!("token `{value}`"))
            }
            Fate::Preset { .. } | Fate::Marker => Err(format!("`{utility}` is a marker")),
            Fate::Gap { why, .. } => Err(why),
        }
    }
}

/// One rule: its part's keys, then its pieces in source order.
struct Rule {
    when: String,
    pieces: Vec<(Translated, String)>,
}

/// `{wraps}{leaf}` written over the closure variable `var`.
fn render(wraps: &[Wrap], leaf: &str, important: bool) -> String {
    match wraps.split_first() {
        None => leaf.to_owned(),
        Some((Wrap::Paint(method), rest)) => {
            format!(".{method}(|p| p{})", render(rest, leaf, important))
        }
        Some((Wrap::Rule { method, when }, rest)) => {
            let var = if *method == "has" { "b" } else { "t" };
            let imp = if important && rest.is_empty() {
                ".important()"
            } else {
                ""
            };
            format!(
                ".{method}({when}, |{var}| {var}{}{imp})",
                render(rest, leaf, important)
            )
        }
    }
}

/// The ring widths a rule sets, by the closures they sit in.
fn ring_widths(pieces: &[&(Translated, String)]) -> Vec<(Vec<Wrap>, f32)> {
    pieces
        .iter()
        .filter_map(|(t, _)| match t.leaf {
            Leaf::RingWidth(w) => Some((t.wraps.clone(), w)),
            _ => None,
        })
        .collect()
}

/// A ring's width and colour under one set of closures, gathered from
/// separate classes, and where its call goes among the rule's calls.
struct Ring {
    wraps: Vec<Wrap>,
    width: Option<f32>,
    color: Option<String>,
    sources: Vec<String>,
    at: usize,
}

/// The calls of one rule for one half: the ring's width and colour joined
/// under the same closures, each call with its sources. A ring is one
/// property in `Tw`, so a colour with no width here takes the width the base
/// rule gives under the same closures (`base`), as CSS keeps it.
fn calls(
    pieces: &[&(Translated, String)],
    base: &[(Vec<Wrap>, f32)],
) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut rings: Vec<Ring> = Vec::new();
    for (t, source) in pieces {
        let ring = match &t.leaf {
            Leaf::Call(call) if call.is_empty() => {
                out.push((String::new(), vec![source.clone()]));
                continue;
            }
            Leaf::Call(call) => {
                out.push((render(&t.wraps, call, t.important), vec![source.clone()]));
                continue;
            }
            Leaf::RingWidth(w) => (Some(*w), None),
            Leaf::RingColor(c) => (None, Some(c.clone())),
        };
        match rings.iter_mut().find(|r| r.wraps == t.wraps) {
            Some(r) => {
                r.width = ring.0.or(r.width);
                r.color = ring.1.or(r.color.take());
                r.sources.push(source.clone());
            }
            None => {
                out.push((String::new(), Vec::new()));
                rings.push(Ring {
                    wraps: t.wraps.clone(),
                    width: ring.0,
                    color: ring.1,
                    sources: vec![source.clone()],
                    at: out.len() - 1,
                });
            }
        }
    }
    for Ring {
        wraps,
        width,
        color,
        sources,
        at,
    } in rings
    {
        let inherited = base
            .iter()
            .rev()
            .find(|(w, _)| *w == wraps)
            .map(|(_, w)| *w);
        let call = match (width.or(inherited), color) {
            (Some(w), Some(c)) => format!(".ring({w:?}, {c})"),
            (Some(w), None) => format!(".ring({w:?}, k.ring) /* no ring colour */"),
            (None, Some(c)) => format!(".ring(0.0, {c}) /* no ring width */"),
            (None, None) => String::new(),
        };
        out[at] = (render(&wraps, &call, false), sources);
    }
    out
}

/// The `When` of a component class's origin.
fn when_of(origin: &Origin) -> Option<String> {
    match origin {
        Origin::Base { .. } => Some("When::always()".to_owned()),
        Origin::Variant { variant, name, .. } if variant == "variant" => {
            Some(format!("When::variant({name:?})"))
        }
        Origin::Variant { variant, name, .. } if variant == "size" => {
            Some(format!("When::size({name:?})"))
        }
        _ => None,
    }
}

/// A rule's pieces in one half: all but the `dark:` ones, which the dark
/// half adds after the rest, as their selector outranks the base classes.
fn half_of<'r>(rule: &'r Rule, half: &str) -> Vec<&'r (Translated, String)> {
    let mut pieces: Vec<_> = rule.pieces.iter().filter(|(t, _)| !t.dark).collect();
    if half == "dark" {
        pieces.extend(rule.pieces.iter().filter(|(t, _)| t.dark));
    }
    pieces
}

/// `file:line` of a class.
fn source_of(row: &Row, sources: &Sources) -> String {
    let file = &sources.files[row.class.file];
    let line = file.text[..row.class.span.start].matches('\n').count() + 1;
    let short = file.name.rsplit('/').next().unwrap_or(&file.name);
    format!("{short}:{line}  {}", row.class.text)
}

/// What the emitter made of one component in one style.
pub struct Emitted {
    /// The light half's function, then the dark half's.
    pub rust: String,
    pub kinds: BTreeMap<Kind, usize>,
    /// Each unmapped class, with why, once.
    pub unmapped: BTreeMap<String, String>,
    /// `Tw` methods the rules call that `Tw` does not have yet.
    pub needs: BTreeSet<String>,
}

/// `Tw` methods the emitter writes that `Tw` lacks today.
const MISSING: [&str; 1] = [".text_size("];

/// The rules of `component`'s rows, for its part `part` (`ButtonPart`).
pub fn emit(
    classifier: &Classifier<'_>,
    rows: &[Row],
    sources: &Sources,
    component: &str,
    part: &str,
    style: &str,
) -> Emitted {
    let mut rules: Vec<Rule> = Vec::new();
    let mut kinds = BTreeMap::new();
    let mut unmapped = BTreeMap::new();
    let mut needs = BTreeSet::new();
    for row in rows.iter().filter(|r| r.component == component) {
        if matches!(row.fate, Fate::Marker) {
            continue;
        }
        let source = source_of(row, sources);
        let Some(base) = when_of(&row.origin) else {
            *kinds.entry(Kind::Unmapped).or_insert(0) += 1;
            unmapped.insert(source, "not on the root's cva table".to_owned());
            continue;
        };
        match classifier.translate(&row.class.text) {
            Err(why) => {
                *kinds.entry(Kind::Unmapped).or_insert(0) += 1;
                unmapped.entry(source).or_insert(why);
            }
            Ok(t) => {
                for kind in &t.kinds {
                    *kinds.entry(*kind).or_insert(0) += 1;
                }
                if let Leaf::Call(call) = &t.leaf {
                    needs.extend(
                        MISSING
                            .iter()
                            .filter(|m| call.contains(*m))
                            .map(|m| m.trim_matches(|c| c == '.' || c == '(').to_owned()),
                    );
                }
                let when = match t.key {
                    Some(key) => format!("{base}.aria({key})"),
                    None => base,
                };
                match rules.iter_mut().find(|r| r.when == when) {
                    Some(rule) => rule.pieces.push((t, source)),
                    None => rules.push(Rule {
                        when,
                        pieces: vec![(t, source)],
                    }),
                }
            }
        }
    }
    let mut rust = String::new();
    let _ = writeln!(
        rust,
        "// Generated by `cargo xtask shadcn-import` from shadcn's `{component}` in `style-{style}.css`. Do not edit."
    );
    for half in ["light", "dark"] {
        let _ = writeln!(
            rust,
            "\npub fn {component}_{half}(s: &mut Sheet, k: &Tokens) {{\n    use {part}::*;"
        );
        let base = rules
            .iter()
            .find(|r| r.when == "When::always()")
            .map(|r| ring_widths(&half_of(r, half)))
            .unwrap_or_default();
        for rule in &rules {
            let pieces = half_of(rule, half);
            if pieces.is_empty() {
                continue;
            }
            let _ = writeln!(rust, "    s.rule(Root, {}, |t| {{\n        t", rule.when);
            for (call, sources) in calls(&pieces, &base) {
                for source in &sources {
                    let _ = writeln!(rust, "            // {source}");
                }
                if !call.is_empty() {
                    let _ = writeln!(rust, "            {call}");
                }
            }
            let _ = writeln!(rust, "    }});");
        }
        for (source, why) in &unmapped {
            let _ = writeln!(rust, "    // unmapped: {source}  ({why})");
        }
        let _ = writeln!(rust, "}}");
    }
    Emitted {
        rust,
        kinds,
        unmapped,
        needs,
    }
}

#[cfg(test)]
mod tests {
    use super::super::classes::{Radius, TwTable};
    use super::*;

    fn table() -> TwTable {
        TwTable::read(&[r#"
    spacing! {
        px, "`px-<units>`: left and right padding.", |s, v| {};
        pl, "`pl-<units>`: left padding.", |s, v| {};
    }
    lengths! {
        h, h_px, h_frac, h_full, h_auto, "the height", |s, l| {};
        size, size_px, size_frac, size_full, size_auto, "the width and height", |s, l| {};
    }
    /// `hover:`. The pointer is over the node.
    pub fn hover(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `focus-visible:`. Keyboard focus.
    pub fn focus(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `shrink-0`.
    pub fn shrink_0(self) -> Self {
"#
        .to_owned()])
    }

    fn with<T>(test: impl FnOnce(&Classifier<'_>) -> T) -> T {
        let tw = table();
        let tokens: BTreeSet<String> = ["primary", "destructive", "ring", "muted"]
            .map(str::to_owned)
            .into();
        test(&Classifier {
            tw: &tw,
            tokens: &tokens,
            radius: Radius { base: 10.0 },
        })
    }

    fn one(c: &Classifier<'_>, class: &str) -> String {
        let t = c
            .translate(class)
            .unwrap_or_else(|why| panic!("{class}: {why}"));
        let pieces = [(t, String::new())];
        let refs: Vec<_> = pieces.iter().collect();
        calls(&refs, &[])
            .into_iter()
            .map(|(call, _)| call)
            .collect()
    }

    #[test]
    fn plain_classes_tokens_and_constants_become_calls() {
        with(|c| {
            assert_eq!(one(c, "h-9"), ".h(9.0)");
            assert_eq!(one(c, "px-2.5"), ".px(2.5)");
            assert_eq!(
                one(c, "bg-primary/80"),
                ".bg(k.primary.gamma_multiply(0.8))"
            );
            assert_eq!(
                one(c, "border-transparent"),
                ".border_color(Color32::TRANSPARENT)"
            );
            assert_eq!(one(c, "rounded-md"), ".rounded_px(8.0)");
            assert_eq!(
                one(c, "rounded-[min(var(--radius-md),8px)]"),
                ".rounded_px(8.0)"
            );
            assert_eq!(one(c, "text-sm"), ".text_size(14.0, 20.0)");
            assert_eq!(one(c, "border"), ".border(1.0)");
            assert_eq!(one(c, "shadow-xs"), ".shadow(k.shadow_xs)");
            assert!(c.translate("shadow-2xl").is_err());
        });
    }

    #[test]
    fn selectors_become_rules_over_other_nodes() {
        with(|c| {
            assert_eq!(
                one(c, "[&_svg:not([class*='size-'])]:size-4"),
                ".descendants(Match::glyph(), |t| t.size(4.0))"
            );
            assert_eq!(
                one(c, "[&>svg]:size-3!"),
                ".children(Match::glyph(), |t| t.size(3.0).important())"
            );
            assert_eq!(
                one(c, "*:data-[slot=input-group]:h-8"),
                ".children(Match::any().slot(\"input-group\"), |t| t.h(8.0))"
            );
            assert_eq!(
                one(c, "has-data-[icon=inline-start]:pl-2"),
                ".has(Match::any().attr(\"icon\", \"inline-start\"), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-aria-invalid:border-destructive"),
                ".has(Match::any().aria(Aria::Invalid), |b| b.border_color(k.destructive))"
            );
        });
    }

    #[test]
    fn states_become_keys_halves_and_draw_variants() {
        with(|c| {
            let invalid = c
                .translate("dark:aria-invalid:border-destructive/50")
                .unwrap();
            assert!(invalid.dark);
            assert_eq!(invalid.key, Some("Aria::Invalid"));
            assert_eq!(
                one(c, "hover:bg-primary/80"),
                ".hover(|p| p.bg(k.primary.gamma_multiply(0.8)))"
            );
            assert_eq!(
                c.translate("disabled:opacity-50").unwrap().key,
                Some("Aria::Disabled")
            );
        });
    }

    #[test]
    fn a_rings_width_and_colour_join_under_the_same_closures() {
        with(|c| {
            let pieces = [
                (c.translate("focus-visible:ring-3").unwrap(), "a".to_owned()),
                (
                    c.translate("focus-visible:ring-ring/50").unwrap(),
                    "b".to_owned(),
                ),
            ];
            let refs: Vec<_> = pieces.iter().collect();
            let calls = calls(&refs, &[]);
            assert_eq!(calls.len(), 1);
            assert_eq!(
                calls[0].0,
                ".focus(|p| p.ring(3.0, k.ring.gamma_multiply(0.5)))"
            );
            assert_eq!(calls[0].1, ["a", "b"]);
        });
    }

    #[test]
    fn a_ring_colour_alone_keeps_the_base_rules_width() {
        with(|c| {
            let base = [(
                c.translate("focus-visible:ring-3").unwrap(),
                "base".to_owned(),
            )];
            let widths = ring_widths(&base.iter().collect::<Vec<_>>());
            let variant = [(
                c.translate("focus-visible:ring-destructive/20").unwrap(),
                "variant".to_owned(),
            )];
            let calls = calls(&variant.iter().collect::<Vec<_>>(), &widths);
            assert_eq!(
                calls[0].0,
                ".focus(|p| p.ring(3.0, k.destructive.gamma_multiply(0.2)))"
            );
        });
    }

    #[test]
    fn what_has_no_form_yet_says_why() {
        with(|c| {
            assert!(
                c.translate("in-data-[slot=button-group]:rounded-md")
                    .is_err()
            );
            assert!(
                c.translate("active:not-aria-[haspopup]:translate-y-px")
                    .is_err()
            );
            assert_eq!(
                c.translate("has-data-[icon=inline-start]:text-primary"),
                Err("`has-` sets box classes only, not inherited text".to_owned())
            );
            assert_eq!(c.translate("size-3!"), Err("`!` outside a rule".to_owned()));
        });
    }
}
