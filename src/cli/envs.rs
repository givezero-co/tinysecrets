use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let envs = store.list_environments(project)?;

    if envs.is_empty() {
        eprintln!(
            "{} No environments found for project '{}'",
            "○".yellow(),
            project.cyan()
        );
        return Ok(());
    }

    println!(
        "{} {} environments:",
        "📦".to_string(),
        project.cyan().bold()
    );
    for env in envs {
        println!("  {} {}", "└".dimmed(), env.yellow());
    }

    Ok(())
}
