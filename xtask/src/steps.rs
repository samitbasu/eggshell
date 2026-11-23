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
    pub expect: Expect,
}

/// What a successful step must also show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Success,
    /// A filter that matches nothing passes silently; this fails it.
    SomeTestPassed,
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
            expect: Expect::Success,
        }
    }

    pub fn internal(name: &'static str, check: fn() -> Result<()>) -> Self {
        Self {
            name,
            args: Vec::new(),
            env: Vec::new(),
            internal: Some(check),
            expect: Expect::Success,
        }
    }

    /// Points Vulkan at the lavapipe driver whose manifest is `icd`.
    pub fn on_lavapipe(self, icd: &Path) -> Self {
        let icd = icd.display().to_string();
        self.with_env("VK_ICD_FILENAMES", icd.clone())
            .with_env("VK_DRIVER_FILES", icd)
    }

    pub fn expecting_tests(mut self) -> Self {
        self.expect = Expect::SomeTestPassed;
        self
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
        if self.expect == Expect::SomeTestPassed && summary::tests_passed(&output) == 0 {
            bail!("step `{}` ran no tests; its filter matches none", self.name);
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

/// Cases per fuzz test for `ci` and `cargo xtask fuzz`.
pub const FUZZ_CASES: u32 = 500;

pub fn fuzz(cases: u32) -> Step {
    Step::cargo(
        "fuzz",
        &["test", "--test", "it", "fuzz::", "--profile", "fuzz"],
    )
    .with_env("PROPTEST_CASES", cases.to_string())
    .expecting_tests()
}

/// The profile of the [`release`] step: dev without debug assertions.
pub const RELEASE_GATES: &str = "release-gates";

/// The tests of what a release build does (#336), in a build without debug
/// assertions: the checks gated on them are proven off. They live in the
/// integration tests' `release` module, which a debug build ignores.
pub fn release() -> Step {
    Step::cargo(
        "release",
        &[
            "test",
            "--test",
            "it",
            "release::",
            "--profile",
            RELEASE_GATES,
        ],
    )
    .expecting_tests()
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

/// The `screenshots` feature's tests and doctests (#337), which render on
/// lavapipe. The `snapshots` feature carries it, so the build is the test step's.
#[cfg(target_os = "linux")]
pub fn screenshots() -> Step {
    Step::internal("screenshots", || {
        screenshots_step(&snapshots::lavapipe_icd()?).run()
    })
}

/// The child sees only the lavapipe driver, as the goldens' does: enumerating
/// every adapter in-process loads the GPU's driver too, which can crash when
/// runs overlap (#367).
#[cfg(target_os = "linux")]
fn screenshots_step(icd: &Path) -> Step {
    Step::cargo(
        "screenshots",
        &[
            "test",
            "--workspace",
            "--features",
            "eggshell-ui/snapshots",
            "--",
            "screenshot",
        ],
    )
    .on_lavapipe(icd)
    .expecting_tests()
}

#[cfg(not(target_os = "linux"))]
pub fn screenshots() -> Step {
    Step::internal("screenshots", || {
        eprintln!("screenshots: skipped, the feature is Linux-only for now (#337)");
        Ok(())
    })
}

/// The crates with optional features, built with none of them: what is
/// gated on a feature must build, and lint, without it.
pub fn features() -> Step {
    Step::cargo(
        "features",
        &[
            "clippy",
            "-p",
            "eggshell-ui-shadcn",
            "-p",
            "eggshell-ui-extras",
            "--no-default-features",
            "--",
            "-D",
            "warnings",
        ],
    )
}

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
                "--features",
                "eggshell-ui/snapshots",
                "--",
                "-D",
                "warnings",
            ],
        ),
        features(),
        Step::internal("shadcn-sheets", crate::shadcn_import::check),
        Step::internal("test", snapshots::test_with_goldens),
        Step::cargo("doctest", &["test", "--workspace", "--doc"]),
        Step::cargo("rustdoc", &["doc", "--workspace", "--no-deps"])
            .with_env("RUSTDOCFLAGS", "-D warnings"),
        screenshots(),
        wasm(),
        release(),
        fuzz(FUZZ_CASES),
    ]
}

/// CI's steps an integration leaves to CI, for their time.
const LEFT_TO_CI: [&str; 2] = ["wasm", "fuzz"];

/// What `cargo xtask integrate` runs: CI's steps but the slowest, in CI's
/// order, so an item is checked as CI checks it.
pub fn integrate_steps() -> Vec<Step> {
    ci_steps()
        .into_iter()
        .filter(|step| !LEFT_TO_CI.contains(&step.name))
        .collect()
}

/// The CI steps without those named in `skip`, so a runner can split them
/// across parallel jobs; a name that is not a step is an error.
pub fn ci_steps_skipping(skip: &[String]) -> Result<Vec<Step>> {
    let steps = ci_steps();
    if let Some(unknown) = skip
        .iter()
        .find(|name| !steps.iter().any(|step| step.name == name.as_str()))
    {
        let names: Vec<_> = steps.iter().map(|step| step.name).collect();
        bail!(
            "no CI step is named `{unknown}`; the steps are: {}",
            names.join(", ")
        );
    }
    Ok(steps
        .into_iter()
        .filter(|step| !skip.iter().any(|name| name == step.name))
        .collect())
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
            process::target_dir().join("llvm-cov/html/index.html"),
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
                "cargo clippy --workspace --all-targets --features eggshell-ui/snapshots -- -D warnings",
                "cargo clippy -p eggshell-ui-shadcn -p eggshell-ui-extras --no-default-features -- -D warnings",
                "built-in shadcn-sheets check",
                "built-in test check",
                "cargo test --workspace --doc",
                "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps",
                screenshots().to_string().as_str(),
                "cargo check --target wasm32-unknown-unknown --workspace --exclude xtask --lib --examples",
                "cargo test --test it release:: --profile release-gates",
                "PROPTEST_CASES=\"500\" cargo test --test it fuzz:: --profile fuzz",
            ]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn screenshots_select_lavapipe_for_the_child() {
        let step = screenshots_step(Path::new("/icd/lvp.json"));
        assert_eq!(
            step.to_string(),
            "VK_ICD_FILENAMES=\"/icd/lvp.json\" VK_DRIVER_FILES=\"/icd/lvp.json\" \
             cargo test --workspace --features eggshell-ui/snapshots -- screenshot"
        );
        assert_eq!(step.expect, Expect::SomeTestPassed);
    }

    #[test]
    fn ci_skips_the_named_steps_only() {
        let skip = ["fuzz".to_owned(), "wasm".to_owned()];
        let names: Vec<_> = ci_steps_skipping(&skip)
            .unwrap()
            .iter()
            .map(|step| step.name)
            .collect();
        assert_eq!(
            names,
            [
                "fmt",
                "clock",
                "clippy",
                "features",
                "shadcn-sheets",
                "test",
                "doctest",
                "rustdoc",
                "screenshots",
                "release"
            ]
        );
        assert_eq!(ci_steps_skipping(&[]).unwrap().len(), ci_steps().len());
    }

    #[test]
    fn the_release_and_fuzz_steps_fail_when_their_filters_match_no_test() {
        assert_eq!(release().expect, Expect::SomeTestPassed);
        assert_eq!(fuzz(FUZZ_CASES).expect, Expect::SomeTestPassed);
    }

    #[test]
    fn integrate_runs_cis_steps_but_the_slowest_in_cis_order() {
        let skip = LEFT_TO_CI.map(str::to_owned);
        assert_eq!(
            commands(&integrate_steps()),
            commands(&ci_steps_skipping(&skip).unwrap())
        );
        assert!(
            integrate_steps().iter().any(|step| step.name == "release"),
            "the release gates are built"
        );
    }

    #[test]
    fn ci_rejects_skipping_an_unknown_step() {
        let error = ci_steps_skipping(&["fuz".to_owned()]).unwrap_err();
        assert!(error.to_string().contains("`fuz`"), "{error}");
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
