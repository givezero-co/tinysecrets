//! Compose subcommand handlers

use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_passphrase;
use crate::config::ConfigResolver;
use crate::store::Store;

pub fn run_show(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    reveal: bool,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let compose_list = resolver
        .config()
        .and_then(|c| c.compose.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No compose list in .tinysecrets.toml. Add:\n\n\
                 compose = [\"pack1\", \"pack2\"]"
            )
        })?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    println!(
        "📋 Compose: {}/{}",
        project.cyan().bold(),
        environment.yellow()
    );
    println!();

    let mut total_keys = 0;

    for pack_name in &compose_list {
        let entries = store.pack_show(&project, &environment, pack_name)?;
        if entries.is_empty() {
            println!("{} {} {}", "✗".red(), pack_name.bold(), "(not found)".red());
            continue;
        }

        println!("{}", pack_name.bold());

        if reveal {
            let secrets = store.pack_get_all(&project, &environment, pack_name)?;
            let value_map: std::collections::HashMap<String, String> =
                secrets.into_iter().collect();
            for entry in &entries {
                let val = value_map.get(&entry.key).map(|v| v.as_str()).unwrap_or("?");
                println!("  {} {} = {}", "•".dimmed(), entry.key, val);
            }
        } else {
            for entry in &entries {
                println!("  {} {}", "•".dimmed(), entry.key);
            }
        }

        total_keys += entries.len();
    }

    println!();

    // Check for conflicts
    let result = store.compose(&project, &environment, &compose_list)?;
    if result.conflicts.is_empty() {
        println!(
            "Total: {} env vars from {} packs",
            total_keys.to_string().bold(),
            compose_list.len()
        );
        println!("{} No conflicts", "✓".green());
    } else {
        println!(
            "Total: {} env vars from {} packs",
            total_keys.to_string().bold(),
            compose_list.len()
        );
        for conflict in &result.conflicts {
            println!(
                "{} Key conflict: {} defined in {}",
                "✗".red(),
                conflict.key.bold(),
                conflict.packs.join(", ")
            );
        }
    }

    Ok(())
}

pub fn run_check(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let compose_list = resolver
        .config()
        .and_then(|c| c.compose.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No compose list in .tinysecrets.toml. Add:\n\n\
                 compose = [\"pack1\", \"pack2\"]"
            )
        })?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let mut all_ok = true;
    let mut total_keys = 0;

    // Check all packs exist
    for pack_name in &compose_list {
        let entries = store.pack_show(&project, &environment, pack_name)?;
        if entries.is_empty() {
            eprintln!(
                "{} Pack '{}' not found in {}/{}",
                "✗".red(),
                pack_name.bold(),
                project.cyan(),
                environment.yellow()
            );
            all_ok = false;
        } else {
            total_keys += entries.len();
        }
    }

    // Check for conflicts
    let result = store.compose(&project, &environment, &compose_list);
    match result {
        Ok(r) => {
            if !r.conflicts.is_empty() {
                for conflict in &r.conflicts {
                    eprintln!(
                        "{} Key conflict: {} defined in both {}",
                        "✗".red(),
                        conflict.key.bold(),
                        conflict.packs.join(" and ")
                    );
                }
                all_ok = false;
            }
        }
        Err(e) => {
            eprintln!("{} {}", "✗".red(), e);
            all_ok = false;
        }
    }

    if all_ok {
        eprintln!("{} All {} packs exist", "✓".green(), compose_list.len());
        eprintln!("{} No key conflicts", "✓".green());
        eprintln!(
            "{} {} env vars will be injected",
            "✓".green(),
            total_keys.to_string().bold()
        );
    } else {
        std::process::exit(1);
    }

    Ok(())
}
