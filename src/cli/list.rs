use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: Option<&str>, environment: Option<&str>) -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let has_packs = match (project, environment) {
        (Some(p), Some(e)) => store.has_packs(p, e)?,
        _ => false,
    };

    if has_packs {
        let project = project.unwrap();
        let environment = environment.unwrap();

        let packs = store.pack_list(project, environment)?;

        println!("📦 {}/{}", project.cyan().bold(), environment.yellow());

        for (i, pack) in packs.iter().enumerate() {
            let is_last_pack = i == packs.len() - 1;
            let connector = if is_last_pack { "└─" } else { "├─" };
            let key_label = if pack.key_count == 1 { "key" } else { "keys" };

            println!(
                "  {} {} ({} {})",
                connector.dimmed(),
                pack.name.bold(),
                pack.key_count,
                key_label
            );

            let entries = store.pack_show(project, environment, &pack.name)?;
            let prefix = if is_last_pack { "   " } else { "  │" };
            for entry in &entries {
                println!(
                    "  {}  {} {}  {}",
                    prefix.dimmed(),
                    "•".dimmed(),
                    entry.key,
                    format!("v{}", entry.version).dimmed()
                );
            }
        }

        // Also show remaining flat secrets if any
        let flat_entries = store.list(Some(project), Some(environment))?;
        if !flat_entries.is_empty() {
            println!();
            println!("  {} (flat secrets)", "ungrouped".dimmed());
            for entry in &flat_entries {
                let version_str = format!("v{}", entry.version);
                println!(
                    "    {} {} {}",
                    "•".dimmed(),
                    entry.key.bold(),
                    version_str.dimmed()
                );
            }
        }
    } else {
        // Legacy view — flat secrets only
        let entries = store.list(project, environment)?;

        if entries.is_empty() {
            eprintln!("{} No secrets found", "○".yellow());
            return Ok(());
        }

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
    }

    Ok(())
}
