pub mod config;
pub mod delete;
pub mod envs;
pub mod examples;
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
  tinysecrets init                              # Create encrypted store
  tinysecrets config init myapp dev             # Create .tinysecrets.toml
  tinysecrets set API_KEY                       # Set a secret (opens editor)
  tinysecrets get API_KEY                       # Get a secret
  tinysecrets run -- npm start                  # Run command with secrets

WITH EXPLICIT PROJECT/ENV:
  tinysecrets set -p myapp -e prod API_KEY      # Specify project/env explicitly
  tinysecrets run -p myapp -e prod -- npm start
  tinysecrets list -p myapp                     # List all secrets for project

BULK IMPORT:
  heroku config | tinysecrets import-env -p myapp -e staging
  cat .env | tinysecrets import-env -p myapp -e dev
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
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
        /// Secret key name
        key: String,
        /// Secret value (opens $EDITOR if not provided)
        value: Option<String>,
    },

    /// Get a secret value
    #[command(visible_alias = "g")]
    Get {
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
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
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
        /// Secret key name
        key: String,
    },

    /// Run a command with secrets injected as environment variables
    #[command(visible_alias = "r")]
    Run {
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
        /// Command and arguments to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Export secrets to an encrypted bundle
    Export {
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
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
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
        /// Read from file instead of stdin
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Show secret history
    History {
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
        /// Environment (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        environment: Option<String>,
        /// Secret key name
        key: String,
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        /// Show the actual values
        #[arg(short, long)]
        show: bool,
    },

    /// List all projects
    Projects,

    /// List environments for a project
    Envs {
        /// Project name (uses .tinysecrets.toml if not specified)
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Manage system keychain integration
    Keychain {
        #[command(subcommand)]
        action: KeychainAction,
    },

    /// Manage local project configuration (.tinysecrets.toml)
    #[command(visible_alias = "c")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show detailed examples and common workflows
    #[command(visible_alias = "ex")]
    Examples,
}

#[derive(Subcommand)]
pub enum KeychainAction {
    /// Show keychain status
    Status,
    /// Remove passphrase from keychain
    Clear,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Create a .tinysecrets.toml in the current directory
    Init {
        /// Project name
        project: String,
        /// Default environment (optional)
        environment: Option<String>,
    },
    /// Show current configuration
    Show,
    /// Update configuration values
    Set {
        /// Set default project
        #[arg(short, long)]
        project: Option<String>,
        /// Set default environment
        #[arg(short, long)]
        environment: Option<String>,
    },
}

/// Prompt for passphrase with confirmation for new stores
/// In CI (env var set), uses that passphrase without prompting
pub fn prompt_new_passphrase() -> anyhow::Result<secrecy::SecretString> {
    use colored::Colorize;

    // Check environment variable first (for CI/automation)
    if let Ok(pass) = std::env::var(PASSPHRASE_ENV_VAR) {
        if !pass.is_empty() {
            if pass.len() < 8 {
                anyhow::bail!("Passphrase must be at least 8 characters");
            }
            eprintln!(
                "🔐 Using passphrase from {} for new store",
                PASSPHRASE_ENV_VAR.cyan()
            );
            return Ok(secrecy::SecretString::new(pass));
        }
    }

    // Interactive mode
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

/// Environment variable name for passphrase (CI/automation)
pub const PASSPHRASE_ENV_VAR: &str = "TINYSECRETS_PASSPHRASE";

/// Prompt for existing passphrase
/// Priority: 1) env var, 2) keychain, 3) interactive prompt
pub fn prompt_passphrase() -> anyhow::Result<secrecy::SecretString> {
    use colored::Colorize;

    // 1. Check environment variable first (for CI/automation)
    if let Ok(pass) = std::env::var(PASSPHRASE_ENV_VAR) {
        if !pass.is_empty() {
            eprintln!("🔐 Using passphrase from {}", PASSPHRASE_ENV_VAR.cyan());
            return Ok(secrecy::SecretString::new(pass));
        }
    }

    // 2. Try keychain
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

    // 3. Interactive prompt
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
