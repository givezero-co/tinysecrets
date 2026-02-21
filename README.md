# 🔐 TinySecrets

**An encrypted SQLite-backed .env replacement that never writes secrets to disk in plaintext.**

```
┌──────────────┐
│  tinysecrets CLI      │
│              │
│  ┌────────┐  │
│  │SQLite  │  │  ~/.tinysecrets/store.db
│  │(file)  │◄─┼── encrypted at rest
│  └────────┘  │
└──────────────┘
```

No daemon. No ports. No migrations service. No infra. Just:

- **One binary** (`tinysecrets`)
- **One encrypted SQLite file** (`~/.tinysecrets/store.db`)

## Installation

### Quick Install (macOS & Linux)

```bash
curl -sSfL https://raw.githubusercontent.com/givezero-co/tinysecrets/main/install.sh | sh
```

### From Source

```bash
cargo install --git https://github.com/givezero-co/tinysecrets
```

### Homebrew (coming soon)

```bash
brew install givezero-co/tap/tinysecrets
```

## Quick Start

```bash
# Initialize your secrets store (creates ~/.tinysecrets/store.db)
tinysecrets init

# Set some secrets
tinysecrets set -p myapp -e staging DATABASE_URL "postgres://localhost/myapp_staging"
tinysecrets set -p myapp -e staging API_KEY      # Opens $EDITOR for secure input
tinysecrets set -p myapp -e prod DATABASE_URL "postgres://prod-server/myapp"

# Bulk import from existing sources
heroku config | tinysecrets import-env -p myapp -e staging
cat .env | tinysecrets import-env -p myapp -e dev

# Get a secret
tinysecrets get -p myapp -e staging API_KEY

# List all secrets
tinysecrets list

# Run a command with secrets injected as environment variables
tinysecrets run -p myapp -e staging -- npm start
tinysecrets run -p myapp -e prod -- ./deploy.sh
```

## Project Configuration

Create a `.tinysecrets.toml` in your project root to avoid typing `-p`/`-e` every time:

```bash
# Create config for this project
tinysecrets config init myapp staging

# Now commands are much cleaner!
tinysecrets set API_KEY
tinysecrets get API_KEY
tinysecrets run -- npm start
tinysecrets list
```

The config file is simple TOML:

```toml
project = "myapp"
environment = "staging"
```

### Config Commands

```bash
# Create config
tinysecrets config init <project> [environment]

# Show current config
tinysecrets config show

# Update config values
tinysecrets config set -p newproject
tinysecrets config set -e production
tinysecrets config set -p api -e dev
```

Config files are searched up the directory tree, so you can have different configs for different subdirectories if needed.

## Packs: Organize Secrets into Composable Groups

**Packs** let you organize secrets into logical groups and compose them dynamically. Think of packs as "modules" for your secrets - you can create different variants, mix and match them per branch, and keep your secrets organized as your project grows.

### What is a Pack?

A pack is a named collection of related secrets. For example:
- `openai` pack contains `OPENAI_KEY`, `OPENAI_ENDPOINT`, `OPENAI_ORG`
- `stripe` pack contains `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
- `database` pack contains `DATABASE_URL`, `DB_POOL_SIZE`

### Quick Example: Organizing Your Secrets

Let's say you have these flat secrets in your project:

```bash
OPENAI_KEY=sk-abc...
OPENAI_ENDPOINT=https://api.openai.com
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
DATABASE_URL=postgres://localhost/mydb
REDIS_URL=redis://localhost:6379
```

**Step 1: Group them into packs**

Run the interactive grouping wizard:

```bash
ts pack group

# The wizard analyzes prefixes and suggests groups:
# ✓ Created 3 packs:
#   openai    (2 keys)
#   stripe    (2 keys)
#   other     (2 keys)
```

This automatically creates packs and updates your `.tinysecrets.toml`:

```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "other",
]
```

**Step 2: Refine your organization**

Move keys from `other` into better-named packs:

```bash
# Create an infra pack with database and redis
ts pack move other infra DATABASE_URL REDIS_URL
# ✓ Moved 2 keys: other → infra
# ✓ Added 'infra' to compose
```

**Step 3: Run your app with composed secrets**

```bash
ts run -- npm start
# ✓ Composed 6 secrets from 3 packs (myapp/dev)
```

### Creating Pack Variants

Packs shine when you need different versions of the same secrets. Common use case: testing a new API provider without affecting your main setup.

```bash
# Clone your current openai pack to create a backup
ts pack clone openai openai.old

# Create a new variant with different credentials
ts pack set openai.new \
    OPENAI_KEY="sk-new-key-123" \
    OPENAI_ENDPOINT="https://api.openai.com/v2"

# Switch which variant your branch uses by editing .tinysecrets.toml:
compose = [
    "openai.new",    # Changed from "openai"
    "stripe",
    "infra",
]

# Now run your app with the new OpenAI credentials
ts run -- npm start
```

This is powerful for:
- **Canary deployments**: Test new API keys on a feature branch
- **Provider migrations**: Run old and new providers side-by-side
- **Per-developer customization**: Each dev has their own variant

### Working with Packs

```bash
# List all your packs
ts pack list
# 📦 myapp/dev
#   ├─ openai (2 keys)
#   │  ├─ .old (2 keys)
#   │  └─ .new (2 keys)
#   ├─ stripe (2 keys)
#   └─ infra (2 keys)

# Show what's in a specific pack
ts pack show openai.new
# 📦 openai.new (myapp/dev)
#   • OPENAI_ENDPOINT  v1
#   • OPENAI_KEY       v1

# Preview what will be injected
ts compose show
# 📋 Compose: myapp/dev
#
# openai.new
#   • OPENAI_ENDPOINT
#   • OPENAI_KEY
# stripe
#   • STRIPE_SECRET_KEY
#   • STRIPE_WEBHOOK_SECRET
# infra
#   • DATABASE_URL
#   • REDIS_URL
#
# Total: 6 env vars from 3 packs
# ✓ No conflicts

# Add temporary packs without editing the config
ts run --with monitoring -- npm test
```

### Backward Compatibility

Don't want to use packs? No problem. All existing commands work exactly as before:

```bash
# These still work without packs
ts set API_KEY "value"
ts get API_KEY
ts list
ts run -- npm start
```

If you do create packs, existing commands become smarter:
- `ts get DATABASE_URL` searches across all packs automatically
- `ts set DATABASE_URL "new-value"` updates it wherever it exists
- `ts run` loads all packs when no compose is specified

### Real-World Example: Multi-Environment Setup

```bash
# You're working on the 'api' project
cd ~/projects/api
ts config init api dev

# Import your existing .env file
cat .env | ts import-env

# Organize into packs
ts pack group
# ✓ Created packs: auth, payments, database, cache, monitoring

# Create production environment with different credentials
ts pack set -e prod auth \
    JWT_SECRET="prod-secret-xyz" \
    AUTH0_CLIENT_ID="prod-client-abc"

ts pack set -e prod payments \
    STRIPE_SECRET_KEY="sk_live_..."

ts pack set -e prod database \
    DATABASE_URL="postgres://prod-server/api"

# Update prod config
ts config set -e prod

# Your .tinysecrets.toml now has:
# project = "api"
# environment = "prod"
# compose = ["auth", "payments", "database", "cache", "monitoring"]

# Deploy with prod secrets
ts run -- ./deploy.sh
# ✓ Composed 15 secrets from 5 packs (api/prod)
```

### Branch-Specific Secrets

Different branches can compose different pack variants by having different `.tinysecrets.toml` files:

**main branch** (`.tinysecrets.toml`):
```toml
project = "api"
environment = "prod"
compose = ["openai", "stripe", "database"]
```

**feature/test-anthropic branch** (`.tinysecrets.toml`):
```toml
project = "api"
environment = "prod"
compose = ["anthropic", "stripe", "database"]  # Different AI provider!
```

Each branch gets its own secret composition, all stored safely in the same encrypted database.

### What Does the `.tinysecrets.toml` File Do?

The TOML file controls which secrets are loaded. Here's a quick comparison:

**Without packs (basic):**
```toml
project = "myapp"
environment = "dev"
```
Running `ts run -- npm start` loads all flat secrets for myapp/dev.

**With packs + compose:**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "database",
]
```
Running `ts run -- npm start` loads ONLY these 3 packs (6 secrets total).

**With packs, no compose:**
```toml
project = "myapp"
environment = "dev"
# No compose field
```
Running `ts run -- npm start` loads ALL packs for myapp/dev automatically.

**Different branch, different packs:**
```toml
# feature branch
compose = ["openai.new", "stripe", "database"]
```
Same commands, different secrets - perfect for testing new API providers!

### Learn More About Packs

- **[TOML Configuration Guide](docs/TOML_CONFIG_GUIDE.md)** - Detailed explanation of what the `.tinysecrets.toml` file does with different configurations
- **[Complete Example Walkthrough](docs/PACKS_EXAMPLE.md)** - Step-by-step guide with common patterns
- **[Technical Specification](docs/PACKS_SPEC.md)** - Full feature spec and implementation details
- **[Example Config File](.tinysecrets.toml.example)** - Template configuration with comments

## Why TinySecrets?

### The Problem with .env Files

- **Plaintext on disk** - anyone with file access can read your secrets
- **Accidentally committed to git** - a constant security risk
- **No versioning** - can't track changes or roll back
- **No metadata** - what is this secret for? when was it added?
- **Scattered files** - .env.local, .env.staging, .env.production...

### The TinySecrets Solution

| Feature | .env | TinySecrets |
|---------|------|-------------|
| Encrypted at rest | ❌ | ✅ |
| Version history | ❌ | ✅ |
| Metadata/descriptions | ❌ | ✅ |
| Multiple environments | 🟡 Multiple files | ✅ One database |
| Atomic updates | ❌ | ✅ |
| Search & query | ❌ | ✅ |
| Backup | Copy files | Copy one file |

## Commands

### `tinysecrets init`

Create a new encrypted secrets store. You'll be prompted to create a passphrase.

```bash
tinysecrets init
```

### `tinysecrets set [-p project] [-e environment] <key> [value]`

Set a secret. If no value is provided, opens `$EDITOR` for secure input.
Project/environment can be omitted if `.tinysecrets.toml` exists.

```bash
# With explicit project/environment
tinysecrets set -p api -e staging DATABASE_URL "postgres://..."

# With .tinysecrets.toml (cleaner!)
tinysecrets set DATABASE_URL "postgres://..."

# Opens editor (recommended for sensitive values)
tinysecrets set API_KEY

# Aliases: tinysecrets s
```

### `tinysecrets get [-p project] [-e environment] <key>`

Get a secret value. Outputs just the value (great for scripts).
Project/environment can be omitted if `.tinysecrets.toml` exists.

```bash
tinysecrets get -p api -e staging DATABASE_URL
# postgres://...

# With .tinysecrets.toml
tinysecrets get DATABASE_URL

# Use in scripts
export DB=$(ts get DATABASE_URL)

# Get a previous version
tinysecrets get DATABASE_URL --version 1

# Aliases: tinysecrets g
```

### `tinysecrets list [-p project] [-e environment]`

List secrets with optional filtering.

```bash
tinysecrets list                    # All secrets
tinysecrets list -p api             # All secrets for 'api' project
tinysecrets list -p api -e staging  # Secrets for api/staging

# Aliases: tinysecrets ls
```

### `tinysecrets run [-p project] [-e environment] -- <command>`

Run a command with secrets injected as environment variables. **Secrets are only in process memory** - never written to disk or passed via CLI args.
Project/environment can be omitted if `.tinysecrets.toml` exists.

```bash
# With explicit flags
tinysecrets run -p api -e staging -- npm start

# With .tinysecrets.toml (much cleaner!)
tinysecrets run -- npm start
tinysecrets run -- ./deploy.sh
tinysecrets run -- env | grep API  # See what's injected

# Aliases: tinysecrets r
```

### `tinysecrets delete <project> <environment> <key>`

Delete a secret (archived in history).

```bash
tinysecrets delete api staging OLD_KEY

# Aliases: tinysecrets rm
```

### `tinysecrets history <project> <environment> <key>`

View the change history of a secret.

```bash
# Show history (versions and timestamps)
tinysecrets history api staging DATABASE_URL

# Show history with actual values
tinysecrets history api staging DATABASE_URL --show
```

**Example output with `--show`:**
```
📜 History for api/staging/DATABASE_URL

  • v2 - current (latest)
    postgres://newhost/db

  • v1 - archived at 2026-01-06 23:56:01 UTC
    postgres://oldhost/db
```

### `tinysecrets projects`

List all projects.

```bash
tinysecrets projects
```

### `tinysecrets envs <project>`

List all environments for a project.

```bash
tinysecrets envs api
# staging
# production
# development
```

### `tinysecrets import-env <project> <environment>`

Bulk import environment variables from stdin or a file. Supports multiple formats:
- `KEY=VALUE` (dotenv style)
- `KEY: VALUE` (heroku config style)
- `export KEY=VALUE` (shell exports)

```bash
# From heroku
heroku config | tinysecrets import-env myapp staging

# From .env file
cat .env | tinysecrets import-env myapp staging

# Or directly from file
tinysecrets import-env myapp staging -f .env.production

# From AWS Parameter Store
aws ssm get-parameters-by-path --path /myapp/staging \
  --query 'Parameters[*].[Name,Value]' --output text \
  | awk '{print $1"="$2}' \
  | tinysecrets import-env myapp staging

# From 1Password
op item get "API Keys" --format json \
  | jq -r '.fields[] | "\(.label)=\(.value)"' \
  | tinysecrets import-env myapp staging

# Aliases: tinysecrets ie
```

### `tinysecrets export / import`

Export secrets to an encrypted bundle (for sharing or backup).

```bash
# Export
tinysecrets export -p api -e staging -o api-staging.tsb

# Import (requires same passphrase)
tinysecrets import api-staging.tsb
```

## Encryption

TinySecrets uses [age](https://age-encryption.org/) for encryption:

- **Passphrase-based encryption** with scrypt key derivation
- **Modern cryptography**: X25519, ChaCha20-Poly1305
- **Each secret is encrypted individually** before storage
- **Verification hash** ensures passphrase correctness without storing it

## Storage

All data is stored in `~/.tinysecrets/store.db`, a single SQLite file:

```sql
-- Current secrets
CREATE TABLE secrets (
    project TEXT,
    environment TEXT,
    key TEXT,
    encrypted_value TEXT,  -- age-encrypted
    description TEXT,
    version INTEGER,
    created_at TEXT,
    updated_at TEXT
);

-- Full history for audit trail
CREATE TABLE secret_history (...);
```

### Backup

Just copy the file:

```bash
cp ~/.tinysecrets/store.db ~/backup/
```

Or sync it (still encrypted!):

```bash
rsync ~/.tinysecrets/store.db remote:backup/
```

## Sharing Secrets

### Option 1: Export Bundle

```bash
# On your machine
tinysecrets export -p api -e staging -o api-staging.tsb

# Share the file + passphrase securely (Signal, 1Password, etc.)

# On teammate's machine
tinysecrets import api-staging.tsb
```

### Option 2: Shared Store File

For small trusted teams, sync the SQLite file directly:

```bash
# Use Dropbox, rsync, git-crypt, etc.
# Everyone uses the same passphrase
```

## Keychain Integration

TinySecrets can store your passphrase in the system keychain so you don't have to type it every time:

- **macOS**: Keychain
- **Linux**: Secret Service (GNOME Keyring, KWallet)
- **Windows**: Credential Manager

```bash
# Check keychain status
tinysecrets keychain status

# Remove passphrase from keychain
tinysecrets keychain clear
```

When you first run a command, you'll be asked if you want to save your passphrase to the keychain. This is secure because:

- The keychain is protected by your system login password
- Your secrets database is still encrypted - the keychain just stores the key
- You can clear it anytime with `tinysecrets keychain clear`

## Security Model

### What TinySecrets Protects Against

- ✅ Secrets in plaintext on disk
- ✅ Accidental git commits
- ✅ Shoulder surfing (editor input)
- ✅ Process listing (secrets not in CLI args)
- ✅ `/proc` snooping (secrets in env vars, not files)

### What TinySecrets Does NOT Protect Against

- ❌ Keyloggers / compromised machine
- ❌ Memory forensics
- ❌ Someone who knows your passphrase
- ❌ Root access on the same machine

For higher security needs, consider hardware keys (YubiKey) or dedicated secret managers (Vault, AWS Secrets Manager).

## FAQ

### Why SQLite instead of JSON/YAML?

SQLite gives us:
- ACID transactions
- Concurrent access safety
- Schema evolution
- Efficient queries
- Single-file simplicity

### Why age instead of GPG?

[age](https://age-encryption.org/) is:
- Simpler (no key management complexity)
- Modern cryptography
- Designed for files/data encryption
- No external dependencies

### Can I use this with multiple machines?

Yes! Options:
1. Export/import bundles
2. Sync the store.db file (Dropbox, rsync, etc.)
3. Use the same passphrase everywhere

### What if I forget my passphrase?

**Your secrets are gone.** There's no recovery. This is by design - it's the same as losing a password to an encrypted disk.

Keep your passphrase in a password manager!

## Development

```bash
# Clone
git clone https://github.com/givezero-co/tinysecrets
cd tinysecrets

# Setup pre-commit hooks (runs fmt + clippy before each commit)
git config core.hooksPath .githooks

# Build
cargo build

# Run
cargo run -- init
cargo run -- set test dev API_KEY "secret123"
cargo run -- get test dev API_KEY

# Test
cargo test

# Lint (same as CI)
cargo fmt --all -- --check
cargo clippy -- -D warnings

# Release build
cargo build --release
```

## Roadmap

- [x] Keychain integration (macOS, Linux, Windows)
- [x] Bulk import from pipes (`tinysecrets import-env`)
- [x] Version history with `--show` values
- [x] Retrieve previous versions (`tinysecrets get --version`)
- [x] Project config file (`.tinysecrets.toml`)
- [ ] Shell completions (bash, zsh, fish)
- [ ] `tinysecrets edit` - edit secret in place
- [ ] `tinysecrets env` - output as .env format (for legacy tools)
- [ ] `tinysecrets diff` - compare environments
- [ ] Optional YubiKey/hardware key support
- [ ] Team sync service (Option 2 from design)

## License

MIT
