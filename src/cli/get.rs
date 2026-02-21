use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::{KeyLocation, Store};

pub fn run(project: &str, environment: &str, key: &str, version: Option<i32>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    if let Some(v) = version {
        // Versioned access: only from flat secrets (pack history uses ts pack history)
        match store.get_version(project, environment, key, v)? {
            Some(val) => {
                println!("{}", val);
                return Ok(());
            }
            None => {
                eprintln!(
                    "{} Secret not found: {}/{}/{} (v{})",
                    "✗".red(),
                    project.cyan(),
                    environment.yellow(),
                    key.bold(),
                    v
                );
                std::process::exit(1);
            }
        }
    }

    // Search across packs and flat secrets
    match store.find_key_across_packs(project, environment, key)? {
        KeyLocation::InPack { pack_name } => {
            let val = store
                .pack_get(project, environment, &pack_name, key)?
                .unwrap();
            println!("{}", val);
        }
        KeyLocation::InFlatSecrets => {
            let val = store.get(project, environment, key)?.unwrap();
            println!("{}", val);
        }
        KeyLocation::InMultiplePacks { pack_names } => {
            eprintln!(
                "{} '{}' found in multiple packs: {}",
                "✗".red(),
                key.bold(),
                pack_names.join(", ")
            );
            eprintln!(
                "  Use: {} {} {} {}",
                "ts pack get".dimmed(),
                pack_names[0],
                key,
                "(specify the pack)".dimmed()
            );
            std::process::exit(1);
        }
        KeyLocation::NotFound => {
            eprintln!(
                "{} Secret not found: {}/{}/{}",
                "✗".red(),
                project.cyan(),
                environment.yellow(),
                key.bold()
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
