use anyhow::{Context, Result};
use colored::Colorize;
use std::fs::File;
use std::io::Write;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, output: Option<&str>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let bundle = store.export(project, environment)?;
    let json = serde_json::to_string_pretty(&bundle)?;

    match output {
        Some(path) => {
            let mut file = File::create(path)
                .context(format!("Failed to create output file: {}", path))?;
            file.write_all(json.as_bytes())?;
            
            eprintln!(
                "{} Exported {} secrets to {}",
                "✓".green(),
                bundle.secrets.len().to_string().bold(),
                path.cyan()
            );
            eprintln!(
                "{} Bundle is encrypted with your passphrase",
                "ℹ".blue()
            );
        }
        None => {
            // Output to stdout for piping
            println!("{}", json);
        }
    }

    Ok(())
}

