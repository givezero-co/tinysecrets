//! Encryption module with fast symmetric encryption
//!
//! Uses scrypt for key derivation (once per session) and ChaCha20-Poly1305 for
//! encrypting individual secrets (fast). This provides strong security while
//! keeping operations snappy.
//!
//! Format: version(1) || nonce(12) || ciphertext || tag(16)

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use scrypt::{scrypt, Params};
use secrecy::{ExposeSecret, Secret, SecretString};
use std::io::{Read, Write};

/// Current encryption format version
const CRYPTO_VERSION: u8 = 2;

/// Legacy version (age-based)
const LEGACY_VERSION: u8 = 1;

/// Scrypt parameters (N=2^15, r=8, p=1) - ~100ms on modern hardware
/// These are reasonable for interactive use while still being secure
const SCRYPT_LOG_N: u8 = 15;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

/// Salt for key derivation (fixed per-store, stored in metadata)
const SALT_LEN: usize = 32;

/// Derived master key for fast encryption
pub struct MasterKey {
    key: [u8; 32],
}

impl MasterKey {
    /// Derive a master key from passphrase and salt using scrypt
    pub fn derive(passphrase: &SecretString, salt: &[u8]) -> Result<Self> {
        let params = Params::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, 32)
            .map_err(|e| anyhow::anyhow!("Invalid scrypt params: {}", e))?;

        let mut key = [0u8; 32];
        scrypt(
            passphrase.expose_secret().as_bytes(),
            salt,
            &params,
            &mut key,
        )
        .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        Ok(Self { key })
    }

    /// Generate a random salt for a new store
    pub fn generate_salt() -> [u8; SALT_LEN] {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }
}

/// Encrypts plaintext using ChaCha20-Poly1305 (fast)
pub fn encrypt(plaintext: &str, master_key: &MasterKey) -> Result<String> {
    let cipher = ChaCha20Poly1305::new_from_slice(&master_key.key)
        .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Format: version || nonce || ciphertext
    let mut output = Vec::with_capacity(1 + 12 + ciphertext.len());
    output.push(CRYPTO_VERSION);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&output))
}

/// Decrypts ciphertext - handles both v2 (fast) and v1 (legacy age) formats
pub fn decrypt(
    ciphertext: &str,
    master_key: &MasterKey,
    passphrase: &SecretString,
) -> Result<Secret<String>> {
    let data = BASE64
        .decode(ciphertext)
        .context("Failed to decode base64 ciphertext")?;

    if data.is_empty() {
        anyhow::bail!("Empty ciphertext");
    }

    let version = data[0];

    match version {
        CRYPTO_VERSION => decrypt_v2(&data[1..], master_key),
        LEGACY_VERSION => decrypt_legacy(ciphertext, passphrase),
        _ if is_age_format(&data) => decrypt_legacy(ciphertext, passphrase),
        _ => anyhow::bail!("Unknown encryption format version: {}", version),
    }
}

/// Decrypt v2 format (ChaCha20-Poly1305)
fn decrypt_v2(data: &[u8], master_key: &MasterKey) -> Result<Secret<String>> {
    if data.len() < 12 {
        anyhow::bail!("Ciphertext too short");
    }

    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];

    let cipher = ChaCha20Poly1305::new_from_slice(&master_key.key)
        .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed - invalid key or corrupted data"))?;

    let text = String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")?;
    Ok(Secret::new(text))
}

/// Check if data looks like age-encrypted format
fn is_age_format(data: &[u8]) -> bool {
    // Age format starts with "age-encryption.org" header
    data.starts_with(b"age-encryption.org") || 
    // Or if base64-decoded starts with age header
    String::from_utf8_lossy(data).starts_with("age-encryption.org")
}

/// Decrypt legacy age format (v1)
fn decrypt_legacy(ciphertext: &str, passphrase: &SecretString) -> Result<Secret<String>> {
    let encrypted = BASE64
        .decode(ciphertext)
        .context("Failed to decode base64 ciphertext")?;

    let decryptor =
        match age::Decryptor::new(&encrypted[..]).context("Failed to create decryptor")? {
            age::Decryptor::Passphrase(d) => d,
            _ => anyhow::bail!("Expected passphrase-encrypted data"),
        };

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(passphrase, None)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    reader
        .read_to_end(&mut decrypted)
        .context("Failed to read decrypted data")?;

    let plaintext = String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")?;
    Ok(Secret::new(plaintext))
}

/// Derives a verification hash from the passphrase (still uses age for compatibility)
/// This only runs once when opening the store
pub fn derive_verification(passphrase: &SecretString) -> Result<String> {
    let encryptor = age::Encryptor::with_user_passphrase(passphrase.clone());

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .context("Failed to create encryption writer")?;

    writer
        .write_all(b"tinysecrets-verification-v1")
        .context("Failed to write encrypted data")?;

    writer.finish().context("Failed to finish encryption")?;

    Ok(BASE64.encode(&encrypted))
}

/// Verifies the passphrase against a stored verification hash
pub fn verify_passphrase(passphrase: &SecretString, verification: &str) -> bool {
    let Ok(encrypted) = BASE64.decode(verification) else {
        return false;
    };

    let Ok(decryptor) = age::Decryptor::new(&encrypted[..]) else {
        return false;
    };

    let age::Decryptor::Passphrase(d) = decryptor else {
        return false;
    };

    let Ok(mut reader) = d.decrypt(passphrase, None) else {
        return false;
    };

    let mut decrypted = vec![];
    if reader.read_to_end(&mut decrypted).is_err() {
        return false;
    }

    decrypted == b"tinysecrets-verification-v1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let passphrase = SecretString::new("test-passphrase".to_string());
        let salt = MasterKey::generate_salt();
        let master_key = MasterKey::derive(&passphrase, &salt).unwrap();
        let plaintext = "my-secret-value";

        let encrypted = encrypt(plaintext, &master_key).unwrap();
        let decrypted = decrypt(&encrypted, &master_key, &passphrase).unwrap();

        assert_eq!(decrypted.expose_secret(), plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let passphrase1 = SecretString::new("correct-passphrase".to_string());
        let passphrase2 = SecretString::new("wrong-passphrase".to_string());
        let salt = MasterKey::generate_salt();
        let key1 = MasterKey::derive(&passphrase1, &salt).unwrap();
        let key2 = MasterKey::derive(&passphrase2, &salt).unwrap();
        let plaintext = "my-secret-value";

        let encrypted = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&encrypted, &key2, &passphrase2);

        assert!(result.is_err());
    }

    #[test]
    fn test_verification() {
        let passphrase = SecretString::new("test-passphrase".to_string());
        let verification = derive_verification(&passphrase).unwrap();

        assert!(verify_passphrase(&passphrase, &verification));

        let wrong_passphrase = SecretString::new("wrong".to_string());
        assert!(!verify_passphrase(&wrong_passphrase, &verification));
    }
}
