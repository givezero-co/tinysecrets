# 🔐 TinySecrets

**An encrypted SQLite-backed .env replacement that never writes secrets to disk in plaintext.**

```
┌──────────────┐
│  ts CLI      │
│              │
│  ┌────────┐  │
│  │SQLite  │  │  ~/.tinysecrets/store.db
│  │(file)  │◄─┼── encrypted at rest
│  └────────┘  │
└──────────────┘
```

No daemon. No ports. No migrations service. No infra. Just:

- **One binary** (`ts`)
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
ts init

# Set some secrets
ts set myapp staging DATABASE_URL "postgres://localhost/myapp_staging"
ts set myapp staging API_KEY      # Opens $EDITOR for secure input
ts set myapp prod DATABASE_URL "postgres://prod-server/myapp"

# Bulk import from existing sources
heroku config | ts import-env myapp staging
cat .env | ts import-env myapp dev

# Get a secret
ts get myapp staging API_KEY

# List all secrets
ts list

# Run a command with secrets injected as environment variables
ts run -p myapp -e staging -- npm start
ts run -p myapp -e prod -- ./deploy.sh
```

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

### `ts init`

Create a new encrypted secrets store. You'll be prompted to create a passphrase.

```bash
ts init
```

### `ts set <project> <environment> <key> [value]`

Set a secret. If no value is provided, opens `$EDITOR` for secure input.

```bash
# Direct value (be careful with shell history!)
ts set api staging DATABASE_URL "postgres://..."

# Opens editor (recommended for sensitive values)
ts set api staging API_KEY

# Aliases: ts s
```

### `ts get <project> <environment> <key>`

Get a secret value. Outputs just the value (great for scripts).

```bash
ts get api staging DATABASE_URL
# postgres://...

# Use in scripts
export DB=$(ts get api staging DATABASE_URL)

# Aliases: ts g
```

### `ts list [-p project] [-e environment]`

List secrets with optional filtering.

```bash
ts list                    # All secrets
ts list -p api             # All secrets for 'api' project
ts list -p api -e staging  # Secrets for api/staging

# Aliases: ts ls
```

### `ts run -p <project> -e <environment> -- <command>`

Run a command with secrets injected as environment variables. **Secrets are only in process memory** - never written to disk or passed via CLI args.

```bash
ts run -p api -e staging -- npm start
ts run -p api -e prod -- ./deploy.sh
ts run -p api -e dev -- env | grep API  # See what's injected

# Aliases: ts r
```

### `ts delete <project> <environment> <key>`

Delete a secret (archived in history).

```bash
ts delete api staging OLD_KEY

# Aliases: ts rm
```

### `ts history <project> <environment> <key>`

View the change history of a secret.

```bash
ts history api staging DATABASE_URL
```

### `ts projects`

List all projects.

```bash
ts projects
```

### `ts envs <project>`

List all environments for a project.

```bash
ts envs api
# staging
# production
# development
```

### `ts import-env <project> <environment>`

Bulk import environment variables from stdin or a file. Supports multiple formats:
- `KEY=VALUE` (dotenv style)
- `KEY: VALUE` (heroku config style)
- `export KEY=VALUE` (shell exports)

```bash
# From heroku
heroku config | ts import-env myapp staging

# From .env file
cat .env | ts import-env myapp staging

# Or directly from file
ts import-env myapp staging -f .env.production

# From AWS Parameter Store
aws ssm get-parameters-by-path --path /myapp/staging \
  --query 'Parameters[*].[Name,Value]' --output text \
  | awk '{print $1"="$2}' \
  | ts import-env myapp staging

# From 1Password
op item get "API Keys" --format json \
  | jq -r '.fields[] | "\(.label)=\(.value)"' \
  | ts import-env myapp staging

# Aliases: ts ie
```

### `ts export / import`

Export secrets to an encrypted bundle (for sharing or backup).

```bash
# Export
ts export -p api -e staging -o api-staging.tsb

# Import (requires same passphrase)
ts import api-staging.tsb
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
ts export -p api -e staging -o api-staging.tsb

# Share the file + passphrase securely (Signal, 1Password, etc.)

# On teammate's machine
ts import api-staging.tsb
```

### Option 2: Shared Store File

For small trusted teams, sync the SQLite file directly:

```bash
# Use Dropbox, rsync, git-crypt, etc.
# Everyone uses the same passphrase
```

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

# Build
cargo build

# Run
cargo run -- init
cargo run -- set test dev API_KEY "secret123"
cargo run -- get test dev API_KEY

# Test
cargo test

# Release build
cargo build --release
```

## Roadmap

- [ ] Shell completions (bash, zsh, fish)
- [ ] `ts edit` - edit secret in place
- [ ] `ts env` - output as .env format (for legacy tools)
- [ ] `ts diff` - compare environments
- [ ] Optional YubiKey/hardware key support
- [ ] Team sync service (Option 2 from design)

## License

MIT
