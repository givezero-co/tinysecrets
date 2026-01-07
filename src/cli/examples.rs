use colored::Colorize;

pub fn run() {
    let examples = r#"
┌─────────────────────────────────────────────────────────────────────────────┐
│                        🔐 TinySecrets Examples                              │
└─────────────────────────────────────────────────────────────────────────────┘

FIRST TIME SETUP
────────────────
  # Create your encrypted secrets store (one-time)
  tinysecrets init

  # Set up a project config so you don't have to type -p/-e every time
  cd ~/myproject
  tinysecrets config init myapp dev


DAILY WORKFLOW
──────────────
  # With .tinysecrets.toml in your project:
  tinysecrets set API_KEY                    # Opens $EDITOR for secure input
  tinysecrets set DATABASE_URL "postgres://localhost/mydb"
  tinysecrets get API_KEY                    # Print value to stdout
  tinysecrets list                           # Show all secrets for this project/env
  tinysecrets run -- npm start               # Run with secrets as env vars

  # Override config with flags when needed:
  tinysecrets run -e prod -- ./deploy.sh     # Use prod environment
  tinysecrets get -p other -e staging KEY    # Different project entirely


IMPORTING SECRETS
─────────────────
  # From a .env file:
  cat .env | tinysecrets import-env

  # From Heroku:
  heroku config -s | tinysecrets import-env

  # From a file directly:
  tinysecrets import-env -f .env.production

  # From AWS Parameter Store:
  aws ssm get-parameters-by-path --path /myapp/prod \
    --query 'Parameters[*].[Name,Value]' --output text \
    | awk -F'\t' '{split($1,a,"/"); print a[length(a)]"="$2}' \
    | tinysecrets import-env


MANAGING ENVIRONMENTS
─────────────────────
  # See all your projects:
  tinysecrets projects

  # See environments for a project:
  tinysecrets envs -p myapp

  # List secrets across all projects:
  tinysecrets list

  # List for specific project/env:
  tinysecrets list -p myapp -e prod


SECRET HISTORY
──────────────
  # View change history:
  tinysecrets history API_KEY

  # Show actual values in history:
  tinysecrets history API_KEY --show

  # Retrieve an old version:
  tinysecrets get API_KEY --version 2


SHARING SECRETS
───────────────
  # Export for a teammate (encrypted bundle):
  tinysecrets export -o secrets.tsb

  # They import with same passphrase:
  tinysecrets import secrets.tsb


TIPS & TRICKS
─────────────
  • Use $EDITOR for sensitive values - avoids shell history:
      tinysecrets set API_KEY              # Opens editor

  • Pipe secrets into commands:
      tinysecrets get DATABASE_URL | pbcopy

  • Use in scripts:
      export API_KEY=$(tinysecrets get API_KEY)

  • Check what env vars will be injected:
      tinysecrets run -- env | grep -E '^(API|DB|SECRET)'

  • Config files are searched upward - put one at repo root

  • Passphrase is cached in system keychain after first use


COMMON PATTERNS
───────────────
  # Development workflow:
  tinysecrets config init myapp dev
  tinysecrets import-env -f .env.example    # Import starter secrets
  tinysecrets run -- npm run dev

  # Multiple environments in monorepo:
  cd services/api && tinysecrets config init api prod
  cd services/web && tinysecrets config init web prod


CI/CD
─────
  # Set TINYSECRETS_PASSPHRASE in your CI secrets, then:
  tinysecrets run -- ./deploy.sh            # Auto-detects env var

  # GitHub Actions example:
  #   env:
  #     TINYSECRETS_PASSPHRASE: ${{ secrets.TINYSECRETS_PASSPHRASE }}
  #   run: tinysecrets run -- npm test

  # Import encrypted bundle from repo:
  tinysecrets import .secrets/prod.tsb
  tinysecrets run -- ./deploy.sh


"#;

    // Print with some color highlighting
    for line in examples.lines() {
        if line.starts_with("  #") {
            // Comments in dim
            println!("{}", line.dimmed());
        } else if line.contains("tinysecrets ") && line.starts_with("  ") {
            // Commands in cyan
            println!("{}", line.cyan());
        } else if line.contains("────")
            || line.starts_with("│")
            || line.starts_with("┌")
            || line.starts_with("└")
        {
            // Box drawing in yellow
            println!("{}", line.yellow());
        } else if line.ends_with("─")
            || (line.chars().all(|c| c == '─' || c.is_whitespace()) && line.contains("─"))
        {
            // Section headers
            println!("{}", line.yellow());
        } else if line
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
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
