use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// Path expression to resolve within the JSON structure
    /// The path format is similar to JSONPath, e.g., /store/book[category=fiction]/title
    /// It supports field access and filtering based on key-value pairs.
    ///
    /// The provided path MUST resolve to a single value; otherwise, an error will be returned.
    pub path: String,

    /// Path to the JSON file to be processed
    pub file: Option<PathBuf>,
}
