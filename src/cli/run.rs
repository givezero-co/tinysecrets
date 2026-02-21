use anyhow::{Context, Result};
use colored::Colorize;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(
    project: &str,
    environment: &str,
    command: &[String],
    config_compose: Option<&[String]>,
    extra_packs: &[String],
    override_compose: Option<&[String]>,
) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    crate::cli::maybe_offer_pack_migration(&store, project, environment);

    let result = if let Some(compose_list) = override_compose {
        // --compose flag overrides everything
        let mut packs: Vec<String> = compose_list.to_vec();
        packs.extend(extra_packs.iter().cloned());
        store.compose(project, environment, &packs)?
    } else if let Some(compose_list) = config_compose {
        // .tinysecrets.toml compose list
        let mut packs: Vec<String> = compose_list.to_vec();
        packs.extend(extra_packs.iter().cloned());
        store.compose(project, environment, &packs)?
    } else {
        // No compose — load all packs + flat secrets
        store.compose_all(project, environment)?
    };

    if !result.conflicts.is_empty() {
        for conflict in &result.conflicts {
            eprintln!(
                "{} Key conflict: {} defined in {}",
                "✗".red(),
                conflict.key.bold(),
                conflict.packs.join(", ")
            );
        }
        anyhow::bail!("Cannot run: key conflicts detected. Resolve them first.");
    }

    let secrets = result.secrets;

    if secrets.is_empty() {
        eprintln!(
            "{} No secrets found for {}/{}",
            "⚠".yellow(),
            project.cyan(),
            environment.yellow()
        );
    } else if !result.packs_resolved.is_empty() {
        eprintln!(
            "{} Composed {} secrets from {} packs ({}/{})",
            "✓".green(),
            secrets.len().to_string().bold(),
            result.packs_resolved.len(),
            project.cyan(),
            environment.yellow()
        );
    } else {
        eprintln!(
            "{} Loaded {} secrets for {}/{}",
            "✓".green(),
            secrets.len().to_string().bold(),
            project.cyan(),
            environment.yellow()
        );
    }

    let program = &command[0];
    let args = &command[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    for (key, value) in &secrets {
        cmd.env(key, value);
    }

    let err = cmd.exec();

    Err(err).context(format!("Failed to execute: {}", program))
}
