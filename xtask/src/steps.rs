use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::{clock, process, snapshots, summary};

/// One cargo invocation. Every task is a list of these, so each is announced
/// and failure-reported the same way.
#[derive(Debug, Clone)]
pub struct Step {
    pub name: &'static str,
    pub args: Vec<String>,
    pub env: Vec<(&'static str, String)>,
    /// A check run in this process instead of a cargo invocation.
    pub internal: Option<fn() -> Result<()>>,
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
            internal: None,
        }
    }

    pub fn internal(name: &'static str, check: fn() -> Result<()>) -> Self {
        Self {
            name,
            args: Vec::new(),
            env: Vec::new(),
            internal: Some(check),
        }
    }

    /// Points Vulkan at the lavapipe driver whose manifest is `icd`.
    pub fn on_lavapipe(self, icd: &Path) -> Self {
        let icd = icd.display().to_string();
        self.with_env("VK_ICD_FILENAMES", icd.clone())
            .with_env("VK_DRIVER_FILES", icd)
    }

    pub fn with_env(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.env.push((key, value.into()));
        self
    }

    pub fn run(&self) -> Result<()> {
        eprintln!("==> {}: {self}", self.name);
        if let Some(check) = self.internal {
            return check().with_context(|| format!("{} failed", self.name));
        }
        let (status, output) = process::run_cargo(&self.args, &self.env)?;
        if !status.success() {
            bail!(
                "{}",
                summary::failure_line(self.name, &status.to_string(), &output)
            );
        }
        Ok(())
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.internal.is_some() {
            return write!(f, "built-in {} check", self.name);
        }
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

/// Cases per fuzz test for `cargo xtask fuzz`; `ci` runs the tests' small default.
pub const FUZZ_CASES: u32 = 2000;

pub fn fuzz(cases: u32) -> Step {
    Step::cargo("fuzz", &["test", "--test", "it", "fuzz::", "--release"])
        .with_env("PROPTEST_CASES", cases.to_string())
}

pub fn example(name: &str, profile: Profile) -> Step {
    let mut step = Step::cargo("example", &["run", "--example", name]);
    if profile == Profile::Release {
        step.args.push("--release".to_owned());
    }
    step
}

/// Every library crate and every example must build for the browser.
pub fn wasm() -> Step {
    Step::cargo(
        "wasm",
        &[
            "check",
            "--target",
            WASM_TARGET,
            "--workspace",
            "--exclude",
            "xtask",
            "--lib",
            "--examples",
        ],
    )
}

pub const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// What CI runs, in order. The test step carries the golden check, which needs
/// Mesa lavapipe, locally and on the runner alike.
pub fn ci_steps() -> Vec<Step> {
    vec![
        fmt(FmtMode::Check),
        Step::internal("clock", clock::check_library),
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
        Step::internal("test", snapshots::test_with_goldens),
        Step::cargo("doctest", &["test", "--workspace", "--doc"]),
        Step::cargo("rustdoc", &["doc", "--workspace", "--no-deps"])
            .with_env("RUSTDOCFLAGS", "-D warnings"),
        wasm(),
    ]
}

/// Runs every step, so a failure does not hide the next one, and ends with
/// one line per failed step, which is the error `main` prints last.
pub fn run_all(steps: &[Step]) -> Result<()> {
    let failures: Vec<String> = steps
        .iter()
        .filter_map(|step| step.run().err())
        .map(|error| format!("{error:#}"))
        .collect();
    if failures.is_empty() {
        return Ok(());
    }
    eprintln!("==> {} of {} step(s) failed", failures.len(), steps.len());
    bail!("{}", failures.join("\n       "))
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
                "built-in clock check",
                "cargo clippy --workspace --all-targets -- -D warnings",
                "built-in test check",
                "cargo test --workspace --doc",
                "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps",
                "cargo check --target wasm32-unknown-unknown --workspace --exclude xtask --lib --examples",
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
    fn fuzz_raises_the_case_count() {
        let step = fuzz(7);
        assert!(step.args.contains(&"fuzz::".to_owned()));
        assert_eq!(step.env, [("PROPTEST_CASES", "7".to_owned())]);
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
