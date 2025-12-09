mod cli;

use std::{error::Error, io::Read};

use clap::Parser;
use cli::Cli;
use spatch::resolve::SerdeValueExt;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let json = if let Some(file_path) = cli.file {
        load_json_file(&file_path)?
    } else {
        read_from_stdin()?
    };

    json.get_value_at(&cli.path)
        .map(|value| {
            println!("{}", value);
        })
        .map_err(|e| {
            eprintln!("Error: {}", e);
            e
        })?;
    Ok(())
}

fn load_json_file(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn Error>> {
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
