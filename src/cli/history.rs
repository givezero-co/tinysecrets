use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, key: &str, limit: usize) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let entries = store.history(project, environment, key, limit)?;

    if entries.is_empty() {
        eprintln!(
            "{} No history found for {}/{}/{}",
            "○".yellow(),
            project.cyan(),
            environment.yellow(),
            key.bold()
        );
        return Ok(());
    }

    println!(
        "{} History for {}/{}/{}",
        "📜",
        project.cyan(),
        environment.yellow(),
        key.bold()
    );
    println!();

    for entry in entries {
        let status = if entry.deleted_at.is_some() {
            "deleted".red()
        } else {
            "updated".green()
        };

        let timestamp = entry.created_at.format("%Y-%m-%d %H:%M:%S UTC");

        println!(
            "  {} v{} - {} at {}",
            "•".dimmed(),
            entry.version.to_string().bold(),
            status,
            timestamp.to_string().dimmed()
        );
    }

    Ok(())
}

