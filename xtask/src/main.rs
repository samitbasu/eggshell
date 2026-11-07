//! Project automation: `cargo xtask --help`.

mod cli;
mod examples;
mod process;
mod steps;

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
