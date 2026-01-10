//! Migrate secrets from legacy age format to fast ChaCha20 format

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use colored::Colorize;
use rusqlite::params;
use secrecy::ExposeSecret;

use crate::cli::prompt_passphrase;
use crate::crypto::{self, MasterKey};
use crate::store::Store;

pub fn run() -> Result<()> {
    eprintln!("{}", "🔄 Migrating secrets to fast encryption format...".cyan());
    eprintln!();

    let passphrase = prompt_passphrase()?;
    
    // Open store (this derives the master key)
    let store = Store::open(passphrase.clone())?;
    
    // Get raw connection for direct queries
    let conn = store.connection();
    
    // Get salt for master key derivation
    let salt_b64: String = conn.query_row(
        "SELECT value FROM metadata WHERE key = 'encryption_salt'",
        [],
        |row| row.get(0),
    )?;
    let salt_vec = BASE64.decode(&salt_b64)?;
    let salt: [u8; 32] = salt_vec.try_into().map_err(|_| anyhow::anyhow!("Invalid salt"))?;
    let master_key = MasterKey::derive(&passphrase, &salt)?;
    
    // Get all secrets
    let mut stmt = conn.prepare(
        "SELECT id, project, environment, key, encrypted_value FROM secrets"
    )?;
    
    let secrets: Vec<(i64, String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    
    let total = secrets.len();
    let mut migrated = 0;
    let mut already_new = 0;
    
    for (id, project, env, key, encrypted) in secrets {
        // Check if already in new format (starts with version byte 0x02 after base64 decode)
        let data = BASE64.decode(&encrypted)?;
        
        if !data.is_empty() && data[0] == 2 {
            // Already in new format
            already_new += 1;
            continue;
        }
        
        // Decrypt with legacy format
        let decrypted = crypto::decrypt(&encrypted, &master_key, &passphrase)?;
        
        // Re-encrypt with new fast format
        let new_encrypted = crypto::encrypt(decrypted.expose_secret(), &master_key)?;
        
        // Update in database
        conn.execute(
            "UPDATE secrets SET encrypted_value = ?1 WHERE id = ?2",
            params![new_encrypted, id],
        )?;
        
        migrated += 1;
        eprint!("\r  {} {}/{} - {}/{}/{}          ", "✓".green(), migrated, total - already_new, project, env, key);
    }
    
    if migrated > 0 {
        eprintln!();
    }
    
    eprintln!();
    eprintln!("{} Migrated {} secrets to fast format", "✅".green(), migrated.to_string().bold());
    if already_new > 0 {
        eprintln!("   ({} were already in new format)", already_new);
    }
    
    Ok(())
}
