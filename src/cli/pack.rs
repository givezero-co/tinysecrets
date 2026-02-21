//! Pack subcommand handlers

use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, IsTerminal, Write};

use crate::cli::prompt_passphrase;
use crate::config::{Config, ConfigResolver};
use crate::keypath;
use crate::store::Store;

pub fn run_set(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_input: &str,
    entries: &[String],
) -> Result<()> {
    let kp = keypath::parse_keypath(
        pack_input,
        cli_project.or_else(|| resolver.config().and_then(|c| c.project.as_deref())),
        cli_env.or_else(|| resolver.config().and_then(|c| c.environment.as_deref())),
    )?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    if entries.len() == 1 && !entries[0].contains('=') {
        let key = &entries[0];
        let template = format!(
            "# Enter the value for {}/{}/{} [pack: {}]\n# Lines starting with # will be ignored\n",
            kp.project, kp.environment, key, kp.pack
        );
        let edited = edit::edit(&template)
            .context("Failed to open editor. Set $EDITOR or pass KEY=VALUE.")?;
        let value: String = edited
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if value.is_empty() {
            anyhow::bail!("Secret value cannot be empty");
        }

        let ver = store.pack_set(&kp.project, &kp.environment, &kp.pack, key, &value)?;
        eprintln!(
            "{} Set {}/{}/{} [{}] (v{})",
            "✓".green(),
            kp.project.cyan(),
            kp.environment.yellow(),
            key.bold(),
            kp.pack.dimmed(),
            ver
        );
    } else {
        for entry in entries {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("Invalid format '{}'. Use KEY=VALUE", entry))?;

            if key.is_empty() || value.is_empty() {
                anyhow::bail!("Key and value cannot be empty in '{}'", entry);
            }

            let ver = store.pack_set(&kp.project, &kp.environment, &kp.pack, key, value)?;
            eprintln!("  {} {} (v{})", "✓".green(), key.bold(), ver);
        }
        eprintln!(
            "{} Set {} keys in pack '{}' ({}/{})",
            "✓".green(),
            entries.len(),
            kp.pack.bold(),
            kp.project.cyan(),
            kp.environment.yellow()
        );
    }

    Ok(())
}

pub fn run_get(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_input: &str,
    key: &str,
) -> Result<()> {
    let kp = keypath::parse_keypath(
        pack_input,
        cli_project.or_else(|| resolver.config().and_then(|c| c.project.as_deref())),
        cli_env.or_else(|| resolver.config().and_then(|c| c.environment.as_deref())),
    )?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    match store.pack_get(&kp.project, &kp.environment, &kp.pack, key)? {
        Some(val) => println!("{}", val),
        None => {
            eprintln!(
                "{} Key '{}' not found in pack '{}'",
                "✗".red(),
                key.bold(),
                kp.pack.bold()
            );
            std::process::exit(1);
        }
    }

    Ok(())
}

pub fn run_show(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_input: &str,
    reveal: bool,
) -> Result<()> {
    let kp = keypath::parse_keypath(
        pack_input,
        cli_project.or_else(|| resolver.config().and_then(|c| c.project.as_deref())),
        cli_env.or_else(|| resolver.config().and_then(|c| c.environment.as_deref())),
    )?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let entries = store.pack_show(&kp.project, &kp.environment, &kp.pack)?;

    if entries.is_empty() {
        eprintln!("{} Pack '{}' not found or empty", "○".yellow(), kp.pack);
        return Ok(());
    }

    println!(
        "📦 {} ({}/{})",
        kp.pack.bold(),
        kp.project.cyan(),
        kp.environment.yellow()
    );

    if reveal {
        let secrets = store.pack_get_all(&kp.project, &kp.environment, &kp.pack)?;
        let value_map: std::collections::HashMap<String, String> = secrets.into_iter().collect();
        for entry in &entries {
            let val = value_map.get(&entry.key).map(|v| v.as_str()).unwrap_or("?");
            println!(
                "  {} {} = {}  {}",
                "•".dimmed(),
                entry.key.bold(),
                val,
                format!("v{}", entry.version).dimmed()
            );
        }
    } else {
        for entry in &entries {
            println!(
                "  {} {}  {}",
                "•".dimmed(),
                entry.key.bold(),
                format!("v{}", entry.version).dimmed()
            );
        }
    }

    Ok(())
}

pub fn run_list(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    show_keys: bool,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let packs = store.pack_list(&project, &environment)?;

    if packs.is_empty() {
        eprintln!(
            "{} No packs found in {}/{}",
            "○".yellow(),
            project.cyan(),
            environment.yellow()
        );
        return Ok(());
    }

    println!("📦 {}/{}", project.cyan().bold(), environment.yellow());

    // Build hierarchical view: group by prefix before first '.'
    for (i, pack) in packs.iter().enumerate() {
        let is_last = i == packs.len() - 1;
        let connector = if is_last { "└─" } else { "├─" };
        let key_label = if pack.key_count == 1 { "key" } else { "keys" };

        println!(
            "  {} {} ({} {})",
            connector.dimmed(),
            pack.name.bold(),
            pack.key_count,
            key_label
        );

        if show_keys {
            let entries = store.pack_show(&project, &environment, &pack.name)?;
            let prefix = if is_last { "   " } else { "  │" };
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
    }

    Ok(())
}

pub fn run_clone(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    source: &str,
    destination: &str,
    force: bool,
) -> Result<()> {
    let src_kp = keypath::parse_keypath(
        source,
        cli_project.or_else(|| resolver.config().and_then(|c| c.project.as_deref())),
        cli_env.or_else(|| resolver.config().and_then(|c| c.environment.as_deref())),
    )?;
    let dst_kp = keypath::parse_keypath(
        destination,
        cli_project.or_else(|| resolver.config().and_then(|c| c.project.as_deref())),
        cli_env.or_else(|| resolver.config().and_then(|c| c.environment.as_deref())),
    )?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let count = store.pack_clone(
        &src_kp.project,
        &src_kp.environment,
        &src_kp.pack,
        &dst_kp.project,
        &dst_kp.environment,
        &dst_kp.pack,
        force,
    )?;

    eprintln!(
        "{} Cloned '{}' → '{}' ({} keys)",
        "✓".green(),
        src_kp.pack.bold(),
        dst_kp.pack.bold(),
        count
    );

    Ok(())
}

pub fn run_delete(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_name: &str,
    skip_confirm: bool,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let entries = store.pack_show(&project, &environment, pack_name)?;
    if entries.is_empty() {
        eprintln!("{} Pack '{}' not found", "✗".red(), pack_name.bold());
        std::process::exit(1);
    }

    if !skip_confirm && io::stdin().is_terminal() {
        eprint!(
            "{} Pack '{}' contains {} secrets. Delete? [y/N] ",
            "⚠".yellow(),
            pack_name.bold(),
            entries.len()
        );
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    store.pack_delete(&project, &environment, pack_name)?;
    eprintln!(
        "{} Deleted pack '{}' ({} secrets archived)",
        "✓".green(),
        pack_name.bold(),
        entries.len()
    );

    // Remove from compose if present
    if let Ok(Some(mut config)) = Config::load() {
        config.remove_from_compose(pack_name);
        config.save()?;
    }

    Ok(())
}

pub fn run_history(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_name: &str,
    key: &str,
    limit: usize,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let entries = store.pack_history(&project, &environment, pack_name, key, limit)?;

    if entries.is_empty() {
        eprintln!(
            "{} No history for '{}' in pack '{}'",
            "○".yellow(),
            key.bold(),
            pack_name.bold()
        );
        return Ok(());
    }

    // Also show current version
    let current = store.pack_show(&project, &environment, pack_name)?;
    let current_entry = current.iter().find(|e| e.key == key);

    if let Some(curr) = current_entry {
        println!(
            "  v{}  {}  {}",
            curr.version,
            curr.updated_at.format("%Y-%m-%dT%H:%M:%SZ"),
            "(current)".green()
        );
    }

    for entry in &entries {
        let status = entry
            .deleted_at
            .map(|_| "(deleted)".red().to_string())
            .unwrap_or_default();
        println!(
            "  v{}  {}  {}",
            entry.version,
            entry.created_at.format("%Y-%m-%dT%H:%M:%SZ"),
            status
        );
    }

    Ok(())
}

pub fn run_group(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    auto_yes: bool,
    dry_run: bool,
    min_size: usize,
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let flat_count = store.count_flat_secrets(&project, &environment)?;
    if flat_count == 0 {
        eprintln!(
            "{} No flat secrets to group in {}/{}",
            "○".yellow(),
            project.cyan(),
            environment.yellow()
        );
        return Ok(());
    }

    let suggestion = store.suggest_groups(&project, &environment, min_size)?;

    eprintln!(
        "📋 Found {} flat secrets in {}/{}",
        flat_count.to_string().bold(),
        project.cyan(),
        environment.yellow()
    );
    eprintln!();

    if !suggestion.groups.is_empty() {
        eprintln!("Suggested groups (by prefix, {}+ keys):", min_size);
        for group in &suggestion.groups {
            eprintln!(
                "  {}  ← {}",
                group.name.bold(),
                group.keys.join(", ").dimmed()
            );
        }
        eprintln!();
    }

    if !suggestion.ungrouped.is_empty() {
        eprintln!("Ungrouped ({} keys):", suggestion.ungrouped.len());
        eprintln!("  {}", suggestion.ungrouped.join(", ").dimmed());
        eprintln!();
    }

    if dry_run {
        eprintln!("{} Dry run — no changes made", "○".yellow());
        return Ok(());
    }

    let proceed = if auto_yes {
        true
    } else if io::stdin().is_terminal() {
        eprint!("Accept suggested groups? [Y/n] ");
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        input.is_empty() || input == "y" || input == "yes"
    } else {
        true
    };

    if !proceed {
        eprintln!("Cancelled.");
        return Ok(());
    }

    let mut created_packs = Vec::new();

    // Adopt suggested groups
    for group in &suggestion.groups {
        let count = store.pack_adopt_keys(&project, &environment, &group.name, &group.keys)?;
        eprintln!("  {} {} ({} keys)", "✓".green(), group.name.bold(), count);
        created_packs.push(group.name.clone());
    }

    // Handle ungrouped → "other"
    if !suggestion.ungrouped.is_empty() {
        let move_to_other = if auto_yes {
            true
        } else if io::stdin().is_terminal() {
            eprint!(
                "Move {} remaining keys to 'other'? [Y/n] ",
                suggestion.ungrouped.len()
            );
            io::stderr().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            input.is_empty() || input == "y" || input == "yes"
        } else {
            true
        };

        if move_to_other {
            let count =
                store.pack_adopt_keys(&project, &environment, "other", &suggestion.ungrouped)?;
            eprintln!("  {} other ({} keys)", "✓".green(), count);
            created_packs.push("other".to_string());
        }
    }

    // Update .tinysecrets.toml with compose list
    if !created_packs.is_empty() {
        if let Ok(Some(mut config)) = Config::load() {
            for pack_name in &created_packs {
                config.add_to_compose(pack_name);
            }
            config.save()?;
            eprintln!();
            eprintln!("{} Updated .tinysecrets.toml with compose", "✓".green());
        }
    }

    eprintln!();
    eprintln!(
        "{} Created {} packs",
        "✓".green(),
        created_packs.len().to_string().bold()
    );

    Ok(())
}

pub fn run_adopt(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    pack_name: &str,
    keys: &[String],
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let count = store.pack_adopt_keys(&project, &environment, pack_name, keys)?;

    eprintln!(
        "{} Moved {} secrets into pack '{}'",
        "✓".green(),
        count.to_string().bold(),
        pack_name.bold()
    );

    // Update compose
    if let Ok(Some(mut config)) = Config::load() {
        config.add_to_compose(pack_name);
        config.save()?;
        eprintln!("{} Added '{}' to compose", "✓".green(), pack_name);
    }

    Ok(())
}

pub fn run_move(
    resolver: &ConfigResolver,
    cli_project: Option<&str>,
    cli_env: Option<&str>,
    source: &str,
    destination: &str,
    keys: &[String],
) -> Result<()> {
    let project = resolver.project(cli_project)?;
    let environment = resolver.environment(cli_env)?;

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let result = store.pack_move(&project, &environment, source, destination, keys)?;

    eprintln!(
        "{} Moved {} keys: {} → {}",
        "✓".green(),
        result.moved,
        source.bold(),
        destination.bold()
    );

    // Update compose for new pack
    if let Ok(Some(mut config)) = Config::load() {
        config.add_to_compose(destination);
        config.save()?;
    }

    // Warn if source is now empty
    if result.source_remaining == 0 && io::stdin().is_terminal() {
        eprint!(
            "{} Pack '{}' is now empty. Delete it? [Y/n] ",
            "⚠".yellow(),
            source.bold()
        );
        io::stderr().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input.is_empty() || input == "y" || input == "yes" {
            store.pack_delete(&project, &environment, source)?;
            eprintln!("{} Deleted empty pack '{}'", "✓".green(), source.bold());

            if let Ok(Some(mut config)) = Config::load() {
                config.remove_from_compose(source);
                config.save()?;
                eprintln!("{} Removed '{}' from compose", "✓".green(), source);
            }
        }
    }

    Ok(())
}
