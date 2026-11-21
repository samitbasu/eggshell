//! `cargo xtask shadcn-import`: reads shadcn's components (the registry
//! `.tsx` and each style's CSS) at the pinned commit (`xtask/shadcn.pin`),
//! maps every Tailwind class onto eggshell, and writes each widget's `Sheet`
//! rules with a report of how each class mapped and what does not yet.
//! `--write` puts the rules of the migrated widgets into the shadcn preset's
//! crate (`src/sheets/`); `--check` fails if those are not what the pinned
//! source gives.

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
const COMPONENTS: [&str; 7] = [
    "button",
    "badge",
    "combobox",
    "input",
    "input-group",
    "select",
    "tabs",
];

/// The widgets whose `Sheet` rules are emitted, and how eggshell names
/// their shadcn variants and sizes: shadcn's `default` button is eggshell's
/// `primary`, and a button that names no variant is `secondary`.
const WIDGETS: [emit::Widget; 6] = [
    emit::Widget {
        component: "button",
        checked_in: true,
        variants: &[
            ("default", &["primary"]),
            ("secondary", &["secondary", "default"]),
        ],
        sizes: &[],
        facts: &[],
        parts: None,
    },
    emit::Widget {
        component: "input",
        checked_in: true,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
    },
    emit::Widget {
        component: "input-group",
        checked_in: false,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
    },
    emit::Widget {
        component: "select",
        checked_in: false,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
    },
    emit::Widget {
        component: "combobox",
        checked_in: false,
        variants: &[],
        sizes: &[],
        facts: &[],
        parts: None,
    },
    emit::Widget {
        component: "tabs",
        checked_in: false,
        variants: &[],
        sizes: &[],
        facts: &TABS,
        parts: None,
    },
];

/// eggshell's tabs are horizontal, and their list has only the default look.
const TABS: [emit::Assumed; 6] = {
    use emit::{Assumed, Fact::*};
    const HORIZONTAL: &str = "eggshell's tabs are horizontal only";
    const DEFAULT: &str = "eggshell's tab list has only the default look";
    [
        Assumed {
            variant: "data-horizontal",
            fact: Holds,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "data-vertical",
            fact: Never,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-horizontal/tabs",
            fact: Holds,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-vertical/tabs",
            fact: Never,
            why: HORIZONTAL,
        },
        Assumed {
            variant: "group-data-[variant=default]/tabs-list",
            fact: Holds,
            why: DEFAULT,
        },
        Assumed {
            variant: "group-data-[variant=line]/tabs-list",
            fact: Never,
            why: DEFAULT,
        },
    ]
};

/// The `sheets` module of the preset's crate: a module per migrated
/// `component` and style, and `write`, which writes them all for a style and
/// theme, in the order of `components`.
fn index(components: &[&str]) -> String {
    let module = |style: &str, component: &str| format!("{style}_{}", component.replace('-', "_"));
    let mut out = String::from(
        "//! The widgets' rules in each shadcn style, generated from the pinned shadcn\n\
         //! source by `cargo xtask shadcn-import --write`; CI fails if they drift\n\
         //! (`--check`). Each rule's classes carry the shadcn class they came from.\n\
         //!\n\
         //! This file is generated too. Do not edit.\n\n\
         use eggshell_ui::sheet::Sheet;\n\
         use egui::Theme;\n\n\
         use crate::ShadcnStyle;\n\n\
         /// One half of a widget's rules: its light or its dark theme.\n\
         type Half = fn(&mut Sheet);\n",
    );
    for style in STYLES {
        for component in components {
            let _ = write!(out, "\nmod {};", module(style, component));
        }
    }
    out.push_str(
        "\n\n/// Writes the rules of `style` for `theme` into `sheet`. Their colours are\n\
         /// the style's core tokens, which the sheet is resolved with.\n\
         pub(crate) fn write(sheet: &mut Sheet, style: ShadcnStyle, theme: Theme) {\n\
         \x20   let halves: &[(Half, Half)] = match style {\n",
    );
    for style in STYLES {
        let variant = format!("{}{}", style[..1].to_uppercase(), &style[1..]);
        let _ = writeln!(out, "        ShadcnStyle::{variant} => &[");
        for component in components {
            let m = module(style, component);
            let _ = writeln!(out, "            ({m}::light, {m}::dark),");
        }
        out.push_str("        ],\n");
    }
    out.push_str(
        "    };\n\
         \x20   for (light, dark) in halves {\n\
         \x20       match theme {\n\
         \x20           Theme::Light => light(sheet),\n\
         \x20           Theme::Dark => dark(sheet),\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    out
}

/// Where a widget's checked-in rules for `style` live.
fn checked_in(root: &Path, style: &str, component: &str) -> PathBuf {
    root.join("crates/eggshell-ui-shadcn/src/sheets")
        .join(format!("{style}_{}.rs", component.replace('-', "_")))
}

/// What the run does besides the report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The report and every widget's rules, under `out`.
    Report,
    /// The migrated widgets' rules into the preset's crate.
    Write,
    /// Fail if the preset's crate holds other rules than the source gives.
    Check,
}

/// The commit the reference must be at.
fn pin(root: &Path) -> Result<String> {
    Ok(read(&root.join("xtask/shadcn.pin"))?.trim().to_owned())
}

/// Fails unless the shadcn checkout at `reference` is at the pinned commit.
fn check_pin(root: &Path, reference: &Path) -> Result<()> {
    let want = pin(root)?;
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(reference)
        .args(["rev-parse", "HEAD"])
        .output()
        .with_context(|| {
            format!(
                "reading the commit of {}; check out shadcn-ui/ui at {want} there \
                 (a sparse checkout of apps/v4/registry is enough)",
                reference.display()
            )
        })?;
    let have = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    anyhow::ensure!(
        head.status.success() && have == want,
        "{} is at `{have}`, not the pinned {want} (xtask/shadcn.pin)",
        reference.display()
    );
    Ok(())
}

/// The migrated widgets' rules for every style, written or compared.
fn sheets(root: &Path, registry: &Path, classifier: &Classifier<'_>, mode: Mode) -> Result<()> {
    let migrated: Vec<&str> = WIDGETS
        .iter()
        .filter(|w| w.checked_in)
        .map(|w| w.component)
        .collect();
    let mut files = vec![(
        root.join("crates/eggshell-ui-shadcn/src/sheets/mod.rs"),
        index(&migrated),
    )];
    for style in STYLES {
        let mut scratch = Sources::default();
        let (rows, _) = extract(registry, style, classifier, &mut scratch)?;
        for widget in WIDGETS.iter().filter(|w| w.checked_in) {
            let emitted = emit::emit(classifier, &rows, &scratch, widget, style);
            files.push((checked_in(root, style, widget.component), emitted.rust));
        }
    }
    let mut stale = Vec::new();
    for (path, rust) in files {
        match mode {
            Mode::Write => fs::write(&path, &rust)?,
            Mode::Check | Mode::Report => {
                if fs::read_to_string(&path).ok().as_deref() != Some(rust.as_str()) {
                    stale.push(path.display().to_string());
                }
            }
        }
    }
    anyhow::ensure!(
        stale.is_empty(),
        "the generated sheets differ from the pinned shadcn source; run `cargo xtask shadcn-import --write`:\n  {}",
        stale.join("\n  ")
    );
    Ok(())
}

pub struct Plan {
    /// What the run does besides the report.
    pub mode: Mode,
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
    /// The part it styles, as shadcn's `data-slot` names it.
    slot: String,
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
        for (origin, slot, classes) in source::component(&text, file).parts {
            for class in classes {
                match rules.get(&class.text) {
                    Some(applied) => rows.extend(applied.iter().map(|inner| Row {
                        component: name,
                        origin: origin.clone(),
                        slot: slot.clone(),
                        via: Some(class.text.clone()),
                        fate: classifier.classify(&inner.text),
                        class: inner.clone(),
                    })),
                    None => rows.push(Row {
                        component: name,
                        origin: origin.clone(),
                        slot: slot.clone(),
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

/// `--check` with the defaults, for `cargo xtask ci`.
pub fn check() -> Result<()> {
    run(&Plan {
        mode: Mode::Check,
        reference: PathBuf::from("../ref/shadcn"),
        style: "vega".to_owned(),
        out: PathBuf::from("target/shadcn-import"),
    })
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
    let tokens = classes::token_names(&read(&root.join("src/contract.rs"))?);
    let radius = theme_radius(&read(&registry.join("themes.ts"))?)
        .context("no `radius` in registry/themes.ts")?;
    let classifier = Classifier {
        tw: &tw,
        tokens: &tokens,
        radius: Radius { base: radius },
    };
    check_pin(&root, &root.join(&plan.reference))?;
    if plan.mode != Mode::Report {
        sheets(&root, &registry, &classifier, plan.mode)?;
        println!(
            "sheets: {}",
            if plan.mode == Mode::Write {
                "written"
            } else {
                "up to date"
            }
        );
        return Ok(());
    }
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
        let sheets = root.join(&plan.out).join("sheets").join(style);
        fs::create_dir_all(&sheets)?;
        for widget in &WIDGETS {
            let emitted = emit::emit(&classifier, &rows, &scratch, widget, style);
            let file = format!("{}.rs", widget.component.replace('-', "_"));
            fs::write(sheets.join(&file), &emitted.rust)?;
            sheet_section(&mut report, style, widget, &file, &emitted);
        }
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
/// what it settles for the widget, and what is not there yet.
fn sheet_section(
    report: &mut String,
    style: &str,
    widget: &emit::Widget,
    file: &str,
    emitted: &emit::Emitted,
) {
    let _ = writeln!(
        report,
        "### The `Sheet` rules for `{}` (`sheets/{style}/{file}`, `{}`)\n",
        widget.component,
        widget.part_enum()
    );
    let _ = writeln!(report, "| mapping kind | classes |\n|---|---|");
    for (kind, count) in &emitted.kinds {
        let _ = writeln!(report, "| {} | {count} |", kind.label());
    }
    let _ = writeln!(report, "\nUnmapped, once each:\n");
    for (source, why) in &emitted.unmapped {
        let _ = writeln!(report, "- `{source}`: {why}");
    }
    if !emitted.assumed.is_empty() {
        let _ = writeln!(report, "\nTaken to hold:\n");
        for (variant, why) in &emitted.assumed {
            let _ = writeln!(report, "- `{variant}:`: {why}");
        }
    }
    if !emitted.dropped.is_empty() {
        let _ = writeln!(
            report,
            "\nDropped, a state the widget never has, once each:\n"
        );
        for (source, why) in &emitted.dropped {
            let _ = writeln!(report, "- `{source}`: {why}");
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheets_index_declares_and_writes_every_migrated_widget_in_each_style() {
        let index = index(&["button", "input-group"]);
        for module in [
            "vega_button",
            "vega_input_group",
            "lyra_button",
            "lyra_input_group",
        ] {
            assert!(index.contains(&format!("\nmod {module};")), "{module}");
            assert!(
                index.contains(&format!("({module}::light, {module}::dark),")),
                "{module} is written"
            );
        }
        let vega = index.find("ShadcnStyle::Vega => &[").expect("vega");
        let lyra = index.find("ShadcnStyle::Lyra => &[").expect("lyra");
        assert!(index[vega..lyra].contains("vega_input_group::light"));
        assert!(
            !index[vega..lyra].contains("lyra_"),
            "each style writes its own"
        );
        assert!(
            index.find("(vega_button::light").expect("button")
                < index.find("(vega_input_group::light").expect("group"),
            "in the table's order"
        );
    }
}
