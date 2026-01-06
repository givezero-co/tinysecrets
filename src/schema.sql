-- TinySecrets SQLite Schema v1
-- All secret values are encrypted with age before storage

-- Store metadata (passphrase verification, schema version, etc.)
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Current secrets (one row per project/env/key combination)
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

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_secrets_project ON secrets(project);
CREATE INDEX IF NOT EXISTS idx_secrets_project_env ON secrets(project, environment);
CREATE INDEX IF NOT EXISTS idx_history_project_env_key ON secret_history(project, environment, key);

