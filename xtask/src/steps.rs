use std::fmt;
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::process;

/// One cargo invocation. Every task is a list of these, so each is announced
/// and failure-reported the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: &'static str,
    pub args: Vec<String>,
    pub env: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtMode {
    Write,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Html,
    Lcov,
}

impl Step {
    pub fn cargo(name: &'static str, args: &[&str]) -> Self {
        Self {
            name,
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            env: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.env.push((key, value.into()));
        self
    }

    pub fn run(&self) -> Result<()> {
        eprintln!("==> {}: {self}", self.name);
        let status = process::run_cargo(&self.args, &self.env)?;
        if !status.success() {
            bail!("step `{}` failed ({status})", self.name);
        }
        Ok(())
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.env {
            write!(f, "{key}={value:?} ")?;
        }
        write!(f, "cargo {}", self.args.join(" "))
    }
}

pub fn fmt(mode: FmtMode) -> Step {
    match mode {
        FmtMode::Write => Step::cargo("fmt", &["fmt", "--all"]),
        FmtMode::Check => Step::cargo("fmt", &["fmt", "--all", "--check"]),
    }
}

pub fn test(extra: &[String]) -> Step {
    let mut step = Step::cargo("test", &["test", "--workspace"]);
    if !extra.is_empty() {
        step.args.push("--".to_owned());
        step.args.extend_from_slice(extra);
    }
    step
}

pub fn example(name: &str, profile: Profile) -> Step {
    let mut step = Step::cargo("example", &["run", "--example", name]);
    if profile == Profile::Release {
        step.args.push("--release".to_owned());
    }
    step
}

/// What CI runs, in order.
///
/// `cargo xtask snapshots` is deliberately not here: it needs Mesa lavapipe
/// on the runner, which is a separate decision. Until then the snapshot check
/// is local, and the PR template asks for it.
pub fn ci_steps() -> Vec<Step> {
    vec![
        fmt(FmtMode::Check),
        Step::cargo(
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        Step::cargo("test", &["test", "--workspace", "--all-targets"]),
        Step::cargo("doctest", &["test", "--workspace", "--doc"]),
        Step::cargo("rustdoc", &["doc", "--no-deps"]).with_env("RUSTDOCFLAGS", "-D warnings"),
    ]
}

pub fn run_all(steps: &[Step]) -> Result<()> {
    steps.iter().try_for_each(Step::run)
}

pub fn coverage(format: ReportFormat) -> Result<()> {
    if !process::cargo_subcommand_exists("llvm-cov")? {
        bail!("`cargo-llvm-cov` is not installed; install it with: cargo install cargo-llvm-cov");
    }
    let root = process::workspace_root();
    let (args, report): (&[&str], PathBuf) = match format {
        ReportFormat::Html => (
            &["llvm-cov", "--workspace", "--html"],
            root.join("target/llvm-cov/html/index.html"),
        ),
        ReportFormat::Lcov => (
            &[
                "llvm-cov",
                "--workspace",
                "--lcov",
                "--output-path",
                "lcov.info",
            ],
            root.join("lcov.info"),
        ),
    };
    Step::cargo("coverage", args).run()?;
    println!("coverage report: {}", report.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(steps: &[Step]) -> Vec<String> {
        steps.iter().map(Step::to_string).collect()
    }

    #[test]
    fn ci_runs_expected_steps_in_order() {
        assert_eq!(
            commands(&ci_steps()),
            [
                "cargo fmt --all --check",
                "cargo clippy --workspace --all-targets -- -D warnings",
                "cargo test --workspace --all-targets",
                "cargo test --workspace --doc",
                "RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps",
            ]
        );
    }

    #[test]
    fn ci_step_names_are_unique() {
        let steps = ci_steps();
        let mut names: Vec<_> = steps.iter().map(|step| step.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), steps.len());
    }

    #[test]
    fn fmt_check_differs_from_write_only_by_flag() {
        assert_eq!(fmt(FmtMode::Write).to_string(), "cargo fmt --all");
        assert_eq!(fmt(FmtMode::Check).to_string(), "cargo fmt --all --check");
    }

    #[test]
    fn test_passes_extra_arguments_after_separator() {
        assert_eq!(test(&[]).to_string(), "cargo test --workspace");
        assert_eq!(
            test(&["measure".to_owned()]).to_string(),
            "cargo test --workspace -- measure"
        );
    }

    #[test]
    fn example_passes_release_through() {
        assert_eq!(
            example("hello", Profile::Debug).to_string(),
            "cargo run --example hello"
        );
        assert_eq!(
            example("hello", Profile::Release).to_string(),
            "cargo run --example hello --release"
        );
    }
}
