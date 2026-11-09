//! `cargo xtask loc`: lines of code per source file, so each milestone can
//! quote the size of the library. Blank lines, comments (line, doc and
//! block) and items under `#[cfg(test)]` are not counted.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Where a scan is relative to what it skips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lexing {
    Code,
    LineComment,
    BlockComment { depth: usize },
    Str,
    RawStr { hashes: usize },
}

/// Whether the scanner is inside an item that `#[cfg(test)]` removes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Skipping {
    No,
    /// The attribute was seen; the item has not started.
    Pending,
    /// Inside the item's braces.
    Braces {
        depth: usize,
    },
}

/// Counts the lines of `source` that hold code outside test-only items.
pub fn code_lines(source: &str) -> usize {
    let mut lexing = Lexing::Code;
    let mut skipping = Skipping::No;
    let mut count = 0;
    for line in source.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut has_code = false;
        let mut counted_before = skipping == Skipping::No;
        let mut index = 0;
        if lexing == Lexing::LineComment {
            lexing = Lexing::Code;
        }
        if lexing == Lexing::Code
            && skipping == Skipping::No
            && line.trim().starts_with("#[cfg(test)]")
        {
            skipping = Skipping::Pending;
            continue;
        }
        while index < chars.len() {
            let c = chars[index];
            let next = chars.get(index + 1).copied();
            match lexing {
                Lexing::LineComment => break,
                Lexing::BlockComment { depth } => {
                    if c == '*' && next == Some('/') {
                        index += 1;
                        lexing = if depth == 1 {
                            Lexing::Code
                        } else {
                            Lexing::BlockComment { depth: depth - 1 }
                        };
                    } else if c == '/' && next == Some('*') {
                        index += 1;
                        lexing = Lexing::BlockComment { depth: depth + 1 };
                    }
                }
                Lexing::Str => {
                    has_code = true;
                    match c {
                        '\\' => index += 1,
                        '"' => lexing = Lexing::Code,
                        _ => {}
                    }
                }
                Lexing::RawStr { hashes } => {
                    has_code = true;
                    let closes = c == '"'
                        && chars[index + 1..].iter().take_while(|&&h| h == '#').count() >= hashes;
                    if closes {
                        index += hashes;
                        lexing = Lexing::Code;
                    }
                }
                Lexing::Code => {
                    if c == '/' && next == Some('/') {
                        lexing = Lexing::LineComment;
                        continue;
                    }
                    if c == '/' && next == Some('*') {
                        index += 1;
                        lexing = Lexing::BlockComment { depth: 1 };
                    } else if !c.is_whitespace() {
                        has_code = true;
                        match c {
                            '"' => lexing = Lexing::Str,
                            'r' if matches!(next, Some('"' | '#'))
                                && !chars[..index]
                                    .last()
                                    .is_some_and(|p| p.is_alphanumeric() || *p == '_') =>
                            {
                                let hashes =
                                    chars[index + 1..].iter().take_while(|&&h| h == '#').count();
                                if chars.get(index + 1 + hashes) == Some(&'"') {
                                    index += 1 + hashes;
                                    lexing = Lexing::RawStr { hashes };
                                }
                            }
                            '\'' => {
                                // A char literal closes within a few characters; a
                                // lifetime does not.
                                let close = if next == Some('\\') { 3 } else { 2 };
                                if chars.get(index + close) == Some(&'\'') {
                                    index += close;
                                }
                            }
                            '{' => {
                                skipping = match skipping {
                                    Skipping::Pending => Skipping::Braces { depth: 1 },
                                    Skipping::Braces { depth } => {
                                        Skipping::Braces { depth: depth + 1 }
                                    }
                                    Skipping::No => Skipping::No,
                                };
                            }
                            '}' => {
                                if let Skipping::Braces { depth } = skipping {
                                    skipping = if depth == 1 {
                                        Skipping::No
                                    } else {
                                        Skipping::Braces { depth: depth - 1 }
                                    };
                                    if skipping == Skipping::No {
                                        counted_before = false;
                                    }
                                }
                            }
                            ';' if skipping == Skipping::Pending => {
                                skipping = Skipping::No;
                                counted_before = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
            index += 1;
        }
        if has_code && counted_before {
            count += 1;
        }
    }
    count
}

struct Row {
    path: PathBuf,
    code: usize,
    total: usize,
}

fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, into)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
    Ok(())
}

/// The table of code lines per file under `root`, and per directory.
pub fn report(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    rust_files(root, &mut files).with_context(|| format!("reading {}", root.display()))?;
    let mut rows = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        rows.push(Row {
            path: path.strip_prefix(root).unwrap_or(&path).to_owned(),
            code: code_lines(&source),
            total: source.lines().count(),
        });
    }
    let mut out = String::new();
    writeln!(out, "{:<28} {:>6} {:>6}", "module", "code", "total")?;
    let mut directories: Vec<(PathBuf, usize, usize)> = Vec::new();
    for row in &rows {
        writeln!(
            out,
            "{:<28} {:>6} {:>6}",
            row.path.display(),
            row.code,
            row.total
        )?;
        if let Some(parent) = row.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            match directories.iter_mut().find(|(dir, ..)| dir == parent) {
                Some((_, code, total)) => {
                    *code += row.code;
                    *total += row.total;
                }
                None => directories.push((parent.to_owned(), row.code, row.total)),
            }
        }
    }
    for (dir, code, total) in directories {
        writeln!(
            out,
            "{:<28} {code:>6} {total:>6}",
            format!("{}/ (sum)", dir.display())
        )?;
    }
    writeln!(
        out,
        "{:<28} {:>6} {:>6}",
        "total",
        rows.iter().map(|row| row.code).sum::<usize>(),
        rows.iter().map(|row| row.total).sum::<usize>()
    )?;
    Ok(out)
}

pub fn run(roots: &[PathBuf]) -> Result<()> {
    for root in roots {
        println!("== {}", root.display());
        print!("{}", report(root)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_and_comments_are_not_code() {
        let source = "//! doc\n\n/// doc\nfn a() {\n    // note\n    let x = 1; // trailing\n}\n/* block\n   still */\n";
        assert_eq!(code_lines(source), 3);
    }

    #[test]
    fn test_modules_are_skipped_including_braces_in_strings() {
        let source = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn b() { let s = \"}\"; }\n    #[test]\n    fn c() {}\n}\nfn d() {}\n";
        assert_eq!(code_lines(source), 2);
    }

    #[test]
    fn a_cfg_test_use_is_skipped_to_its_semicolon() {
        assert_eq!(code_lines("#[cfg(test)]\nuse std::fmt;\nfn a() {}\n"), 1);
    }

    #[test]
    fn lifetimes_and_chars_do_not_open_strings() {
        let source =
            "fn a<'x>(c: char) -> &'x str { if c == '\"' { \"a\" } else { \"b\" } }\nfn b() {}\n";
        assert_eq!(code_lines(source), 2);
    }

    #[test]
    fn raw_strings_may_hold_comment_markers() {
        let source = "const A: &str = r#\"\n// not a comment\n\"#;\nfn a() {}\n";
        assert_eq!(code_lines(source), 4);
    }

    #[test]
    fn code_after_a_block_comment_on_the_same_line_counts() {
        assert_eq!(code_lines("/* c */ fn a() {}\n/* only */\n"), 1);
    }
}
