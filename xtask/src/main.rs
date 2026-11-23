//! Project automation: `cargo xtask --help`.

mod cli;
mod clock;
mod examples;
mod loc;
mod process;
mod shadcn_import;
mod snapshots;
mod steps;
mod summary;
#[cfg(test)]
mod testing;
mod web;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    if let Err(error) = process::ensure_this_worktrees() {
        eprintln!("error: {error:#}");
        return ExitCode::FAILURE;
    }
    let cli = cli::Cli::parse();
    let result = process::install_signal_handler().and_then(|()| cli.command.run());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
