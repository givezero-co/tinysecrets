use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::{KeyLocation, Store};

pub fn run(project: &str, environment: &str, key: &str) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    match store.find_key_across_packs(project, environment, key)? {
        KeyLocation::InPack { pack_name } => {
            store.pack_delete_key(project, environment, &pack_name, key)?;
            eprintln!(
                "{} Deleted {}/{}/{} from pack '{}'",
                "✓".green(),
                project.cyan(),
                environment.yellow(),
                key.bold(),
                pack_name
            );
        }
        KeyLocation::InFlatSecrets => {
            if store.delete(project, environment, key)? {
                eprintln!(
                    "{} Deleted {}/{}/{}",
                    "✓".green(),
                    project.cyan(),
                    environment.yellow(),
                    key.bold()
                );
            }
        }
        KeyLocation::InMultiplePacks { pack_names } => {
            eprintln!(
                "{} '{}' found in multiple packs: {}",
                "✗".red(),
                key.bold(),
                pack_names.join(", ")
            );
            eprintln!(
                "  Delete from a specific pack: {} {} {}",
                "ts pack delete".dimmed(),
                pack_names[0],
                key,
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
