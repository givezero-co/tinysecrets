use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, key: &str, value: Option<&str>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let secret_value = match value {
        Some(v) => v.to_string(),
        None => {
            // Open editor for multiline/sensitive input
            let template = format!(
                "# Enter the value for {}/{}/{}\n# Lines starting with # will be ignored\n",
                project, environment, key
            );
            
            let edited = edit::edit(&template)
                .context("Failed to open editor. Set $EDITOR or pass value directly.")?;
            
            // Filter out comments and trim
            edited
                .lines()
                .filter(|line| !line.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        }
    };

    if secret_value.is_empty() {
        anyhow::bail!("Secret value cannot be empty");
    }

    // Check if updating existing
    let existing = store.get(project, environment, key)?;
    
    store.set(project, environment, key, &secret_value, None)?;

    if existing.is_some() {
        eprintln!(
            "{} Updated {}/{}/{}",
            "✓".green(),
            project.cyan(),
            environment.yellow(),
            key.bold()
        );
    } else {
        eprintln!(
            "{} Created {}/{}/{}",
            "✓".green(),
            project.cyan(),
            environment.yellow(),
            key.bold()
        );
    }

    Ok(())
}

