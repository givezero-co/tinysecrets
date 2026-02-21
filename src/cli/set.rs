use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::{KeyLocation, Store};

pub fn run(project: &str, environment: &str, key: &str, value: Option<&str>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let secret_value = match value {
        Some(v) => v.to_string(),
        None => {
            let template = format!(
                "# Enter the value for {}/{}/{}\n# Lines starting with # will be ignored\n",
                project, environment, key
            );

            let edited = edit::edit(&template)
                .context("Failed to open editor. Set $EDITOR or pass value directly.")?;

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

    // Search across packs and flat secrets for existing key
    match store.find_key_across_packs(project, environment, key)? {
        KeyLocation::InPack { pack_name } => {
            let ver = store.pack_set(project, environment, &pack_name, key, &secret_value)?;
            eprintln!(
                "{} Updated {}/{}/{} in pack '{}' (v{})",
                "✓".green(),
                project.cyan(),
                environment.yellow(),
                key.bold(),
                pack_name,
                ver
            );
        }
        KeyLocation::InFlatSecrets => {
            store.set(project, environment, key, &secret_value, None)?;
            eprintln!(
                "{} Updated {}/{}/{}",
                "✓".green(),
                project.cyan(),
                environment.yellow(),
                key.bold()
            );
        }
        KeyLocation::InMultiplePacks { pack_names } => {
            eprintln!(
                "{} '{}' found in multiple packs: {}",
                "✗".red(),
                key.bold(),
                pack_names.join(", ")
            );
            eprintln!(
                "  Use: {} {} {}=VALUE",
                "ts pack set".dimmed(),
                pack_names[0],
                key,
            );
            std::process::exit(1);
        }
        KeyLocation::NotFound => {
            // Check if packs exist — if so, guide to use pack set
            if store.has_packs(project, environment)? {
                eprintln!("{} '{}' not found in any pack", "✗".red(), key.bold(),);
                eprintln!(
                    "  Add to a pack: {} <pack> {}=VALUE",
                    "ts pack set".dimmed(),
                    key
                );
                std::process::exit(1);
            }

            // No packs at all — create as flat secret (legacy behavior)
            store.set(project, environment, key, &secret_value, None)?;
            eprintln!(
                "{} Created {}/{}/{}",
                "✓".green(),
                project.cyan(),
                environment.yellow(),
                key.bold()
            );
        }
    }

    Ok(())
}
