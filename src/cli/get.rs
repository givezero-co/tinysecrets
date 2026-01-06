use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, key: &str, version: Option<i32>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let value = match version {
        Some(v) => store.get_version(project, environment, key, v)?,
        None => store.get(project, environment, key)?,
    };

    match value {
        Some(val) => {
            // Print just the value so it can be used in scripts: $(ts get ...)
            println!("{}", val);
        }
        None => {
            let version_str = version.map(|v| format!(" (v{})", v)).unwrap_or_default();
            eprintln!(
                "{} Secret not found: {}/{}/{}{}",
                "✗".red(),
                project.cyan(),
                environment.yellow(),
                key.bold(),
                version_str.dimmed()
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
