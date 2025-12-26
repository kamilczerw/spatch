mod cli;

use std::error::Error;

use clap::Parser;
use cli::Cli;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.cmd {
        cli::Command::Query(args) => cli::query::handle_query_command(args)?,
        cli::Command::Diff(diff_args) => cli::diff::handle_diff_command(diff_args)?,
    }

    Ok(())
}
