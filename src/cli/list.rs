use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: Option<&str>, environment: Option<&str>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let entries = store.list(project, environment)?;

    if entries.is_empty() {
        eprintln!("{} No secrets found", "○".yellow());
        return Ok(());
    }

    // Group by project/environment
    let mut current_project = String::new();
    let mut current_env = String::new();

    for entry in entries {
        if entry.project != current_project {
            if !current_project.is_empty() {
                println!();
            }
            current_project = entry.project.clone();
            current_env = String::new();
            println!("📦 {}", entry.project.cyan().bold());
        }

        if entry.environment != current_env {
            current_env = entry.environment.clone();
            println!("  {} {}", "└".dimmed(), entry.environment.yellow());
        }

        let version_str = format!("v{}", entry.version);
        println!(
            "    {} {} {}",
            "•".dimmed(),
            entry.key.bold(),
            version_str.dimmed()
        );
    }

    Ok(())
}
