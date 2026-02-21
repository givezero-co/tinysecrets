//! SQLite-based encrypted secrets store
//!
//! Schema design:
//! - secrets: current values (project, env, key, encrypted_value, metadata) — legacy flat
//! - secret_history: all previous versions for audit trail
//! - packs: named groups of secrets scoped to project/environment
//! - pack_secrets: secrets within packs
//! - pack_secrets_history: audit trail for pack secrets
//! - metadata: store-level config (passphrase verification, version)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::crypto::{self, MasterKey};

const SCHEMA_VERSION: i32 = 3;

/// Secret entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    pub project: String,
    pub environment: String,
    pub key: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i32,
}

/// Historical secret entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretHistoryEntry {
    pub project: String,
    pub environment: String,
    pub key: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// The encrypted secrets store
pub struct Store {
    conn: Connection,
    passphrase: SecretString,
    master_key: MasterKey,
}

impl Store {
    /// Get the default store path (~/.tinysecrets/store.db)
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".tinysecrets").join("store.db"))
    }

    /// Initialize a new store with the given passphrase
    pub fn init(passphrase: SecretString) -> Result<Self> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let path = Self::default_path()?;

        if path.exists() {
            anyhow::bail!(
                "Store already exists at {}. Use `ts` commands to interact with it.",
                path.display()
            );
        }

        // Create directory
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create ~/.tinysecrets directory")?;
        }

        let conn = Connection::open(&path).context("Failed to create SQLite database")?;

        // Create schema
        conn.execute_batch(include_str!("schema.sql"))
            .context("Failed to initialize database schema")?;

        // Generate salt and derive master key
        let salt = MasterKey::generate_salt();
        let master_key = MasterKey::derive(&passphrase, &salt)?;

        // Store passphrase verification (still uses age - only runs once)
        let verification = crypto::derive_verification(&passphrase)?;
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('passphrase_verification', ?1)",
            params![verification],
        )?;
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;
        // Store salt for key derivation
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('encryption_salt', ?1)",
            params![BASE64.encode(salt)],
        )?;

        Ok(Self {
            conn,
            passphrase,
            master_key,
        })
    }

    /// Open an existing store
    pub fn open(passphrase: SecretString) -> Result<Self> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

        let path = Self::default_path()?;

        if !path.exists() {
            anyhow::bail!("No store found. Run `ts init` first to create one.");
        }

        let conn = Connection::open(&path).context("Failed to open SQLite database")?;

        // Verify passphrase (this is the slow operation - runs once)
        let verification: String = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'passphrase_verification'",
                [],
                |row| row.get(0),
            )
            .context("Store appears corrupted - no passphrase verification found")?;

        if !crypto::verify_passphrase(&passphrase, &verification) {
            anyhow::bail!("Invalid passphrase");
        }

        // Get or create salt for key derivation
        let salt: [u8; 32] = match conn.query_row(
            "SELECT value FROM metadata WHERE key = 'encryption_salt'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(salt_b64) => {
                let salt_vec = BASE64
                    .decode(&salt_b64)
                    .context("Failed to decode encryption salt")?;
                salt_vec
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid salt length"))?
            }
            Err(_) => {
                // Legacy store without salt - create one and migrate
                let salt = MasterKey::generate_salt();
                conn.execute(
                    "INSERT INTO metadata (key, value) VALUES ('encryption_salt', ?1)",
                    params![BASE64.encode(salt)],
                )?;
                salt
            }
        };

        // Derive master key (fast - ~100ms)
        let master_key = MasterKey::derive(&passphrase, &salt)?;

        // Run schema migrations if needed
        Self::migrate_if_needed(&conn)?;

        Ok(Self {
            conn,
            passphrase,
            master_key,
        })
    }

    fn migrate_if_needed(conn: &Connection) -> Result<()> {
        let version: i32 = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| {
                    let v: String = row.get(0)?;
                    Ok(v.parse::<i32>().unwrap_or(1))
                },
            )
            .unwrap_or(1);

        if version < 3 {
            Self::migrate_to_v3(conn)?;
        }

        Ok(())
    }

    fn migrate_to_v3(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS packs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project TEXT NOT NULL,
                environment TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(project, environment, name)
            );

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

            CREATE TABLE IF NOT EXISTS pack_secrets_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pack_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                encrypted_value TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                deleted_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_packs_project_env ON packs(project, environment);
            CREATE INDEX IF NOT EXISTS idx_pack_secrets_pack_id ON pack_secrets(pack_id);
            CREATE INDEX IF NOT EXISTS idx_pack_history_pack_key ON pack_secrets_history(pack_id, key);

            INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '3');",
        )
        .context("Failed to migrate schema to v3")?;

        Ok(())
    }

    /// Check if a store exists
    pub fn exists() -> Result<bool> {
        Ok(Self::default_path()?.exists())
    }

    /// Get a reference to the underlying connection (for migrations)
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Set a secret value
    pub fn set(
        &self,
        project: &str,
        environment: &str,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let encrypted_value = crypto::encrypt(value, &self.master_key)?;
        let now = Utc::now();

        // Check if secret exists
        let existing: Option<(i32, String)> = self
            .conn
            .query_row(
                "SELECT version, encrypted_value FROM secrets 
                 WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, environment, key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let tx = self.conn.unchecked_transaction()?;

        if let Some((version, _old_encrypted)) = existing {
            // Archive old version
            tx.execute(
                "INSERT INTO secret_history (project, environment, key, encrypted_value, version, created_at)
                 SELECT project, environment, key, encrypted_value, version, updated_at
                 FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, environment, key],
            )?;

            // Update existing
            tx.execute(
                "UPDATE secrets SET encrypted_value = ?1, description = ?2, updated_at = ?3, version = ?4
                 WHERE project = ?5 AND environment = ?6 AND key = ?7",
                params![
                    encrypted_value,
                    description,
                    now.to_rfc3339(),
                    version + 1,
                    project,
                    environment,
                    key
                ],
            )?;
        } else {
            // Insert new
            tx.execute(
                "INSERT INTO secrets (project, environment, key, encrypted_value, description, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1)",
                params![
                    project,
                    environment,
                    key,
                    encrypted_value,
                    description,
                    now.to_rfc3339()
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get a secret value
    pub fn get(&self, project: &str, environment: &str, key: &str) -> Result<Option<String>> {
        let encrypted: Option<String> = self
            .conn
            .query_row(
                "SELECT encrypted_value FROM secrets 
                 WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, environment, key],
                |row| row.get(0),
            )
            .ok();

        match encrypted {
            Some(enc) => {
                let decrypted = crypto::decrypt(&enc, &self.master_key, &self.passphrase)?;
                Ok(Some(decrypted.expose_secret().clone()))
            }
            None => Ok(None),
        }
    }

    /// List secrets (optionally filtered)
    pub fn list(
        &self,
        project: Option<&str>,
        environment: Option<&str>,
    ) -> Result<Vec<SecretEntry>> {
        let mut sql = String::from(
            "SELECT project, environment, key, description, created_at, updated_at, version FROM secrets WHERE 1=1"
        );
        let mut params_vec: Vec<String> = vec![];

        if let Some(p) = project {
            sql.push_str(" AND project = ?");
            params_vec.push(p.to_string());
        }
        if let Some(e) = environment {
            sql.push_str(" AND environment = ?");
            params_vec.push(e.to_string());
        }
        sql.push_str(" ORDER BY project, environment, key");

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let entries = stmt
            .query_map(params.as_slice(), |row| {
                let created_str: String = row.get(4)?;
                let updated_str: String = row.get(5)?;
                Ok(SecretEntry {
                    project: row.get(0)?,
                    environment: row.get(1)?,
                    key: row.get(2)?,
                    description: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    version: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Delete a secret
    pub fn delete(&self, project: &str, environment: &str, key: &str) -> Result<bool> {
        // First archive to history
        self.conn.execute(
            "INSERT INTO secret_history (project, environment, key, encrypted_value, version, created_at, deleted_at)
             SELECT project, environment, key, encrypted_value, version, updated_at, ?4
             FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
            params![project, environment, key, Utc::now().to_rfc3339()],
        )?;

        let deleted = self.conn.execute(
            "DELETE FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
            params![project, environment, key],
        )?;

        Ok(deleted > 0)
    }

    /// Get all secrets for a project/environment (for `ts run`)
    pub fn get_all(&self, project: &str, environment: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, encrypted_value FROM secrets 
             WHERE project = ?1 AND environment = ?2",
        )?;

        let secrets = stmt
            .query_map(params![project, environment], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut decrypted = Vec::new();
        for (key, encrypted) in secrets {
            let value = crypto::decrypt(&encrypted, &self.master_key, &self.passphrase)?;
            decrypted.push((key, value.expose_secret().clone()));
        }

        Ok(decrypted)
    }

    /// Get secret history
    pub fn history(
        &self,
        project: &str,
        environment: &str,
        key: &str,
        limit: usize,
    ) -> Result<Vec<SecretHistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT project, environment, key, version, created_at, deleted_at 
             FROM secret_history 
             WHERE project = ?1 AND environment = ?2 AND key = ?3
             ORDER BY version DESC
             LIMIT ?4",
        )?;

        let entries = stmt
            .query_map(params![project, environment, key, limit as i64], |row| {
                let created_str: String = row.get(4)?;
                let deleted_str: Option<String> = row.get(5)?;
                Ok(SecretHistoryEntry {
                    project: row.get(0)?,
                    environment: row.get(1)?,
                    key: row.get(2)?,
                    version: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    deleted_at: deleted_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Get a specific version of a secret from history
    pub fn get_version(
        &self,
        project: &str,
        environment: &str,
        key: &str,
        version: i32,
    ) -> Result<Option<String>> {
        // First check if requesting current version
        let current: Option<(i32, String)> = self
            .conn
            .query_row(
                "SELECT version, encrypted_value FROM secrets 
                 WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, environment, key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((current_version, encrypted)) = current {
            if current_version == version {
                let decrypted = crypto::decrypt(&encrypted, &self.master_key, &self.passphrase)?;
                return Ok(Some(decrypted.expose_secret().clone()));
            }
        }

        // Check history
        let encrypted: Option<String> = self
            .conn
            .query_row(
                "SELECT encrypted_value FROM secret_history 
                 WHERE project = ?1 AND environment = ?2 AND key = ?3 AND version = ?4",
                params![project, environment, key, version],
                |row| row.get(0),
            )
            .ok();

        match encrypted {
            Some(enc) => {
                let decrypted = crypto::decrypt(&enc, &self.master_key, &self.passphrase)?;
                Ok(Some(decrypted.expose_secret().clone()))
            }
            None => Ok(None),
        }
    }

    /// List all projects
    pub fn list_projects(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT project FROM secrets ORDER BY project")?;

        let projects = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    /// List all environments for a project
    pub fn list_environments(&self, project: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT environment FROM secrets WHERE project = ?1 ORDER BY environment",
        )?;

        let envs = stmt
            .query_map(params![project], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(envs)
    }

    /// Export secrets for a project/environment
    pub fn export(&self, project: &str, environment: &str) -> Result<ExportBundle> {
        let entries = self.list(Some(project), Some(environment))?;
        let mut secrets = Vec::new();

        for entry in entries {
            let encrypted: String = self.conn.query_row(
                "SELECT encrypted_value FROM secrets 
                 WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![entry.project, entry.environment, entry.key],
                |row| row.get(0),
            )?;

            secrets.push(ExportedSecret {
                key: entry.key,
                encrypted_value: encrypted,
                description: entry.description,
                version: entry.version,
            });
        }

        // Get passphrase verification for bundle
        let verification: String = self.conn.query_row(
            "SELECT value FROM metadata WHERE key = 'passphrase_verification'",
            [],
            |row| row.get(0),
        )?;

        Ok(ExportBundle {
            version: 1,
            project: project.to_string(),
            environment: environment.to_string(),
            passphrase_verification: verification,
            exported_at: Utc::now(),
            secrets,
        })
    }

    /// Import secrets from a bundle
    pub fn import(&self, bundle: &ExportBundle) -> Result<usize> {
        // Verify bundle passphrase matches our passphrase
        if !crypto::verify_passphrase(&self.passphrase, &bundle.passphrase_verification) {
            anyhow::bail!(
                "Bundle was encrypted with a different passphrase. \
                 You need the original passphrase to import these secrets."
            );
        }

        let mut imported = 0;
        for secret in &bundle.secrets {
            // Decrypt and re-encrypt to verify integrity
            let decrypted =
                crypto::decrypt(&secret.encrypted_value, &self.master_key, &self.passphrase)?;
            self.set(
                &bundle.project,
                &bundle.environment,
                &secret.key,
                decrypted.expose_secret(),
                secret.description.as_deref(),
            )?;
            imported += 1;
        }

        Ok(imported)
    }
}

// ---------------------------------------------------------------------------
// Pack types
// ---------------------------------------------------------------------------

/// Pack entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub description: Option<String>,
    pub key_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Secret entry within a pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSecretEntry {
    pub key: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Result of composing packs together
pub struct ComposeResult {
    pub secrets: Vec<(String, String)>,
    pub packs_resolved: Vec<String>,
    pub conflicts: Vec<ComposeConflict>,
}

/// A key conflict between packs
pub struct ComposeConflict {
    pub key: String,
    pub packs: Vec<String>,
}

/// Result of moving keys between packs
pub struct MoveResult {
    pub moved: usize,
    pub source_remaining: usize,
}

/// Where a key was found when searching across packs
pub enum KeyLocation {
    InPack { pack_name: String },
    InFlatSecrets,
    InMultiplePacks { pack_names: Vec<String> },
    NotFound,
}

/// Suggested grouping from prefix analysis
pub struct GroupSuggestion {
    pub groups: Vec<SuggestedGroup>,
    pub ungrouped: Vec<String>,
}

pub struct SuggestedGroup {
    pub name: String,
    pub keys: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pack methods
// ---------------------------------------------------------------------------

impl Store {
    fn get_or_create_pack(&self, project: &str, env: &str, name: &str) -> Result<i64> {
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM packs WHERE project = ?1 AND environment = ?2 AND name = ?3",
                params![project, env, name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            return Ok(id);
        }

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO packs (project, environment, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![project, env, name, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    fn get_pack_id(&self, project: &str, env: &str, name: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM packs WHERE project = ?1 AND environment = ?2 AND name = ?3",
                params![project, env, name],
                |row| row.get(0),
            )
            .ok())
    }

    /// Set a key in a pack (creates the pack if needed)
    pub fn pack_set(
        &self,
        project: &str,
        env: &str,
        pack: &str,
        key: &str,
        value: &str,
    ) -> Result<i32> {
        let pack_id = self.get_or_create_pack(project, env, pack)?;
        let encrypted_value = crypto::encrypt(value, &self.master_key)?;
        let now = Utc::now().to_rfc3339();

        let existing: Option<(i32, String)> = self
            .conn
            .query_row(
                "SELECT version, encrypted_value FROM pack_secrets
                 WHERE pack_id = ?1 AND key = ?2",
                params![pack_id, key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let tx = self.conn.unchecked_transaction()?;

        let new_version = if let Some((version, _)) = existing {
            // Archive old version
            tx.execute(
                "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at)
                 SELECT pack_id, key, encrypted_value, version, updated_at
                 FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
                params![pack_id, key],
            )?;

            let new_ver = version + 1;
            tx.execute(
                "UPDATE pack_secrets SET encrypted_value = ?1, updated_at = ?2, version = ?3
                 WHERE pack_id = ?4 AND key = ?5",
                params![encrypted_value, now, new_ver, pack_id, key],
            )?;
            new_ver
        } else {
            tx.execute(
                "INSERT INTO pack_secrets (pack_id, key, encrypted_value, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?4, 1)",
                params![pack_id, key, encrypted_value, now],
            )?;
            1
        };

        // Touch pack updated_at
        tx.execute(
            "UPDATE packs SET updated_at = ?1 WHERE id = ?2",
            params![now, pack_id],
        )?;

        tx.commit()?;
        Ok(new_version)
    }

    /// Get a single key from a pack
    pub fn pack_get(
        &self,
        project: &str,
        env: &str,
        pack: &str,
        key: &str,
    ) -> Result<Option<String>> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(None),
        };

        let encrypted: Option<String> = self
            .conn
            .query_row(
                "SELECT encrypted_value FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
                params![pack_id, key],
                |row| row.get(0),
            )
            .ok();

        match encrypted {
            Some(enc) => {
                let decrypted = crypto::decrypt(&enc, &self.master_key, &self.passphrase)?;
                Ok(Some(decrypted.expose_secret().clone()))
            }
            None => Ok(None),
        }
    }

    /// Get all key-value pairs from a pack (decrypted)
    pub fn pack_get_all(
        &self,
        project: &str,
        env: &str,
        pack: &str,
    ) -> Result<Vec<(String, String)>> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let mut stmt = self
            .conn
            .prepare("SELECT key, encrypted_value FROM pack_secrets WHERE pack_id = ?1")?;

        let rows = stmt
            .query_map(params![pack_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut decrypted = Vec::new();
        for (key, enc) in rows {
            let val = crypto::decrypt(&enc, &self.master_key, &self.passphrase)?;
            decrypted.push((key, val.expose_secret().clone()));
        }

        Ok(decrypted)
    }

    /// List all packs for a project/environment
    pub fn pack_list(&self, project: &str, env: &str) -> Result<Vec<PackEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.name, p.description, p.created_at, p.updated_at,
                    (SELECT COUNT(*) FROM pack_secrets ps WHERE ps.pack_id = p.id) as key_count
             FROM packs p
             WHERE p.project = ?1 AND p.environment = ?2
             ORDER BY p.name",
        )?;

        let entries = stmt
            .query_map(params![project, env], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(PackEntry {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    key_count: row.get::<_, i64>(4)? as usize,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Show keys in a specific pack
    pub fn pack_show(&self, project: &str, env: &str, pack: &str) -> Result<Vec<PackSecretEntry>> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let mut stmt = self.conn.prepare(
            "SELECT key, version, created_at, updated_at
             FROM pack_secrets WHERE pack_id = ?1 ORDER BY key",
        )?;

        let entries = stmt
            .query_map(params![pack_id], |row| {
                let created_str: String = row.get(2)?;
                let updated_str: String = row.get(3)?;
                Ok(PackSecretEntry {
                    key: row.get(0)?,
                    version: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Delete a pack and all its secrets (archives to history)
    pub fn pack_delete(&self, project: &str, env: &str, pack: &str) -> Result<bool> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(false),
        };

        let now = Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;

        // Archive all secrets to history
        tx.execute(
            "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at, deleted_at)
             SELECT pack_id, key, encrypted_value, version, updated_at, ?2
             FROM pack_secrets WHERE pack_id = ?1",
            params![pack_id, now],
        )?;

        // CASCADE deletes pack_secrets
        tx.execute("DELETE FROM packs WHERE id = ?1", params![pack_id])?;

        tx.commit()?;
        Ok(true)
    }

    /// Clone a pack (all its secrets) to a new pack
    #[allow(clippy::too_many_arguments)]
    pub fn pack_clone(
        &self,
        src_project: &str,
        src_env: &str,
        src_pack: &str,
        dst_project: &str,
        dst_env: &str,
        dst_pack: &str,
        force: bool,
    ) -> Result<usize> {
        let src_id = self
            .get_pack_id(src_project, src_env, src_pack)?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Source pack '{}' not found in {}/{}",
                    src_pack,
                    src_project,
                    src_env
                )
            })?;

        // Check if target exists
        if let Some(dst_id) = self.get_pack_id(dst_project, dst_env, dst_pack)? {
            if !force {
                anyhow::bail!(
                    "Pack '{}' already exists in {}/{}. Use --force to overwrite.",
                    dst_pack,
                    dst_project,
                    dst_env
                );
            }
            // Delete existing target
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at, deleted_at)
                 SELECT pack_id, key, encrypted_value, version, updated_at, ?2
                 FROM pack_secrets WHERE pack_id = ?1",
                params![dst_id, now],
            )?;
            self.conn
                .execute("DELETE FROM packs WHERE id = ?1", params![dst_id])?;
        }

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO packs (project, environment, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![dst_project, dst_env, dst_pack, now],
        )?;
        let dst_id = self.conn.last_insert_rowid();

        // Copy all secrets (keep encrypted values as-is, same encryption key)
        let copied = self.conn.execute(
            "INSERT INTO pack_secrets (pack_id, key, encrypted_value, created_at, updated_at, version)
             SELECT ?2, key, encrypted_value, ?3, ?3, 1
             FROM pack_secrets WHERE pack_id = ?1",
            params![src_id, dst_id, now],
        )?;

        Ok(copied)
    }

    /// Move keys from one pack to another (atomic)
    pub fn pack_move(
        &self,
        project: &str,
        env: &str,
        src_pack: &str,
        dst_pack: &str,
        keys: &[String],
    ) -> Result<MoveResult> {
        let src_id = self
            .get_pack_id(project, env, src_pack)?
            .ok_or_else(|| anyhow::anyhow!("Source pack '{}' not found", src_pack))?;

        let dst_id = self.get_or_create_pack(project, env, dst_pack)?;
        let now = Utc::now().to_rfc3339();

        let tx = self.conn.unchecked_transaction()?;

        let mut moved = 0usize;
        for key in keys {
            // Get the secret from source
            let row: Option<(String, i32)> = tx
                .query_row(
                    "SELECT encrypted_value, version FROM pack_secrets
                     WHERE pack_id = ?1 AND key = ?2",
                    params![src_id, key],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            let (encrypted_value, version) = match row {
                Some(r) => r,
                None => continue,
            };

            // Archive source to history
            tx.execute(
                "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![src_id, key, &encrypted_value, version, now],
            )?;

            // Delete from source
            tx.execute(
                "DELETE FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
                params![src_id, key],
            )?;

            // Insert into destination (or update if exists)
            let existing_dst: Option<i32> = tx
                .query_row(
                    "SELECT version FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
                    params![dst_id, key],
                    |row| row.get(0),
                )
                .ok();

            if let Some(dst_ver) = existing_dst {
                tx.execute(
                    "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at)
                     SELECT pack_id, key, encrypted_value, version, updated_at
                     FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
                    params![dst_id, key],
                )?;
                tx.execute(
                    "UPDATE pack_secrets SET encrypted_value = ?1, updated_at = ?2, version = ?3
                     WHERE pack_id = ?4 AND key = ?5",
                    params![encrypted_value, now, dst_ver + 1, dst_id, key],
                )?;
            } else {
                tx.execute(
                    "INSERT INTO pack_secrets (pack_id, key, encrypted_value, created_at, updated_at, version)
                     VALUES (?1, ?2, ?3, ?4, ?4, 1)",
                    params![dst_id, key, encrypted_value, now],
                )?;
            }

            moved += 1;
        }

        // Touch both packs
        tx.execute(
            "UPDATE packs SET updated_at = ?1 WHERE id = ?2",
            params![now, src_id],
        )?;
        tx.execute(
            "UPDATE packs SET updated_at = ?1 WHERE id = ?2",
            params![now, dst_id],
        )?;

        let source_remaining: i64 = tx.query_row(
            "SELECT COUNT(*) FROM pack_secrets WHERE pack_id = ?1",
            params![src_id],
            |row| row.get(0),
        )?;

        tx.commit()?;

        Ok(MoveResult {
            moved,
            source_remaining: source_remaining as usize,
        })
    }

    /// Get history for a key in a pack
    pub fn pack_history(
        &self,
        project: &str,
        env: &str,
        pack: &str,
        key: &str,
        limit: usize,
    ) -> Result<Vec<SecretHistoryEntry>> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let mut stmt = self.conn.prepare(
            "SELECT key, version, created_at, deleted_at
             FROM pack_secrets_history
             WHERE pack_id = ?1 AND key = ?2
             ORDER BY version DESC
             LIMIT ?3",
        )?;

        let entries = stmt
            .query_map(params![pack_id, key, limit as i64], |row| {
                let created_str: String = row.get(2)?;
                let deleted_str: Option<String> = row.get(3)?;
                Ok(SecretHistoryEntry {
                    project: project.to_string(),
                    environment: env.to_string(),
                    key: row.get(0)?,
                    version: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    deleted_at: deleted_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Move flat secrets into a pack
    pub fn pack_adopt_keys(
        &self,
        project: &str,
        env: &str,
        pack_name: &str,
        keys: &[String],
    ) -> Result<usize> {
        let pack_id = self.get_or_create_pack(project, env, pack_name)?;
        let now = Utc::now().to_rfc3339();

        let tx = self.conn.unchecked_transaction()?;
        let mut adopted = 0usize;

        for key in keys {
            let row: Option<(String, i32)> = tx
                .query_row(
                    "SELECT encrypted_value, version FROM secrets
                     WHERE project = ?1 AND environment = ?2 AND key = ?3",
                    params![project, env, key],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            let (encrypted_value, _version) = match row {
                Some(r) => r,
                None => continue,
            };

            // Insert into pack
            tx.execute(
                "INSERT INTO pack_secrets (pack_id, key, encrypted_value, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?4, 1)",
                params![pack_id, key, encrypted_value, now],
            )?;

            // Archive and delete from flat secrets
            tx.execute(
                "INSERT INTO secret_history (project, environment, key, encrypted_value, version, created_at, deleted_at)
                 SELECT project, environment, key, encrypted_value, version, updated_at, ?4
                 FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, env, key, now],
            )?;

            tx.execute(
                "DELETE FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, env, key],
            )?;

            adopted += 1;
        }

        tx.execute(
            "UPDATE packs SET updated_at = ?1 WHERE id = ?2",
            params![now, pack_id],
        )?;

        tx.commit()?;
        Ok(adopted)
    }

    /// Suggest groups based on key name prefixes
    pub fn suggest_groups(
        &self,
        project: &str,
        env: &str,
        min_size: usize,
    ) -> Result<GroupSuggestion> {
        let entries = self.list(Some(project), Some(env))?;

        let mut prefix_map: HashMap<String, Vec<String>> = HashMap::new();
        for entry in &entries {
            let prefix = entry
                .key
                .split('_')
                .next()
                .unwrap_or(&entry.key)
                .to_lowercase();
            prefix_map
                .entry(prefix)
                .or_default()
                .push(entry.key.clone());
        }

        let mut groups = Vec::new();
        let mut ungrouped = Vec::new();

        for (prefix, keys) in prefix_map {
            if keys.len() >= min_size {
                groups.push(SuggestedGroup { name: prefix, keys });
            } else {
                ungrouped.extend(keys);
            }
        }

        groups.sort_by(|a, b| a.name.cmp(&b.name));
        ungrouped.sort();

        Ok(GroupSuggestion { groups, ungrouped })
    }

    /// Compose an environment from a list of pack names
    pub fn compose(&self, project: &str, env: &str, packs: &[String]) -> Result<ComposeResult> {
        let mut all_secrets: Vec<(String, String)> = Vec::new();
        let mut key_sources: HashMap<String, Vec<String>> = HashMap::new();
        let mut packs_resolved = Vec::new();

        for pack_name in packs {
            let pack_id = self.get_pack_id(project, env, pack_name)?.ok_or_else(|| {
                anyhow::anyhow!("Pack '{}' not found in {}/{}", pack_name, project, env)
            })?;

            let mut stmt = self
                .conn
                .prepare("SELECT key, encrypted_value FROM pack_secrets WHERE pack_id = ?1")?;

            let rows = stmt
                .query_map(params![pack_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            for (key, enc) in rows {
                key_sources
                    .entry(key.clone())
                    .or_default()
                    .push(pack_name.clone());

                let val = crypto::decrypt(&enc, &self.master_key, &self.passphrase)?;
                all_secrets.push((key, val.expose_secret().clone()));
            }

            packs_resolved.push(pack_name.clone());
        }

        let conflicts: Vec<ComposeConflict> = key_sources
            .iter()
            .filter(|(_, sources)| sources.len() > 1)
            .map(|(key, sources)| ComposeConflict {
                key: key.clone(),
                packs: sources.clone(),
            })
            .collect();

        Ok(ComposeResult {
            secrets: all_secrets,
            packs_resolved,
            conflicts,
        })
    }

    /// Load all packs + flat secrets for a project/env (no compose list needed)
    pub fn compose_all(&self, project: &str, env: &str) -> Result<ComposeResult> {
        let packs = self.pack_list(project, env)?;
        let pack_names: Vec<String> = packs.iter().map(|p| p.name.clone()).collect();

        let mut result = if pack_names.is_empty() {
            ComposeResult {
                secrets: Vec::new(),
                packs_resolved: Vec::new(),
                conflicts: Vec::new(),
            }
        } else {
            self.compose(project, env, &pack_names)?
        };

        // Also include any remaining flat secrets
        let flat = self.get_all(project, env)?;
        if !flat.is_empty() {
            let mut key_sources: HashMap<String, Vec<String>> = HashMap::new();
            for (key, _) in &result.secrets {
                key_sources
                    .entry(key.clone())
                    .or_default()
                    .push("(pack)".to_string());
            }

            for (key, value) in flat {
                key_sources
                    .entry(key.clone())
                    .or_default()
                    .push("(flat)".to_string());
                result.secrets.push((key, value));
            }

            // Detect any new conflicts between flat and pack secrets
            for (key, sources) in &key_sources {
                if sources.len() > 1 && !result.conflicts.iter().any(|c| c.key == *key) {
                    result.conflicts.push(ComposeConflict {
                        key: key.clone(),
                        packs: sources.clone(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// Find a key across all packs and flat secrets
    pub fn find_key_across_packs(
        &self,
        project: &str,
        env: &str,
        key: &str,
    ) -> Result<KeyLocation> {
        // Search packs
        let mut stmt = self.conn.prepare(
            "SELECT p.name FROM packs p
             JOIN pack_secrets ps ON ps.pack_id = p.id
             WHERE p.project = ?1 AND p.environment = ?2 AND ps.key = ?3",
        )?;

        let pack_names: Vec<String> = stmt
            .query_map(params![project, env, key], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        if pack_names.len() > 1 {
            return Ok(KeyLocation::InMultiplePacks { pack_names });
        }

        if pack_names.len() == 1 {
            return Ok(KeyLocation::InPack {
                pack_name: pack_names.into_iter().next().unwrap(),
            });
        }

        // Check flat secrets
        let flat_exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM secrets WHERE project = ?1 AND environment = ?2 AND key = ?3",
                params![project, env, key],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if flat_exists {
            return Ok(KeyLocation::InFlatSecrets);
        }

        Ok(KeyLocation::NotFound)
    }

    /// Count remaining flat secrets for a project/environment
    pub fn count_flat_secrets(&self, project: &str, env: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM secrets WHERE project = ?1 AND environment = ?2",
            params![project, env],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Check if any packs exist for a project/environment
    pub fn has_packs(&self, project: &str, env: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM packs WHERE project = ?1 AND environment = ?2",
            params![project, env],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Delete a single key from a pack
    pub fn pack_delete_key(&self, project: &str, env: &str, pack: &str, key: &str) -> Result<bool> {
        let pack_id = match self.get_pack_id(project, env, pack)? {
            Some(id) => id,
            None => return Ok(false),
        };

        let now = Utc::now().to_rfc3339();

        // Archive to history
        self.conn.execute(
            "INSERT INTO pack_secrets_history (pack_id, key, encrypted_value, version, created_at, deleted_at)
             SELECT pack_id, key, encrypted_value, version, updated_at, ?3
             FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
            params![pack_id, key, now],
        )?;

        let deleted = self.conn.execute(
            "DELETE FROM pack_secrets WHERE pack_id = ?1 AND key = ?2",
            params![pack_id, key],
        )?;

        Ok(deleted > 0)
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok())
    }

    /// Set metadata value
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

/// Export bundle format
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: i32,
    pub project: String,
    pub environment: String,
    pub passphrase_verification: String,
    pub exported_at: DateTime<Utc>,
    pub secrets: Vec<ExportedSecret>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedSecret {
    pub key: String,
    pub encrypted_value: String,
    pub description: Option<String>,
    pub version: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use tempfile::NamedTempFile;

    fn test_store() -> Store {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Keep the file handle alive by leaking it (test only)
        std::mem::forget(tmp);

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("schema.sql")).unwrap();

        let passphrase = SecretString::new("testpassphrase12".to_string());
        let salt = MasterKey::generate_salt();
        let master_key = MasterKey::derive(&passphrase, &salt).unwrap();

        let verification = crypto::derive_verification(&passphrase).unwrap();
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('passphrase_verification', ?1)",
            params![verification],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('schema_version', '3')",
            [],
        )
        .unwrap();

        Store {
            conn,
            passphrase,
            master_key,
        }
    }

    #[test]
    fn test_pack_set_and_get() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "API_KEY", "sk-test")
            .unwrap();

        let val = store.pack_get("proj", "dev", "openai", "API_KEY").unwrap();
        assert_eq!(val, Some("sk-test".to_string()));
    }

    #[test]
    fn test_pack_set_updates_version() {
        let store = test_store();
        let v1 = store
            .pack_set("proj", "dev", "openai", "KEY", "v1")
            .unwrap();
        assert_eq!(v1, 1);

        let v2 = store
            .pack_set("proj", "dev", "openai", "KEY", "v2")
            .unwrap();
        assert_eq!(v2, 2);

        let val = store.pack_get("proj", "dev", "openai", "KEY").unwrap();
        assert_eq!(val, Some("v2".to_string()));
    }

    #[test]
    fn test_pack_get_all() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "KEY", "sk-1")
            .unwrap();
        store
            .pack_set(
                "proj",
                "dev",
                "openai",
                "ENDPOINT",
                "https://api.openai.com",
            )
            .unwrap();

        let all = store.pack_get_all("proj", "dev", "openai").unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_pack_list() {
        let store = test_store();
        store.pack_set("proj", "dev", "openai", "KEY", "v").unwrap();
        store.pack_set("proj", "dev", "stripe", "KEY", "v").unwrap();

        let packs = store.pack_list("proj", "dev").unwrap();
        assert_eq!(packs.len(), 2);
        assert_eq!(packs[0].name, "openai");
        assert_eq!(packs[1].name, "stripe");
    }

    #[test]
    fn test_pack_delete() {
        let store = test_store();
        store.pack_set("proj", "dev", "openai", "KEY", "v").unwrap();

        assert!(store.pack_delete("proj", "dev", "openai").unwrap());
        assert!(store
            .pack_get("proj", "dev", "openai", "KEY")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_pack_clone() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "KEY", "sk-1")
            .unwrap();
        store
            .pack_set("proj", "dev", "openai", "ENDPOINT", "https://old")
            .unwrap();

        let count = store
            .pack_clone("proj", "dev", "openai", "proj", "dev", "openai.old", false)
            .unwrap();
        assert_eq!(count, 2);

        let val = store.pack_get("proj", "dev", "openai.old", "KEY").unwrap();
        assert_eq!(val, Some("sk-1".to_string()));
    }

    #[test]
    fn test_pack_move() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "other", "DB_URL", "postgres://")
            .unwrap();
        store
            .pack_set("proj", "dev", "other", "REDIS_URL", "redis://")
            .unwrap();
        store
            .pack_set("proj", "dev", "other", "JWT", "secret")
            .unwrap();

        let result = store
            .pack_move(
                "proj",
                "dev",
                "other",
                "infra",
                &["DB_URL".to_string(), "REDIS_URL".to_string()],
            )
            .unwrap();

        assert_eq!(result.moved, 2);
        assert_eq!(result.source_remaining, 1);

        assert!(store
            .pack_get("proj", "dev", "infra", "DB_URL")
            .unwrap()
            .is_some());
        assert!(store
            .pack_get("proj", "dev", "other", "DB_URL")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_suggest_groups() {
        let store = test_store();
        store.set("proj", "dev", "OPENAI_KEY", "v", None).unwrap();
        store
            .set("proj", "dev", "OPENAI_ENDPOINT", "v", None)
            .unwrap();
        store.set("proj", "dev", "STRIPE_KEY", "v", None).unwrap();
        store
            .set("proj", "dev", "STRIPE_SECRET", "v", None)
            .unwrap();
        store.set("proj", "dev", "JWT_SECRET", "v", None).unwrap();

        let suggestion = store.suggest_groups("proj", "dev", 2).unwrap();
        assert_eq!(suggestion.groups.len(), 2);
        assert_eq!(suggestion.ungrouped.len(), 1);
        assert_eq!(suggestion.ungrouped[0], "JWT_SECRET");
    }

    #[test]
    fn test_compose_detects_conflicts() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "pack1", "API_KEY", "v1")
            .unwrap();
        store
            .pack_set("proj", "dev", "pack2", "API_KEY", "v2")
            .unwrap();

        let result = store
            .compose("proj", "dev", &["pack1".to_string(), "pack2".to_string()])
            .unwrap();
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].key, "API_KEY");
    }

    #[test]
    fn test_compose_no_conflicts() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "OPENAI_KEY", "v1")
            .unwrap();
        store
            .pack_set("proj", "dev", "stripe", "STRIPE_KEY", "v2")
            .unwrap();

        let result = store
            .compose("proj", "dev", &["openai".to_string(), "stripe".to_string()])
            .unwrap();
        assert!(result.conflicts.is_empty());
        assert_eq!(result.secrets.len(), 2);
    }

    #[test]
    fn test_compose_all_includes_flat_and_packs() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "OPENAI_KEY", "v1")
            .unwrap();
        store
            .set("proj", "dev", "DATABASE_URL", "postgres://", None)
            .unwrap();

        let result = store.compose_all("proj", "dev").unwrap();
        assert_eq!(result.secrets.len(), 2);
    }

    #[test]
    fn test_find_key_across_packs() {
        let store = test_store();
        store
            .pack_set("proj", "dev", "openai", "OPENAI_KEY", "v1")
            .unwrap();
        store
            .set("proj", "dev", "DATABASE_URL", "pg", None)
            .unwrap();

        match store
            .find_key_across_packs("proj", "dev", "OPENAI_KEY")
            .unwrap()
        {
            KeyLocation::InPack { pack_name } => assert_eq!(pack_name, "openai"),
            _ => panic!("Expected InPack"),
        }

        match store
            .find_key_across_packs("proj", "dev", "DATABASE_URL")
            .unwrap()
        {
            KeyLocation::InFlatSecrets => {}
            _ => panic!("Expected InFlatSecrets"),
        }

        match store
            .find_key_across_packs("proj", "dev", "NONEXISTENT")
            .unwrap()
        {
            KeyLocation::NotFound => {}
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_pack_adopt_keys() {
        let store = test_store();
        store
            .set("proj", "dev", "OPENAI_KEY", "sk-1", None)
            .unwrap();
        store
            .set("proj", "dev", "OPENAI_EP", "https://", None)
            .unwrap();

        let count = store
            .pack_adopt_keys(
                "proj",
                "dev",
                "openai",
                &["OPENAI_KEY".to_string(), "OPENAI_EP".to_string()],
            )
            .unwrap();
        assert_eq!(count, 2);

        // Should be in pack now
        assert!(store
            .pack_get("proj", "dev", "openai", "OPENAI_KEY")
            .unwrap()
            .is_some());
        // Should be gone from flat
        assert!(store.get("proj", "dev", "OPENAI_KEY").unwrap().is_none());
    }
}
