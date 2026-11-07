use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::examples;
use crate::steps::{self, FmtMode, Profile, ReportFormat, Step};

/// Project automation for bullseye.
#[derive(Debug, Parser)]
#[command(name = "cargo xtask", bin_name = "cargo xtask")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run everything CI runs, in CI's order, stopping at the first failure:
    /// fmt --check, clippy -D warnings, tests, doctests, rustdoc -D warnings
    Ci,
    /// Format the workspace with `cargo fmt --all`
    Fmt {
        /// Verify formatting without writing
        #[arg(long)]
        check: bool,
    },
    /// Run all tests and doctests with `cargo test --workspace`
    Test {
        /// Arguments passed to `cargo test` after `--`, for example a name filter
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Measure coverage with `cargo llvm-cov` and write an HTML report
    Coverage {
        /// Write `lcov.info` instead of an HTML report
        #[arg(long)]
        lcov: bool,
    },
    /// Run an example with `cargo run --example NAME`, or list the examples
    Example {
        /// Example to run; omit to list the examples
        name: Option<String>,
        /// Build with optimizations
        #[arg(long)]
        release: bool,
    },
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Command::Ci => steps::run_all(&steps::ci_steps()),
            Command::Fmt { check } => {
                let mode = if check {
                    FmtMode::Check
                } else {
                    FmtMode::Write
                };
                steps::run_all(&[steps::fmt(mode)])
            }
            Command::Test { args } => steps::run_all(&[steps::test(&args)]),
            Command::Coverage { lcov } => {
                let format = if lcov {
                    ReportFormat::Lcov
                } else {
                    ReportFormat::Html
                };
                steps::coverage(format)
            }
            Command::Example { name: None, .. } => examples::print_listing(),
            Command::Example {
                name: Some(name),
                release,
            } => {
                let profile = if release {
                    Profile::Release
                } else {
                    Profile::Debug
                };
                Step::run(&steps::example(&name, profile))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Command {
        let argv = ["xtask"].into_iter().chain(args.iter().copied());
        Cli::try_parse_from(argv).expect("arguments parse").command
    }

    #[test]
    fn command_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn ci_parses() {
        assert!(matches!(parse(&["ci"]), Command::Ci));
    }

    #[test]
    fn fmt_parses_with_and_without_check() {
        assert!(matches!(parse(&["fmt"]), Command::Fmt { check: false }));
        assert!(matches!(
            parse(&["fmt", "--check"]),
            Command::Fmt { check: true }
        ));
    }

    #[test]
    fn test_collects_arguments_after_double_dash() {
        let Command::Test { args } = parse(&["test"]) else {
            panic!("expected test")
        };
        assert!(args.is_empty());
        let Command::Test { args } = parse(&["test", "--", "measure", "--nocapture"]) else {
            panic!("expected test")
        };
        assert_eq!(args, ["measure", "--nocapture"]);
    }

    #[test]
    fn coverage_parses_with_and_without_lcov() {
        assert!(matches!(
            parse(&["coverage"]),
            Command::Coverage { lcov: false }
        ));
        assert!(matches!(
            parse(&["coverage", "--lcov"]),
            Command::Coverage { lcov: true }
        ));
    }

    #[test]
    fn example_parses_name_and_release() {
        assert!(matches!(
            parse(&["example"]),
            Command::Example {
                name: None,
                release: false
            }
        ));
        let Command::Example { name, release } = parse(&["example", "hello", "--release"]) else {
            panic!("expected example")
        };
        assert_eq!(name.as_deref(), Some("hello"));
        assert!(release);
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["xtask", "bogus"]).is_err());
    }
}
