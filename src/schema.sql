-- TinySecrets SQLite Schema v3
-- All secret values are encrypted before storage

-- Store metadata (passphrase verification, schema version, etc.)
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Current secrets (one row per project/env/key combination) — legacy flat secrets
CREATE TABLE IF NOT EXISTS secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    environment TEXT NOT NULL,
    key TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(project, environment, key)
);

-- Secret history (audit trail of all changes)
CREATE TABLE IF NOT EXISTS secret_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    environment TEXT NOT NULL,
    key TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);

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

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_secrets_project ON secrets(project);
CREATE INDEX IF NOT EXISTS idx_secrets_project_env ON secrets(project, environment);
CREATE INDEX IF NOT EXISTS idx_history_project_env_key ON secret_history(project, environment, key);
CREATE INDEX IF NOT EXISTS idx_packs_project_env ON packs(project, environment);
CREATE INDEX IF NOT EXISTS idx_pack_secrets_pack_id ON pack_secrets(pack_id);
CREATE INDEX IF NOT EXISTS idx_pack_history_pack_key ON pack_secrets_history(pack_id, key);

