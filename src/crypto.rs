//! Encryption module using age (Actually Good Encryption)
//! 
//! Uses passphrase-based encryption with scrypt for key derivation.
//! All secrets are encrypted before storage and decrypted only in memory.

use age::secrecy::ExposeSecret;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use secrecy::{Secret, SecretString};
use std::io::{Read, Write};

/// Encrypts plaintext using age with a passphrase
pub fn encrypt(plaintext: &str, passphrase: &SecretString) -> Result<String> {
    let encryptor = age::Encryptor::with_user_passphrase(passphrase.clone());
    
    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .context("Failed to create encryption writer")?;
    
    writer
        .write_all(plaintext.as_bytes())
        .context("Failed to write encrypted data")?;
    
    writer.finish().context("Failed to finish encryption")?;
    
    Ok(BASE64.encode(&encrypted))
}

/// Decrypts ciphertext using age with a passphrase
pub fn decrypt(ciphertext: &str, passphrase: &SecretString) -> Result<Secret<String>> {
    let encrypted = BASE64
        .decode(ciphertext)
        .context("Failed to decode base64 ciphertext")?;
    
    let decryptor = match age::Decryptor::new(&encrypted[..])
        .context("Failed to create decryptor")?
    {
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
    
    let plaintext = String::from_utf8(decrypted)
        .context("Decrypted data is not valid UTF-8")?;
    
    Ok(Secret::new(plaintext))
}

/// Derives a verification hash from the passphrase
/// Used to verify the passphrase is correct without storing it
pub fn derive_verification(passphrase: &SecretString) -> Result<String> {
    // Encrypt a known value - if we can decrypt it later, passphrase is correct
    encrypt("tinysecrets-verification-v1", passphrase)
}

/// Verifies the passphrase against a stored verification hash
pub fn verify_passphrase(passphrase: &SecretString, verification: &str) -> bool {
    decrypt(verification, passphrase)
        .map(|s| s.expose_secret() == "tinysecrets-verification-v1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let passphrase = SecretString::new("test-passphrase".to_string());
        let plaintext = "my-secret-value";
        
        let encrypted = encrypt(plaintext, &passphrase).unwrap();
        let decrypted = decrypt(&encrypted, &passphrase).unwrap();
        
        assert_eq!(decrypted.expose_secret(), plaintext);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let passphrase1 = SecretString::new("correct-passphrase".to_string());
        let passphrase2 = SecretString::new("wrong-passphrase".to_string());
        let plaintext = "my-secret-value";
        
        let encrypted = encrypt(plaintext, &passphrase1).unwrap();
        let result = decrypt(&encrypted, &passphrase2);
        
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

