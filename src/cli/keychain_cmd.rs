use anyhow::Result;
use colored::Colorize;

use crate::cli::KeychainAction;
use crate::keychain;

pub fn run(action: KeychainAction) -> Result<()> {
    match action {
        KeychainAction::Status => status(),
        KeychainAction::Clear => clear(),
    }
}

fn status() -> Result<()> {
    if keychain::has_passphrase() {
        eprintln!("🔑 Passphrase is stored in system keychain");
        eprintln!("  Commands will use it automatically.");
        eprintln!();
        eprintln!("  To remove: {}", "tinysecrets keychain clear".cyan());
    } else {
        eprintln!("{} No passphrase stored in keychain", "○".yellow());
        eprintln!("  You'll be prompted each time you run a command.");
        eprintln!();
        eprintln!("  To save: run any command and answer 'y' when asked.");
    }
    Ok(())
}

fn clear() -> Result<()> {
    match keychain::delete_passphrase()? {
        true => {
            eprintln!("{} Passphrase removed from keychain", "✓".green());
            eprintln!("  You'll be prompted for passphrase on next command.");
        }
        false => {
            eprintln!("{} No passphrase was stored in keychain", "○".yellow());
        }
    }
    Ok(())
}
