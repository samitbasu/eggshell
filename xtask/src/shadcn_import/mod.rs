//! `cargo xtask shadcn-import`: a spike. Reads shadcn's Button, Badge and
//! Combobox (the registry `.tsx` and a style's CSS), maps every Tailwind class
//! onto eggshell, and writes a report: how much maps, what does not (each
//! with its source span), and the generated Button and Badge values against
//! today's preset. It changes nothing in the preset.

mod classes;
mod compare;
mod emit;
mod source;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan, MietteDiagnostic, NamedSource};

use classes::{Classifier, Fate, Gap, Radius, TwTable};
use source::{Class, Origin, Sources};

/// The components of the spike, by their registry file name.
const COMPONENTS: [&str; 3] = ["button", "badge", "combobox"];

pub struct Plan {
    /// The shadcn checkout.
    pub reference: PathBuf,
    /// The style, as `style-<name>.css`.
    pub style: String,
    /// Where the report goes.
    pub out: PathBuf,
}

/// One class of one component, as the style expands it.
struct Row {
    component: &'static str,
    origin: Origin,
    /// The `cn-*` class it came from, if it came through the CSS.
    via: Option<String>,
    class: Class,
    fate: Fate,
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// The theme's `--radius` in points, from `registry/themes.ts`.
fn theme_radius(themes: &str) -> Option<f32> {
    let at = themes.find("radius: \"")? + "radius: \"".len();
    let value = &themes[at..at + themes[at..].find('"')?];
    value
        .strip_suffix("rem")?
        .parse::<f32>()
        .ok()
        .map(|r| r * 16.0)
}

/// The styles the preset has, as shadcn names their CSS.
const STYLES: [&str; 2] = ["vega", "lyra"];

/// Every class of the components in `style`'s CSS, with its fate, and the
/// style's `@apply` rules.
fn extract(
    registry: &Path,
    style: &str,
    classifier: &Classifier<'_>,
    sources: &mut Sources,
) -> Result<(Vec<Row>, source::Rules)> {
    let css = read(&registry.join(format!("styles/style-{style}.css")))?;
    let css_file = sources.add(format!("registry/styles/style-{style}.css"), css.clone());
    let rules = source::apply_rules(&css, css_file);
    let mut rows = Vec::new();
    for name in COMPONENTS {
        let text = read(&registry.join(format!("bases/radix/ui/{name}.tsx")))?;
        let file = sources.add(format!("registry/bases/radix/ui/{name}.tsx"), text.clone());
        for (origin, classes) in source::component(&text, file).parts {
            for class in classes {
                match rules.get(&class.text) {
                    Some(applied) => rows.extend(applied.iter().map(|inner| Row {
                        component: name,
                        origin: origin.clone(),
                        via: Some(class.text.clone()),
                        fate: classifier.classify(&inner.text),
                        class: inner.clone(),
                    })),
                    None => rows.push(Row {
                        component: name,
                        origin: origin.clone(),
                        via: None,
                        fate: classifier.classify(&class.text),
                        class,
                    }),
                }
            }
        }
    }
    Ok((rows, rules))
}

pub fn run(plan: &Plan) -> Result<()> {
    let root = crate::process::workspace_root();
    let registry = root.join(&plan.reference).join("apps/v4/registry");
    let tw_sources = fs::read_dir(root.join("src/tw"))?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| read(&p))
        .collect::<Result<Vec<_>>>()?;
    let tw = TwTable::read(&tw_sources);
    let tokens = classes::token_names(&read(
        &root.join("crates/eggshell-ui-shadcn/src/tokens.rs"),
    )?);
    let radius = theme_radius(&read(&registry.join("themes.ts"))?)
        .context("no `radius` in registry/themes.ts")?;
    let classifier = Classifier {
        tw: &tw,
        tokens: &tokens,
        radius: Radius { base: radius },
    };
    let mut sources = Sources::default();
    let (rows, _) = extract(&registry, &plan.style, &classifier, &mut sources)?;
    let diagnostics = diagnostics(&rows, &sources);
    eprint!("{diagnostics}");
    let dump = compare::Dump::parse(&preset_dump(&root)?);
    let mut report = report(plan, &rows, &tw, radius);
    for style in STYLES {
        let mut scratch = Sources::default();
        let (rows, rules) = extract(&registry, style, &classifier, &mut scratch)?;
        let radius = Radius { base: radius };
        let _ = writeln!(
            report,
            "## 3. `{style}`: shadcn against today's preset
"
        );
        let _ = writeln!(
            report,
            "### Button sizes
"
        );
        compare::button_sizes(&mut report, style, &rows, &dump, &radius);
        let _ = writeln!(
            report,
            "### Badge
"
        );
        compare::badge_metrics(&mut report, style, &rows, &dump, &radius);
        let _ = writeln!(
            report,
            "### Combobox parts
"
        );
        compare::combobox_parts(&mut report, style, &rules, &dump, &radius);
        let _ = writeln!(
            report,
            "### Colours, button and badge variants
"
        );
        compare::colours(&mut report, style, &rows, &dump);
        let emitted = emit::emit(&classifier, &rows, &scratch, "button", "ButtonPart", style);
        let sheets = root.join(&plan.out).join("sheets").join(style);
        fs::create_dir_all(&sheets)?;
        fs::write(sheets.join("button.rs"), &emitted.rust)?;
        sheet_section(&mut report, style, &emitted);
    }
    report.push_str(&listings(&rows));
    let out = root.join(&plan.out);
    fs::create_dir_all(&out)?;
    let report_path = out.join("report.md");
    fs::write(&report_path, &report)?;
    fs::write(out.join("diagnostics.txt"), &diagnostics)?;
    println!("{}", summary_line(&rows));
    println!("report: {}", report_path.display());
    Ok(())
}

/// The report's account of a generated sheet: how each class got there,
/// what is not there yet, and what `Tw` lacks for it.
fn sheet_section(report: &mut String, style: &str, emitted: &emit::Emitted) {
    let _ = writeln!(
        report,
        "### The `Sheet` rules for Button (`sheets/{style}/button.rs`)\n"
    );
    let _ = writeln!(report, "| mapping kind | classes |\n|---|---|");
    for (kind, count) in &emitted.kinds {
        let _ = writeln!(report, "| {} | {count} |", kind.label());
    }
    let _ = writeln!(report, "\nUnmapped, once each:\n");
    for (source, why) in &emitted.unmapped {
        let _ = writeln!(report, "- `{source}`: {why}");
    }
    if !emitted.needs.is_empty() {
        let needs: Vec<_> = emitted.needs.iter().map(|n| format!("`Tw::{n}`")).collect();
        let _ = writeln!(report, "\n`Tw` lacks: {}.", needs.join(", "));
    }
    let _ = writeln!(report);
}

fn is_clean(fate: &Fate) -> bool {
    !matches!(fate, Fate::Gap { .. } | Fate::Marker)
}

fn summary_line(rows: &[Row]) -> String {
    let counted: Vec<_> = rows.iter().filter(|r| r.fate != Fate::Marker).collect();
    let clean = counted.iter().filter(|r| is_clean(&r.fate)).count();
    format!(
        "{clean} of {} classes map ({}%); {} have no equivalent yet",
        counted.len(),
        clean * 100 / counted.len().max(1),
        counted.len() - clean
    )
}

/// One miette diagnostic per distinct unsupported class, at its first
/// occurrence, rendered without colour.
fn diagnostics(rows: &[Row], sources: &Sources) -> String {
    let mut first: BTreeMap<&str, (&Row, usize)> = BTreeMap::new();
    for row in rows {
        if let Fate::Gap { .. } = row.fate {
            first.entry(&row.class.text).or_insert((row, 0)).1 += 1;
        }
    }
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor());
    let mut out = String::new();
    for (class, (row, count)) in first {
        let Fate::Gap { gap, why } = &row.fate else {
            continue;
        };
        let file = &sources.files[row.class.file];
        let help = match count {
            1 => gap.label().to_owned(),
            n => format!("{}; {n} occurrences", gap.label()),
        };
        let diagnostic = MietteDiagnostic::new(format!("`{class}` has no eggshell equivalent"))
            .with_code(format!("shadcn_import::{gap:?}"))
            .with_label(LabeledSpan::at(row.class.span.clone(), why.clone()))
            .with_help(help);
        let report = miette::Report::new(diagnostic)
            .with_source_code(NamedSource::new(file.name.clone(), file.text.clone()));
        let _ = handler.render_report(&mut out, report.as_ref());
        out.push('\n');
    }
    out
}

/// The output of `examples/preset_dump.rs`: what the preset makes of
/// shadcn, one fact a line.
fn preset_dump(root: &Path) -> Result<String> {
    duct::cmd!(
        "cargo",
        "run",
        "-q",
        "-p",
        "eggshell-ui-shadcn",
        "--features",
        "extras",
        "--example",
        "preset_dump"
    )
    .dir(root)
    .read()
    .context("running the preset_dump example")
}

fn report(plan: &Plan, rows: &[Row], tw: &TwTable, radius: f32) -> String {
    let mut r = String::new();
    let _ = writeln!(
        r,
        "# shadcn-import: Button, Badge, Combobox (`style-{}`)\n",
        plan.style
    );
    let _ = writeln!(
        r,
        "Read from `{}`. `Tw`'s table: {} class patterns and {} state variants, read from `src/tw/*.rs`'s docs. Theme radius: {radius} pt (`themes.ts`); the `--radius-*` scale (sm −4, md −2, lg ±0, xl +4) is shadcn's `globals.css`, not in the snapshot.\n",
        plan.reference.display(),
        tw.len(),
        tw.variants.len()
    );
    let _ = writeln!(r, "**{}**\n", summary_line(rows));

    let _ = writeln!(r, "## 1. How much maps\n");
    let _ = writeln!(
        r,
        "| component | classes | `Tw` call | preset token | preset state (`dark:`, `disabled:`) | no equivalent | maps |"
    );
    let _ = writeln!(r, "|---|---|---|---|---|---|---|");
    for name in COMPONENTS {
        let of: Vec<_> = rows
            .iter()
            .filter(|row| row.component == name && row.fate != Fate::Marker)
            .collect();
        let count = |f: fn(&Fate) -> bool| of.iter().filter(|row| f(&row.fate)).count();
        let tw = count(|f| matches!(f, Fate::Tw { .. }));
        let token = count(|f| matches!(f, Fate::Token { .. }));
        let preset = count(|f| matches!(f, Fate::Preset { .. }));
        let gap = count(|f| matches!(f, Fate::Gap { .. }));
        let _ = writeln!(
            r,
            "| {name} | {} | {tw} | {token} | {preset} | {gap} | {}% |",
            of.len(),
            (tw + token + preset) * 100 / of.len().max(1)
        );
    }

    let _ = writeln!(r, "\n## 2. No equivalent yet, grouped\n");
    let _ = writeln!(
        r,
        "Each distinct class once, with its count; the spans are in `diagnostics.txt`.\n"
    );
    let mut groups: BTreeMap<Gap, BTreeMap<&str, (usize, BTreeSet<&str>)>> = BTreeMap::new();
    for row in rows {
        if let Fate::Gap { gap, .. } = &row.fate {
            let entry = groups
                .entry(*gap)
                .or_default()
                .entry(row.class.text.as_str())
                .or_default();
            entry.0 += 1;
            entry.1.insert(row.component);
        }
    }
    for (gap, classes) in &groups {
        let total: usize = classes.values().map(|(n, _)| n).sum();
        let _ = writeln!(
            r,
            "### {} — {total} uses, {} distinct\n",
            gap.label(),
            classes.len()
        );
        for (class, (n, components)) in classes {
            let components: Vec<_> = components.iter().copied().collect();
            let _ = writeln!(r, "- `{class}` ×{n} ({})", components.join(", "));
        }
        let _ = writeln!(r);
    }

    r
}

/// What each variant's classes of the button and badge map to.
fn listings(rows: &[Row]) -> String {
    let mut r = String::new();
    for name in ["button", "badge"] {
        let _ = writeln!(r, "## Generated `{name}` looks, by variant\n");
        let _ = writeln!(
            r,
            "What each variant's classes map to (the selected style's CSS).\n"
        );
        let mut by_origin: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in rows
            .iter()
            .filter(|row| row.component == name && row.fate != Fate::Marker)
        {
            let key = match &row.origin {
                Origin::Base { .. } => "base".to_owned(),
                Origin::Variant { variant, name, .. } => format!("{variant}={name}"),
                Origin::Slot { function } => format!("slot {function}"),
            };
            let mark = if is_clean(&row.fate) { "" } else { "✗ " };
            by_origin.entry(key).or_default().push(match &row.via {
                Some(via) => format!("{mark}`{}` → {} (from `{via}`)", row.class.text, row.fate),
                None => format!("{mark}`{}` → {}", row.class.text, row.fate),
            });
        }
        for (origin, lines) in by_origin {
            let _ = writeln!(
                r,
                "<details><summary><code>{origin}</code> ({} classes)</summary>\n",
                lines.len()
            );
            for line in lines {
                let _ = writeln!(r, "- {line}");
            }
            let _ = writeln!(r, "\n</details>\n");
        }
    }
    r
}
