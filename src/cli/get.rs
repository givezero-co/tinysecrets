use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, key: &str) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    match store.get(project, environment, key)? {
        Some(value) => {
            // Print just the value so it can be used in scripts: $(ts get ...)
            println!("{}", value);
        }
        None => {
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
