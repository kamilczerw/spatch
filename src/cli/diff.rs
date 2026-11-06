use spatch::diff::{DiffOptions, diff};

use crate::cli::{DiffArgs, query::load_json_file};

pub fn handle_diff_command(args: DiffArgs) -> Result<(), Box<dyn std::error::Error>> {
    let file1 = load_json_file(&args.file1)?;
    let file2 = load_json_file(&args.file2)?;
    let schema = if let Some(schema_path) = args.schema {
        Some(&load_json_file(&schema_path)?)
    } else {
        None
    };

    let diff_options = DiffOptions::new().with_granularity(args.granularity.into());
    let diff_options = match schema {
        Some(schema) => diff_options.with_schema(schema),
        None => diff_options,
    };

    let result = diff(&file1, &file2, diff_options)?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
