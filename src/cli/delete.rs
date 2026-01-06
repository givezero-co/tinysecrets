use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, key: &str) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    if store.delete(project, environment, key)? {
        eprintln!(
            "{} Deleted {}/{}/{}",
            "✓".green(),
            project.cyan(),
            environment.yellow(),
            key.bold()
        );
    } else {
        eprintln!(
            "{} Secret not found: {}/{}/{}",
            "✗".red(),
            project.cyan(),
            environment.yellow(),
            key.bold()
        );
        std::process::exit(1);
    }

    Ok(())
}
