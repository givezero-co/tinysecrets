//! System keychain integration for storing the passphrase securely.
//!
//! Uses:
//! - macOS: Keychain
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//! - Windows: Credential Manager

use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};

const SERVICE_NAME: &str = "tinysecrets";
const ACCOUNT_NAME: &str = "default";

/// Store passphrase in system keychain
pub fn store_passphrase(passphrase: &SecretString) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    entry.set_password(passphrase.expose_secret())?;
    Ok(())
}

/// Retrieve passphrase from system keychain
pub fn get_passphrase() -> Result<Option<SecretString>> {
    let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    match entry.get_password() {
        Ok(password) => Ok(Some(SecretString::new(password))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Delete passphrase from system keychain
pub fn delete_passphrase() -> Result<bool> {
    let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Check if passphrase is stored in keychain
pub fn has_passphrase() -> bool {
    get_passphrase().ok().flatten().is_some()
}
