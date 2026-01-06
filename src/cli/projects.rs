use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run() -> Result<()> {
    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let projects = store.list_projects()?;

    if projects.is_empty() {
        eprintln!("{} No projects found", "○".yellow());
        eprintln!("  Create your first secret with: ts set <project> <env> <key>");
        return Ok(());
    }

    println!("{}", "Projects:".bold());
    for project in projects {
        println!("  {} {}", "📦".to_string(), project.cyan());
    }

    Ok(())
}

