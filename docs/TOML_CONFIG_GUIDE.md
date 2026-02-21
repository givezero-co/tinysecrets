# `.tinysecrets.toml` Configuration Guide

This guide shows exactly what the `.tinysecrets.toml` file looks like and what it does with packs.

---

## Basic Configuration (No Packs)

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"
```

**What it does:**
- Sets the default project to `myapp` and environment to `dev`
- All commands use these values, so you don't need to type `-p myapp -e dev` every time
- When you run `ts run -- npm start`, it loads all flat secrets for `myapp/dev`

**Example:**
```bash
# Instead of:
ts set -p myapp -e dev DATABASE_URL "postgres://..."

# You can just type:
ts set DATABASE_URL "postgres://..."
```

---

## Configuration with Packs

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "database",
    "redis",
]
```

**What it does:**
- Still sets default project and environment
- **The `compose` array tells `ts run` which packs to load**
- When you run `ts run -- npm start`, it:
  1. Looks up each pack in the compose list (`openai`, `stripe`, `database`, `redis`)
  2. Loads all secrets from those packs
  3. Checks for duplicate keys across packs (errors if found)
  4. Injects all secrets as environment variables
  5. Runs your command

**Example behavior:**

Let's say your packs contain:
- `openai` pack: `OPENAI_KEY`, `OPENAI_ENDPOINT`
- `stripe` pack: `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
- `database` pack: `DATABASE_URL`
- `redis` pack: `REDIS_URL`

```bash
ts run -- node app.js
# ✓ Composed 6 secrets from 4 packs (myapp/dev)
# 
# Your app.js now has these environment variables:
#   OPENAI_KEY=sk-...
#   OPENAI_ENDPOINT=https://api.openai.com/v1
#   STRIPE_SECRET_KEY=sk_test_...
#   STRIPE_WEBHOOK_SECRET=whsec_...
#   DATABASE_URL=postgres://localhost/mydb
#   REDIS_URL=redis://localhost:6379
```

---

## Packs WITHOUT Compose (Load All)

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

# No compose field!
```

**What it does:**
- When you have packs but NO `compose` array, `ts run` loads **ALL packs** for this project/environment
- This is backward-compatible: after running `ts pack group`, everything still works without editing the TOML

**Example:**

You have 5 packs: `openai`, `stripe`, `database`, `redis`, `monitoring`

```bash
ts run -- node app.js
# ✓ Loaded 15 secrets from 5 packs (myapp/dev)
#
# All 5 packs are loaded automatically
```

---

## Pack Variants for Testing

**File: `.tinysecrets.toml` (main branch)**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",      # Use the main OpenAI credentials
    "stripe",
    "database",
]
```

**File: `.tinysecrets.toml` (feature/test-anthropic branch)**
```toml
project = "myapp"
environment = "dev"

compose = [
    "anthropic",   # Switch to Anthropic instead of OpenAI!
    "stripe",
    "database",
]
```

**What it does:**
- Each branch can have a different compose list
- When you switch branches, the TOML file changes (it's in git)
- Running the same command on different branches loads different packs

**Example:**

```bash
# On main branch
git checkout main
ts run -- node app.js
# ✓ Composed secrets with OPENAI_KEY from 'openai' pack

# On feature branch
git checkout feature/test-anthropic
ts run -- node app.js
# ✓ Composed secrets with ANTHROPIC_KEY from 'anthropic' pack

# Same command, different secrets!
```

---

## Using Dot-Suffix Variants

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai.new",    # Note the .new suffix!
    "stripe",
    "database",
]
```

**What it does:**
- Pack names can have dot-separated suffixes like `.new`, `.old`, `.v2`, `.alice`
- These are **separate, independent packs** - no inheritance
- Useful for A/B testing or keeping backups

**Example setup:**

You have three separate OpenAI packs:
- `openai` - current production credentials
- `openai.old` - backup of previous credentials
- `openai.new` - testing new API endpoint

Each pack contains the same key names but different values:

```bash
ts pack show openai
# 📦 openai
#   • OPENAI_KEY = sk-current-abc
#   • OPENAI_ENDPOINT = https://api.openai.com/v1

ts pack show openai.new
# 📦 openai.new
#   • OPENAI_KEY = sk-new-xyz
#   • OPENAI_ENDPOINT = https://api.openai.com/v2
```

Now you can switch between them in your TOML:

```toml
# Use current
compose = ["openai", "stripe", "database"]

# Or use new variant
compose = ["openai.new", "stripe", "database"]
```

---

## Multiple Environments

You can have different TOML files for different environments, or use one file and change it:

**Option 1: Single TOML, change environment**

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"     # Change this to "prod" for production

compose = [
    "openai",
    "stripe",
    "database",
]
```

```bash
# Development
ts run -- npm run dev
# Loads: myapp/dev/openai, myapp/dev/stripe, myapp/dev/database

# Change to prod
ts config set -e prod
ts run -- npm start
# Loads: myapp/prod/openai, myapp/prod/stripe, myapp/prod/database
```

**Option 2: Multiple config files**

```bash
# .tinysecrets.toml (dev)
project = "myapp"
environment = "dev"
compose = ["openai", "stripe.test", "database.local"]

# .tinysecrets.prod.toml (prod)
project = "myapp"
environment = "prod"
compose = ["openai", "stripe", "database"]
```

---

## CLI Overrides

You can override the TOML compose at runtime:

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "database",
]
```

**What you can do:**

```bash
# Add extra packs on top of what's in compose
ts run --with monitoring --with logging -- npm test
# Loads: openai, stripe, database, monitoring, logging

# Completely replace the compose
ts run --compose "openai.experimental,database" -- npm test
# Loads: ONLY openai.experimental and database (ignores TOML)

# Combine with environment override
ts run -e prod --with canary -- ./deploy.sh
# Loads: prod packs from compose + prod/canary pack
```

---

## Real-World Example: Web Application

**Project structure:**
```
myapp/
├── .tinysecrets.toml
├── package.json
├── src/
│   └── app.js
└── deploy.sh
```

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

# These packs will be loaded and their secrets injected
compose = [
    "openai",       # AI provider credentials
    "stripe",       # Payment processing
    "database",     # PostgreSQL connection
    "redis",        # Cache connection
    "sendgrid",     # Email service
    "auth",         # JWT secrets, Auth0 config
]
```

**What each pack contains:**

```bash
ts pack show openai
# 📦 openai
#   • OPENAI_KEY
#   • OPENAI_ENDPOINT
#   • OPENAI_ORG

ts pack show stripe
# 📦 stripe
#   • STRIPE_SECRET_KEY
#   • STRIPE_WEBHOOK_SECRET
#   • STRIPE_PUBLIC_KEY

ts pack show database
# 📦 database
#   • DATABASE_URL
#   • DB_POOL_SIZE

ts pack show redis
# 📦 redis
#   • REDIS_URL

ts pack show sendgrid
# 📦 sendgrid
#   • SENDGRID_API_KEY
#   • SENDGRID_FROM_EMAIL

ts pack show auth
# 📦 auth
#   • JWT_SECRET
#   • AUTH0_CLIENT_ID
#   • AUTH0_CLIENT_SECRET
```

**When you run your app:**

```bash
ts run -- node src/app.js
```

**What happens:**
1. TinySecrets reads `.tinysecrets.toml`
2. Sees `project = "myapp"` and `environment = "dev"`
3. Sees `compose = ["openai", "stripe", ...]`
4. Loads all 6 packs from the encrypted database
5. Decrypts all secrets from those packs
6. Checks for conflicts (e.g., if two packs define the same key name)
7. Injects 13 environment variables into your app
8. Executes `node src/app.js`

**Your app receives these environment variables:**
```javascript
// In your app.js, you can access:
process.env.OPENAI_KEY           // From openai pack
process.env.OPENAI_ENDPOINT      // From openai pack
process.env.STRIPE_SECRET_KEY    // From stripe pack
process.env.DATABASE_URL         // From database pack
process.env.REDIS_URL            // From redis pack
process.env.SENDGRID_API_KEY     // From sendgrid pack
process.env.JWT_SECRET           // From auth pack
// ... etc
```

---

## Different TOML = Different Behavior

### Scenario 1: Minimal Compose (Local Dev)

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "local"

compose = [
    "database",    # Just local database
    "redis",       # Just local redis
]
```

**Result:**
```bash
ts run -- npm run dev
# ✓ Composed 2 secrets from 2 packs (myapp/local)
# Only DATABASE_URL and REDIS_URL are injected
# No external API keys - perfect for offline development
```

### Scenario 2: Full Compose (Production)

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "prod"

compose = [
    "openai",
    "stripe",
    "database",
    "redis",
    "sendgrid",
    "auth",
    "monitoring",
    "logging",
]
```

**Result:**
```bash
ts run -- ./deploy.sh
# ✓ Composed 20 secrets from 8 packs (myapp/prod)
# All production services loaded
```

### Scenario 3: Testing New Provider

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai.new",     # Testing new OpenAI credentials/endpoint
    "stripe.test",    # Using Stripe test mode
    "database",
    "redis",
]
```

**Result:**
```bash
ts run -- npm test
# ✓ Composed 6 secrets from 4 packs (myapp/dev)
# Uses new/test variants instead of production packs
```

---

## What Happens When Keys Conflict?

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",      # Contains: OPENAI_KEY, API_KEY
    "anthropic",   # Contains: ANTHROPIC_KEY, API_KEY  ⚠️ duplicate!
    "database",
]
```

**Result:**
```bash
ts run -- node app.js
# ✗ Error: Key conflict detected
# ✗ API_KEY defined in multiple packs: openai, anthropic
# 
# Fix by renaming keys in one pack:
#   ts pack set openai OPENAI_API_KEY="$(ts pack get openai API_KEY)"
#   ts pack delete openai API_KEY
```

**What happens:**
- TinySecrets loads both packs
- Finds that both define `API_KEY`
- **Refuses to run** - conflicts are hard errors
- You must fix by renaming keys or removing from one pack

---

## No TOML File vs With TOML File

### Without `.tinysecrets.toml`:

```bash
# You must specify project and environment every time
ts set -p myapp -e dev DATABASE_URL "postgres://..."
ts get -p myapp -e dev DATABASE_URL
ts run -p myapp -e dev -- npm start
```

### With `.tinysecrets.toml`:

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"
```

```bash
# Much cleaner - project/env are automatic
ts set DATABASE_URL "postgres://..."
ts get DATABASE_URL
ts run -- npm start

# You can still override when needed
ts run -e prod -- ./deploy.sh
```

---

## Complete Workflow Example

Let's walk through a complete real-world setup:

### 1. Start with flat secrets

```bash
cd ~/projects/myapp
ts config init myapp dev

# Add secrets the old way
ts set OPENAI_KEY "sk-dev-123"
ts set OPENAI_ENDPOINT "https://api.openai.com/v1"
ts set STRIPE_SECRET_KEY "sk_test_abc"
ts set DATABASE_URL "postgres://localhost/mydb"

# Your .tinysecrets.toml:
```
```toml
project = "myapp"
environment = "dev"
```

### 2. Organize into packs

```bash
ts pack group
# ✓ Created 2 packs: openai (2 keys), stripe (1 key)
# ✓ 'other' pack created with 1 key
# ✓ Updated .tinysecrets.toml with compose
```

**Your `.tinysecrets.toml` now looks like:**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "other",
]
```

**What changed:**
- Your secrets are now organized into packs
- The compose array lists all packs
- `ts run` behavior is **identical** to before - same secrets loaded

### 3. Refine organization

```bash
# Rename 'other' pack to 'infra'
ts pack move other infra DATABASE_URL
```

**Your `.tinysecrets.toml` automatically updates to:**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "infra",    # 'other' replaced with 'infra'
]
```

### 4. Create a variant for testing

```bash
# Clone openai pack to create a new variant
ts pack clone openai openai.new

# Update the new variant with experimental credentials
ts pack set openai.new \
    OPENAI_KEY="sk-new-experimental-456" \
    OPENAI_ENDPOINT="https://api.openai.com/v2"

# To use the new variant, edit .tinysecrets.toml:
```

```toml
project = "myapp"
environment = "dev"

compose = [
    "openai.new",    # Changed from "openai"
    "stripe",
    "infra",
]
```

**Now when you run:**
```bash
ts run -- npm test
# ✓ Composed 4 secrets from 3 packs (myapp/dev)
# Uses OPENAI_KEY from openai.new (the experimental key)
```

### 5. Branch-specific variants

Create a feature branch with different secrets:

```bash
git checkout -b feature/payment-v2

# Edit .tinysecrets.toml on this branch:
```

```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe.v2",     # New Stripe integration
    "infra",
]
```

```bash
# Create the stripe.v2 pack
ts pack set stripe.v2 \
    STRIPE_SECRET_KEY="sk_test_new_api_xyz" \
    STRIPE_API_VERSION="2024-01-01"

# Run on feature branch
ts run -- npm test
# ✓ Composed 5 secrets from 3 packs
# Uses stripe.v2 pack with new API

# Switch back to main
git checkout main
ts run -- npm test
# ✓ Composed 4 secrets from 3 packs
# Uses original stripe pack
```

---

## Advanced: Per-Environment Compose

You can compose different packs for different environments:

**Development** (`.tinysecrets.toml`):
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",          # Dev OpenAI key
    "stripe.test",     # Stripe test mode
    "database.local",  # Local postgres
    "redis.local",     # Local redis
]
```

**Production** (switch environment):
```bash
ts config set -e prod
```

Your TOML might become:
```toml
project = "myapp"
environment = "prod"

compose = [
    "openai.prod",     # Production OpenAI key
    "stripe",          # Live Stripe
    "database",        # Cloud database
    "redis",           # Cloud redis
    "monitoring",      # Only in prod
    "logging",         # Only in prod
]
```

**What happens:**

```bash
# Development
ts run -e dev -- npm run dev
# Loads: myapp/dev packs (local services, test APIs)

# Production
ts run -e prod -- ./deploy.sh
# Loads: myapp/prod packs (cloud services, live APIs, monitoring)
```

---

## Summary: TOML Fields and Their Effects

| Field | Required | What It Does |
|-------|----------|--------------|
| `project` | No* | Default project name for all commands |
| `environment` | No* | Default environment for all commands |
| `compose` | No | List of packs to load for `ts run`. If omitted, ALL packs are loaded |

*Optional but recommended - makes commands much cleaner

### How `compose` Works

**With compose:**
```toml
compose = ["pack1", "pack2"]
```
- `ts run` loads **ONLY** pack1 and pack2
- Flat secrets (legacy non-pack secrets) are **ignored**
- Explicit selection

**Without compose:**
```toml
# No compose field
```
- `ts run` loads **ALL packs** for this project/environment
- **PLUS** any remaining flat secrets
- Everything included

---

## Pro Tips

### Tip 1: Start Simple
```toml
# Begin with just project and environment
project = "myapp"
environment = "dev"

# Add compose later when you have packs
```

### Tip 2: Gitignore Personal Variants
```bash
# .gitignore
.tinysecrets.toml.local

# Everyone commits .tinysecrets.toml with shared packs
# You keep .tinysecrets.toml.local with your personal overrides
```

### Tip 3: Preview Before Running
```bash
# Check what will be loaded
ts compose show

# Validate no conflicts
ts compose check

# Then run
ts run -- npm start
```

### Tip 4: Use Variants for Canaries
```toml
# Canary branch
compose = ["api.canary", "database", "redis"]

# Main branch
compose = ["api", "database", "redis"]

# Gradually roll out new credentials
```

---

## Quick Decision Tree

**Should you use packs?**

- **Few secrets (< 5):** Probably not needed, flat secrets are fine
- **Many secrets (10+):** Yes! Packs help organize them
- **Need variants:** Yes! Packs are perfect for this
- **Multiple environments:** Yes! Same pack names, different values per env
- **Team collaboration:** Yes! Share packs via compose lists
- **Branch-specific needs:** Yes! Each branch can compose differently

**Should you use `compose` in your TOML?**

- **Want everything loaded:** No, omit `compose` and all packs load
- **Want selective loading:** Yes, specify which packs
- **Different secrets per branch:** Yes, compose per branch
- **Testing variants:** Yes, include variant in compose

---

For more examples and patterns, see:
- [`PACKS_EXAMPLE.md`](./PACKS_EXAMPLE.md) - Complete walkthrough
- [`PACKS_SPEC.md`](./PACKS_SPEC.md) - Technical specification
- [`../.tinysecrets.toml.example`](../.tinysecrets.toml.example) - Example configuration
