# `.tinysecrets.toml` Behavior Examples

This guide shows **exactly** what output you'll see when running commands with different TOML configurations.

---

## Example 1: Basic Config (No Packs)

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"
```

### Your Secrets (flat, no packs):
```bash
$ ts list
myapp/dev
  • DATABASE_URL
  • REDIS_URL
  • API_KEY
```

### What Happens When You Run:
```bash
$ ts run -- npm start

✓ Loaded 3 secrets from myapp/dev
> myapp@1.0.0 start
> node server.js
Server listening on port 3000...
```

**Environment variables your app receives:**
- `DATABASE_URL`
- `REDIS_URL`
- `API_KEY`

---

## Example 2: After Organizing into Packs

You ran `ts pack group` and organized your secrets.

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "infra",
]
```

### Your Packs:
```bash
$ ts pack list
📦 myapp/dev
  ├─ openai (2 keys)
  ├─ stripe (2 keys)
  └─ infra (2 keys)
```

### What Happens When You Run:
```bash
$ ts run -- npm start

✓ Composed 6 secrets from 3 packs (myapp/dev)
> myapp@1.0.0 start
> node server.js
Server listening on port 3000...
```

**Environment variables your app receives:**
- `OPENAI_KEY` (from openai pack)
- `OPENAI_ENDPOINT` (from openai pack)
- `STRIPE_SECRET_KEY` (from stripe pack)
- `STRIPE_WEBHOOK_SECRET` (from stripe pack)
- `DATABASE_URL` (from infra pack)
- `REDIS_URL` (from infra pack)

---

## Example 3: Preview Before Running

### Check what will be loaded:
```bash
$ ts compose show

📋 Compose: myapp/dev

openai
  • OPENAI_ENDPOINT
  • OPENAI_KEY
stripe
  • STRIPE_SECRET_KEY
  • STRIPE_WEBHOOK_SECRET
infra
  • DATABASE_URL
  • REDIS_URL

Total: 6 env vars from 3 packs
✓ No conflicts
```

### Validate composition:
```bash
$ ts compose check

✓ All 3 packs exist
✓ No key conflicts
✓ 6 env vars will be injected
```

---

## Example 4: Using Pack Variants

You want to test new OpenAI credentials without changing your main setup.

### Step 1: Create a variant
```bash
$ ts pack clone openai openai.new
✓ Cloned openai → openai.new (2 keys)

$ ts pack set openai.new OPENAI_KEY "sk-new-experimental-key-123"
✓ Updated OPENAI_KEY in pack 'openai.new' (v2)
```

### Step 2: Update your TOML to use the variant

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai.new",    # Changed from "openai"
    "stripe",
    "infra",
]
```

### Step 3: Run with the new variant
```bash
$ ts run -- npm test

✓ Composed 6 secrets from 3 packs (myapp/dev)
Running tests with OPENAI_KEY from openai.new pack...
✓ All tests passed
```

### Step 4: Switch back easily
```bash
# Edit .tinysecrets.toml - change "openai.new" back to "openai"
# Or keep both and switch via git branches!

$ ts run -- npm test
✓ Composed 6 secrets from 3 packs (myapp/dev)
Running tests with OPENAI_KEY from openai pack...
```

---

## Example 5: No Compose = Load All Packs

You have packs but don't want to list them all.

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"

# No compose field - load everything!
```

### Your Packs:
```bash
$ ts pack list
📦 myapp/dev
  ├─ openai (2 keys)
  ├─ stripe (2 keys)
  ├─ database (1 key)
  ├─ redis (1 key)
  ├─ sendgrid (1 key)
  ├─ auth (3 keys)
  ├─ monitoring (2 keys)
  └─ experimental (1 key)
```

### What Happens When You Run:
```bash
$ ts run -- npm start

✓ Loaded 13 secrets from 8 packs (myapp/dev)
> myapp@1.0.0 start
> node server.js
All services initialized...
```

**All 8 packs are loaded** - no need to list them in compose!

---

## Example 6: Conflict Detection

You have two packs that define the same key.

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",       # Has: OPENAI_KEY, API_KEY
    "anthropic",    # Has: ANTHROPIC_KEY, API_KEY  ⚠️
    "database",
]
```

### What Happens:
```bash
$ ts run -- npm start

✗ Error: Key conflict detected
✗ API_KEY is defined in multiple packs: openai, anthropic

To fix this, either:
  1. Rename the key in one pack:
     ts pack set openai OPENAI_API_KEY="$(ts pack get openai API_KEY)"
     
  2. Remove from one pack:
     ts pack delete openai API_KEY
     
  3. Update compose to include only one:
     compose = ["openai", "database"]  # Remove anthropic
```

**TinySecrets refuses to run** until you resolve the conflict. This prevents accidental overwrites.

---

## Example 7: CLI Overrides

Your TOML has a standard compose, but you want to add packs temporarily.

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "database",
]
```

### Add temporary packs:
```bash
# Add monitoring pack just for this run
$ ts run --with monitoring -- npm test

✓ Composed 8 secrets from 4 packs (myapp/dev)
  • openai (from compose)
  • stripe (from compose)
  • database (from compose)
  • monitoring (from --with flag)
```

### Completely replace compose:
```bash
# Ignore TOML, use only these packs
$ ts run --compose "openai.experimental,database" -- npm test

✓ Composed 3 secrets from 2 packs (myapp/dev)
  • openai.experimental (from --compose flag)
  • database (from --compose flag)
  [stripe is NOT loaded, even though it's in TOML]
```

---

## Example 8: Multi-Environment with Same TOML

You have one TOML file and switch environments via CLI.

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"    # Default to dev

compose = [
    "openai",
    "stripe",
    "database",
]
```

### Development (default):
```bash
$ ts run -- npm run dev

✓ Composed 6 secrets from 3 packs (myapp/dev)
[Loads: myapp/dev/openai, myapp/dev/stripe, myapp/dev/database]
```

### Switch to production:
```bash
$ ts run -e prod -- ./deploy.sh

✓ Composed 6 secrets from 3 packs (myapp/prod)
[Loads: myapp/PROD/openai, myapp/PROD/stripe, myapp/PROD/database]
[Same pack names, different environment = different secrets!]
```

---

## Example 9: Branch-Specific Secrets

The real power of packs: each git branch can have different compose.

### Main Branch

**File: `.tinysecrets.toml`**
```toml
project = "myapp"
environment = "prod"

compose = [
    "openai",       # Stable OpenAI
    "stripe",       # Production Stripe
    "database",
]
```

```bash
$ git checkout main
$ ts run -- npm start

✓ Composed 6 secrets from 3 packs (myapp/prod)
[Using openai pack]
```

### Feature Branch

**File: `.tinysecrets.toml`** (different content!)
```toml
project = "myapp"
environment = "prod"

compose = [
    "anthropic",    # Testing Anthropic instead!
    "stripe",
    "database",
]
```

```bash
$ git checkout feature/test-anthropic
$ ts run -- npm start

✓ Composed 6 secrets from 3 packs (myapp/prod)
[Using anthropic pack instead of openai]
```

**The key insight:**
- `.tinysecrets.toml` is tracked in git
- Different branches can have different TOML files
- Same `ts run` command loads different secrets depending on branch
- No secret values are in git (only pack names in compose)

---

## Example 10: Show Pack Contents

See exactly what's in each pack:

```bash
$ ts pack show openai

📦 openai (myapp/dev)
  • OPENAI_ENDPOINT  v1
  • OPENAI_KEY       v1
  
$ ts pack show openai --reveal

📦 openai (myapp/dev)
  • OPENAI_ENDPOINT = https://api.openai.com/v1  v1
  • OPENAI_KEY      = sk-dev-abc123...           v1
```

---

## Example 11: Working Without Packs

You don't need to use packs - everything works without them:

### Your File: `.tinysecrets.toml`
```toml
project = "myapp"
environment = "dev"
# No compose field, no packs
```

### Commands work normally:
```bash
$ ts set DATABASE_URL "postgres://localhost/mydb"
✓ Set DATABASE_URL (v1)

$ ts get DATABASE_URL
postgres://localhost/mydb

$ ts run -- npm start
✓ Loaded 3 secrets from myapp/dev
```

**Packs are completely optional.** Use them when you need organization or variants.

---

## Decision Guide

### Use this TOML:
```toml
project = "myapp"
environment = "dev"
```
**If:** You have few secrets and don't need organization.

### Use this TOML:
```toml
project = "myapp"
environment = "dev"

compose = ["database", "redis"]
```
**If:** You want to load only specific packs (selective loading).

### Use this TOML:
```toml
project = "myapp"
environment = "dev"
# No compose
```
**If:** You have packs but want all of them loaded automatically.

### Use this TOML:
```toml
project = "myapp"
environment = "dev"

compose = ["openai.new", "stripe.test", "database.local"]
```
**If:** You're testing variants or need branch-specific compositions.

---

## Summary Table

| TOML Config | `ts run` Behavior | Use Case |
|-------------|-------------------|----------|
| Just project/env | Loads all flat secrets | Basic setup, no packs |
| + `compose = [...]` | Loads ONLY listed packs | Selective composition |
| + no compose field | Loads ALL packs + flat secrets | Post-migration, load everything |
| + pack variants in compose | Loads specific variants | Testing, A/B, canaries |
| Different per branch | Different packs per branch | Feature development |

---

For more details:
- [Complete TOML Configuration Guide](./TOML_CONFIG_GUIDE.md)
- [Packs Example Walkthrough](./PACKS_EXAMPLE.md)
