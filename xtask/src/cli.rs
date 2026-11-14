use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::examples;
use crate::loc;
use crate::snapshots::{self, Mode};
use crate::steps::{self, FmtMode, Profile, ReportFormat, Step};
use crate::web;

/// Project automation for eggshell.
#[derive(Debug, Parser)]
#[command(name = "cargo xtask", bin_name = "cargo xtask")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run everything CI runs, in CI's order, and list the failed steps at
    /// the end: fmt --check, the no-wall-clock check, clippy -D warnings,
    /// tests, doctests, rustdoc -D warnings, wasm32 check, fuzz
    Ci {
        /// Leave out the step of this name; repeatable. CI runs `fuzz` as a
        /// parallel job with `cargo xtask fuzz`
        #[arg(long, value_name = "STEP")]
        skip: Vec<String>,
    },
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
    /// Run the randomised continuity fuzzers with many cases (the `fuzz` step
    /// of `ci`); a failing sequence is shrunk and saved for `cargo xtask test`
    Fuzz {
        /// Cases per fuzz test
        #[arg(long, default_value_t = steps::FUZZ_CASES)]
        cases: u32,
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
    /// Build an example for wasm32 and serve it at a local URL until Ctrl+C
    Web {
        /// Example to build; see `cargo xtask example`
        name: String,
        /// Build with optimizations (slower to build, much faster to run)
        #[arg(long)]
        release: bool,
        /// Port to serve on; 0 picks a free one
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Count lines of code per source file, without comments, blank lines
    /// or `#[cfg(test)]` items
    Loc {
        /// Directories to scan: the library crates by default
        #[arg(default_values = crate::clock::LIBRARY_SOURCES)]
        roots: Vec<std::path::PathBuf>,
    },
    /// Check the golden images against a fresh render, with Mesa lavapipe
    Snapshots {
        /// Regenerate the goldens and list the added, modified and removed ones
        #[arg(long, conflicts_with = "clean")]
        update: bool,
        /// Delete the stray .new.png and .diff.png files of failed checks
        #[arg(long, conflicts_with = "filter")]
        clean: bool,
        /// Only run the snapshot tests whose name contains this
        #[arg(long)]
        filter: Option<String>,
    },
}

impl Command {
    pub fn run(self) -> Result<()> {
        match self {
            Command::Ci { skip } => steps::run_all(&steps::ci_steps_skipping(&skip)?),
            Command::Fmt { check } => {
                let mode = if check {
                    FmtMode::Check
                } else {
                    FmtMode::Write
                };
                steps::run_all(&[steps::fmt(mode)])
            }
            Command::Test { args } => steps::run_all(&[steps::test(&args)]),
            Command::Fuzz { cases } => steps::run_all(&[steps::fuzz(cases)]),
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
            Command::Web {
                name,
                release,
                port,
            } => web::run(&web::Plan {
                example: name,
                profile: if release {
                    Profile::Release
                } else {
                    Profile::Debug
                },
                port,
            }),
            Command::Loc { roots } => loc::run(&roots),
            Command::Snapshots {
                update,
                clean,
                filter,
            } => {
                let mode = match (clean, update) {
                    (true, _) => Mode::Clean,
                    (false, true) => Mode::Update,
                    (false, false) => Mode::Check,
                };
                snapshots::run(&snapshots::Plan { mode, filter })
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
        let Command::Ci { skip } = parse(&["ci"]) else {
            panic!("expected ci")
        };
        assert!(skip.is_empty());
        let Command::Ci { skip } = parse(&["ci", "--skip", "fuzz", "--skip", "wasm"]) else {
            panic!("expected ci")
        };
        assert_eq!(skip, ["fuzz", "wasm"]);
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
    fn fuzz_parses_with_and_without_cases() {
        assert!(matches!(
            parse(&["fuzz"]),
            Command::Fuzz {
                cases: steps::FUZZ_CASES
            }
        ));
        assert!(matches!(
            parse(&["fuzz", "--cases", "50"]),
            Command::Fuzz { cases: 50 }
        ));
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
    fn web_parses_name_release_and_port() {
        let Command::Web {
            name,
            release,
            port,
        } = parse(&["web", "hello"])
        else {
            panic!("expected web")
        };
        assert_eq!((name.as_str(), release, port), ("hello", false, 8080));
        assert!(matches!(
            parse(&["web", "hello", "--release", "--port", "0"]),
            Command::Web {
                release: true,
                port: 0,
                ..
            }
        ));
        assert!(Cli::try_parse_from(["xtask", "web"]).is_err());
    }

    #[test]
    fn snapshots_parses_its_modes_and_rejects_conflicts() {
        let Command::Snapshots {
            update,
            clean,
            filter,
        } = parse(&["snapshots", "--update", "--filter", "nested"])
        else {
            panic!("expected snapshots")
        };
        assert!(update && !clean);
        assert_eq!(filter.as_deref(), Some("nested"));
        assert!(matches!(
            parse(&["snapshots", "--clean"]),
            Command::Snapshots {
                clean: true,
                update: false,
                filter: None
            }
        ));
        assert!(Cli::try_parse_from(["xtask", "snapshots", "--update", "--clean"]).is_err());
        assert!(Cli::try_parse_from(["xtask", "snapshots", "--clean", "--filter", "x"]).is_err());
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(Cli::try_parse_from(["xtask", "bogus"]).is_err());
    }
}
