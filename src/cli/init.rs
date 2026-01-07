use anyhow::Result;
use colored::Colorize;

use crate::cli::prompt_new_passphrase;
use crate::store::Store;

pub fn run() -> Result<()> {
    if Store::exists()? {
        let path = Store::default_path()?;
        eprintln!(
            "{} Store already exists at {}",
            "✗".red(),
            path.display().to_string().yellow()
        );
        eprintln!("  Use other tinysecrets commands to manage your secrets.");
        return Ok(());
    }

    let passphrase = prompt_new_passphrase()?;
    let _store = Store::init(passphrase)?;

    let path = Store::default_path()?;

    eprintln!();
    eprintln!(
        "{} Secrets store created at {}",
        "✓".green(),
        path.display().to_string().cyan()
    );
    eprintln!();
    eprintln!("{}", "Quick start:".bold());
    eprintln!(
        "  {} set a secret    tinysecrets set myapp staging DATABASE_URL",
        "→".cyan()
    );
    eprintln!(
        "  {} get a secret    tinysecrets get myapp staging DATABASE_URL",
        "→".cyan()
    );
    eprintln!(
        "  {} run with secrets tinysecrets run -p myapp -e staging -- npm start",
        "→".cyan()
    );
    eprintln!();
    eprintln!(
        "{}",
        "⚠  Remember your passphrase! It cannot be recovered.".yellow()
    );

    Ok(())
}
