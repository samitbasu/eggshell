//! shadcn's values against the preset's, from `examples/preset_dump.rs`:
//! the button's sizes, the badge's metrics, the combobox's parts, and the
//! colours of every button and badge variant at rest and under the pointer,
//! in both themes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::Row;
use super::classes::{Fate, Paint, Radius, split_variants, text_size};
use super::source::Origin;

/// The dump's lines: a kind (`token`, `button`, `look` …) and its fields.
pub struct Dump(Vec<(String, BTreeMap<String, String>)>);

impl Dump {
    pub fn parse(output: &str) -> Self {
        Self(
            output
                .lines()
                .filter_map(|line| {
                    let mut words = line.split_whitespace();
                    let kind = words.next()?.to_owned();
                    let fields = words
                        .filter_map(|kv| kv.split_once('='))
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect();
                    Some((kind, fields))
                })
                .collect(),
        )
    }

    /// The lines of `kind` whose fields include every one of `with`.
    fn find(&self, kind: &str, with: &[(&str, &str)]) -> Vec<&BTreeMap<String, String>> {
        self.0
            .iter()
            .filter(|(k, fields)| {
                k == kind
                    && with
                        .iter()
                        .all(|(key, value)| fields.get(*key).is_some_and(|v| v == value))
            })
            .map(|(_, fields)| fields)
            .collect()
    }

    fn first(&self, kind: &str, with: &[(&str, &str)]) -> Option<&BTreeMap<String, String>> {
        self.find(kind, with).into_iter().next()
    }
}

/// `#RR_GG_BB_AA`, premultiplied, as egui prints a colour.
type Rgba = [u8; 4];

fn hex(value: &str) -> Option<Rgba> {
    let digits: String = value.trim_start_matches('#').replace('_', "");
    let n = u32::from_str_radix(&digits, 16).ok()?;
    Some(n.to_be_bytes())
}

fn show(c: Option<Rgba>) -> String {
    c.map_or_else(
        || "?".to_owned(),
        |[r, g, b, a]| format!("`#{r:02x}{g:02x}{b:02x}{a:02x}`"),
    )
}

fn close(a: Rgba, b: Rgba) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2)
}

/// A whole-number value of a dump field.
fn number(fields: &BTreeMap<String, String>, key: &str) -> Option<f32> {
    fields.get(key)?.parse().ok()
}

/// A size's or part's values, as shadcn's classes or the preset give them.
#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub height: Option<f32>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
    pub padding_left: Option<f32>,
    pub padding_right: Option<f32>,
    pub padding: Option<f32>,
    pub gap: Option<f32>,
    pub radius: Option<f32>,
    pub glyph: Option<f32>,
    pub text: Option<String>,
}

/// Points from a spacing class value: `2.5` is 10 points.
fn units(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().map(|u| u * 4.0)
}

/// The metrics `classes` set, read straight from the class text; an
/// `[&_svg]:size-n` or `[&>svg]:size-n!` gives the icon size.
pub fn metrics_of<'a>(classes: impl Iterator<Item = &'a str>, radius: &Radius) -> Metrics {
    let mut m = Metrics::default();
    for class in classes {
        let class = class.trim_end_matches('!');
        if class.starts_with("[&_svg") || class.starts_with("[&>svg") {
            if let Some(v) = class.rsplit_once(":size-").and_then(|(_, v)| units(v)) {
                m.glyph = Some(v);
            }
            continue;
        }
        if class.contains(':') {
            continue;
        }
        let spacing = |prefix: &str| class.strip_prefix(prefix).and_then(units);
        if let Some(v) = spacing("h-").or_else(|| spacing("size-")) {
            m.height = Some(v);
        } else if let Some(v) = spacing("px-") {
            m.padding_x = Some(v);
        } else if let Some(v) = spacing("py-") {
            m.padding_y = Some(v);
        } else if let Some(v) = spacing("pl-") {
            m.padding_left = Some(v);
        } else if let Some(v) = spacing("pr-") {
            m.padding_right = Some(v);
        } else if let Some(v) = spacing("p-") {
            m.padding = Some(v);
        } else if let Some(v) = spacing("gap-") {
            m.gap = Some(v);
        } else if let Some(v) = radius.rounded(class) {
            m.radius = Some(v);
        } else if let Some((size, line)) = text_size(class) {
            m.text = Some(format!("{size}/{line}"));
        }
    }
    m
}

fn cell(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| format!("{v}"))
}

/// `ours`, bold where it differs from `theirs`.
fn marked(ours: Option<f32>, theirs: Option<f32>) -> String {
    match (ours, theirs) {
        (Some(o), Some(t)) if (o - t).abs() > 0.01 => format!("**{o}**"),
        (o, _) => cell(o),
    }
}

/// The preset's style name for shadcn's `style`: `vega` is `Vega`.
fn preset_style(style: &str) -> String {
    let mut chars = style.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

/// The classes of `component`'s base and, if any, one variant value.
fn classes_of<'r>(
    rows: &'r [Row],
    component: &'r str,
    variant: Option<(&'r str, &'r str)>,
) -> impl Iterator<Item = &'r Row> + 'r {
    rows.iter().filter(move |row| {
        row.component == component
            && match (&row.origin, variant) {
                (Origin::Base { .. }, _) => true,
                (Origin::Variant { variant, name, .. }, Some((v, n))) => variant == v && name == n,
                _ => false,
            }
    })
}

/// The variant values of `component`'s `variant` table, in source order.
fn variant_names(rows: &[Row], component: &str, variant: &str) -> Vec<String> {
    let mut names = Vec::new();
    for row in rows.iter().filter(|row| row.component == component) {
        if let Origin::Variant {
            variant: v, name, ..
        } = &row.origin
            && v == variant
            && !names.contains(name)
        {
            names.push(name.clone());
        }
    }
    names
}

/// The button's sizes, shadcn's against the preset's.
pub fn button_sizes(r: &mut String, style: &str, rows: &[Row], dump: &Dump, radius: &Radius) {
    let base = metrics_of(
        classes_of(rows, "button", None).map(|row| row.class.text.as_str()),
        radius,
    );
    let preset = preset_style(style);
    let _ = writeln!(
        r,
        "| size | | height | padding x | gap | radius | icon | text |"
    );
    let _ = writeln!(r, "|---|---|---|---|---|---|---|---|");
    for size in variant_names(rows, "button", "size") {
        let own = metrics_of(
            classes_of(rows, "button", Some(("size", size.as_str())))
                .filter(|row| matches!(row.origin, Origin::Variant { .. }))
                .map(|row| row.class.text.as_str()),
            radius,
        );
        let shadcn = Metrics {
            height: own.height.or(base.height),
            padding_x: own.padding_x.or(base.padding_x).or(Some(0.0)),
            gap: own.gap.or(base.gap),
            radius: own.radius.or(base.radius),
            glyph: own.glyph.or(base.glyph),
            text: own.text.or(base.text.clone()),
            ..Metrics::default()
        };
        let _ = writeln!(
            r,
            "| `{size}` | shadcn | {} | {} | {} | {} | {} | {} |",
            cell(shadcn.height),
            cell(shadcn.padding_x),
            cell(shadcn.gap),
            cell(shadcn.radius),
            cell(shadcn.glyph),
            shadcn.text.as_deref().unwrap_or("-")
        );
        match dump.first(
            "button",
            &[("style", &preset), ("theme", "Light"), ("size", &size)],
        ) {
            Some(p) => {
                let _ = writeln!(
                    r,
                    "| | preset | {} | {} | {} | {} | {} | {} |",
                    marked(number(p, "height"), shadcn.height),
                    marked(number(p, "padding_x"), shadcn.padding_x),
                    marked(number(p, "gap"), shadcn.gap),
                    marked(number(p, "radius"), shadcn.radius),
                    marked(number(p, "glyph"), shadcn.glyph),
                    p.get("text").map_or("-", String::as_str)
                );
            }
            None => {
                let _ = writeln!(r, "| | preset | *no such size* | | | | | |");
            }
        }
    }
    let _ = writeln!(
        r,
        "\nBold: the preset differs. `role` text is the style's text role.\n"
    );
}

/// The badge's metrics, shadcn's against the preset's. A radius at least
/// half the height is a pill, which the preset writes as 255.
pub fn badge_metrics(r: &mut String, style: &str, rows: &[Row], dump: &Dump, radius: &Radius) {
    let shadcn = metrics_of(
        classes_of(rows, "badge", None).map(|row| row.class.text.as_str()),
        radius,
    );
    let preset = preset_style(style);
    let Some(p) = dump.first("badge", &[("style", &preset), ("theme", "Light")]) else {
        let _ = writeln!(r, "The preset dump has no badge for {preset}.\n");
        return;
    };
    let pill = |radius: Option<f32>, height: Option<f32>| match (radius, height) {
        (Some(r), Some(h)) if r >= h / 2.0 => Some(255.0),
        (r, _) => r,
    };
    let _ = writeln!(r, "| | height | padding x | gap | radius | icon | text |");
    let _ = writeln!(r, "|---|---|---|---|---|---|---|");
    let shadcn_radius = pill(shadcn.radius, shadcn.height);
    let _ = writeln!(
        r,
        "| shadcn | {} | {} | {} | {} | {} | {} |",
        cell(shadcn.height),
        cell(shadcn.padding_x),
        cell(shadcn.gap),
        shadcn
            .radius
            .map_or("-".to_owned(), |v| match shadcn_radius {
                Some(255.0) => format!("{v} (a pill)"),
                _ => format!("{v}"),
            }),
        cell(shadcn.glyph),
        shadcn.text.as_deref().unwrap_or("-")
    );
    let _ = writeln!(
        r,
        "| preset | {} | {} | {} | {} | {} | {} |\n",
        marked(number(p, "height"), shadcn.height),
        marked(number(p, "padding_x"), shadcn.padding_x),
        marked(number(p, "gap"), shadcn.gap),
        marked(number(p, "radius"), shadcn_radius),
        marked(number(p, "glyph"), shadcn.glyph),
        p.get("text").map_or("-", String::as_str)
    );
}

/// The combobox's parts against the preset's analogues: the input group
/// against the input, the popup against the menu's box, an item against a
/// menu item.
pub fn combobox_parts(
    r: &mut String,
    style: &str,
    rules: &super::source::Rules,
    dump: &Dump,
    radius: &Radius,
) {
    let preset = preset_style(style);
    let of = |rule: &str| {
        metrics_of(
            rules
                .get(rule)
                .into_iter()
                .flatten()
                .map(|c| c.text.as_str()),
            radius,
        )
    };
    let part = |name: &str| {
        dump.first(
            "part",
            &[("style", &preset), ("theme", "Light"), ("part", name)],
        )
    };
    let _ = writeln!(r, "| part | shadcn (rule) | shadcn | preset |");
    let _ = writeln!(r, "|---|---|---|---|");
    let input = of("cn-input-group");
    if let Some(p) = part("input") {
        let _ = writeln!(
            r,
            "| input height | `cn-input-group` | {} | {} |",
            cell(input.height),
            marked(number(p, "height"), input.height)
        );
        let _ = writeln!(
            r,
            "| input radius | `cn-input-group` | {} | {} |",
            cell(input.radius),
            marked(number(p, "radius"), input.radius)
        );
    }
    let content = of("cn-combobox-content");
    let list = of("cn-combobox-list");
    if let Some(p) = part("content") {
        let _ = writeln!(
            r,
            "| popup radius | `cn-combobox-content` | {} | {} |",
            cell(content.radius),
            marked(number(p, "radius"), content.radius)
        );
        let _ = writeln!(
            r,
            "| popup padding | `cn-combobox-list` | {} | {} |",
            cell(list.padding),
            marked(number(p, "padding"), list.padding)
        );
    }
    let item = of("cn-combobox-item");
    if let Some(p) = part("item") {
        for (label, ours, theirs) in [
            ("item padding left", "padding_x", item.padding_left),
            ("item padding y", "padding_y", item.padding_y),
            ("item radius", "radius", item.radius),
            ("item gap", "gap", item.gap),
        ] {
            let _ = writeln!(
                r,
                "| {label} | `cn-combobox-item` | {} | {} |",
                cell(theirs),
                marked(number(p, ours), theirs)
            );
        }
        let _ = writeln!(
            r,
            "| item padding right | `cn-combobox-item` | {} (room for the check) | {} (one padding) |",
            cell(item.padding_right),
            p.get("padding_x").map_or("-", String::as_str)
        );
    }
    let _ = writeln!(r);
}

/// The colour classes of `fate`, with the states written before it.
fn painted(fate: &Fate) -> Option<(Vec<&str>, &Paint)> {
    let mut states = Vec::new();
    let mut fate = fate;
    loop {
        match fate {
            Fate::Preset { state, inner } => {
                states.push(state.as_str());
                fate = inner;
            }
            Fate::Token {
                paint: Some(paint), ..
            } => return Some((states, paint)),
            _ => return None,
        }
    }
}

/// The class that sets `property` in `theme` and `state`, of `classes`: the
/// most specific that applies, the later on a tie, as CSS orders them.
fn winner<'r>(
    classes: impl Iterator<Item = &'r Row>,
    property: &str,
    theme: &str,
    state: &str,
) -> Option<(&'r Row, &'r Paint)> {
    let mut best: Option<(usize, &Row, &Paint)> = None;
    for row in classes {
        let Some((states, paint)) = painted(&row.fate) else {
            continue;
        };
        if paint.property != property {
            continue;
        }
        let applies = states.iter().all(|s| match *s {
            "dark" => theme == "Dark",
            "hover" => state == "hover",
            _ => false,
        });
        if applies && best.is_none_or(|(score, ..)| states.len() >= score) {
            best = Some((states.len(), row, paint));
        }
    }
    best.map(|(_, row, paint)| (row, paint))
}

/// A class `classes` hold that would set `property` in `theme` and `state`,
/// more specifically than `than` does, but whose value is not a token
/// (`hover:bg-[color-mix(…)]`): the colour is then not known.
fn unresolved<'r>(
    classes: impl Iterator<Item = &'r Row>,
    property: &str,
    theme: &str,
    state: &str,
    than: usize,
) -> Option<&'r Row> {
    let prefix = match property {
        "fill" => "bg-",
        "text" => "text-",
        _ => "border-",
    };
    classes
        .filter(|row| matches!(row.fate, Fate::Gap { .. }))
        .find(|row| {
            let (variants, utility) = split_variants(&row.class.text);
            utility.starts_with(prefix)
                && utility.contains('[')
                && variants.len() > than
                && variants.iter().all(|v| match *v {
                    "dark" => theme == "Dark",
                    "hover" => state == "hover",
                    _ => false,
                })
        })
}

/// A paint's colour in `tokens`: the token times its alpha, premultiplied.
fn resolve(paint: &Paint, tokens: &BTreeMap<String, Rgba>) -> Option<Rgba> {
    let base = match paint.token.as_str() {
        "white" => [255, 255, 255, 255],
        "black" => [0, 0, 0, 255],
        "transparent" => [0, 0, 0, 0],
        name => *tokens.get(&name.replace('-', "_"))?,
    };
    let alpha = paint.alpha.unwrap_or(1.0);
    Some(base.map(|c| (f32::from(c) * alpha).round() as u8))
}

/// The preset's name for a shadcn variant: shadcn's `default` is the
/// preset's `primary`.
fn preset_variant(name: &str) -> &str {
    match name {
        "default" => "primary",
        other => other,
    }
}

/// Every button and badge variant's colours, shadcn's against the preset's.
pub fn colours(r: &mut String, style: &str, rows: &[Row], dump: &Dump) {
    let preset = preset_style(style);
    let mut agree = 0;
    let mut table = String::new();
    let mut unknowns = String::new();
    let mut missing = Vec::new();
    for widget in ["button", "badge"] {
        for variant in variant_names(rows, widget, "variant") {
            let theirs = preset_variant(&variant);
            for theme in ["Light", "Dark"] {
                let tokens: BTreeMap<String, Rgba> = dump
                    .find("token", &[("style", &preset), ("theme", theme)])
                    .into_iter()
                    .filter_map(|f| Some((f.get("name")?.clone(), hex(f.get("value")?)?)))
                    .collect();
                for state in ["rest", "hover"] {
                    let Some(look) = dump.first(
                        "look",
                        &[
                            ("widget", widget),
                            ("style", &preset),
                            ("theme", theme),
                            ("variant", theirs),
                            ("state", state),
                        ],
                    ) else {
                        if theme == "Light" && state == "rest" {
                            missing.push(format!("{widget} `{variant}`"));
                        }
                        continue;
                    };
                    for property in ["fill", "text", "border"] {
                        let classes = classes_of(rows, widget, Some(("variant", variant.as_str())));
                        let won = winner(classes, property, theme, state);
                        let specificity = won.map_or(0, |(row, _)| {
                            painted(&row.fate).map_or(0, |(states, _)| states.len())
                        });
                        let unknown = unresolved(
                            classes_of(rows, widget, Some(("variant", variant.as_str()))),
                            property,
                            theme,
                            state,
                            specificity,
                        );
                        if let Some(row) = unknown {
                            let ours = look.get(property).and_then(|v| hex(v));
                            let _ = writeln!(
                                unknowns,
                                "| {widget} | `{variant}` | {theme} | {state} | {property} | `{}` | unresolved | {} |",
                                row.class.text,
                                show(ours)
                            );
                            continue;
                        }
                        let (class, shadcn) = match won {
                            Some((row, paint)) => {
                                (format!("`{}`", row.class.text), resolve(paint, &tokens))
                            }
                            None if property == "text" => (
                                "(inherits `foreground`)".to_owned(),
                                tokens.get("foreground").copied(),
                            ),
                            None => ("(none: transparent)".to_owned(), Some([0, 0, 0, 0])),
                        };
                        let ours = look.get(property).and_then(|v| hex(v));
                        match (shadcn, ours) {
                            (Some(s), Some(o)) if close(s, o) => agree += 1,
                            _ => {
                                let _ = writeln!(
                                    table,
                                    "| {widget} | `{variant}` | {theme} | {state} | {property} | {class} | {} | {} |",
                                    show(shadcn),
                                    show(ours)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let disagree = table.lines().count();
    let _ = writeln!(
        r,
        "{agree} of {} compared colours agree (to 2 levels a channel, premultiplied).\n",
        agree + disagree
    );
    if !missing.is_empty() {
        let _ = writeln!(r, "Variants the preset lacks: {}.\n", missing.join(", "));
    }
    if !unknowns.is_empty() {
        let _ = writeln!(
            r,
            "Not compared: shadcn writes these as arbitrary values, which the importer does not resolve.\n"
        );
        let _ = writeln!(
            r,
            "| widget | variant | theme | state | property | shadcn's class | shadcn | preset |"
        );
        let _ = writeln!(r, "|---|---|---|---|---|---|---|---|");
        r.push_str(&unknowns);
        let _ = writeln!(r);
    }
    if disagree > 0 {
        let _ = writeln!(
            r,
            "| widget | variant | theme | state | property | shadcn's class | shadcn | preset |"
        );
        let _ = writeln!(r, "|---|---|---|---|---|---|---|---|");
        r.push_str(&table);
        let _ = writeln!(
            r,
            "\nA badge's hover in shadcn is `[a]:hover:`, a badge that is a link; a plain badge has none. The preset's badge hover is the chip's, so its hover rows differ by design."
        );
    }
    let _ = writeln!(r);
}
