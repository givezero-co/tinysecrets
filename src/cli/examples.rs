use colored::Colorize;

pub fn run() {
    let examples = r#"
┌─────────────────────────────────────────────────────────────────────────────┐
│                        🔐 TinySecrets Examples                              │
└─────────────────────────────────────────────────────────────────────────────┘

FIRST TIME SETUP
────────────────
  # Create your encrypted secrets store (one-time)
  ts init

  # Set up a project config so you don't have to type -p/-e every time
  cd ~/myproject
  ts config init myapp dev


DAILY WORKFLOW
──────────────
  # With .tinysecrets.toml in your project:
  ts set API_KEY                    # Opens $EDITOR for secure input
  ts set DATABASE_URL "postgres://localhost/mydb"
  ts get API_KEY                    # Print value to stdout
  ts list                           # Show all secrets for this project/env
  ts run -- npm start               # Run with secrets as env vars

  # Override config with flags when needed:
  ts run -e prod -- ./deploy.sh     # Use prod environment
  ts get -p other -e staging KEY    # Different project entirely


IMPORTING SECRETS
─────────────────
  # From a .env file:
  cat .env | ts import-env

  # From Heroku:
  heroku config -s | ts import-env

  # From a file directly:
  ts import-env -f .env.production

  # From AWS Parameter Store:
  aws ssm get-parameters-by-path --path /myapp/prod \
    --query 'Parameters[*].[Name,Value]' --output text \
    | awk -F'\t' '{split($1,a,"/"); print a[length(a)]"="$2}' \
    | ts import-env


MANAGING ENVIRONMENTS
─────────────────────
  # See all your projects:
  ts projects

  # See environments for a project:
  ts envs -p myapp

  # List secrets across all projects:
  ts list

  # List for specific project/env:
  ts list -p myapp -e prod


SECRET HISTORY
──────────────
  # View change history:
  ts history API_KEY

  # Show actual values in history:
  ts history API_KEY --show

  # Retrieve an old version:
  ts get API_KEY --version 2


SHARING SECRETS
───────────────
  # Export for a teammate (encrypted bundle):
  ts export -o secrets.tsb

  # They import with same passphrase:
  ts import secrets.tsb


TIPS & TRICKS
─────────────
  • Use $EDITOR for sensitive values - avoids shell history:
      ts set API_KEY              # Opens editor

  • Pipe secrets into commands:
      ts get DATABASE_URL | pbcopy

  • Use in scripts:
      export API_KEY=$(ts get API_KEY)

  • Check what env vars will be injected:
      ts run -- env | grep -E '^(API|DB|SECRET)'

  • Config files are searched upward - put one at repo root

  • Passphrase is cached in system keychain after first use


COMMON PATTERNS
───────────────
  # Development workflow:
  ts config init myapp dev
  ts import-env -f .env.example    # Import starter secrets
  ts run -- npm run dev

  # CI/CD (passphrase from env var):
  echo "$TS_PASSPHRASE" | ts run -- ./deploy.sh

  # Multiple environments in monorepo:
  cd services/api && ts config init api prod
  cd services/web && ts config init web prod


"#;

    // Print with some color highlighting
    for line in examples.lines() {
        if line.starts_with("  #") {
            // Comments in dim
            println!("{}", line.dimmed());
        } else if line.starts_with("  ts ") || line.starts_with("      ts ") {
            // Commands in cyan
            println!("{}", line.cyan());
        } else if line.contains("────") || line.starts_with("│") || line.starts_with("┌") || line.starts_with("└") {
            // Box drawing in yellow
            println!("{}", line.yellow());
        } else if line.ends_with("─") || (line.chars().all(|c| c == '─' || c.is_whitespace()) && line.contains("─")) {
            // Section headers
            println!("{}", line.yellow());
        } else if line.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) 
            && !line.starts_with("  ") 
            && !line.is_empty() 
        {
            // Section titles in bold
            println!("{}", line.bold());
        } else {
            println!("{}", line);
        }
    }
}

