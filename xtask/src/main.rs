//! Project automation: `cargo xtask --help`.

mod cli;
mod clock;
mod examples;
mod process;
mod snapshots;
mod steps;
mod summary;
#[cfg(test)]
mod testing;
mod web;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
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
