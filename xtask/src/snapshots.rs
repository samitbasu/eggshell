//! `cargo xtask snapshots`: check, update or clean the golden images.
//!
//! The goldens are rendered by Mesa lavapipe so they are the same on every
//! machine; the child is pointed at its Vulkan driver explicitly.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::process;
use crate::steps::Step;

/// Where kittest writes goldens, relative to the workspace root.
const GOLDEN_DIR: &str = "tests/snapshots";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Fail on any difference from the goldens.
    Check,
    /// Regenerate the goldens and list what changed.
    Update,
    /// Delete the `.new.png` and `.diff.png` files a failed check leaves.
    Clean,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub mode: Mode,
    pub filter: Option<String>,
}

pub fn run(plan: &Plan) -> Result<()> {
    let golden_dir = process::workspace_root().join(GOLDEN_DIR);
    match plan.mode {
        Mode::Clean => {
            let removed = clean(&golden_dir)?;
            println!("removed {} stray file(s)", removed.len());
            Ok(())
        }
        Mode::Check => check(plan, &golden_dir),
        Mode::Update => update(plan, &golden_dir),
    }
}

fn check(plan: &Plan, golden_dir: &Path) -> Result<()> {
    let result = renderer_step(plan, &lavapipe_icd()?).run();
    let strays = strays(golden_dir)?;
    for stray in &strays {
        println!("mismatch: {}", stray.display());
    }
    if result.is_err() || !strays.is_empty() {
        bail!(
            "the snapshots differ from the goldens; open the .diff.png files above, \
             and if the change is intended run `cargo xtask snapshots --update` and \
             review the images in the diff"
        );
    }
    Ok(())
}

fn update(plan: &Plan, golden_dir: &Path) -> Result<()> {
    let icd = lavapipe_icd()?;
    clean(golden_dir)?;
    renderer_step(plan, &icd).run()?;
    let changes = parse_changes(&process::git(&[
        "status",
        "--porcelain",
        "--untracked-files=all",
        "--",
        GOLDEN_DIR,
    ])?);
    if changes.is_empty() {
        println!("no golden changed");
    }
    for Change { kind, path } in changes {
        println!("{kind}: {path}");
    }
    Ok(())
}

/// The test run, with the software renderer selected for the child only.
fn renderer_step(plan: &Plan, icd: &Path) -> Step {
    let mut step = Step::cargo(
        "snapshots",
        &[
            "test",
            "--package",
            "eggshell-ui",
            "--features",
            "snapshots",
            "--test",
            "it",
            "--",
            "--nocapture",
        ],
    )
    .with_env("VK_ICD_FILENAMES", icd.display().to_string())
    .with_env("VK_DRIVER_FILES", icd.display().to_string());
    if plan.mode == Mode::Update {
        step = step.with_env("UPDATE_SNAPSHOTS", "1");
    }
    step.args.push(snapshot_filter(plan.filter.as_deref()));
    step
}

/// The libtest filter selecting the `snapshots` module of the `it` binary,
/// narrowed to the golden whose name starts with `name`.
fn snapshot_filter(name: Option<&str>) -> String {
    format!("snapshots::{}", name.unwrap_or_default())
}

fn is_stray(file_name: &str) -> bool {
    file_name.ends_with(".new.png") || file_name.ends_with(".diff.png")
}

fn strays(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(found),
        Err(error) => return Err(error).with_context(|| format!("reading {}", dir.display())),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(strays(&path)?);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_stray)
        {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

fn clean(dir: &Path) -> Result<Vec<PathBuf>> {
    let found = strays(dir)?;
    for path in &found {
        fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
        println!("removed {}", path.display());
    }
    Ok(found)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Removed,
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Removed => "removed",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub kind: ChangeKind,
    pub path: String,
}

/// The golden images in `git status --porcelain` output.
fn parse_changes(porcelain: &str) -> Vec<Change> {
    porcelain
        .lines()
        .filter_map(|line| {
            let (status, path) = (line.get(..2)?, line.get(3..)?);
            if !path.ends_with(".png") || is_stray(path) {
                return None;
            }
            let kind = if status == "??" || status.contains('A') {
                ChangeKind::Added
            } else if status.contains('D') {
                ChangeKind::Removed
            } else {
                ChangeKind::Modified
            };
            Some(Change {
                kind,
                path: path.to_owned(),
            })
        })
        .collect()
}

/// The lavapipe ICD manifest of this machine's architecture.
fn lavapipe_icd() -> Result<PathBuf> {
    let dirs = icd_dirs(std::env::var("XDG_DATA_DIRS").ok().as_deref());
    find_lavapipe_icd(&dirs, std::env::consts::ARCH).with_context(|| {
        format!(
            "Mesa lavapipe is needed for deterministic goldens, but no lvp_icd.{}.json was found in {}; \
             install it (the mesa-vulkan-drivers package on Fedora, Debian and Ubuntu)",
            std::env::consts::ARCH,
            dirs.iter()
                .map(|dir| dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn icd_dirs(xdg_data_dirs: Option<&str>) -> Vec<PathBuf> {
    let data_dirs = xdg_data_dirs
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or("/usr/local/share:/usr/share");
    data_dirs
        .split(':')
        .map(|dir| Path::new(dir).join("vulkan/icd.d"))
        .chain([PathBuf::from("/etc/vulkan/icd.d")])
        .collect()
}

fn find_lavapipe_icd(dirs: &[PathBuf], arch: &str) -> Option<PathBuf> {
    let wanted = [format!("lvp_icd.{arch}.json"), "lvp_icd.json".to_owned()];
    wanted
        .iter()
        .flat_map(|name| dirs.iter().map(move |dir| dir.join(name)))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    fn plan(mode: Mode, filter: Option<&str>) -> Plan {
        Plan {
            mode,
            filter: filter.map(str::to_owned),
        }
    }

    #[test]
    fn check_selects_lavapipe_and_does_not_update() {
        let step = renderer_step(&plan(Mode::Check, None), Path::new("/icd/lvp.json"));
        assert_eq!(
            step.to_string(),
            "VK_ICD_FILENAMES=\"/icd/lvp.json\" VK_DRIVER_FILES=\"/icd/lvp.json\" \
             cargo test --package eggshell-ui --features snapshots --test it -- --nocapture snapshots::"
        );
    }

    #[test]
    fn update_sets_the_variable_for_the_child_and_filter_follows_the_separator() {
        let step = renderer_step(
            &plan(Mode::Update, Some("kaolin_nested")),
            Path::new("/i.json"),
        );
        assert!(step.env.contains(&("UPDATE_SNAPSHOTS", "1".to_owned())));
        assert_eq!(
            step.args.last().map(String::as_str),
            Some("snapshots::kaolin_nested")
        );
        let separator = step.args.iter().position(|arg| arg == "--").unwrap();
        assert!(step.args.len() > separator + 1);
    }

    #[test]
    fn strays_are_new_and_diff_images_only() {
        assert!(is_stray("a.new.png"));
        assert!(is_stray("a.diff.png"));
        assert!(!is_stray("a.png"));
        assert!(!is_stray("renew.pngx"));
    }

    #[test]
    fn clean_removes_strays_below_the_directory_and_keeps_goldens() {
        let tmp = TempDir::new("strays");
        fs::create_dir(tmp.path().join("sub")).unwrap();
        for name in [
            "a.png",
            "a.new.png",
            "a.diff.png",
            "sub/b.diff.png",
            "notes.txt",
        ] {
            fs::write(tmp.path().join(name), "").unwrap();
        }
        assert_eq!(strays(tmp.path()).unwrap().len(), 3, "precondition");

        assert_eq!(clean(tmp.path()).unwrap().len(), 3);

        assert!(tmp.path().join("a.png").exists());
        assert!(tmp.path().join("notes.txt").exists());
        assert!(strays(tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn a_missing_golden_directory_has_no_strays() {
        let tmp = TempDir::new("no-goldens");
        assert!(strays(&tmp.path().join("nope")).unwrap().is_empty());
    }

    #[test]
    fn porcelain_status_becomes_added_modified_and_removed() {
        let porcelain = "?? tests/snapshots/new.png\n\
                         A  tests/snapshots/staged.png\n \
                         M tests/snapshots/changed.png\n\
                         M  tests/snapshots/staged_change.png\n \
                         D tests/snapshots/gone.png\n\
                         ?? tests/snapshots/x.new.png\n\
                         ?? tests/snapshots/readme.txt\n";
        let kinds: Vec<_> = parse_changes(porcelain)
            .into_iter()
            .map(|change| (change.kind, change.path))
            .collect();
        assert_eq!(
            kinds,
            [
                (ChangeKind::Added, "tests/snapshots/new.png".to_owned()),
                (ChangeKind::Added, "tests/snapshots/staged.png".to_owned()),
                (
                    ChangeKind::Modified,
                    "tests/snapshots/changed.png".to_owned()
                ),
                (
                    ChangeKind::Modified,
                    "tests/snapshots/staged_change.png".to_owned()
                ),
                (ChangeKind::Removed, "tests/snapshots/gone.png".to_owned()),
            ]
        );
    }

    #[test]
    fn icd_directories_follow_xdg_with_the_defaults_and_etc() {
        assert_eq!(
            icd_dirs(Some("/a:/b")),
            [
                PathBuf::from("/a/vulkan/icd.d"),
                PathBuf::from("/b/vulkan/icd.d"),
                PathBuf::from("/etc/vulkan/icd.d")
            ]
        );
        assert_eq!(icd_dirs(None)[1], PathBuf::from("/usr/share/vulkan/icd.d"));
        assert_eq!(icd_dirs(Some("")).len(), 3);
    }

    #[test]
    fn the_icd_of_the_right_architecture_is_found() {
        let tmp = TempDir::new("icd");
        let early = tmp.path().join("early");
        let late = tmp.path().join("late");
        fs::create_dir_all(&early).unwrap();
        fs::create_dir_all(&late).unwrap();
        fs::write(early.join("lvp_icd.i686.json"), "").unwrap();
        fs::write(early.join("radeon_icd.x86_64.json"), "").unwrap();
        fs::write(late.join("lvp_icd.x86_64.json"), "").unwrap();
        let dirs = [early, late.clone()];

        assert_eq!(
            find_lavapipe_icd(&dirs, "x86_64"),
            Some(late.join("lvp_icd.x86_64.json"))
        );
        assert_eq!(find_lavapipe_icd(&dirs, "aarch64"), None);
    }
}
