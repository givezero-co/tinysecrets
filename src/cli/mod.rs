pub mod compose;
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
pub mod migrate;
pub mod pack;
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
        /// Additional packs to include beyond compose list
        #[arg(short, long)]
        with: Vec<String>,
        /// Override compose list entirely (comma-separated)
        #[arg(long, value_delimiter = ',')]
        compose: Option<Vec<String>>,
        /// Command and arguments to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Manage packs (composable groups of secrets)
    #[command(visible_alias = "p")]
    Pack {
        #[command(subcommand)]
        action: PackAction,
    },

    /// Manage compose (assemble packs into environments)
    Compose {
        #[command(subcommand)]
        action: ComposeAction,
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

    /// Migrate secrets from legacy format to fast encryption
    Migrate,
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

#[derive(Subcommand)]
pub enum PackAction {
    /// Set keys in a pack (creates the pack if needed)
    #[command(visible_alias = "s")]
    Set {
        /// Project name
        #[arg(short, long)]
        project: Option<String>,
        /// Environment
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name or keypath (e.g. "openai" or "gzback.prod.openai")
        pack: String,
        /// KEY=VALUE pairs or a single KEY (opens $EDITOR)
        #[arg(required = true)]
        entries: Vec<String>,
    },
    /// Get a value from a pack
    #[command(visible_alias = "g")]
    Get {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name or keypath
        pack: String,
        /// Secret key name
        key: String,
    },
    /// Show keys in a specific pack
    Show {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name or keypath
        pack: String,
        /// Show decrypted values
        #[arg(long)]
        reveal: bool,
    },
    /// List all packs
    #[command(visible_alias = "ls")]
    List {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Show keys inside each pack
        #[arg(short, long)]
        keys: bool,
    },
    /// Clone a pack to create a variant or copy
    Clone {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Source pack name
        source: String,
        /// Destination pack name
        destination: String,
        /// Overwrite if destination exists
        #[arg(long)]
        force: bool,
    },
    /// Delete a pack
    #[command(visible_alias = "rm")]
    Delete {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name
        pack: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Show history for a key in a pack
    History {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name
        pack: String,
        /// Key name
        key: String,
        /// Number of entries
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
    /// Interactively group flat secrets into packs
    Group {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Accept all suggestions without prompting
        #[arg(short, long)]
        yes: bool,
        /// Show what would happen without doing it
        #[arg(long)]
        dry_run: bool,
        /// Minimum keys to suggest a group
        #[arg(long, default_value = "2")]
        min_size: usize,
    },
    /// Move specific flat secrets into a pack
    Adopt {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Pack name to adopt into
        pack: String,
        /// Key names to adopt
        #[arg(required = true)]
        keys: Vec<String>,
    },
    /// Move keys between packs
    Move {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Source pack name
        source: String,
        /// Destination pack name
        destination: String,
        /// Key names to move
        #[arg(required = true)]
        keys: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ComposeAction {
    /// Preview the assembled environment
    Show {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        environment: Option<String>,
        /// Show decrypted values
        #[arg(long)]
        reveal: bool,
    },
    /// Validate the composition (check for missing packs and conflicts)
    Check {
        #[arg(short, long)]
        project: Option<String>,
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

/// One-time migration prompt: offer to group flat secrets into packs.
/// Only shown interactively, never in CI, and only once.
pub fn maybe_offer_pack_migration(store: &crate::store::Store, project: &str, environment: &str) {
    use colored::Colorize;
    use std::io::{self, IsTerminal, Write};

    if !io::stdin().is_terminal() {
        return;
    }
    if std::env::var(PASSPHRASE_ENV_VAR).is_ok() {
        return;
    }

    if let Ok(Some(_)) = store.get_metadata("packs_migration_offered") {
        return;
    }

    let flat_count = store.count_flat_secrets(project, environment).unwrap_or(0);
    if flat_count == 0 {
        return;
    }

    if store.has_packs(project, environment).unwrap_or(false) {
        let _ = store.set_metadata("packs_migration_offered", "true");
        return;
    }

    eprintln!();
    eprintln!(
        "💡 You have {} flat secrets in {}/{}.",
        flat_count.to_string().bold(),
        project.cyan(),
        environment.yellow()
    );
    eprint!("   Organize into packs? [Y/n] ");
    let _ = io::stderr().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        let _ = store.set_metadata("packs_migration_offered", "true");
        return;
    }
    let input = input.trim().to_lowercase();

    let _ = store.set_metadata("packs_migration_offered", "true");

    if input.is_empty() || input == "y" || input == "yes" {
        eprintln!("   Run: {} to get started", "ts pack group".bold());
    }
    eprintln!();
}

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
