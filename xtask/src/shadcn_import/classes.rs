//! What each Tailwind class becomes in eggshell: a `Tw` call, a preset
//! token, or nothing yet, with the reason. The `Tw` side is read from
//! `src/tw/*.rs`, whose docs spell out the class each method stands for, so
//! this table cannot drift from `Tw`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// What a placeholder in a `Tw` doc pattern takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hole {
    /// A number: spacing units, a width, points.
    Number,
    /// A shadcn colour token, optionally `/alpha`.
    Color,
    /// Any value (`shadow-*`).
    Any,
}

/// A class pattern a `Tw` method documents: `px-<units>`, `flex`.
#[derive(Clone, Debug)]
struct Pattern {
    prefix: String,
    hole: Option<Hole>,
    suffix: String,
    method: String,
}

/// The classes and state variants `Tw` documents.
pub struct TwTable {
    utilities: Vec<Pattern>,
    /// `hover:` → `hover`.
    pub variants: BTreeMap<String, String>,
    /// The methods that take spacing units: `p`, `h`, `gap`.
    pub units: BTreeSet<String>,
}

fn pattern(doc: &str, method: &str) -> Option<Pattern> {
    let doc = doc.trim();
    match doc.find('<').or_else(|| doc.find('*')) {
        None => Some(Pattern {
            prefix: doc.to_owned(),
            hole: None,
            suffix: String::new(),
            method: method.to_owned(),
        }),
        Some(i) if doc[i..].starts_with('*') => Some(Pattern {
            prefix: doc[..i].to_owned(),
            hole: Some(Hole::Any),
            suffix: String::new(),
            method: method.to_owned(),
        }),
        Some(i) => {
            let close = i + doc[i..].find('>')?;
            let hole = match &doc[i + 1..close] {
                "color" => Hole::Color,
                _ => Hole::Number,
            };
            Some(Pattern {
                prefix: doc[..i].to_owned(),
                hole: Some(hole),
                suffix: doc[close + 1..].to_owned(),
                method: method.to_owned(),
            })
        }
    }
}

/// The first backticked span of `line`.
fn backticked(line: &str) -> Option<&str> {
    let start = line.find('`')? + 1;
    let end = start + line[start..].find('`')?;
    Some(&line[start..end])
}

impl TwTable {
    /// Reads the patterns of every `src/tw/*.rs` source.
    pub fn read(sources: &[String]) -> Self {
        let mut utilities = Vec::new();
        let mut variants = BTreeMap::new();
        let mut units = BTreeSet::new();
        for text in sources {
            let mut doc: Option<String> = None;
            for line in text.lines() {
                let t = line.trim();
                // A macro entry: `name, "`class`: ...", ...`, or a `spacing!`
                // entry, `name, name_px, "`class-<units>`: ...", ...`.
                if let Some((names, rest)) = t.split_once(", \"")
                    && names
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == ',' || c == ' ')
                    && let Some(class) = backticked(rest)
                {
                    let name = names.split_once(", ").map_or(names, |(name, _)| name);
                    if class.contains("<units>") {
                        units.insert(name.to_owned());
                    }
                    match names.split_once(", ") {
                        Some((name, px)) => {
                            utilities.extend(pattern(class, name));
                            if let Some(prefix) = class.strip_suffix("<units>") {
                                utilities.extend(pattern(&format!("{prefix}[<n>px]"), px));
                            }
                        }
                        None => utilities.extend(pattern(class, names)),
                    }
                    continue;
                }
                // A `lengths!` entry: five method names, then "the …".
                let words: Vec<&str> = t.splitn(6, ", ").collect();
                if words.len() == 6
                    && words[5].starts_with("\"the ")
                    && words[..5]
                        .iter()
                        .all(|w| w.chars().all(|c| c.is_alphanumeric() || c == '_'))
                {
                    units.insert(words[0].to_owned());
                    let class = words[0].replace('_', "-");
                    let spellings = [
                        (format!("{class}-<units>"), words[0]),
                        (format!("{class}-[<n>px]"), words[1]),
                        (format!("{class}-full"), words[3]),
                        (format!("{class}-auto"), words[4]),
                    ];
                    for (spelling, method) in spellings {
                        utilities.extend(pattern(&spelling, method));
                    }
                    continue;
                }
                if let Some(comment) = t.strip_prefix("///") {
                    if doc.is_none() {
                        doc = backticked(comment).map(str::to_owned);
                    }
                    continue;
                }
                if t.starts_with("#[") {
                    continue;
                }
                if let Some(rest) = t.strip_prefix("pub fn ")
                    && let Some(class) = doc.take()
                {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    match class.strip_suffix(':') {
                        Some(variant) => {
                            variants.insert(variant.to_owned(), name);
                        }
                        None => {
                            if class.contains("<units>") {
                                units.insert(name.clone());
                            }
                            utilities.extend(pattern(&class, &name));
                        }
                    }
                    continue;
                }
                doc = None;
            }
        }
        // Longer prefixes first, so `border-x-<color>` wins over `border-<color>`.
        utilities.sort_by_key(|p| std::cmp::Reverse(p.prefix.len()));
        Self {
            utilities,
            variants,
            units,
        }
    }

    pub fn len(&self) -> usize {
        self.utilities.len()
    }
}

/// Why a class has no eggshell equivalent yet; the report groups by this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gap {
    /// `has-[…]:`, `has-data-…:`: a style from what the node contains.
    Has,
    /// `[&_svg]:`, `[&>svg]:`: an arbitrary selector into the children.
    ArbitrarySelector,
    /// `*:`, `**:`: every child or descendant.
    Descendant,
    /// `aria-invalid:`, `aria-expanded:`: a state from an ARIA attribute.
    Aria,
    /// `data-open:`, `data-[size=sm]:`: a state from a data attribute.
    Data,
    /// `in-data-…:`, `in-[…]:`: a style from an ancestor.
    In,
    /// `group-…:`, `group/name`: a style from a named ancestor's state.
    Group,
    /// `not-…:`: a negated variant.
    Not,
    /// `sm:`, `md:`: a breakpoint.
    Responsive,
    /// `supports-…:`, `placeholder:`, `selection:`, `file:` and other
    /// variants eggshell does not have.
    OtherVariant,
    /// `rounded-[min(…)]`, `bg-[color-mix(…)]`: an arbitrary value.
    ArbitraryValue,
    /// A utility `Tw` has no method for.
    NoTwMethod,
}

impl Gap {
    pub fn label(self) -> &'static str {
        match self {
            Self::Has => "`has-*` (style from contents)",
            Self::ArbitrarySelector => "`[&…]` (arbitrary child selector)",
            Self::Descendant => "`*:` / `**:` (children, descendants)",
            Self::Aria => "`aria-*` (ARIA attribute state)",
            Self::Data => "`data-*` (data attribute state)",
            Self::In => "`in-*` (style from an ancestor)",
            Self::Group => "`group-*` (named ancestor state)",
            Self::Not => "`not-*` (negated variant)",
            Self::Responsive => "breakpoints (`sm:` …)",
            Self::OtherVariant => "other variants",
            Self::ArbitraryValue => "arbitrary values (`[…]`)",
            Self::NoTwMethod => "utilities with no `Tw` method",
        }
    }
}

/// A colour class's parts: `bg-primary/80` is the fill, `primary`, at 80%.
#[derive(Clone, Debug, PartialEq)]
pub struct Paint {
    /// `fill`, `text`, `border`, `ring` or `icon fill`.
    pub property: &'static str,
    /// The token, as shadcn names it (`primary-foreground`), or `white`,
    /// `black` or `transparent`.
    pub token: String,
    /// The `/alpha`, from 0 to 1.
    pub alpha: Option<f32>,
}

/// Where a class lands.
#[derive(Clone, Debug, PartialEq)]
pub enum Fate {
    /// A `Tw` call, under the state variants written before it.
    Tw { call: String },
    /// A preset value the class reads: a colour token, the text size, the
    /// theme radius. A colour carries its parts, for the colour comparison.
    Token { value: String, paint: Option<Paint> },
    /// A preset state (`dark:`, `disabled:`) that the preset's looks
    /// already split on, with what is inside it.
    Preset { state: String, inner: Box<Fate> },
    /// A `cn-*` class with no rule, or a `group/name`: names, not styles.
    Marker,
    /// Nothing yet.
    Gap { gap: Gap, why: String },
}

impl fmt::Display for Fate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tw { call } => write!(f, "{call}"),
            Self::Token { value, .. } => write!(f, "{value}"),
            Self::Preset { state, inner } => write!(f, "{state}: {inner}"),
            Self::Marker => write!(f, "(marker)"),
            Self::Gap { why, .. } => write!(f, "{why}"),
        }
    }
}

/// The colour tokens a sheet's rules read: the colour fields of the
/// `Tokens` struct in `source`, with `-` for `_`.
pub fn token_names(source: &str) -> BTreeSet<String> {
    let start = source.find("pub struct Tokens {").unwrap_or(0);
    let end = source[start..]
        .find("\n}\n")
        .map_or(source.len(), |i| start + i);
    source[start..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub "))
        .filter_map(|l| l.split_once(": Color32,"))
        .map(|(name, _)| name.replace('_', "-"))
        .collect()
}

/// Splits `hover:dark:bg-x` into its variants and utility, on colons
/// outside brackets.
pub fn split_variants(class: &str) -> (Vec<&str>, &str) {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in class.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ':' if depth == 0 => {
                parts.push(&class[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    (parts, &class[start..])
}

fn variant_gap(variant: &str) -> Gap {
    let v = variant.trim_start_matches("not-");
    if variant.starts_with("not-") {
        return Gap::Not;
    }
    if v.starts_with('[') {
        return Gap::ArbitrarySelector;
    }
    if v == "*" || v == "**" {
        return Gap::Descendant;
    }
    for (prefix, gap) in [
        ("has-", Gap::Has),
        ("aria-", Gap::Aria),
        ("data-", Gap::Data),
        ("in-", Gap::In),
        ("group-", Gap::Group),
    ] {
        if v.starts_with(prefix) {
            return gap;
        }
    }
    if matches!(v, "sm" | "md" | "lg" | "xl" | "2xl") {
        return Gap::Responsive;
    }
    Gap::OtherVariant
}

/// Rem or px to points, at 16 px to the rem.
fn length_points(value: &str) -> Option<f32> {
    if let Some(rem) = value.strip_suffix("rem") {
        return rem.parse::<f32>().ok().map(|r| r * 16.0);
    }
    value.strip_suffix("px")?.parse().ok()
}

/// The theme's radius scale, from `--radius` (shadcn's `globals.css`).
pub struct Radius {
    pub base: f32,
}

impl Radius {
    pub fn of(name: &str, base: f32) -> Option<f32> {
        Some(match name {
            "none" => 0.0,
            "sm" => base - 4.0,
            "md" => base - 2.0,
            "lg" => base,
            "xl" => base + 4.0,
            "2xl" => base + 8.0,
            "3xl" => base + 12.0,
            "4xl" => base + 16.0,
            _ => return None,
        })
    }

    /// `rounded-md`, `rounded-[min(var(--radius-md),8px)]`, `rounded-full`.
    pub fn rounded(&self, utility: &str) -> Option<f32> {
        let (step, cap) = radius_step(utility)?;
        let points = Self::of(step, self.base)?;
        Some(cap.map_or(points, |cap| points.min(cap)))
    }
}

/// The step of the radius scale a `rounded-*` class names, and the cap a
/// `min()` puts on it.
fn radius_step(utility: &str) -> Option<(&str, Option<f32>)> {
    let value = utility.strip_prefix("rounded-")?;
    if Radius::of(value, 0.0).is_some() {
        return Some((value, None));
    }
    let inner = value.strip_prefix("[min(")?.strip_suffix(")]")?;
    let (var, cap) = inner.split_once(',')?;
    let name = var.strip_prefix("var(--radius-")?.strip_suffix(')')?;
    Radius::of(name, 0.0)?;
    Some((name, Some(length_points(cap)?)))
}

/// The points of a `rounded-*` class as read from the tokens `k`:
/// `k.radius.md()`, with a `min()`'s cap kept as written.
pub fn rounded_expr(utility: &str) -> Option<String> {
    let (step, cap) = radius_step(utility)?;
    let method = match step {
        "none" => return Some("0.0".to_owned()),
        "2xl" => "xl2",
        "3xl" => "xl3",
        "4xl" => "xl4",
        step => step,
    };
    let read = format!("k.radius.{method}()");
    Some(match cap {
        Some(cap) => format!("{read}.min({cap:?})"),
        None => read,
    })
}

/// The step of Tailwind's text scale a `text-*` class names.
pub fn text_step(utility: &str) -> Option<&str> {
    let step = utility.strip_prefix("text-")?;
    matches!(step, "xs" | "sm" | "base" | "lg").then_some(step)
}

/// The text sizes of Tailwind's scale, size and line height in points.
pub fn text_size(utility: &str) -> Option<(f32, f32)> {
    Some(match text_step(utility)? {
        "xs" => (12.0, 16.0),
        "sm" => (14.0, 20.0),
        "base" => (16.0, 24.0),
        _ => (18.0, 28.0),
    })
}

pub struct Classifier<'a> {
    pub tw: &'a TwTable,
    pub tokens: &'a BTreeSet<String>,
    pub radius: Radius,
}

/// Preset states: the looks already split on these.
const PRESET_STATES: [&str; 2] = ["dark", "disabled"];

impl Classifier<'_> {
    pub fn classify(&self, class: &str) -> Fate {
        let important = class.trim_end_matches('!');
        let (variants, utility) = split_variants(important);
        // A variant eggshell lacks is the gap, whatever the utility is.
        if let Some(variant) = variants
            .iter()
            .find(|v| !self.tw.variants.contains_key(**v) && !PRESET_STATES.contains(v))
        {
            return Fate::Gap {
                gap: variant_gap(variant),
                why: format!("variant `{variant}:`"),
            };
        }
        let mut fate = self.utility(utility);
        for variant in variants.iter().rev() {
            if matches!(fate, Fate::Gap { .. }) {
                break;
            }
            fate = if let Some(method) = self.tw.variants.get(*variant) {
                match fate {
                    Fate::Tw { call } => Fate::Tw {
                        call: format!(".{method}(|p| p{call})"),
                    },
                    inner => Fate::Preset {
                        state: (*variant).to_owned(),
                        inner: Box::new(inner),
                    },
                }
            } else if PRESET_STATES.contains(variant) {
                Fate::Preset {
                    state: (*variant).to_owned(),
                    inner: Box::new(fate),
                }
            } else {
                Fate::Gap {
                    gap: variant_gap(variant),
                    why: format!("variant `{variant}:`"),
                }
            };
        }
        fate
    }

    /// A colour value's token and alpha, if it names a token.
    fn color(&self, value: &str) -> Option<(String, Option<f32>)> {
        let (name, alpha) = match value.split_once('/') {
            Some((name, alpha)) => (name, Some(alpha)),
            None => (value, None),
        };
        if !(self.tokens.contains(name) || matches!(name, "transparent" | "white" | "black")) {
            return None;
        }
        let alpha = match alpha {
            Some(a) => Some(a.parse::<f32>().ok()? / 100.0),
            None => None,
        };
        Some((name.to_owned(), alpha))
    }

    pub(super) fn utility(&self, utility: &str) -> Fate {
        if utility.starts_with("cn-") || utility.starts_with("group/") {
            return Fate::Marker;
        }
        if let Some(points) = self.radius.rounded(utility) {
            return Fate::Token {
                value: format!("radius {points} (theme)"),
                paint: None,
            };
        }
        if let Some((size, line)) = text_size(utility) {
            return Fate::Token {
                value: format!("text {size}/{line}"),
                paint: None,
            };
        }
        for (prefix, what) in [
            ("bg-", "fill"),
            ("text-", "text"),
            ("border-", "border"),
            ("ring-", "ring"),
            ("fill-", "icon fill"),
        ] {
            if let Some(value) = utility.strip_prefix(prefix)
                && let Some((token, alpha)) = self.color(value)
            {
                let field = format!("t.{}", token.replace('-', "_"));
                let value = match alpha {
                    Some(a) => format!("{what} {field}.gamma_multiply({a})"),
                    None => format!("{what} {field}"),
                };
                return Fate::Token {
                    value,
                    paint: Some(Paint {
                        property: what,
                        token,
                        alpha,
                    }),
                };
            }
        }
        for p in &self.tw.utilities {
            let Some(rest) = utility.strip_prefix(p.prefix.as_str()) else {
                continue;
            };
            let Some(value) = rest.strip_suffix(p.suffix.as_str()) else {
                continue;
            };
            match p.hole {
                None if value.is_empty() => {
                    return Fate::Tw {
                        call: format!(".{}()", p.method),
                    };
                }
                Some(Hole::Number) => {
                    let number = match value {
                        "px" => Some(0.25),
                        v => v.parse::<f32>().ok(),
                    };
                    if let Some(n) = number {
                        return Fate::Tw {
                            call: format!(".{}({n})", p.method),
                        };
                    }
                }
                Some(Hole::Any) if !value.is_empty() && !value.starts_with('[') => {
                    return Fate::Tw {
                        call: format!(".{}(/* {value} */)", p.method),
                    };
                }
                _ => {}
            }
        }
        if utility.contains('[') {
            return Fate::Gap {
                gap: Gap::ArbitraryValue,
                why: format!("arbitrary value `{utility}`"),
            };
        }
        Fate::Gap {
            gap: Gap::NoTwMethod,
            why: format!("no `Tw` method for `{utility}`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_names_are_the_colour_fields_of_tokens_only() {
        let source = "pub struct Paint {\n    pub fill: Color32,\n}\n\
                      pub struct Tokens {\n    pub primary_foreground: Color32,\n    \
                      pub radius: u8,\n}\n";
        let names: Vec<String> = token_names(source).into_iter().collect();
        assert_eq!(names, ["primary-foreground"]);
    }

    fn table() -> TwTable {
        TwTable::read(&[r#"
    spacing! {
        px, px_px, "`px-<units>`: left and right padding.", |s, v| {};
    }
    keywords! {
        flex, "`flex`: a flex container.", |s| {};
    }
    lengths! {
        h, h_px, h_frac, h_full, h_auto, "the height", |s, l| {};
    }
    /// `hover:`. The pointer is over the node.
    pub fn hover(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `focus-visible:`. Keyboard focus.
    #[must_use]
    pub fn focus(self, style: impl FnOnce(Paint) -> Paint) -> Self {
    /// `shadow-*`.
    pub fn shadow(self, shadow: Shadow) -> Self {
"#
        .to_owned()])
    }

    #[test]
    fn classes_map_to_tw_calls_tokens_presets_or_named_gaps() {
        let tw = table();
        let tokens: BTreeSet<String> = ["primary", "muted"].map(str::to_owned).into();
        let c = Classifier {
            tw: &tw,
            tokens: &tokens,
            radius: Radius { base: 10.0 },
        };
        assert_eq!(
            c.classify("px-2.5"),
            Fate::Tw {
                call: ".px(2.5)".into()
            }
        );
        assert_eq!(
            c.classify("h-9"),
            Fate::Tw {
                call: ".h(9)".into()
            }
        );
        assert_eq!(
            c.classify("flex"),
            Fate::Tw {
                call: ".flex()".into()
            }
        );
        assert_eq!(
            c.classify("hover:px-1"),
            Fate::Tw {
                call: ".hover(|p| p.px(1))".into()
            }
        );
        assert_eq!(
            c.classify("bg-primary/80"),
            Fate::Token {
                value: "fill t.primary.gamma_multiply(0.8)".into(),
                paint: Some(Paint {
                    property: "fill",
                    token: "primary".into(),
                    alpha: Some(0.8)
                })
            }
        );
        assert_eq!(
            c.classify("rounded-[min(var(--radius-md),8px)]"),
            Fate::Token {
                value: "radius 8 (theme)".into(),
                paint: None
            }
        );
        assert!(
            matches!(c.classify("dark:bg-muted"), Fate::Preset { state, .. } if state == "dark")
        );
        assert!(matches!(
            c.classify("aria-invalid:px-1"),
            Fate::Gap { gap: Gap::Aria, .. }
        ));
        assert!(matches!(
            c.classify("[&_svg:not([class*='size-'])]:size-4"),
            Fate::Gap {
                gap: Gap::ArbitrarySelector,
                ..
            }
        ));
        assert!(matches!(
            c.classify("has-data-[icon=inline-end]:pr-2"),
            Fate::Gap { gap: Gap::Has, .. }
        ));
        assert!(matches!(
            c.classify("whitespace-nowrap"),
            Fate::Gap {
                gap: Gap::NoTwMethod,
                ..
            }
        ));
        assert!(matches!(c.classify("cn-button"), Fate::Marker));
    }
}
