pub mod delete;
pub mod envs;
pub mod export;
pub mod get;
pub mod history;
pub mod import;
pub mod import_env;
pub mod init;
pub mod keychain_cmd;
pub mod list;
pub mod projects;
pub mod run;
pub mod set;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tinysecrets")]
#[command(
    author,
    version,
    about = "🔐 TinySecrets - Encrypted local secrets manager"
)]
#[command(long_about = r#"
TinySecrets is an encrypted SQLite-backed .env replacement that never 
writes secrets to disk in plaintext.

QUICK START:
  tinysecrets init                           # Create encrypted store
  tinysecrets set api staging API_KEY        # Set a secret (opens editor)
  tinysecrets get api staging API_KEY        # Get a secret
  tinysecrets run -p api -e staging -- cmd   # Run command with secrets

EXAMPLES:
  tinysecrets set myapp prod DATABASE_URL    # Opens $EDITOR for value
  tinysecrets set myapp prod API_KEY "sk-..." # Set directly from CLI
  tinysecrets list -p myapp                  # List all secrets for project
  tinysecrets run -p myapp -e prod -- npm start

BULK IMPORT:
  heroku config | tinysecrets import-env myapp staging
  cat .env | tinysecrets import-env myapp dev
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new secrets store
    #[command(visible_alias = "i")]
    Init,

    /// Set a secret value
    #[command(visible_alias = "s")]
    Set {
        /// Project name
        project: String,
        /// Environment (e.g., dev, staging, prod)
        environment: String,
        /// Secret key name
        key: String,
        /// Secret value (opens $EDITOR if not provided)
        value: Option<String>,
    },

    /// Get a secret value
    #[command(visible_alias = "g")]
    Get {
        /// Project name
        project: String,
        /// Environment
        environment: String,
        /// Secret key name
        key: String,
        /// Get a specific version (from history)
        #[arg(long, visible_alias = "rev")]
        version: Option<i32>,
    },

    /// List secrets
    #[command(visible_alias = "ls")]
    List {
        /// Filter by project
        #[arg(short, long)]
        project: Option<String>,
        /// Filter by environment
        #[arg(short, long)]
        environment: Option<String>,
    },

    /// Delete a secret
    #[command(visible_alias = "rm")]
    Delete {
        /// Project name
        project: String,
        /// Environment
        environment: String,
        /// Secret key name
        key: String,
    },

    /// Run a command with secrets injected as environment variables
    #[command(visible_alias = "r")]
    Run {
        /// Project name
        #[arg(short, long)]
        project: String,
        /// Environment
        #[arg(short, long)]
        environment: String,
        /// Command and arguments to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Export secrets to an encrypted bundle
    Export {
        /// Project name
        #[arg(short, long)]
        project: String,
        /// Environment
        #[arg(short, long)]
        environment: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import secrets from an encrypted bundle
    Import {
        /// Input file path
        input: String,
    },

    /// Import environment variables from stdin or file
    #[command(visible_alias = "ie")]
    ImportEnv {
        /// Project name
        project: String,
        /// Environment
        environment: String,
        /// Read from file instead of stdin
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Show secret history
    History {
        /// Project name
        project: String,
        /// Environment
        environment: String,
        /// Secret key name
        key: String,
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Show the actual values
        #[arg(short, long)]
        show: bool,
    },

    /// List all projects
    Projects,

    /// List environments for a project
    Envs {
        /// Project name
        project: String,
    },

    /// Manage system keychain integration
    Keychain {
        #[command(subcommand)]
        action: KeychainAction,
    },
}

#[derive(Subcommand)]
pub enum KeychainAction {
    /// Show keychain status
    Status,
    /// Remove passphrase from keychain
    Clear,
}

/// Prompt for passphrase with confirmation for new stores
pub fn prompt_new_passphrase() -> anyhow::Result<secrecy::SecretString> {
    use colored::Colorize;

    eprintln!("{}", "Creating new secrets store...".cyan());
    eprintln!();

    let pass1 = rpassword::prompt_password("Enter passphrase: ")?;
    let pass2 = rpassword::prompt_password("Confirm passphrase: ")?;

    if pass1 != pass2 {
        anyhow::bail!("Passphrases do not match");
    }

    if pass1.len() < 8 {
        anyhow::bail!("Passphrase must be at least 8 characters");
    }

    let passphrase = secrecy::SecretString::new(pass1);

    // Offer to save to keychain
    eprintln!();
    eprint!("Save passphrase to system keychain? [Y/n] ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        match crate::keychain::store_passphrase(&passphrase) {
            Ok(()) => eprintln!("{} Passphrase saved to keychain", "✓".green()),
            Err(e) => eprintln!("{} Could not save to keychain: {}", "⚠".yellow(), e),
        }
    }

    Ok(passphrase)
}

/// Prompt for existing passphrase (tries keychain first)
pub fn prompt_passphrase() -> anyhow::Result<secrecy::SecretString> {
    use colored::Colorize;

    // Try keychain first
    match crate::keychain::get_passphrase() {
        Ok(Some(passphrase)) => {
            eprintln!("🔑 Using passphrase from keychain");
            return Ok(passphrase);
        }
        Ok(None) => {} // No stored passphrase, prompt
        Err(e) => {
            eprintln!("{} Keychain error: {}", "⚠".yellow(), e);
        }
    }

    let pass = rpassword::prompt_password("Passphrase: ")?;
    let passphrase = secrecy::SecretString::new(pass);

    // Offer to save for next time
    eprint!("Save to keychain for next time? [Y/n] ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        match crate::keychain::store_passphrase(&passphrase) {
            Ok(()) => eprintln!("{} Passphrase saved to keychain", "✓".green()),
            Err(e) => eprintln!("{} Could not save to keychain: {}", "⚠".yellow(), e),
        }
    }

    Ok(passphrase)
}
