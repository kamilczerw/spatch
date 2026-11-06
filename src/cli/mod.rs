pub mod diff;
pub mod query;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use spatch::diff::DiffGranularity;

#[derive(Debug, Parser)]
#[command(name = "spatch", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Resolve a path within a JSON structure
    Query(QueryArgs),

    /// Diff two JSON files and output the differences
    Diff(DiffArgs),
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    /// Path expression to resolve within the JSON structure
    /// The path format is similar to JSONPath, e.g., /store/book[category=fiction]/title
    /// It supports field access and filtering based on key-value pairs.
    ///
    /// The provided path MUST resolve to a single value; otherwise, an error will be returned.
    pub path: String,

    /// Path to the JSON file to be processed
    pub file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct DiffArgs {
    /// Path to the first JSON file for comparison
    pub file1: PathBuf,

    /// Path to the second JSON file for comparison
    pub file2: PathBuf,

    /// Path to the optional JSON Schema file for validation and generating semantic paths
    #[arg(short, long)]
    pub schema: Option<PathBuf>,

    /// How aggressively to collapse changes into larger operations.
    /// `compact` may replace a whole object when that is smaller; `granular` keeps nested operations.
    #[arg(long, value_enum, default_value_t = Granularity::Compact)]
    pub granularity: Granularity,
}

/// CLI spelling of [`spatch::diff::DiffGranularity`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Granularity {
    Compact,
    Granular,
}

impl From<Granularity> for DiffGranularity {
    fn from(value: Granularity) -> Self {
        match value {
            Granularity::Compact => DiffGranularity::Compact,
            Granularity::Granular => DiffGranularity::Granular,
        }
    }
}
