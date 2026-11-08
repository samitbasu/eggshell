use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};

use crate::process;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    pub name: String,
    pub description: String,
}

/// The first line of the file's leading `//!` doc, without the marker.
fn first_doc_line(source: &str) -> Option<&str> {
    source
        .lines()
        .map(str::trim_start)
        .find_map(|line| line.strip_prefix("//!"))
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Examples are `*.rs` files directly in `dir` and `*/main.rs` below it.
pub fn list(dir: &Path) -> io::Result<Vec<Example>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let (name, source_path) = if path.is_dir() {
            (path.file_name(), path.join("main.rs"))
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            (path.file_stem(), path.clone())
        } else {
            continue;
        };
        let Some(name) = name.and_then(|name| name.to_str()) else {
            continue;
        };
        let Ok(source) = fs::read_to_string(&source_path) else {
            continue;
        };
        found.push(Example {
            name: name.to_owned(),
            description: first_doc_line(&source).unwrap_or_default().to_owned(),
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

pub fn print_listing() -> Result<()> {
    let dir = process::workspace_root().join("examples");
    let examples = list(&dir).with_context(|| format!("reading {}", dir.display()))?;
    let width = examples.iter().map(|e| e.name.len()).max().unwrap_or(0);
    for Example { name, description } in examples {
        println!("{name:<width$}  {description}");
    }
    println!("\nrun one with: cargo xtask example NAME");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn first_doc_line_takes_first_inner_doc_line() {
        let source = "//! First line.\n//!\n//! More.\nfn main() {}\n";
        assert_eq!(first_doc_line(source), Some("First line."));
    }

    #[test]
    fn first_doc_line_skips_plain_comments_and_blank_docs() {
        assert_eq!(first_doc_line("// note\n//!   Spaced  \n"), Some("Spaced"));
        assert_eq!(first_doc_line("//!\n"), None);
        assert_eq!(first_doc_line("fn main() {}\n"), None);
    }

    #[test]
    fn list_finds_files_and_directories_sorted() {
        let tmp = TempDir::new("list");
        fs::write(tmp.path().join("zebra.rs"), "//! Stripes.\nfn main() {}\n").unwrap();
        fs::write(tmp.path().join("alpha.rs"), "fn main() {}\n").unwrap();
        fs::write(tmp.path().join("notes.txt"), "//! not an example\n").unwrap();
        fs::create_dir(tmp.path().join("multi")).unwrap();
        fs::write(tmp.path().join("multi/main.rs"), "//! Many files.\n").unwrap();
        fs::create_dir(tmp.path().join("empty")).unwrap();

        let examples = list(tmp.path()).unwrap();

        let expected = [
            ("alpha", ""),
            ("multi", "Many files."),
            ("zebra", "Stripes."),
        ];
        assert_eq!(examples.len(), expected.len());
        for (example, (name, description)) in examples.iter().zip(expected) {
            assert_eq!(
                (example.name.as_str(), example.description.as_str()),
                (name, description)
            );
        }
    }

    #[test]
    fn list_reports_missing_directory() {
        let tmp = TempDir::new("missing");
        assert!(list(&tmp.path().join("nope")).is_err());
    }
}
