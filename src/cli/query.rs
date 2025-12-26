use std::{error::Error, io::Read};

use spatch::resolve::SerdeValueExt;

use crate::cli::QueryArgs;

pub fn handle_query_command(args: QueryArgs) -> Result<(), Box<dyn Error>> {
    let json = if let Some(file_path) = args.file {
        load_json_file(&file_path)?
    } else {
        read_from_stdin()?
    };

    json.get_value_at(&args.path)
        .map(|value| {
            println!("{}", value);
        })
        .map_err(|e| {
            eprintln!("Error: {}", e);
            e
        })?;
    Ok(())
    // Implementation for the read command goes here
}

pub(super) fn load_json_file(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let data = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&data)?;
    Ok(json)
}

fn read_from_stdin() -> Result<serde_json::Value, Box<dyn Error>> {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    let json: serde_json::Value = serde_json::from_str(&buffer)?;
    Ok(json)
}
