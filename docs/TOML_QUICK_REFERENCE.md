# `.tinysecrets.toml` Quick Reference

One-page guide showing the most common TOML configurations.

---

## 1️⃣ Basic Setup (No Packs)

```toml
project = "myapp"
environment = "dev"
```

**What it does:** Sets defaults so you don't type `-p myapp -e dev` every time.

**Command:** `ts run -- npm start`  
**Loads:** All flat secrets for myapp/dev  
**Output:** `✓ Loaded 5 secrets from myapp/dev`

---

## 2️⃣ With Packs - Explicit Compose

```toml
project = "myapp"
environment = "dev"

compose = ["openai", "stripe", "database"]
```

**What it does:** Loads ONLY the 3 listed packs.

**Command:** `ts run -- npm start`  
**Loads:** Only openai, stripe, database packs  
**Output:** `✓ Composed 8 secrets from 3 packs (myapp/dev)`

---

## 3️⃣ With Packs - No Compose (Load All)

```toml
project = "myapp"
environment = "dev"
```

**What it does:** Loads ALL packs for myapp/dev automatically.

**Command:** `ts run -- npm start`  
**Loads:** All packs for myapp/dev  
**Output:** `✓ Loaded 15 secrets from 6 packs (myapp/dev)`

---

## 4️⃣ Using Pack Variants

```toml
project = "myapp"
environment = "dev"

compose = ["openai.new", "stripe", "database"]
```

**What it does:** Loads the `openai.new` variant instead of `openai`.

**Command:** `ts run -- npm start`  
**Loads:** openai.new (not openai), stripe, database  
**Use case:** Testing new API credentials without affecting main setup

---

## 5️⃣ Minimal Local Development

```toml
project = "myapp"
environment = "local"

compose = ["database", "redis"]
```

**What it does:** Loads only local services, skips external APIs.

**Command:** `ts run -- npm run dev`  
**Loads:** Just database and redis  
**Output:** `✓ Composed 2 secrets from 2 packs (myapp/local)`  
**Use case:** Fast local dev without hitting paid APIs

---

## 6️⃣ Production with Monitoring

```toml
project = "myapp"
environment = "prod"

compose = [
    "openai",
    "stripe",
    "database",
    "redis",
    "monitoring",    # Only in prod
    "logging",       # Only in prod
]
```

**What it does:** Loads extra monitoring/logging packs in production.

**Command:** `ts run -- ./deploy.sh`  
**Loads:** All 6 packs from prod environment  
**Output:** `✓ Composed 18 secrets from 6 packs (myapp/prod)`

---

## 7️⃣ Feature Branch with Different Packs

**Main branch `.tinysecrets.toml`:**
```toml
project = "myapp"
environment = "dev"
compose = ["openai", "stripe", "database"]
```

**Feature branch `.tinysecrets.toml`:**
```toml
project = "myapp"
environment = "dev"
compose = ["anthropic", "stripe", "database"]  # Different AI provider!
```

**What it does:**
- Same command on different branches loads different packs
- Perfect for testing provider migrations

---

## Command Behavior Summary

| TOML Has | Command | What Loads |
|----------|---------|------------|
| No compose | `ts run` | All packs + flat secrets |
| `compose = [...]` | `ts run` | Only listed packs |
| `compose = [...]` | `ts run --with extra` | Listed packs + extra |
| `compose = [...]` | `ts run --compose "a,b"` | Only a and b (ignores TOML) |
| No packs yet | `ts run` | All flat secrets (legacy) |

---

## Common Questions

**Q: Do I need a compose field?**  
A: No! If you omit it, all packs are loaded. Add it when you want selective loading.

**Q: What happens if a pack doesn't exist?**  
A: `ts run` will error immediately and tell you which pack is missing.

**Q: Can I have the same key in multiple packs?**  
A: Yes, but `ts run` will error and refuse to run until you fix the conflict.

**Q: Do pack names affect environment variable names?**  
A: No! Pack names are just for organization. If a pack has `OPENAI_KEY`, that's the env var name.

**Q: Can I override the TOML at runtime?**  
A: Yes! Use `--with` to add packs or `--compose` to replace entirely.

**Q: What if I don't have a TOML file?**  
A: Commands still work - you just need to type `-p project -e env` every time.

---

## Next Steps

1. **Copy** `.tinysecrets.toml.example` to `.tinysecrets.toml`
2. **Edit** project and environment to match your setup
3. **Run** `ts pack group` if you have existing secrets to organize
4. **Add** compose field to select which packs to load
5. **Test** with `ts compose show` before running

See [TOML_CONFIG_GUIDE.md](./TOML_CONFIG_GUIDE.md) for detailed explanations.
