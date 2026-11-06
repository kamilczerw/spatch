mod cli;

use std::{error::Error, process::ExitCode};

use clap::Parser;
use cli::Cli;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // `Display`, not `Debug`: returning the error from `main` would print a Rust dump.
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.cmd {
        cli::Command::Query(args) => cli::query::handle_query_command(args)?,
        cli::Command::Diff(diff_args) => cli::diff::handle_diff_command(diff_args)?,
    }

    Ok(())
}
