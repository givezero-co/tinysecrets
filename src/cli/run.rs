use anyhow::{Context, Result};
use colored::Colorize;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::cli::prompt_passphrase;
use crate::store::Store;

pub fn run(project: &str, environment: &str, command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let passphrase = prompt_passphrase()?;
    let store = Store::open(passphrase)?;

    let secrets = store.get_all(project, environment)?;

    if secrets.is_empty() {
        eprintln!(
            "{} No secrets found for {}/{}",
            "⚠".yellow(),
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

    // Build the command with injected environment variables
    let program = &command[0];
    let args = &command[1..];

    // Use std::process::Command with exec() to replace the current process
    // This way secrets are only in process memory, never written anywhere
    let mut cmd = Command::new(program);
    cmd.args(args);

    // Inject secrets as environment variables
    for (key, value) in &secrets {
        cmd.env(key, value);
    }

    // exec replaces the current process - this doesn't return on success
    let err = cmd.exec();

    // If we get here, exec failed
    Err(err).context(format!("Failed to execute: {}", program))
}
