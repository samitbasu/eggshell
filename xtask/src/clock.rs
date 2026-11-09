//! The library reads time from egui's clock, or from `web_time::Instant` for
//! the debug readout: `std::time::Instant` and `SystemTime` panic on
//! wasm32-unknown-unknown.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::process;

const FORBIDDEN: [&str; 2] = ["Instant", "SystemTime"];

#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub line: usize,
    pub text: String,
}

fn mentions_forbidden(line: &str) -> bool {
    let line = line.replace("web_time::Instant", "");
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| FORBIDDEN.contains(&word))
}

/// Lines of `source` outside `#[cfg(test)]` items and comments that name a
/// forbidden clock type.
pub fn violations(source: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut skipping_test_item = false;
    let mut depth = 0usize;
    let mut entered = false;
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if !skipping_test_item {
            if trimmed.starts_with("#[cfg(test)]") {
                skipping_test_item = true;
                (depth, entered) = (0, false);
            } else if !trimmed.starts_with("//") && mentions_forbidden(line) {
                found.push(Violation {
                    line: index + 1,
                    text: trimmed.trim_end().to_owned(),
                });
            }
            continue;
        }
        for c in line.chars() {
            match c {
                '{' => {
                    depth += 1;
                    entered = true;
                }
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if (entered && depth == 0) || (!entered && line.trim_end().ends_with(';')) {
            skipping_test_item = false;
        }
    }
    found
}

fn rust_files(dir: &Path, into: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, into)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
    Ok(())
}

/// Fails if non-test library code under `dir` names a forbidden clock type.
pub fn check(dir: &Path) -> Result<()> {
    let mut files = Vec::new();
    rust_files(dir, &mut files)?;
    files.sort();
    let mut report = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)?;
        for Violation { line, text } in violations(&source) {
            report.push(format!("{}:{line}: {text}", file.display()));
        }
    }
    if report.is_empty() {
        return Ok(());
    }
    bail!(
        "wall-clock types are unavailable on wasm32; use egui's clock (`ctx.input(|i| i.time)`):\n       {}",
        report.join("\n       ")
    )
}

pub fn check_library() -> Result<()> {
    check(&process::workspace_root().join("src"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn flags_use_and_path_uses_outside_tests() {
        let source = "use std::time::Instant;\nfn f() { let _ = std::time::SystemTime::now(); }\n";
        let lines: Vec<_> = violations(source).iter().map(|v| v.line).collect();
        assert_eq!(lines, [1, 2]);
    }

    #[test]
    fn rejects_std_instant_and_accepts_web_time_instant() {
        assert_eq!(violations("use std::time::Instant;\n").len(), 1);
        assert_eq!(violations("use web_time::Instant;\n"), []);
        assert_eq!(violations("let t = web_time::Instant::now();\n"), []);
        let mixed = "use web_time::Instant;\nuse std::time::Instant;\n";
        assert_eq!(violations(mixed).len(), 1);
    }

    #[test]
    fn ignores_cfg_test_modules_comments_and_longer_words() {
        let source = "\
// Instant replay
fn instant_replay() {}
struct InstantOffer;
#[cfg(test)]
mod tests {
    use std::time::Instant;
    fn t() { if true { let _ = Instant::now(); } }
}
fn after() {}
";
        assert_eq!(violations(source), []);
    }

    #[test]
    fn checking_resumes_after_a_test_module() {
        let source = "#[cfg(test)]\nmod tests {\n    fn t() {}\n}\nuse std::time::Instant;\n";
        let found = violations(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 5);
    }

    #[test]
    fn a_semicolon_item_under_cfg_test_is_skipped() {
        let source = "#[cfg(test)]\nuse std::time::Instant;\nuse std::time::SystemTime;\n";
        let lines: Vec<_> = violations(source).iter().map(|v| v.line).collect();
        assert_eq!(lines, [3]);
    }

    #[test]
    fn check_reports_file_and_line_and_passes_clean_trees() {
        let tmp = TempDir::new("clock");
        fs::write(tmp.path().join("ok.rs"), "fn f() {}\n").unwrap();
        assert!(check(tmp.path()).is_ok());
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub/bad.rs"), "\nuse std::time::Instant;\n").unwrap();
        let message = format!("{:#}", check(tmp.path()).unwrap_err());
        assert!(message.contains("bad.rs:2"), "{message}");
    }

    #[test]
    fn the_library_is_clean() {
        check_library().unwrap();
    }
}
