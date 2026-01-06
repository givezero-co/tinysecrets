//! System keychain integration for storing the passphrase securely.
//!
//! On macOS: Uses the native `security` CLI for reliable Keychain access
//! On other platforms: Uses the keyring crate

use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use std::process::Command;

const SERVICE_NAME: &str = "tinysecrets";
const ACCOUNT_NAME: &str = "passphrase";

/// Store passphrase in system keychain
pub fn store_passphrase(passphrase: &SecretString) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Delete existing entry first (ignore errors)
        let _ = Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                ACCOUNT_NAME,
            ])
            .output();

        // Add new entry
        let output = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                ACCOUNT_NAME,
                "-w",
                passphrase.expose_secret(),
                "-U", // Update if exists
            ])
            .output()
            .context("Failed to run security command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to store in keychain: {}", stderr.trim());
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .context("Failed to access system keychain")?;
        entry
            .set_password(passphrase.expose_secret())
            .context("Failed to store passphrase in keychain")?;
        Ok(())
    }
}

/// Retrieve passphrase from system keychain
pub fn get_passphrase() -> Result<Option<SecretString>> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                ACCOUNT_NAME,
                "-w", // Output password only
            ])
            .output()
            .context("Failed to run security command")?;

        if output.status.success() {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !password.is_empty() {
                return Ok(Some(SecretString::new(password)));
            }
        }
        Ok(None)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .context("Failed to access system keychain")?;
        match entry.get_password() {
            Ok(password) => Ok(Some(SecretString::new(password))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(keyring::Error::Ambiguous(_)) => Ok(None),
            Err(e) => Err(e).context("Failed to retrieve passphrase from keychain"),
        }
    }
}

/// Delete passphrase from system keychain
pub fn delete_passphrase() -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                SERVICE_NAME,
                "-a",
                ACCOUNT_NAME,
            ])
            .output()
            .context("Failed to run security command")?;

        Ok(output.status.success())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
            .context("Failed to access system keychain")?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(e).context("Failed to delete passphrase from keychain"),
        }
    }
}

/// Check if passphrase is stored in keychain
pub fn has_passphrase() -> bool {
    get_passphrase().ok().flatten().is_some()
}
