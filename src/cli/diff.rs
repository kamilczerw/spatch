use spatch::diff::diff;

use crate::cli::{DiffArgs, read::load_json_file};

pub fn handle_diff_command(args: DiffArgs) -> Result<(), Box<dyn std::error::Error>> {
    let file1 = load_json_file(&args.file1)?;
    let file2 = load_json_file(&args.file2)?;
    let schema = if let Some(schema_path) = args.schema {
        Some(&load_json_file(&schema_path)?)
    } else {
        None
    };

    let result = diff(&file1, &file2, schema);

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
