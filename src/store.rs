//! SQLite-based encrypted secrets store
//!
//! Schema design:
//! - secrets: current values (project, env, key, encrypted_value, metadata)
//! - secret_history: all previous versions for audit trail
//! - metadata: store-level config (passphrase verification, version)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::crypto;

const SCHEMA_VERSION: i32 = 1;

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
}

impl Store {
    /// Get the default store path (~/.tinysecrets/store.db)
    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".tinysecrets").join("store.db"))
    }

    /// Initialize a new store with the given passphrase
    pub fn init(passphrase: SecretString) -> Result<Self> {
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

        // Store passphrase verification
        let verification = crypto::derive_verification(&passphrase)?;
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('passphrase_verification', ?1)",
            params![verification],
        )?;
        conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;

        Ok(Self { conn, passphrase })
    }

    /// Open an existing store
    pub fn open(passphrase: SecretString) -> Result<Self> {
        let path = Self::default_path()?;

        if !path.exists() {
            anyhow::bail!("No store found. Run `ts init` first to create one.");
        }

        let conn = Connection::open(&path).context("Failed to open SQLite database")?;

        // Verify passphrase
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

        Ok(Self { conn, passphrase })
    }

    /// Check if a store exists
    pub fn exists() -> Result<bool> {
        Ok(Self::default_path()?.exists())
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
        let encrypted_value = crypto::encrypt(value, &self.passphrase)?;
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
                let decrypted = crypto::decrypt(&enc, &self.passphrase)?;
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
            let value = crypto::decrypt(&encrypted, &self.passphrase)?;
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
            let decrypted = crypto::decrypt(&secret.encrypted_value, &self.passphrase)?;
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
