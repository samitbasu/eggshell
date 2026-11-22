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
    /// Of a state the widget never has: dropped.
    Never,
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
            Self::Never => "a state the widget never has (dropped)",
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
    /// The keys of the rule it belongs to, as `When` methods:
    /// `.aria(Aria::Invalid)`, `.not_attr("variant", "destructive")`.
    keys: Vec<String>,
    wraps: Vec<Wrap>,
    leaf: Leaf,
    important: bool,
    kinds: BTreeSet<Kind>,
}

/// `Aria` variants by their attribute.
const ARIA: [(&str, &str); 9] = [
    ("busy", "Aria::Busy"),
    ("haspopup", "Aria::HasPopup"),
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

/// `bg-[color-mix(in_oklch,var(--a),var(--b)_5%)]` as `a` moved 5 % of the
/// way to `b`. egui mixes in gamma space, not OKLCH: close for a small share.
fn color_mix(utility: &str) -> Option<String> {
    let (property, rest) = utility.split_once("-[color-mix(in_oklch,")?;
    let method = match property {
        "bg" => "bg",
        "text" => "text",
        "border" => "border_color",
        _ => return None,
    };
    let (from, rest) = rest.strip_prefix("var(--")?.split_once("),var(--")?;
    let (to, share) = rest.strip_suffix("%)]")?.split_once(")_")?;
    let share = share.parse::<f32>().ok()? / 100.0;
    let token = |name: &str| format!("k.{}", name.replace('-', "_"));
    Some(format!(
        ".{method}({}.lerp_to_gamma({}, {share}))",
        token(from),
        token(to)
    ))
}

/// The shadows the preset's tokens name.
const SHADOWS: [&str; 4] = ["xs", "sm", "md", "lg"];

/// `Tw` methods whose argument is a count, not a length.
const COUNTS: [&str; 1] = [".line_clamp("];

/// `.h(9)` as `.h(9.0)`: `Tw` takes `f32`s, but for counts.
fn floats(call: &str) -> String {
    if COUNTS.iter().any(|m| call.starts_with(m)) {
        return call.to_owned();
    }
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
/// `[&_svg:not([class*='size-'])]` is a plain `[&_svg]`: a node's own
/// classes win over a rule anyway.
fn selector(variant: &str) -> Option<(&'static str, &'static str)> {
    Some(match variant {
        v if v.starts_with("[&_svg") => ("descendants", "Match::glyph()"),
        v if v.starts_with("[&>svg") => ("children", "Match::glyph()"),
        "*" => ("children", "Match::any()"),
        "**" => ("descendants", "Match::any()"),
        _ => return None,
    })
}

/// `[data-slot][aria-invalid=true]`: each attribute of a compound selector,
/// in turn. A bare `[data-slot]` is any shadcn part, which says nothing more
/// about an eggshell node.
fn attributes(selector: &str) -> Option<String> {
    let inner = selector.strip_prefix('[')?.strip_suffix(']')?;
    inner
        .split("][")
        .filter(|group| *group != "data-slot")
        .map(|group| attribute(&format!("[{group}]")))
        .collect()
}

/// A `[data-slot=x]`, `[data-k=v]` or `[aria-x=true]` attribute selector as
/// `Match` methods.
fn attribute(selector: &str) -> Option<String> {
    let inner = selector.strip_prefix('[')?.strip_suffix(']')?;
    let (name, value) = inner.split_once('=')?;
    if name.ends_with(['$', '^', '*', '~', '|']) {
        return None;
    }
    let value = value.trim_matches(|c| c == '"' || c == '\'');
    if name == "data-slot" {
        return Some(format!(".with_slot({value:?})"));
    }
    if let Some(name) = name.strip_prefix("aria-") {
        return (value == "true")
            .then(|| aria(name))
            .flatten()
            .map(|a| format!(".with_aria({a})"));
    }
    let name = name.strip_prefix("data-")?;
    Some(format!(".attr({name:?}, {value:?})"))
}

/// The match of a `has-…` variant: `has-aria-invalid`, `has-data-[k=v]`,
/// `has-[[data-slot=x]:focus-visible]`, `has-[>[data-k=v]]`.
/// The rule method is `has_child` for `has-[>…]`, `has` otherwise.
fn has(variant: &str) -> Option<(&'static str, String)> {
    // `[&:has([role=checkbox])]`: eggshell's checkbox says its slot.
    if variant == "[&:has([role=checkbox])]" {
        return Some(("has", "Match::slot(\"checkbox\")".to_owned()));
    }
    let rest = variant.strip_prefix("has-")?;
    let any = |m: String| Some(("has", m));
    match rest {
        "[>svg]" => return Some(("has_child", "Match::glyph()".to_owned())),
        "[svg]" => return any("Match::glyph()".to_owned()),
        "disabled" => return any("Match::aria(Aria::Disabled)".to_owned()),
        _ => {}
    }
    if let Some(name) = rest.strip_prefix("aria-") {
        return aria(name).and_then(|a| any(format!("Match::aria({a})")));
    }
    if rest == "[:focus-visible]" {
        return any("Match::focus_visible()".to_owned());
    }
    if let Some(name) = rest
        .strip_prefix("data-")
        .filter(|name| !name.starts_with('['))
    {
        return aria(name).and_then(|a| any(format!("Match::aria({a})")));
    }
    if let Some(attr) = rest.strip_prefix("data-[") {
        return any(format!(
            "Match::any(){}",
            attribute(&format!("[data-{attr}"))?
        ));
    }
    let inner = rest.strip_prefix('[')?.strip_suffix(']')?;
    let (method, inner) = match inner.strip_prefix('>') {
        Some(inner) => ("has_child", inner),
        None => ("has", inner),
    };
    let (attr, focus) = match inner.strip_suffix(":focus-visible") {
        Some(attr) => (attr, ".with_focus_visible()"),
        None => (inner, ""),
    };
    Some((method, format!("Match::any(){}{focus}", attributes(attr)?)))
}

/// A variant on the node's own state as a `When` key: `aria-invalid`,
/// `disabled`, `data-[size=sm]`, `data-inset`, and their `not-` forms.
/// Base UI's `data-disabled`, `data-checked` and the like are their ARIA
/// states, `data-open` is `aria-expanded`, `data-active` (the selected
/// tab) is `aria-selected`, `data-[size=…]` and
/// `data-[variant=…]` are the widget's size and variant props, and a data
/// attribute with no value
/// is `"true"`. `data-highlighted` and `data-focused` (a list's current item)
/// are left to each list's migration.
fn own_key(variant: &str) -> Option<String> {
    let (negated, v) = match variant.strip_prefix("not-") {
        Some(v) => (true, v),
        None => (false, variant),
    };
    let not = if negated { "not_" } else { "" };
    if v == "disabled" {
        return Some(format!(".{not}aria(Aria::Disabled)"));
    }
    if let Some(name) = v.strip_prefix("aria-") {
        let name = name.trim_start_matches('[').trim_end_matches(']');
        return aria(name).map(|a| format!(".{not}aria({a})"));
    }
    let data = v.strip_prefix("data-")?;
    if let Some(attr) = data.strip_prefix('[') {
        let (name, value) = attr.strip_suffix(']')?.split_once('=')?;
        if name == "slot" || name.ends_with('$') || name.ends_with('^') {
            return None;
        }
        return match name {
            "size" | "variant" if negated => None,
            "size" | "variant" => Some(format!(".{name}({value:?})")),
            _ => Some(format!(".{not}attr({name:?}, {value:?})")),
        };
    }
    match data {
        "open" => return Some(format!(".{not}aria(Aria::Expanded)")),
        "active" => return Some(format!(".{not}aria(Aria::Selected)")),
        "closed" if !negated => return Some(".not_aria(Aria::Expanded)".to_owned()),
        "unchecked" if !negated => return Some(".not_aria(Aria::Checked)".to_owned()),
        "highlighted" | "focused" | "closed" | "unchecked" => return None,
        _ => {}
    }
    match aria(data) {
        Some(a) => Some(format!(".{not}aria({a})")),
        None => Some(format!(".{not}attr({data:?}, \"true\")")),
    }
}

/// `class` with an item's `focus:` read as `data-highlighted:`. A list item
/// is focused when it is the list's current one (Radix moves focus to it,
/// Base UI marks it highlighted); eggshell marks the current item the same
/// way whether the keyboard or the pointer made it current. shadcn picks
/// items out the same way, by a `data-slot` ending in `-item`, and a
/// submenu's trigger is an item of its menu.
fn highlighted_on_items(slot: &str, class: &str) -> String {
    if !(slot.ends_with("-item") || slot.ends_with("-sub-trigger")) {
        return class.to_owned();
    }
    let (variants, utility) = split_variants(class);
    variants
        .into_iter()
        .map(|v| if v == "focus" { "data-highlighted" } else { v })
        .chain([utility])
        .collect::<Vec<_>>()
        .join(":")
}

/// `p-[3px]` as `p` and 0.75: padding or a gap in pixels, in the spacing
/// scale's units of 4 px.
fn spacing_px(utility: &str) -> Option<(&str, f32)> {
    const SPACING: [&str; 10] = [
        "p", "px", "py", "pt", "pr", "pb", "pl", "gap", "gap-x", "gap-y",
    ];
    let (name, value) = utility.strip_suffix("px]")?.split_once("-[")?;
    let px: f32 = value.parse().ok()?;
    SPACING.contains(&name).then_some((name, px / 4.0))
}

/// A utility `Tw` spells otherwise, or that has nothing to do.
fn equivalent(utility: &str) -> Option<(Option<&'static str>, Kind)> {
    Some(match utility {
        "border" => (Some(".border(1.0)"), Kind::Equivalent),
        "inline-flex" => (Some(".flex()"), Kind::Equivalent),
        "transition-all" | "transition-colors" => (
            Some(".transition(Duration::from_millis(150))"),
            Kind::Equivalent,
        ),
        "border-0" => (Some(".border(0.0)"), Kind::Equivalent),
        "border-t" => (Some(".border_t(1.0)"), Kind::Equivalent),
        "border-b" => (Some(".border_b(1.0)"), Kind::Equivalent),
        "border-b-0" => (Some(".border_b(0.0)"), Kind::Equivalent),
        // A table cell's `vertical-align: middle`: eggshell's cells are
        // flex boxes.
        "align-middle" => (Some(".flex().items_center()"), Kind::Equivalent),
        "caption-bottom" => (None, Kind::NoOp),
        "pointer-events-none" => (Some(".pointer_events_none()"), Kind::Equivalent),
        "translate-y-px" => (Some(".translate_y(1.0)"), Kind::Equivalent),
        "shadow-none" => (Some(".shadow(Shadow::NONE)"), Kind::Equivalent),
        // Tailwind's container widths, in rem of 16 points.
        "max-w-xs" => (Some(".max_w_px(320.0)"), Kind::Equivalent),
        "max-w-sm" => (Some(".max_w_px(384.0)"), Kind::Equivalent),
        "max-w-md" => (Some(".max_w_px(448.0)"), Kind::Equivalent),
        "max-w-lg" => (Some(".max_w_px(512.0)"), Kind::Equivalent),
        "w-3/4" => (Some(".w_frac(3.0, 4.0)"), Kind::Equivalent),
        "border-l" => (Some(".border_l(1.0)"), Kind::Equivalent),
        "border-r" => (Some(".border_r(1.0)"), Kind::Equivalent),
        "font-normal" => (Some(".font_weight(Weight::Normal)"), Kind::Equivalent),
        "tracking-widest" => (Some(".tracking(Tracking::em(0.1))"), Kind::Equivalent),
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
            keys: Vec::new(),
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
            } else if !t.wraps.iter().any(|w| matches!(w, Wrap::Rule { .. }))
                && let Some(key) = own_key(variant)
            {
                t.keys.push(key);
                t.kinds.insert(Kind::Key);
            } else if let Some(method) = self.tw.variants.get(variant) {
                if t.wraps.iter().any(|w| matches!(w, Wrap::Rule { .. })) {
                    return Err(format!("a state (`{variant}:`) of another node"));
                }
                t.wraps.push(Wrap::Paint(method.clone()));
                t.kinds.insert(Kind::DrawVariant);
            } else if let Some((method, base)) = selector(variant) {
                if t.wraps.iter().any(|w| matches!(w, Wrap::Paint(_))) {
                    return Err(format!("a rule (`{variant}:`) under a state of this node"));
                }
                if t.wraps.iter().any(|w| {
                    matches!(
                        w,
                        Wrap::Rule {
                            method: "has" | "has_child",
                            ..
                        }
                    )
                }) {
                    return Err(format!(
                        "a rule (`{variant}:`) under `has`, which sets box classes only"
                    ));
                }
                let mut when = base.to_owned();
                if variant == "*" && variants.peek() == Some(&"[svg]") {
                    when = "Match::glyph()".to_owned();
                    variants.next();
                } else if let Some(attr) = variants
                    .peek()
                    .filter(|v| v.starts_with("data-["))
                    .and_then(|v| attribute(&format!("[data-{}", &v["data-[".len()..])))
                {
                    when = format!("{when}{attr}");
                    variants.next();
                }
                t.wraps.push(Wrap::Rule { method, when });
                t.kinds.insert(Kind::Rule);
            } else if let Some((method, when)) = has(variant) {
                if t.wraps.iter().any(|w| matches!(w, Wrap::Paint(_))) {
                    return Err(format!("a rule (`{variant}:`) under a state of this node"));
                }
                t.wraps.push(Wrap::Rule { method, when });
                t.kinds.insert(Kind::Rule);
            } else {
                return Err(format!("variant `{variant}:`"));
            }
        }
        if important && !t.wraps.iter().any(|w| matches!(w, Wrap::Rule { .. })) {
            return Err("`!` outside a rule".to_owned());
        }
        t.leaf = self.leaf(utility, &mut t.kinds)?;
        let in_has = t.wraps.iter().any(|w| {
            matches!(
                w,
                Wrap::Rule {
                    method: "has" | "has_child",
                    ..
                }
            )
        });
        if in_has && matches!(&t.leaf, Leaf::Call(c) if c.starts_with(".text")) {
            return Err("`has-` sets box classes only, not inherited text".to_owned());
        }
        Ok(t)
    }

    fn leaf(&self, utility: &str, kinds: &mut BTreeSet<Kind>) -> Result<Leaf, String> {
        const MARGINS: [&str; 7] = ["m-", "mx-", "my-", "mt-", "mr-", "mb-", "ml-"];
        if let Some(margin) = utility
            .strip_prefix('-')
            .filter(|rest| MARGINS.iter().any(|m| rest.starts_with(m)))
        {
            return match self.leaf(margin, kinds)? {
                Leaf::Call(call) if call.contains('(') && !call.contains("(-") => {
                    Ok(Leaf::Call(call.replacen('(', "(-", 1)))
                }
                _ => Err(format!("no negative form of `{margin}`")),
            };
        }
        if let Some((call, kind)) = equivalent(utility) {
            kinds.insert(kind);
            return Ok(Leaf::Call(call.unwrap_or_default().to_owned()));
        }
        if let Some(width) = utility.strip_prefix("ring-").and_then(|w| {
            w.parse::<f32>()
                .ok()
                .or_else(|| w.strip_prefix('[')?.strip_suffix("px]")?.parse().ok())
        }) {
            kinds.insert(Kind::TwCall);
            return Ok(Leaf::RingWidth(width));
        }
        if let Some(mix) = color_mix(utility) {
            kinds.insert(Kind::Equivalent);
            return Ok(Leaf::Call(mix));
        }
        if let Some(z) = utility
            .strip_prefix("z-")
            .and_then(|z| z.parse::<i16>().ok())
        {
            kinds.insert(Kind::Equivalent);
            return Ok(Leaf::Call(format!(".z({z})")));
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
        if let Some((name, units)) = spacing_px(utility)
            && let Fate::Tw { call } = self.utility(&format!("{name}-1"))
            && let Some(method) = call.strip_suffix("(1)")
        {
            kinds.insert(Kind::Constant);
            return Ok(Leaf::Call(format!("{method}({units:?})")));
        }
        match self.utility(utility) {
            Fate::Tw { call } if call.contains("(/*") => {
                Err(format!("no value of `{utility}` for its `Tw` method"))
            }
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

/// One rule: its part and keys, then its pieces in source order.
struct Rule {
    part: String,
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
            let var = if matches!(*method, "has" | "has_child") {
                "b"
            } else {
                "t"
            };
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
fn when_of(origin: &Origin, widget: &Widget) -> Vec<String> {
    match origin {
        Origin::Base { .. } | Origin::Slot { .. } => vec!["When::always()".to_owned()],
        Origin::Variant { variant, name, .. } if variant == "variant" => widget
            .variants_of(name)
            .into_iter()
            .map(|v| format!("When::always().variant(Variant({v:?}))"))
            .collect(),
        Origin::Variant { variant, name, .. } if variant == "size" => widget
            .sizes_of(name)
            .into_iter()
            .map(|v| format!("When::always().size(Size({v:?}))"))
            .collect(),
        Origin::Variant { variant, name, .. } => {
            vec![format!("When::always().attr({variant:?}, {name:?})")]
        }
    }
}

/// How eggshell names a widget's shadcn variants and sizes, where it names
/// them otherwise: each shadcn name, and the eggshell names its rules are
/// written under.
pub struct Widget {
    pub component: &'static str,
    /// Where the widget's part enum lives.
    pub home: Home,
    /// Written into the preset's crate (`sheets/`), not only the report.
    pub checked_in: bool,
    pub variants: &'static [(&'static str, &'static [&'static str])],
    pub sizes: &'static [(&'static str, &'static [&'static str])],
    /// The states shadcn's source asks about that eggshell's widget always
    /// or never has, so no key or rule asks for them.
    pub facts: &'static [Assumed],
    /// The part enum, where eggshell names it otherwise than the component
    /// (`dropdown-menu`'s parts are `MenuPart`).
    pub parts: Option<&'static str>,
    /// Other components a part renders, whose root classes come first in
    /// its rules: input-group's `Input` is an `Input`.
    pub bases: &'static [Base],
    /// Elements a selector names that are parts here, by slot:
    /// input-group's `[&>input]` is its control.
    pub aliases: &'static [(&'static str, &'static str)],
    /// Rules of the root over one part beside another, as a key that part
    /// carries.
    pub relations: &'static [Relation],
    /// Slots shadcn gives several parts that eggshell keeps apart, as the
    /// slot and the others a rule over it also reaches: input-group's control
    /// is its Input or its Textarea.
    pub slot_aliases: &'static [(&'static str, &'static [&'static str])],
    /// An ancestor's state a part carries as its own, as the variants that
    /// ask for each: input-group's addon is disabled with its group.
    pub inherited: &'static [(&'static str, &'static str)],
    /// Parts that draw as another part, by its enum variant: table.tsx's
    /// footer cells are its `TableCell`s, which eggshell names apart.
    pub copies: &'static [(&'static str, &'static [&'static str])],
    /// Parts shadcn's component has that eggshell's widget does not draw,
    /// by slot, and why: their classes are dropped.
    pub absent: &'static [(&'static str, &'static str)],
}

/// Where a widget's part enum lives: core's `owned`, or an extension
/// crate's module, whose rules the preset writes only with its `extras`
/// feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    Core,
    Extension(&'static str),
}

impl Home {
    /// The module that names the part enum.
    fn module(self) -> &'static str {
        match self {
            Self::Core => "eggshell_ui::owned",
            Self::Extension(module) => module,
        }
    }

    /// The attribute an item of the preset's crate takes for it.
    pub fn cfg(self) -> &'static str {
        match self {
            Self::Core => "",
            Self::Extension(_) => "#[cfg(feature = \"extras\")] ",
        }
    }
}

/// `has-[>[data-align=inline-end]]:[&>input]:pr-1.5` on the root: the
/// control beside an inline-end addon. eggshell's control carries that as
/// its own key, `attr("addon", "inline-end")`, so the class becomes a rule
/// of the control's under that key.
#[derive(Debug)]
pub struct Relation {
    /// The variants as shadcn writes them on the root, through the element.
    pub variants: &'static str,
    /// The part the classes go to, as its enum variant.
    pub part: &'static str,
    /// The key that part carries instead.
    pub key: (&'static str, &'static str),
}

impl Widget {
    /// `class` of the root, as the part and class it is under a relation.
    fn relate(&self, class: &str) -> Option<(&'static str, String)> {
        self.relations.iter().find_map(|r| {
            let utility = class.strip_prefix(r.variants)?.strip_prefix(':')?;
            let (name, value) = r.key;
            Some((r.part, format!("data-[{name}={value}]:{utility}")))
        })
    }
}

/// A part that renders another component, whose root classes it takes first.
#[derive(Debug)]
pub struct Base {
    /// The part, as its enum variant.
    pub part: &'static str,
    /// The component it renders.
    pub component: &'static str,
}

/// The breakpoints every widget is drawn at: eggshell's responsiveness is
/// regions measuring their width, not the viewport's, and shadcn's narrower
/// classes are for phones (`text-base` under `md:`, so iOS does not zoom
/// into a native input).
const VIEWPORT: [Assumed; 2] = [
    Assumed {
        variant: "sm",
        fact: Fact::Holds,
        why: "drawn at desktop width (`sm:` holds)",
    },
    Assumed {
        variant: "md",
        fact: Fact::Holds,
        why: "drawn at desktop width (`md:` holds)",
    },
];

/// Whether eggshell's widget always or never has a state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fact {
    Holds,
    Never,
}

/// A state the emitter settles for a widget: a variant as shadcn writes it
/// (`group-data-horizontal/tabs`, `data-horizontal`), whether it holds, and
/// why, for the report.
#[derive(Debug, PartialEq, Eq)]
pub struct Assumed {
    pub variant: &'static str,
    pub fact: Fact,
    pub why: &'static str,
}

impl Widget {
    fn named(table: &[(&str, &'static [&'static str])], name: &str) -> Vec<String> {
        match table.iter().find(|(shadcn, _)| *shadcn == name) {
            Some((_, ours)) => ours.iter().map(|n| (*n).to_owned()).collect(),
            None => vec![name.to_owned()],
        }
    }

    /// `class` without the variants that hold, and the facts that settled
    /// them; or the fact that `class` never applies.
    fn settle(&self, class: &str) -> Result<(String, Vec<&'static Assumed>), &'static Assumed> {
        let (variants, utility) = split_variants(class);
        let mut kept: Vec<String> = Vec::new();
        let mut held = Vec::new();
        for variant in variants {
            if let Some((_, own)) = self.inherited.iter().find(|(group, _)| *group == variant) {
                kept.push((*own).to_owned());
                continue;
            }
            if let Some(slot) = self.alias(variant) {
                kept.push(
                    if variant.starts_with("[&>") {
                        "*"
                    } else {
                        "**"
                    }
                    .to_owned(),
                );
                kept.push(format!("data-[slot={slot}]"));
                continue;
            }
            let (negated, v) = match variant.strip_prefix("not-") {
                Some(v) => (true, v),
                None => (false, variant),
            };
            match self.facts.iter().chain(&VIEWPORT).find(|f| f.variant == v) {
                None => kept.push(variant.to_owned()),
                Some(fact) if (fact.fact == Fact::Holds) != negated => held.push(fact),
                Some(fact) => return Err(fact),
            }
        }
        kept.push(utility.to_owned());
        Ok((kept.join(":"), held))
    }

    /// `t`, and a copy of it for each slot that aliases one its rules match.
    fn over_aliases(&self, t: &Translated) -> Vec<Translated> {
        let mut all = vec![t.clone()];
        for (slot, others) in self.slot_aliases {
            let named = format!(".with_slot({slot:?})");
            if !t
                .wraps
                .iter()
                .any(|w| matches!(w, Wrap::Rule { when, .. } if when.contains(&named)))
            {
                continue;
            }
            for other in *others {
                let mut copy = t.clone();
                for wrap in &mut copy.wraps {
                    if let Wrap::Rule { when, .. } = wrap {
                        *when = when.replace(&named, &format!(".with_slot({other:?})"));
                    }
                }
                all.push(copy);
            }
        }
        all
    }

    /// The slot of the element `[&>name]` or `[&_name]` selects, if it is
    /// a part here.
    fn alias(&self, variant: &str) -> Option<&'static str> {
        let name = variant
            .strip_prefix("[&>")
            .or_else(|| variant.strip_prefix("[&_"))?
            .strip_suffix(']')?;
        self.aliases
            .iter()
            .find_map(|(element, slot)| (*element == name).then_some(*slot))
    }

    /// The enum the widget's parts are variants of.
    pub fn part_enum(&self) -> String {
        self.parts
            .map(str::to_owned)
            .unwrap_or_else(|| part_enum(self.component))
    }

    fn variants_of(&self, name: &str) -> Vec<String> {
        Self::named(self.variants, name)
    }

    fn sizes_of(&self, name: &str) -> Vec<String> {
        Self::named(self.sizes, name)
    }

    /// An own-state key as the `When` method that asks for it, with a size or
    /// variant named as eggshell names it.
    fn key(&self, key: &str) -> String {
        let arg = |prefix: &str| key.strip_prefix(prefix)?.strip_suffix("\")");
        if let Some(name) = arg(".size(\"") {
            return format!(".size(Size({:?}))", self.sizes_of(name)[0]);
        }
        if let Some(name) = arg(".variant(\"") {
            return format!(".variant(Variant({:?}))", self.variants_of(name)[0]);
        }
        key.to_owned()
    }
}

/// The constant a generated file names a source file by: `VEGA` for
/// `registry/styles/style-vega.css`, `BUTTON` for
/// `registry/bases/radix/ui/button.tsx`.
fn file_constant(file: &str) -> String {
    let stem = file.rsplit('/').next().unwrap_or(file);
    let stem = stem.split('.').next().unwrap_or(stem);
    stem.trim_start_matches("style-")
        .replace('-', "_")
        .to_uppercase()
}

/// shadcn's base layer, `* { @apply border-border }`: every element's border
/// colour, under the classes that name one.
const BORDER_BASE: (&str, u32) = ("registry/bases/radix/registry.ts", 24);

/// Whether a rule of `part` in `half` gives it a border width.
fn draws_a_border(rules: &[Rule], part: &str, half: &str) -> bool {
    rules
        .iter()
        .filter(|r| r.part == part)
        .flat_map(|r| half_of(r, half))
        .any(|(t, _)| {
            matches!(&t.leaf, Leaf::Call(call)
                if [".border(", ".border_t(", ".border_b(", ".border_l(", ".border_r("]
                    .iter()
                    .any(|width| call.starts_with(width)))
        })
}

/// A class's source, as `source_of` writes it: its file, line and class.
fn parse_source(source: &str) -> (&str, &str, &str) {
    let (file, rest) = source.split_once(':').unwrap_or((source, ""));
    let (line, class) = rest.split_once("  ").unwrap_or((rest, ""));
    (file, line, class)
}

/// `input-group-addon` as `InputGroupAddon`.
fn pascal(slot: &str) -> String {
    slot.split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// The part enum of a component: `SelectPart`.
pub fn part_enum(component: &str) -> String {
    format!("{}Part", pascal(component))
}

/// A part's variant: its slot without the component's name, `Root` for the
/// component's own; a slot of another component keeps its whole name.
fn part_variant(component: &str, slot: &str) -> String {
    if slot == component || slot.is_empty() {
        return "Root".to_owned();
    }
    match slot
        .strip_prefix(component)
        .and_then(|s| s.strip_prefix('-'))
    {
        Some(rest) => pascal(rest),
        None => pascal(slot),
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
    format!("{}:{line}  {}", file.name, row.class.text)
}

/// What the emitter made of one component in one style.
pub struct Emitted {
    /// The light half's function, then the dark half's.
    pub rust: String,
    pub kinds: BTreeMap<Kind, usize>,
    /// Each unmapped class, with why, once.
    pub unmapped: BTreeMap<String, String>,
    /// Each state taken to hold, with why.
    pub assumed: BTreeSet<(&'static str, &'static str)>,
    /// Each class of a state the widget never has, with why, once.
    pub dropped: BTreeMap<String, &'static str>,
}

/// The rules of `component`'s rows, one per part and keys.
pub fn emit(
    classifier: &Classifier<'_>,
    rows: &[Row],
    sources: &Sources,
    widget: &Widget,
    style: &str,
) -> Emitted {
    let component = widget.component;
    let mut rules: Vec<Rule> = Vec::new();
    let mut kinds = BTreeMap::new();
    let mut unmapped = BTreeMap::new();
    let mut assumed = BTreeSet::new();
    let mut dropped = BTreeMap::new();
    let based = widget.bases.iter().flat_map(|base| {
        rows.iter()
            .filter(move |r| {
                r.component == base.component && part_variant(base.component, &r.slot) == "Root"
            })
            .map(move |r| (r, base.part.to_owned()))
    });
    let own = rows
        .iter()
        .filter(|r| r.component == component)
        .map(|r| (r, part_variant(component, &r.slot)));
    for (row, part) in based.chain(own) {
        if matches!(row.fate, Fate::Marker) {
            continue;
        }
        let source = source_of(row, sources);
        if row.slot.is_empty() {
            *kinds.entry(Kind::Never).or_insert(0) += 1;
            dropped
                .entry(source)
                .or_insert("an element of another component, which names no part here");
            continue;
        }
        if let Some(&(_, why)) = widget.absent.iter().find(|(slot, _)| *slot == row.slot) {
            *kinds.entry(Kind::Never).or_insert(0) += 1;
            dropped.entry(source).or_insert(why);
            continue;
        }
        let bases = when_of(&row.origin, widget);
        let (part, text) = match (part.as_str(), widget.relate(&row.class.text)) {
            ("Root", Some((related, class))) => (related.to_owned(), class),
            _ => (part, row.class.text.clone()),
        };
        let class = match widget.settle(&text) {
            Ok((class, held)) => {
                assumed.extend(held.iter().map(|f| (f.variant, f.why)));
                class
            }
            Err(never) => {
                *kinds.entry(Kind::Never).or_insert(0) += 1;
                dropped.entry(source).or_insert(never.why);
                continue;
            }
        };
        match classifier.translate(&highlighted_on_items(&row.slot, &class)) {
            Err(why) => {
                *kinds.entry(Kind::Unmapped).or_insert(0) += 1;
                unmapped.entry(source).or_insert(why);
            }
            Ok(t) => {
                for kind in &t.kinds {
                    *kinds.entry(*kind).or_insert(0) += 1;
                }
                let keys: Vec<String> = t.keys.iter().map(|key| widget.key(key)).collect();
                for base in &bases {
                    let when = format!("{base}{}", keys.concat());
                    for t in widget.over_aliases(&t) {
                        let piece = (t, source.clone());
                        match rules.iter_mut().find(|r| r.part == part && r.when == when) {
                            Some(rule) => rule.pieces.push(piece),
                            None => rules.push(Rule {
                                part: part.clone(),
                                when: when.clone(),
                                pieces: vec![piece],
                            }),
                        }
                    }
                }
            }
        }
    }
    for &(from, to) in widget.copies {
        let copied: Vec<Rule> = rules
            .iter()
            .filter(|r| r.part == from)
            .flat_map(|r| {
                to.iter().map(|part| Rule {
                    part: (*part).to_owned(),
                    when: r.when.clone(),
                    pieces: r.pieces.clone(),
                })
            })
            .collect();
        rules.extend(copied);
    }
    // A part's base rule replaces the lower layers' rules for it, so it
    // comes before the part's conditional rules, which it would otherwise
    // cut off.
    let first_seen: Vec<String> =
        rules
            .iter()
            .map(|r| r.part.clone())
            .fold(Vec::new(), |mut seen, part| {
                if !seen.contains(&part) {
                    seen.push(part);
                }
                seen
            });
    rules.sort_by_key(|r| {
        (
            first_seen.iter().position(|p| *p == r.part),
            r.when != "When::always()",
        )
    });
    let mut body = String::new();
    let mut constants = BTreeMap::new();
    for half in ["light", "dark"] {
        let _ = writeln!(
            body,
            "\n/// TODO(prose): shadcn's `{component}` in `style-{style}.css`, {half}.\npub(crate) fn {half}(s: &mut Sheet) {{\n    use {}::*;",
            widget.part_enum()
        );
        for rule in &rules {
            let pieces = half_of(rule, half);
            if pieces.is_empty() {
                continue;
            }
            let base = rules
                .iter()
                .find(|r| r.part == rule.part && r.when == "When::always()")
                .map(|r| ring_widths(&half_of(r, half)))
                .unwrap_or_default();
            let mut calls_text = String::new();
            for (call, sources) in calls(&pieces, &base) {
                if call.is_empty() {
                    for source in &sources {
                        let _ = writeln!(calls_text, "        // {source}: nothing to do");
                    }
                    continue;
                }
                let (file, line, _) = parse_source(&sources[0]);
                let class = sources
                    .iter()
                    .map(|s| parse_source(s).2)
                    .collect::<Vec<_>>()
                    .join(" ");
                let constant = file_constant(file);
                let _ = writeln!(
                    calls_text,
                    "            .from({constant}, {line}, {class:?})\n            {call}"
                );
                constants.insert(constant, file.to_owned());
            }
            if !calls_text.contains(".from(") {
                continue;
            }
            let first_base = rule.when == "When::always()"
                && rules
                    .iter()
                    .find(|r| r.part == rule.part)
                    .map(|r| r.when.as_str())
                    == Some("When::always()");
            if first_base && draws_a_border(&rules, &rule.part, half) {
                let constant = file_constant(BORDER_BASE.0);
                calls_text.insert_str(
                    0,
                    &format!(
                        "            .from({constant}, {}, \"border-border\")\n            .border_color(k.border)\n",
                        BORDER_BASE.1
                    ),
                );
                constants.insert(constant, BORDER_BASE.0.to_owned());
            }
            let k = if calls_text.contains("k.") { "k" } else { "_" };
            let (method, when) = match (first_base, rule.when.as_str()) {
                (true, _) => ("replace".to_owned(), String::new()),
                (false, "When::always()") => ("rule".to_owned(), String::new()),
                (false, when) => ("rule_when".to_owned(), format!(", {when}")),
            };
            let _ = write!(
                body,
                "    s.{method}({}{when}, |t, {k}| {{\n        t\n{calls_text}    }});\n",
                rule.part
            );
        }
        for (source, why) in &unmapped {
            let _ = writeln!(body, "    // unmapped: {source} ({why})");
        }
        let _ = writeln!(body, "}}");
    }
    let mut rust = format!(
        "//! Generated by `cargo xtask shadcn-import --write` from shadcn's `{component}`\n//! (`registry/bases/radix/ui/{component}.tsx`, `registry/styles/style-{style}.css`). Do not edit.\n\n"
    );
    for (needle, import) in [
        ("Duration::", "use std::time::Duration;\n\n"),
        ("Color32::", "use egui::Color32;\n"),
        ("Shadow::", "use egui::Shadow;\n"),
    ] {
        if body.contains(needle) {
            rust.push_str(import);
        }
    }
    let mut core = Vec::new();
    for (needle, name) in [
        ("Aria::", "Aria"),
        ("Match::", "Match"),
        ("Opacity::", "Opacity"),
        ("Tracking::", "Tracking"),
        ("Weight::", "Weight"),
    ] {
        if body.contains(needle) {
            core.push(name);
        }
    }
    core.sort_unstable();
    let mut contract = Vec::new();
    for (needle, name) in [("Size(", "Size"), ("Variant(", "Variant")] {
        if body.contains(needle) {
            contract.push(name);
        }
    }
    if !contract.is_empty() {
        let _ = writeln!(
            rust,
            "use eggshell_ui::contract::{{{}}};",
            contract.join(", ")
        );
    }
    let _ = writeln!(
        rust,
        "use {}::{};",
        widget.home.module(),
        widget.part_enum()
    );
    let sheet = match body.contains("When::") {
        true => "{Sheet, When}",
        false => "Sheet",
    };
    let _ = writeln!(rust, "use eggshell_ui::sheet::{sheet};");
    if !core.is_empty() {
        let _ = writeln!(rust, "use eggshell_ui::{{{}}};", core.join(", "));
    }
    let _ = writeln!(rust);
    for (constant, path) in &constants {
        let _ = writeln!(rust, "const {constant}: &str = {path:?};");
    }
    rust.push_str(&body);
    Emitted {
        rust,
        kinds,
        unmapped,
        assumed,
        dropped,
    }
}

#[cfg(test)]
mod tests {
    use super::super::classes::{Radius, TwTable};
    use super::*;

    #[test]
    fn an_items_focus_is_its_highlight_and_nothing_elses_is() {
        assert_eq!(
            highlighted_on_items("select-item", "focus:bg-accent"),
            "data-highlighted:bg-accent"
        );
        assert_eq!(
            highlighted_on_items("select-item", "dark:focus:text-x!"),
            "dark:data-highlighted:text-x!"
        );
        assert_eq!(
            highlighted_on_items("select-trigger", "focus:bg-accent"),
            "focus:bg-accent"
        );
        assert_eq!(
            highlighted_on_items("dropdown-menu-sub-trigger", "focus:bg-accent"),
            "data-highlighted:bg-accent"
        );
    }

    fn table() -> TwTable {
        TwTable::read(&[r#"
    spacing! {
        px, "`px-<units>`: left and right padding.", |s, v| {};
        pl, "`pl-<units>`: left padding.", |s, v| {};
        p, "`p-<units>`: padding on every side.", |s, v| {};
        mx, "`mx-<units>`: left and right margin.", |s, v| {};
    }
    lengths! {
        h, h_px, h_frac, h_full, h_auto, "the height", |s, l| {};
        size, size_px, size_frac, size_full, size_auto, "the width and height", |s, l| {};
    }
    /// `hover:`. The pointer is over the node.
    pub fn hover(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `focus-visible:`. Keyboard focus.
    pub fn focus(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `active:`. Held.
    pub fn active(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `gap-<units>`: spacing between children on both axes.
    pub fn gap(self, units: f32) -> Self {
    /// `shrink-0`.
    pub fn shrink_0(self) -> Self {
    /// `translate-x-*`.
    pub fn translate_x(self, x: f32) -> Self {
    /// `line-clamp-<lines>`: at most so many lines.
    pub fn line_clamp(self, lines: u32) -> Self {
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
            assert!(
                c.translate("translate-x-0").is_err(),
                "a value the method has no spelling of"
            );
            assert_eq!(one(c, "-mx-1"), ".mx(-1.0)", "a negative margin");
            assert_eq!(one(c, "max-w-xs"), ".max_w_px(320.0)", "20rem");
            assert_eq!(one(c, "border-b"), ".border_b(1.0)");
            assert_eq!(one(c, "max-w-md"), ".max_w_px(448.0)", "28rem");
            assert_eq!(
                one(c, "[&:has([role=checkbox])]:pl-2"),
                ".has(Match::slot(\"checkbox\"), |b| b.pl(2.0))"
            );
            assert_eq!(one(c, "font-normal"), ".font_weight(Weight::Normal)");
            assert!(c.translate("-px-1").is_err(), "padding is never negative");
            assert_eq!(one(c, "tracking-widest"), ".tracking(Tracking::em(0.1))");
        });
    }

    #[test]
    fn arbitrary_pixel_padding_and_gaps_are_their_spacing_units() {
        with(|c| {
            assert_eq!(one(c, "p-[3px]"), ".p(0.75)");
            assert_eq!(one(c, "px-[10px]"), ".px(2.5)");
            assert_eq!(one(c, "gap-[6px]"), ".gap(1.5)");
            assert!(c.translate("p-[calc(100%-1px)]").is_err());
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
                ".children(Match::any().with_slot(\"input-group\"), |t| t.h(8.0))"
            );
            assert_eq!(
                one(c, "has-data-[icon=inline-start]:pl-2"),
                ".has(Match::any().attr(\"icon\", \"inline-start\"), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-data-checked:pl-2"),
                ".has(Match::aria(Aria::Checked), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-[:focus-visible]:pl-2"),
                ".has(Match::focus_visible(), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-data-[slot=kbd]:pl-2"),
                ".has(Match::any().with_slot(\"kbd\"), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-aria-invalid:border-destructive"),
                ".has(Match::aria(Aria::Invalid), |b| b.border_color(k.destructive))"
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
            assert_eq!(invalid.keys, [".aria(Aria::Invalid)"]);
            assert_eq!(
                one(c, "hover:bg-primary/80"),
                ".hover(|p| p.bg(k.primary.gamma_multiply(0.8)))"
            );
            assert_eq!(
                c.translate("disabled:opacity-50").unwrap().keys,
                [".aria(Aria::Disabled)"]
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
            assert!(matches!(
                c.translate("focus-visible:ring-[3px]").map(|t| t.leaf),
                Ok(Leaf::RingWidth(3.0))
            ));
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

    fn keys(c: &Classifier<'_>, class: &str) -> Vec<String> {
        c.translate(class)
            .unwrap_or_else(|why| panic!("{class}: {why}"))
            .keys
    }

    #[test]
    fn own_state_variants_become_keys_wherever_they_are_written() {
        with(|c| {
            assert_eq!(keys(c, "data-[size=sm]:h-8"), [".size(\"sm\")"]);
            assert_eq!(keys(c, "data-[side=top]:h-8"), [".attr(\"side\", \"top\")"]);
            assert_eq!(keys(c, "data-inset:pl-8"), [".attr(\"inset\", \"true\")"]);
            assert_eq!(
                keys(c, "data-disabled:opacity-50"),
                [".aria(Aria::Disabled)"]
            );
            assert_eq!(
                keys(c, "not-data-[orientation=vertical]:bg-muted"),
                [".not_attr(\"orientation\", \"vertical\")"]
            );
            assert!(
                c.translate("not-data-[variant=destructive]:bg-muted")
                    .is_err(),
                "When has no negated variant"
            );
            assert_eq!(
                keys(c, "active:not-aria-[haspopup]:translate-y-px"),
                [".not_aria(Aria::HasPopup)"]
            );
            assert_eq!(keys(c, "aria-invalid:data-open:bg-muted").len(), 2);
            assert_eq!(
                keys(c, "data-active:bg-muted"),
                [".aria(Aria::Selected)"],
                "Base UI marks the selected tab `data-active`"
            );
            assert_eq!(
                keys(c, "not-data-active:bg-muted"),
                [".not_aria(Aria::Selected)"]
            );
        });
    }

    #[test]
    fn has_and_selector_forms_with_brackets() {
        with(|c| {
            assert_eq!(
                one(
                    c,
                    "has-[[data-slot=input-group-control]:focus-visible]:border-ring"
                ),
                ".has(Match::any().with_slot(\"input-group-control\").with_focus_visible(), |b| b.border_color(k.ring))"
            );
            assert_eq!(
                one(c, "has-[>[data-align=block-end]]:h-9"),
                ".has_child(Match::any().attr(\"align\", \"block-end\"), |b| b.h(9.0))",
                "`>` is a child, not any descendant"
            );
            assert_eq!(
                one(c, "has-[>svg]:h-9"),
                ".has_child(Match::glyph(), |b| b.h(9.0))"
            );
            assert_eq!(
                one(c, "has-[svg]:h-9"),
                ".has(Match::glyph(), |b| b.h(9.0))"
            );
            assert_eq!(
                one(c, "has-[[data-slot][aria-invalid=true]]:border-destructive"),
                ".has(Match::any().with_aria(Aria::Invalid), |b| b.border_color(k.destructive))",
                "a compound selector: any part that is invalid"
            );
            assert!(
                c.translate("has-[>[data-align=block-end]]:text-muted")
                    .is_err(),
                "`has_child` sets box classes only, like `has`"
            );
            assert_eq!(
                one(c, "has-[[aria-invalid=true]]:border-destructive"),
                ".has(Match::any().with_aria(Aria::Invalid), |b| b.border_color(k.destructive))"
            );
            assert_eq!(
                one(c, "[&_svg:not([class*='text-'])]:text-muted"),
                ".descendants(Match::glyph(), |t| t.text(k.muted))"
            );
            assert_eq!(
                one(c, "**:data-[variant=destructive]:text-muted"),
                ".descendants(Match::any().attr(\"variant\", \"destructive\"), |t| t.text(k.muted))"
            );
            assert_eq!(
                one(c, "has-[>svg]:pl-2"),
                ".has_child(Match::glyph(), |b| b.pl(2.0))"
            );
            assert_eq!(
                one(c, "has-disabled:opacity-50"),
                ".has(Match::aria(Aria::Disabled), |b| b.opacity(Opacity::new(0.5)))"
            );
            assert_eq!(one(c, "shadow-none"), ".shadow(Shadow::NONE)");
            assert_eq!(one(c, "z-50"), ".z(50)");
            assert_eq!(
                one(
                    c,
                    "hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]"
                ),
                ".hover(|p| p.bg(k.secondary.lerp_to_gamma(k.foreground, 0.05)))"
            );
            assert_eq!(one(c, "line-clamp-1"), ".line_clamp(1)");
            assert_eq!(
                one(c, "*:[svg]:size-4"),
                ".children(Match::glyph(), |t| t.size(4.0))"
            );
        });
    }

    #[test]
    fn parts_are_named_from_their_slot_within_the_component() {
        assert_eq!(part_variant("select", "select"), "Root");
        assert_eq!(part_variant("select", "select-trigger-icon"), "TriggerIcon");
        assert_eq!(part_variant("combobox", "input-group"), "InputGroup");
        assert_eq!(part_enum("input-group"), "InputGroupPart");
        assert_eq!(HORIZONTAL.part_enum(), "TabsPart");
        let menu = Widget {
            component: "dropdown-menu",
            parts: Some("MenuPart"),
            ..HORIZONTAL
        };
        assert_eq!(menu.part_enum(), "MenuPart");
        assert_eq!(
            part_variant("dropdown-menu", "dropdown-menu-sub-trigger"),
            "SubTrigger"
        );
    }

    #[test]
    fn what_has_no_form_yet_says_why() {
        with(|c| {
            assert!(
                c.translate("in-data-[slot=button-group]:rounded-md")
                    .is_err()
            );
            assert_eq!(
                c.translate("has-data-[icon=inline-start]:text-primary"),
                Err("`has-` sets box classes only, not inherited text".to_owned())
            );
            assert_eq!(c.translate("size-3!"), Err("`!` outside a rule".to_owned()));
            for class in [
                "group-data-[size=sm]/switch:h-4",
                "not-last:border-b",
                "has-[>textarea]:h-auto",
                "focus:**:text-muted",
                "**:data-[slot$=-item]:h-8",
            ] {
                assert!(c.translate(class).is_err(), "{class}");
            }
        });
    }

    const HORIZONTAL: Widget = Widget {
        component: "tabs",
        home: Home::Core,
        checked_in: false,
        variants: &[],
        sizes: &[],
        parts: None,
        bases: &[],
        aliases: &[("input", "input-group-control")],
        relations: &[Relation {
            variants: "has-[>[data-align=inline-end]]:[&>input]",
            part: "Input",
            key: ("addon", "inline-end"),
        }],
        slot_aliases: &[("input-group-control", &["input-group-textarea"])],
        inherited: &[("group-data-[disabled=true]/input-group", "disabled")],
        absent: &[],
        copies: &[],
        facts: &[
            Assumed {
                variant: "group-data-horizontal/tabs",
                fact: Fact::Holds,
                why: "horizontal only",
            },
            Assumed {
                variant: "group-data-[variant=line]/tabs-list",
                fact: Fact::Never,
                why: "no line variant",
            },
        ],
    };

    #[test]
    fn a_state_the_widget_always_has_is_dropped_from_the_class() {
        let (class, held) = HORIZONTAL
            .settle("group-data-horizontal/tabs:h-9")
            .expect("it holds");
        assert_eq!(class, "h-9");
        assert_eq!(held.len(), 1);
        assert_eq!(
            HORIZONTAL
                .settle("dark:group-data-horizontal/tabs:data-active:shadow-sm!")
                .map(|(class, _)| class),
            Ok("dark:data-active:shadow-sm!".to_owned())
        );
        assert_eq!(
            HORIZONTAL
                .settle("data-active:bg-background")
                .map(|(class, held)| (class, held.len())),
            Ok(("data-active:bg-background".to_owned(), 0))
        );
    }

    #[test]
    fn a_groups_state_a_part_carries_is_its_own_key() {
        let (class, _) = HORIZONTAL
            .settle("group-data-[disabled=true]/input-group:opacity-50")
            .expect("it maps");
        assert_eq!(class, "disabled:opacity-50");
        with(|c| {
            assert_eq!(
                c.translate(&class).map(|t| t.keys),
                Ok(vec![".aria(Aria::Disabled)".to_owned()])
            );
        });
    }

    #[test]
    fn a_rule_over_an_aliased_slot_reaches_each_part_that_has_it() {
        with(|c| {
            let t = c
                .translate("has-[[data-slot=input-group-control]:focus-visible]:border-ring")
                .expect("maps");
            let all = HORIZONTAL.over_aliases(&t);
            let whens: Vec<_> = all
                .iter()
                .flat_map(|t| &t.wraps)
                .filter_map(|w| match w {
                    Wrap::Rule { when, .. } => Some(when.as_str()),
                    Wrap::Paint(_) => None,
                })
                .collect();
            assert_eq!(
                whens,
                [
                    "Match::any().with_slot(\"input-group-control\").with_focus_visible()",
                    "Match::any().with_slot(\"input-group-textarea\").with_focus_visible()",
                ]
            );
            let plain = c.translate("border-ring").expect("maps");
            assert_eq!(HORIZONTAL.over_aliases(&plain).len(), 1, "no rule, no copy");
        });
    }

    #[test]
    fn a_root_rule_over_a_part_beside_another_is_the_parts_own_key() {
        assert_eq!(
            HORIZONTAL.relate("has-[>[data-align=inline-end]]:[&>input]:pr-1.5"),
            Some(("Input", "data-[addon=inline-end]:pr-1.5".to_owned()))
        );
        assert_eq!(
            HORIZONTAL.relate("has-[>[data-align=block-end]]:[&>input]:pt-3"),
            None
        );
        with(|c| {
            assert_eq!(
                c.translate("data-[addon=inline-end]:pl-1.5")
                    .map(|t| t.keys),
                Ok(vec![".attr(\"addon\", \"inline-end\")".to_owned()])
            );
            assert!(
                c.translate("has-[>[data-align=block-end]]:*:data-[slot=input-group-control]:pt-3")
                    .is_err(),
                "a rule under `has` is reported, not emitted"
            );
        });
    }

    #[test]
    fn an_element_a_selector_names_is_its_part_by_slot() {
        assert_eq!(
            HORIZONTAL
                .settle("has-[>[data-align=inline-start]]:[&>input]:pl-1.5")
                .map(|(class, _)| class),
            Ok(
                "has-[>[data-align=inline-start]]:*:data-[slot=input-group-control]:pl-1.5"
                    .to_owned()
            )
        );
        assert_eq!(
            HORIZONTAL.settle("[&_input]:pl-1").map(|(class, _)| class),
            Ok("**:data-[slot=input-group-control]:pl-1".to_owned())
        );
        assert_eq!(
            HORIZONTAL.settle("[&>kbd]:pl-1").map(|(class, _)| class),
            Ok("[&>kbd]:pl-1".to_owned()),
            "an element that is no part here is left for translate"
        );
    }

    #[test]
    fn the_desktop_breakpoints_always_hold() {
        let (class, held) = HORIZONTAL.settle("md:text-sm").expect("it holds");
        assert_eq!(class, "text-sm");
        assert_eq!(held[0].variant, "md");
        assert_eq!(
            HORIZONTAL.settle("sm:max-w-lg").map(|(class, _)| class),
            Ok("max-w-lg".to_owned())
        );
        assert_eq!(
            HORIZONTAL.settle("lg:max-w-lg").map(|(class, _)| class),
            Ok("lg:max-w-lg".to_owned()),
            "`lg:` is left for translate to report"
        );
    }

    #[test]
    fn a_state_the_widget_never_has_drops_the_class() {
        let never = HORIZONTAL
            .settle("group-data-[variant=line]/tabs-list:data-active:bg-transparent")
            .expect_err("it never holds");
        assert_eq!(never.why, "no line variant");
        assert!(
            HORIZONTAL
                .settle("not-group-data-horizontal/tabs:h-fit")
                .is_err(),
            "the negation of a state that holds never does"
        );
        assert_eq!(
            HORIZONTAL
                .settle("not-group-data-[variant=line]/tabs-list:bg-muted")
                .map(|(class, _)| class),
            Ok("bg-muted".to_owned())
        );
    }
}
