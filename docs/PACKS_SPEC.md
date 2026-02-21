# Packs & Compose — Feature Spec

## Overview

Packs turn tinysecrets from "encrypted .env" into composable, modular secret management. A **pack** is a named group of related secrets. A **compose manifest** in the repo declares which packs to assemble at runtime. Different branches can compose different pack variants.

### Mental Model

```
Store (encrypted, local)              Repo (.tinysecrets.toml, git-tracked)
┌─────────────────────────┐           ┌──────────────────────────┐
│ gzback.prod.openai      │           │ project = "gzback"       │
│   OPENAI_ENDPOINT = ... │           │ environment = "prod"     │
│   OPENAI_KEY = ...      │           │                          │
│                         │           │ compose = [              │
│ gzback.prod.openai.old  │           │   "openai.new",          │
│   OPENAI_ENDPOINT = ... │           │   "stripe",              │
│   OPENAI_KEY = ...      │  ◀─────  │   "database",            │
│                         │  selects  │   "redis",               │
│ gzback.prod.openai.new  │           │ ]                        │
│   OPENAI_ENDPOINT = ... │           └──────────────────────────┘
│   OPENAI_KEY = ...      │
│                         │
│ gzback.prod.stripe      │
│   STRIPE_SECRET_KEY=... │
│   STRIPE_WEBHOOK_SEC=.. │
│                         │
│ gzback.prod.database    │
│   DATABASE_URL = ...    │
│                         │
│ gzback.prod.redis       │
│   REDIS_URL = ...       │
└─────────────────────────┘
```

`ts run -- python app.py` reads the compose list, resolves each pack, merges all key-value pairs, and injects them as env vars.

---

## Core Concepts

### Pack

A named bag of key-value pairs scoped to a project and environment. The pack name is for human organization — it never affects the env var names.

- **Full keypath**: `project.environment.pack_name` (e.g. `gzback.prod.openai`)
- **Short name**: `pack_name` (e.g. `openai`) — resolved using project/env from config
- **Variants**: Dot-separated suffixes: `openai`, `openai.old`, `openai.new`
  - Variants are independent packs that happen to share a naming convention
  - No parent/child relationship — `openai.new` doesn't inherit from `openai`

### Compose

A list of pack names in `.tinysecrets.toml` that declares which packs to assemble at runtime. Order matters only for display; key conflicts are hard errors.

### Keypath

Dot-notation addressing: `project.environment.pack[.key]`

```
gzback.prod.openai           → the pack
gzback.prod.openai.ENDPOINT  → a specific key in the pack
gzback.prod.openai.new       → a variant pack (context-dependent)
```

When a keypath is ambiguous (is `openai.new` a variant pack or a key called `new` in pack `openai`?), pack names take precedence. Use explicit key access syntax for the edge case (see CLI section).

---

## Data Model

### Schema v3 (new tables alongside existing)

```sql
-- Pack metadata
CREATE TABLE IF NOT EXISTS packs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    environment TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project, environment, name)
);

-- Secrets within packs
CREATE TABLE IF NOT EXISTS pack_secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pack_id INTEGER NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(pack_id, key)
);

-- Pack secret history (audit trail)
CREATE TABLE IF NOT EXISTS pack_secrets_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pack_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_packs_project_env ON packs(project, environment);
CREATE INDEX IF NOT EXISTS idx_pack_secrets_pack_id ON pack_secrets(pack_id);
CREATE INDEX IF NOT EXISTS idx_pack_history_pack_key ON pack_secrets_history(pack_id, key);
```

The existing `secrets` and `secret_history` tables remain for backward compatibility during migration.

---

## Config Changes

### `.tinysecrets.toml`

```toml
project = "gzback"
environment = "prod"

# Compose env from these packs (required for `ts run` when using packs)
compose = [
    "openai.new",
    "stripe",
    "database",
    "redis",
    "sendgrid",
]
```

### Config struct update

```rust
pub struct Config {
    pub project: Option<String>,
    pub environment: Option<String>,
    pub compose: Option<Vec<String>>,  // NEW
}
```

---

## Compatibility Mode

Packs are an organizational improvement, not a breaking change. Existing commands
work transparently across packs — no `compose` required.

### The Rule: No compose = all packs

When `.tinysecrets.toml` has no `compose` list, every command operates on **all packs**
for the project/environment. The user never has to declare compose to keep working.

### Secret resolution order

All read/write commands resolve secrets in this order:

1. **Pack secrets** — search across all packs (or composed packs if compose is set)
2. **Flat secrets** — legacy `secrets` table (pre-migration data)

This means: after running `pack group`, everything keeps working identically.
After migrating some but not all secrets, both sources are merged.

### How existing commands behave with packs

**`ts run` (no compose)**
```bash
ts run -- python app.py
# No compose in toml → loads ALL packs for project/env
# Also loads any remaining flat secrets
# Merges everything, checks for conflicts, injects
✓ Loaded 18 secrets from 5 packs (gzback/prod)
```

**`ts run` (with compose)**
```bash
ts run -- python app.py
# compose in toml → loads ONLY listed packs
# Flat secrets are NOT included (compose is explicit)
✓ Composed 7 secrets from 3 packs (gzback/prod)
```

**`ts get` — searches across packs transparently**
```bash
ts get DATABASE_URL
# → searches all packs (and flat secrets), returns the value
# Found in pack 'infra'

# Key in multiple packs → error with guidance
ts get API_KEY
# ✗ API_KEY found in multiple packs: openai, anthropic
#   Use: ts pack get openai API_KEY
```

**`ts set` — updates in place, wherever the key lives**
```bash
ts set DATABASE_URL "postgres://new..."
# → found in pack 'infra', updated in place
✓ Updated DATABASE_URL in pack 'infra' (v3)

# Key not found anywhere → error with guidance
ts set BRAND_NEW_KEY "value"
# ✗ BRAND_NEW_KEY not found in any pack
#   Add to a pack: ts pack set <pack> BRAND_NEW_KEY="value"
```

**`ts list` — shows packs when they exist**
```bash
ts list
# If packs exist → shows pack-grouped view (same as ts pack list)
# If only flat secrets → shows legacy view
# If both → shows both sections
```

**`ts delete` — finds and deletes from whichever pack**
```bash
ts delete REDIS_URL
# → found in pack 'infra', deleted (archived to history)
✓ Deleted REDIS_URL from pack 'infra'
```

### Summary

| Scenario | `ts run` behavior |
|---|---|
| Only flat secrets, no compose | Load all flat secrets (legacy, unchanged) |
| Packs exist, no compose | Load all packs + remaining flat secrets |
| Packs exist, compose set | Load only composed packs (flat secrets ignored) |

This means the user journey is:
1. **Day 0**: Flat secrets, everything works
2. **Day 1**: Run `pack group`, secrets move to packs, *everything still works*
3. **Day 2**: Add `compose` to toml — opt in to selective pack composition
4. **Day N**: Per-branch compose, variants, the full power

At no point does anything break.

---

## CLI

### `ts pack set` — Create or update a pack

```bash
# Set multiple keys at once (recommended)
ts pack set gzback.prod.openai \
    OPENAI_ENDPOINT="https://api.openai.com" \
    OPENAI_KEY="sk-abc123"

# With project/env from config
ts pack set openai \
    OPENAI_ENDPOINT="https://api.openai.com" \
    OPENAI_KEY="sk-abc123"

# Set a single key (opens $EDITOR if no value)
ts pack set openai OPENAI_KEY

# Set a single key with value
ts pack set openai OPENAI_KEY="sk-abc123"
```

**Behavior:**
- Creates the pack if it doesn't exist
- Creates or updates individual keys
- Existing keys in the pack that aren't mentioned are untouched
- Old values archived to `pack_secrets_history`

### `ts pack get` — Get a value from a pack

```bash
# Get a specific key
ts pack get openai OPENAI_KEY
# → sk-abc123

# Get from a variant
ts pack get openai.new OPENAI_KEY

# Full keypath
ts pack get gzback.prod.openai.new OPENAI_KEY
```

### `ts pack list` — List packs

```bash
# List all packs for current project/env
ts pack list
# 📦 gzback/prod
#   ├─ openai (2 keys)
#   │  ├─ .old (2 keys)
#   │  └─ .new (2 keys)
#   ├─ stripe (2 keys)
#   ├─ database (1 key)
#   ├─ redis (1 key)
#   └─ sendgrid (1 key)

# List with keys visible
ts pack list --keys
# 📦 gzback/prod
#   ├─ openai
#   │  • OPENAI_ENDPOINT
#   │  • OPENAI_KEY
#   │  ├─ .old
#   │  │  • OPENAI_ENDPOINT
#   │  │  • OPENAI_KEY
#   │  └─ .new
#   │     • OPENAI_ENDPOINT
#   │     • OPENAI_KEY
#   ├─ stripe
#   │  • STRIPE_SECRET_KEY
#   │  • STRIPE_WEBHOOK_SECRET
#   ...
```

Display groups variants hierarchically by prefix even though they're stored flat. `openai`, `openai.old`, `openai.new` renders as a tree.

### `ts pack show` — Show keys in a specific pack

```bash
ts pack show openai.new
# 📦 openai.new (gzback/prod)
#   • OPENAI_ENDPOINT  v2
#   • OPENAI_KEY       v1

# Show values (decrypted)
ts pack show openai.new --reveal
# 📦 openai.new (gzback/prod)
#   • OPENAI_ENDPOINT = https://new.openai.com  v2
#   • OPENAI_KEY      = sk-new...               v1
```

### `ts pack clone` — Clone a pack

```bash
# Clone to create a variant
ts pack clone openai openai.old

# Clone across environments
ts pack clone gzback.prod.openai gzback.staging.openai

# Clone across projects
ts pack clone gzback.prod.stripe otherproject.prod.stripe
```

**Behavior:**
- Copies all keys and their current values to the new pack
- New pack is fully independent (no link to source)
- Fails if target pack already exists (use `--force` to overwrite)

### `ts pack delete` — Delete a pack

```bash
ts pack delete openai.old

# Delete with confirmation prompt
ts pack delete openai
# ⚠ Pack 'openai' contains 2 secrets. Delete? [y/N]
```

**Behavior:**
- Archives all pack secrets to `pack_secrets_history`
- Deletes the pack and its secrets

### `ts pack history` — History for a pack secret

```bash
ts pack history openai OPENAI_KEY
# v3  2025-02-20T10:30:00Z  (current)
# v2  2025-02-15T08:00:00Z
# v1  2025-01-01T12:00:00Z
```

---

### `ts compose show` — Preview the assembled environment

```bash
ts compose show
# 📋 Compose: gzback/prod
#
# openai.new
#   • OPENAI_ENDPOINT
#   • OPENAI_KEY
# stripe
#   • STRIPE_SECRET_KEY
#   • STRIPE_WEBHOOK_SECRET
# database
#   • DATABASE_URL
# redis
#   • REDIS_URL
# sendgrid
#   • SENDGRID_API_KEY
#
# Total: 7 env vars from 5 packs
# ✓ No conflicts

# With values
ts compose show --reveal
```

### `ts compose check` — Validate composition

```bash
ts compose check
# ✓ All 5 packs exist
# ✓ No key conflicts
# ✓ 7 env vars will be injected

# On error:
ts compose check
# ✗ Pack 'monitoring' not found in gzback/prod
# ✗ Key conflict: API_KEY defined in both 'openai.new' and 'anthropic'
```

---

### `ts run` — Updated to use compose

```bash
ts run -- python app.py
```

**Resolution order** (see Compatibility Mode for details):
1. Read `.tinysecrets.toml`
2. If `compose` is present → assemble env from listed packs only
3. If `compose` is absent → load all packs + any remaining flat secrets
4. Check for key conflicts → hard error if found
5. Inject assembled env vars
6. `exec()` the command

**Output:**
```
# With compose
✓ Composed 7 secrets from 3 packs (gzback/prod)

# Without compose
✓ Loaded 18 secrets from 5 packs (gzback/prod)
```

**Override compose from CLI:**
```bash
# Add extra packs beyond what's in compose
ts run --with monitoring -- python app.py

# Use completely different compose (ignore toml)
ts run --compose openai.old,stripe,database -- python app.py
```

---

## Grouping & Migration

The main onboarding flow for packs. Converts flat secrets into organized packs with
an interactive wizard, then lets you reorganize over time.

### `ts pack group` — Interactive grouping wizard

The primary way to go from flat secrets to packs. Analyzes key prefixes and suggests groups.

```bash
ts pack group

# Step 1: Scan and suggest
📋 Found 18 flat secrets in gzback/prod

Suggested groups (by prefix, 2+ keys):
  openai    ← OPENAI_ENDPOINT, OPENAI_KEY, OPENAI_ORG
  stripe    ← STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, STRIPE_PUBLIC_KEY
  sendgrid  ← SENDGRID_API_KEY, SENDGRID_FROM_EMAIL
  supabase  ← SUPABASE_URL, SUPABASE_KEY, SUPABASE_SERVICE_KEY

Ungrouped (6 keys):
  DATABASE_URL, REDIS_URL, JWT_SECRET, APP_SECRET_KEY, SENTRY_DSN, LOG_LEVEL

# Step 2: Confirm groups
Accept suggested groups? [Y/n] y

# Step 3: Handle ungrouped
Move 6 remaining keys to 'other'? [Y/n] y

✓ Created 5 packs:
  openai    (3 keys)
  stripe    (3 keys)
  sendgrid  (2 keys)
  supabase  (3 keys)
  other     (6 keys)

✓ Removed 18 flat secrets
✓ Updated .tinysecrets.toml with compose
```

**Resulting `.tinysecrets.toml`:**
```toml
project = "gzback"
environment = "prod"

compose = [
    "openai",
    "stripe",
    "sendgrid",
    "supabase",
    "other",
]
```

**Grouping algorithm:**
1. Split each key name by `_` → take the first segment as prefix
2. Group keys sharing the same prefix (case-insensitive)
3. Prefixes with 2+ keys become suggested packs (pack name = lowercased prefix)
4. Prefixes with 1 key → ungrouped
5. All ungrouped keys → `other` pack

**Flags:**
- `ts pack group` — interactive (default)
- `ts pack group --yes` — accept all suggestions without prompting
- `ts pack group --dry-run` — show what would happen without doing it
- `ts pack group --min-size 3` — require 3+ keys to suggest a group (default: 2)

### `ts pack adopt` — Move specific flat secrets into a pack

For manual grouping when you know exactly what you want.

```bash
# Move specific keys into a named pack
ts pack adopt infra DATABASE_URL REDIS_URL SENTRY_DSN
# ✓ Moved 3 secrets into pack 'infra'
# ✓ Removed from flat secrets
# ✓ Added 'infra' to compose

# Move all remaining flat secrets into one pack
ts pack adopt everything
# ✓ Moved 15 secrets into pack 'everything'
```

### `ts pack move` — Reorganize keys between packs

After initial grouping, move keys from one pack to another. The classic use case:
pull things out of `other` into proper packs.

```bash
# Move keys from 'other' to a new pack 'infra'
ts pack move other infra DATABASE_URL REDIS_URL SENTRY_DSN
# ✓ Moved 3 keys: other → infra
# ✓ Added 'infra' to compose

# Move one more key later
ts pack move other infra CORS_ORIGINS
# ✓ Moved 1 key: other → infra
# (infra already in compose, no change needed)

# Move between any packs
ts pack move stripe payments STRIPE_PRICE_ID
# ✓ Moved 1 key: stripe → payments
# ✓ Added 'payments' to compose
```

**Behavior:**
- Creates the target pack if it doesn't exist
- Removes the key from the source pack (archives to history)
- Adds the target pack to compose if not already present
- Warns if the source pack becomes empty after the move
- Atomic: all keys move together or none do

```bash
# If source pack is now empty after move:
ts pack move other infra LOG_LEVEL
# ✓ Moved 1 key: other → infra
# ⚠ Pack 'other' is now empty. Delete it? [Y/n] y
# ✓ Deleted pack 'other'
# ✓ Removed 'other' from compose
```

### Schema migration

```rust
fn migrate_v2_to_v3(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS packs (...);
        CREATE TABLE IF NOT EXISTS pack_secrets (...);
        CREATE TABLE IF NOT EXISTS pack_secrets_history (...);
        -- indexes --
        UPDATE metadata SET value = '3' WHERE key = 'schema_version';
    ")?;
    Ok(())
}
```

Schema migration is automatic on `store.open()`. No data migration needed — flat
secrets remain in the `secrets` table until the user explicitly runs `pack group`
or `pack adopt` to move them.

---

## Store Layer Changes

### New methods

```rust
impl Store {
    // Pack CRUD
    fn get_or_create_pack(&self, project: &str, env: &str, name: &str) -> Result<i64>;
    pub fn pack_set(&self, project: &str, env: &str, pack: &str, key: &str, value: &str) -> Result<()>;
    pub fn pack_get(&self, project: &str, env: &str, pack: &str, key: &str) -> Result<Option<String>>;
    pub fn pack_get_all(&self, project: &str, env: &str, pack: &str) -> Result<Vec<(String, String)>>;
    pub fn pack_list(&self, project: &str, env: &str) -> Result<Vec<PackEntry>>;
    pub fn pack_show(&self, project: &str, env: &str, pack: &str) -> Result<Vec<PackSecretEntry>>;
    pub fn pack_clone(&self, src_project: &str, src_env: &str, src_pack: &str,
                      dst_project: &str, dst_env: &str, dst_pack: &str) -> Result<()>;
    pub fn pack_delete(&self, project: &str, env: &str, pack: &str) -> Result<bool>;
    pub fn pack_history(&self, project: &str, env: &str, pack: &str, key: &str, limit: usize) -> Result<Vec<...>>;

    // Move keys between packs (atomic)
    pub fn pack_move(&self, project: &str, env: &str,
                     src_pack: &str, dst_pack: &str, keys: &[String]) -> Result<MoveResult>;

    // Migrate flat secrets into a pack (used by group/adopt)
    pub fn pack_adopt_keys(&self, project: &str, env: &str,
                           pack_name: &str, keys: &[String]) -> Result<usize>;

    // Get flat secrets grouped by prefix (used by pack group wizard)
    pub fn suggest_groups(&self, project: &str, env: &str,
                          min_size: usize) -> Result<GroupSuggestion>;

    // Compose
    pub fn compose(&self, project: &str, env: &str, packs: &[String]) -> Result<ComposeResult>;

    // Compatibility: load all packs + flat secrets (used when no compose is set)
    pub fn compose_all(&self, project: &str, env: &str) -> Result<ComposeResult>;

    // Compatibility: find a key across all packs (used by ts get/set/delete without pack arg)
    pub fn find_key_across_packs(&self, project: &str, env: &str, key: &str) -> Result<KeyLocation>;
}

/// Where a key was found when searching across packs
pub enum KeyLocation {
    InPack { pack_name: String },
    InFlatSecrets,
    InMultiplePacks { pack_names: Vec<String> },
    NotFound,
}

pub struct PackEntry {
    pub name: String,
    pub key_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PackSecretEntry {
    pub key: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ComposeResult {
    pub secrets: Vec<(String, String)>,  // key, value — ready to inject
    pub packs_resolved: Vec<String>,
    pub conflicts: Vec<ComposeConflict>,
}

pub struct ComposeConflict {
    pub key: String,
    pub packs: Vec<String>,  // which packs define this key
}

pub struct MoveResult {
    pub moved: usize,
    pub source_remaining: usize,  // 0 = source pack is now empty
}

pub struct GroupSuggestion {
    pub groups: Vec<SuggestedGroup>,
    pub ungrouped: Vec<String>,  // key names that didn't fit a group
}

pub struct SuggestedGroup {
    pub name: String,       // suggested pack name (lowercased prefix)
    pub keys: Vec<String>,  // key names in this group
}
```

---

## Keypath Resolution

Parsing `gzback.prod.openai.new`:

```
Input: "gzback.prod.openai.new"

1. Split by '.'
2. First segment = project candidate
3. Second segment = environment candidate  
4. Remaining = pack name: "openai.new"

But what if the user types "openai.new" and we have project/env from config?
→ The whole string is the pack name.
```

Resolution strategy:
- If project/env are provided (via config or `-p`/`-e`), the full input is the pack name
- If not, first two segments are project.env, rest is pack name
- Explicit flags always win: `ts pack get -p gzback -e prod openai.new OPENAI_KEY`

```rust
struct Keypath {
    project: String,
    environment: String,
    pack: String,
    key: Option<String>,
}

fn parse_keypath(input: &str, config_project: Option<&str>, config_env: Option<&str>) -> Result<Keypath> {
    // ...
}
```

---

## Implementation Order

### Phase 1: Foundation
1. Schema migration (v2 → v3, add tables)
2. `pack set` / `pack get` / `pack show` / `pack list`
3. `pack delete`
4. Keypath parsing

### Phase 2: Compose
5. Config changes (add `compose` field)
6. `compose show` / `compose check`
7. Update `run` to use compose (with flat-secret fallback)

### Phase 3: Grouping & Migration
8. `pack group` (interactive wizard with prefix-based suggestions)
9. `pack adopt` (manual flat-to-pack migration)
10. `pack move` (reorganize keys between packs, auto-update compose)
11. `pack clone`

### Phase 4: Workflows
12. `pack history`
13. `run --with` / `run --compose` overrides
14. Hierarchical display in `pack list`

### Phase 5: Polish
15. Export/import support for packs
16. `pack group --dry-run` / `--min-size` flags
17. Cross-environment `pack clone`

---

## Open Questions

1. **Pack description**: Should packs have a description field? (Included in schema, optional)
2. **Pack-level export/import**: Export a single pack as a bundle? Or always export the full compose?
3. **Compose inheritance**: Should a compose list be able to include another compose list? (Probably not for MVP)
4. **Empty packs**: Allow creating an empty pack and adding keys later? Or require at least one key?
