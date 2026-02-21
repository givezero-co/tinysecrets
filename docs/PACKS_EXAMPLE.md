# Packs: Complete Example Walkthrough

This guide walks through a realistic scenario of using packs to organize secrets for a web application.

## Scenario: Building a SaaS Application

You're building a web app that uses:
- OpenAI for AI features
- Stripe for payments
- PostgreSQL database
- Redis for caching
- SendGrid for emails

Let's see how packs help you manage these secrets across development and production.

---

## Step 1: Initial Setup (Flat Secrets)

Start with the traditional flat approach:

```bash
# Initialize your project
cd ~/projects/myapp
tinysecrets config init myapp dev

# Add your secrets
ts set OPENAI_KEY "sk-dev-abc123"
ts set OPENAI_ENDPOINT "https://api.openai.com/v1"
ts set STRIPE_SECRET_KEY "sk_test_abc123"
ts set STRIPE_WEBHOOK_SECRET "whsec_test_xyz"
ts set DATABASE_URL "postgres://localhost/myapp_dev"
ts set REDIS_URL "redis://localhost:6379"
ts set SENDGRID_API_KEY "SG.test.abc123"

# List what you have
ts list
# myapp/dev
#   • OPENAI_KEY
#   • OPENAI_ENDPOINT
#   • STRIPE_SECRET_KEY
#   • STRIPE_WEBHOOK_SECRET
#   • DATABASE_URL
#   • REDIS_URL
#   • SENDGRID_API_KEY
```

---

## Step 2: Organize into Packs

As your project grows, flat secrets become hard to manage. Let's organize them:

```bash
# Run the grouping wizard
ts pack group

# Output:
# 📋 Found 7 flat secrets in myapp/dev
#
# Suggested groups (by prefix, 2+ keys):
#   openai    ← OPENAI_ENDPOINT, OPENAI_KEY
#   stripe    ← STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET
#
# Ungrouped (3 keys):
#   DATABASE_URL, REDIS_URL, SENDGRID_API_KEY
#
# Accept suggested groups? [Y/n] y
# Move 3 remaining keys to 'other'? [Y/n] y
#
# ✓ Created 3 packs:
#   openai    (2 keys)
#   stripe    (2 keys)
#   other     (3 keys)
```

Your `.tinysecrets.toml` is now:

```toml
project = "myapp"
environment = "dev"

compose = [
    "openai",
    "stripe",
    "other",
]
```

---

## Step 3: Refine Organization

Let's break out `other` into proper packs:

```bash
# Move database and redis to an 'infra' pack
ts pack move other infra DATABASE_URL REDIS_URL
# ✓ Moved 2 keys: other → infra
# ✓ Added 'infra' to compose

# Move SendGrid to its own 'email' pack
ts pack move other email SENDGRID_API_KEY
# ✓ Moved 1 key: other → email
# ✓ Added 'email' to compose
# ⚠ Pack 'other' is now empty. Delete it? [Y/n] y
# ✓ Deleted pack 'other'
# ✓ Removed 'other' from compose

# Check your new structure
ts pack list
# 📦 myapp/dev
#   ├─ openai (2 keys)
#   ├─ stripe (2 keys)
#   ├─ infra (2 keys)
#   └─ email (1 key)
```

---

## Step 4: Add Production Environment

```bash
# Switch to prod environment
ts config set -e prod

# Create production packs with real credentials
ts pack set openai \
    OPENAI_KEY="sk-prod-live-xyz" \
    OPENAI_ENDPOINT="https://api.openai.com/v1"

ts pack set stripe \
    STRIPE_SECRET_KEY="sk_live_prod_xyz" \
    STRIPE_WEBHOOK_SECRET="whsec_prod_xyz"

ts pack set infra \
    DATABASE_URL="postgres://prod.example.com/myapp" \
    REDIS_URL="redis://prod-redis.example.com:6379"

ts pack set email \
    SENDGRID_API_KEY="SG.prod.real.key"

# Your production .tinysecrets.toml:
# project = "myapp"
# environment = "prod"
# compose = ["openai", "stripe", "infra", "email"]

# Run production deploy
ts run -- ./deploy.sh
# ✓ Composed 7 secrets from 4 packs (myapp/prod)
```

---

## Step 5: Create Pack Variants for Testing

You want to test switching from OpenAI to Anthropic without touching your main setup:

```bash
# Switch back to dev
ts config set -e dev

# Clone your current openai pack as a backup
ts pack clone openai openai.v1

# Create an anthropic pack with similar keys
ts pack set anthropic \
    ANTHROPIC_KEY="sk-ant-dev-123" \
    ANTHROPIC_ENDPOINT="https://api.anthropic.com"

# Update your compose to use anthropic instead
# Edit .tinysecrets.toml:
compose = [
    "anthropic",   # Changed from "openai"
    "stripe",
    "infra",
    "email",
]

# Test the new setup
ts compose check
# ✓ All 4 packs exist
# ✓ No key conflicts
# ✓ 7 env vars will be injected

ts run -- npm test
# ✓ Composed 7 secrets from 4 packs (myapp/dev)
```

If you need to switch back:

```bash
# Edit .tinysecrets.toml:
compose = [
    "openai.v1",   # Back to the backup
    "stripe",
    "infra",
    "email",
]
```

---

## Step 6: Branch-Specific Composition

Create a feature branch with its own secret composition:

```bash
# Create feature branch
git checkout -b feature/test-new-payment-provider

# Edit .tinysecrets.toml on this branch:
compose = [
    "openai",
    "stripe.test",      # Use test mode Stripe
    "paypal",           # Add new payment provider
    "infra",
    "email",
]

# Create the paypal pack
ts pack set paypal \
    PAYPAL_CLIENT_ID="test-client" \
    PAYPAL_SECRET="test-secret"

# Create stripe.test variant
ts pack clone stripe stripe.test
ts pack set stripe.test \
    STRIPE_SECRET_KEY="sk_test_restricted_mode"

# Run your feature branch with its own composition
ts run -- npm run dev
# ✓ Composed 9 secrets from 5 packs (myapp/dev)

# When you switch back to main:
git checkout main
ts run -- npm run dev
# ✓ Composed 7 secrets from 4 packs (myapp/dev)
# (Different compose, different secrets!)
```

---

## Common Patterns

### Pattern 1: Per-Developer Customization

Each developer can have their own pack variants:

```bash
# Alice creates her own OpenAI variant with her API key
ts pack set openai.alice \
    OPENAI_KEY="sk-alice-personal-key" \
    OPENAI_ENDPOINT="https://api.openai.com/v1"

# In Alice's .tinysecrets.toml (not committed to git):
compose = ["openai.alice", "stripe", "infra", "email"]

# Bob uses the shared dev pack
# In Bob's .tinysecrets.toml:
compose = ["openai", "stripe", "infra", "email"]
```

### Pattern 2: Canary Deployments

Test new API keys in production with a canary:

```bash
# Production main branch uses openai pack
compose = ["openai", "stripe", "infra"]

# Canary branch uses new credentials
compose = ["openai.canary", "stripe", "infra"]

# Create the canary pack
ts pack set -e prod openai.canary \
    OPENAI_KEY="sk-prod-new-key-xyz" \
    OPENAI_ENDPOINT="https://api.openai.com/v2"

# Deploy canary
git checkout canary
ts run -- ./deploy.sh --target canary-servers
```

### Pattern 3: Local Development Overrides

Skip external services during local development:

```bash
# Create a minimal local compose
# .tinysecrets.toml:
project = "myapp"
environment = "local"

compose = [
    "database",  # Local postgres
    "redis",     # Local redis
]

# Run without hitting external APIs
ts run -- npm run dev
```

### Pattern 4: Temporary Pack for Testing

```bash
# Add a monitoring pack just for this test run
ts run --with monitoring -- npm test

# Or completely override compose
ts run --compose openai.experimental,stripe.test,infra -- npm test
```

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `ts pack group` | Organize flat secrets into packs (interactive) |
| `ts pack list` | Show all packs |
| `ts pack show <pack>` | Show keys in a specific pack |
| `ts pack set <pack> KEY=value` | Create/update secrets in a pack |
| `ts pack get <pack> KEY` | Get a specific secret from a pack |
| `ts pack clone <src> <dst>` | Duplicate a pack (for variants) |
| `ts pack move <src> <dst> KEY...` | Move keys between packs |
| `ts pack delete <pack>` | Delete a pack |
| `ts compose show` | Preview what will be injected |
| `ts compose check` | Validate your composition |
| `ts run --with <pack>` | Add temporary pack to compose |
| `ts run --compose <packs>` | Override compose from CLI |

---

## Benefits of Packs

1. **Organization**: Group related secrets together (all Stripe keys in one pack)
2. **Variants**: Create multiple versions (openai, openai.new, openai.old)
3. **Branch-specific secrets**: Different branches compose different packs
4. **Selective loading**: Only load what you need for each command
5. **Conflict detection**: Automatic detection of duplicate keys across packs
6. **No migration required**: Old commands still work, packs are additive

---

For the complete technical specification, see [`PACKS_SPEC.md`](./PACKS_SPEC.md).
