use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::cli::prompt_passphrase;
use crate::store::{ExportBundle, Store};

pub fn run(input: &str) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let json = fs::read_to_string(input)
        .context(format!("Failed to read input file: {}", input))?;
    
    let bundle: ExportBundle = serde_json::from_str(&json)
        .context("Failed to parse export bundle (invalid format)")?;

    eprintln!(
        "{} Importing {}/{} ({} secrets)...",
        "→".cyan(),
        bundle.project.cyan(),
        bundle.environment.yellow(),
        bundle.secrets.len()
    );

    let imported = store.import(&bundle)?;

    eprintln!(
        "{} Imported {} secrets into {}/{}",
        "✓".green(),
        imported.to_string().bold(),
        bundle.project.cyan(),
        bundle.environment.yellow()
    );

    Ok(())
}

