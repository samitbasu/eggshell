//! A `Tw` method that takes spacing units scales with the theme's spacing, so
//! a length in points must not be passed to it as `points / 4.0`: it would
//! scale where the points were meant to stay. The method's `_px` twin takes
//! the points.

use std::collections::BTreeSet;
use std::fs;

use anyhow::{Result, bail};

use crate::process;

const SOURCES: [&str; 4] = ["src", "examples", "eggshell-examples/src", "tests"];

/// What turns points into spacing units at Tailwind's 4 points a unit.
const QUARTERS: [&str; 3] = ["/ 4.0", "/4.0", "* 0.25"];

/// The argument of the call whose `(` is at `open`, without its parentheses.
fn argument(text: &str, open: usize) -> &str {
    let mut depth = 0usize;
    for (offset, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &text[open + 1..open + offset];
                }
            }
            _ => {}
        }
    }
    &text[open + 1..]
}

/// The calls in `source` of a method in `units` whose argument quarters a
/// length, as `(line, call)`.
pub fn violations(source: &str, units: &BTreeSet<String>) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (at, _) in source.match_indices('.') {
        let name: String = source[at + 1..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let open = at + 1 + name.len();
        if !units.contains(&name) || !source[open..].starts_with('(') {
            continue;
        }
        let argument = argument(source, open);
        if QUARTERS.iter().any(|quarter| argument.contains(quarter)) {
            let line = source[..at].lines().count().max(1);
            found.push((line, format!(".{name}({argument})")));
        }
    }
    found
}

/// Fails if a source passes a length in points to a method in spacing units.
pub fn check_sources() -> Result<()> {
    let root = process::workspace_root();
    let units = crate::shadcn_import::unit_methods(&root)?;
    let mut files = Vec::new();
    for dir in SOURCES {
        crate::clock::rust_files(&root.join(dir), &mut files)?;
    }
    files.sort();
    let mut report = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)?;
        for (line, call) in violations(&source, &units) {
            report.push(format!("{}:{line}: {call}", file.display()));
        }
    }
    if report.is_empty() {
        return Ok(());
    }
    bail!(
        "points passed to a method in spacing units scale with the theme; use its `_px` twin:\n       {}",
        report.join("\n       ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units() -> BTreeSet<String> {
        ["px", "gap"].map(str::to_owned).into()
    }

    #[test]
    fn a_quartered_length_in_a_units_method_is_found() {
        let source = "t.flex()\n    .px(m.padding_x / 4.0)\n    .gap((a + b) * 0.25)";
        assert_eq!(
            violations(source, &units()),
            [
                (2, ".px(m.padding_x / 4.0)".to_owned()),
                (3, ".gap((a + b) * 0.25)".to_owned())
            ]
        );
    }

    #[test]
    fn points_in_a_px_twin_and_units_as_written_pass() {
        let source = "t.px_px(m.padding_x).px(2.5).gap(rect.width() / 4.0 - 1.0).w(x / 4.0)";
        assert_eq!(
            violations(source, &units()),
            [(1, ".gap(rect.width() / 4.0 - 1.0)".to_owned())],
            "only a units method is checked, and any quartering in its argument counts"
        );
    }
}
