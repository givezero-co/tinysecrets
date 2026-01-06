use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(
    project: &str,
    environment: &str,
    key: &str,
    limit: usize,
    show_values: bool,
) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    // Get current version info
    let current = store.get(project, environment, key)?;
    let entries = store.history(project, environment, key, limit)?;

    if current.is_none() && entries.is_empty() {
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
        "📜 History for {}/{}/{}",
        project.cyan(),
        environment.yellow(),
        key.bold()
    );
    println!();

    // Show current version first
    if let Some(value) = current {
        let current_version: i32 = entries.first().map(|e| e.version + 1).unwrap_or(1);

        print!(
            "  {} v{} - {} {}",
            "•".dimmed(),
            current_version.to_string().bold(),
            "current".green().bold(),
            "(latest)".dimmed()
        );

        if show_values {
            println!();
            println!("    {}", value.dimmed());
        } else {
            println!();
        }
    }

    // Show history
    for entry in entries {
        let status = if entry.deleted_at.is_some() {
            "deleted".red()
        } else {
            "archived".dimmed()
        };

        let timestamp = entry.created_at.format("%Y-%m-%d %H:%M:%S UTC");

        print!(
            "  {} v{} - {} at {}",
            "•".dimmed(),
            entry.version.to_string().bold(),
            status,
            timestamp.to_string().dimmed()
        );

        if show_values && entry.deleted_at.is_none() {
            if let Ok(Some(value)) = store.get_version(project, environment, key, entry.version) {
                println!();
                println!("    {}", value.dimmed());
            } else {
                println!();
            }
        } else {
            println!();
        }
    }

    if !show_values {
        println!();
        println!("  {} Use {} to show values", "ℹ".blue(), "--show".cyan());
    }

    Ok(())
}
