use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use rand::RngCore;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use thiserror::Error;

type Nonce = [u8; 12];

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Hex decode error: {0}")]
    HexDecode(String),
    #[error("Base64 decode error: {0}")]
    Base64Decode(String),
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(String),
}

/// Encrypt a token using AES256-GCM with versioning
/// Returns base64-encoded data: `[version_byte][nonce(12)][ciphertext]`
pub fn encrypt_token(token: &str, key_hex: &str) -> Result<String, CryptoError> {
    // Decode the hex key
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| CryptoError::HexDecode(e.to_string()))?;

    if key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKey(
            "Encryption key must be 32 bytes (256 bits)".to_string(),
        ));
    }

    // Create key from array slice
    let key: [u8; 32] = key_bytes.try_into()
        .map_err(|_| CryptoError::InvalidKey("Key conversion failed".to_string()))?;
    let cipher = Aes256Gcm::new(&key.into());

    // Generate random nonce (12 bytes for GCM) using cryptographically secure RNG
    let mut nonce_bytes: Nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    
    let ciphertext = cipher
        .encrypt((&nonce_bytes).into(), token.as_bytes())
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    // Build versioned format: [version_byte][nonce(12)][ciphertext]
    let mut encrypted_data = Vec::with_capacity(1 + 12 + ciphertext.len());
    encrypted_data.push(0x01); // Version 1
    encrypted_data.extend_from_slice(&nonce_bytes);
    encrypted_data.extend_from_slice(&ciphertext);

    // Encode as base64 for transport/storage
    Ok(BASE64.encode(encrypted_data))
}

/// Decrypt a token using AES256-GCM
/// Input is base64-encoded with versioning: `[version_byte][nonce(12)][ciphertext]`
pub fn decrypt_token(encrypted_b64: &str, key_hex: &str) -> Result<String, CryptoError> {
    // Decode base64
    let encrypted_data = BASE64
        .decode(encrypted_b64)
        .map_err(|e| CryptoError::Base64Decode(e.to_string()))?;

    if encrypted_data.len() < 13 {
        return Err(CryptoError::InvalidData(
            "Encrypted data too short (need at least 1 + 12 bytes for version + nonce)"
                .to_string(),
        ));
    }

    // Check version
    let version = encrypted_data[0];
    if version != 0x01 {
        return Err(CryptoError::InvalidData(format!(
            "Unsupported encryption version: {}",
            version
        )));
    }

    // Decode the hex key
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| CryptoError::HexDecode(e.to_string()))?;

    if key_bytes.len() != 32 {
        return Err(CryptoError::InvalidKey(
            "Encryption key must be 32 bytes (256 bits)".to_string(),
        ));
    }

    // Extract nonce and ciphertext
    let nonce: Nonce = encrypted_data[1..13]
        .try_into()
        .map_err(|_| CryptoError::InvalidData("Failed to extract nonce".to_string()))?;
    let ciphertext = &encrypted_data[13..];

    let key: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKey("Key conversion failed".to_string()))?;
    let cipher = Aes256Gcm::new(&key.into());

    let plaintext = cipher
        .decrypt((&nonce).into(), ciphertext)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError::Utf8Error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let token = "test_token_12345";

        let encrypted = encrypt_token(token, key_hex).expect("Encryption failed");
        let decrypted = decrypt_token(&encrypted, key_hex).expect("Decryption failed");

        assert_eq!(token, decrypted);
    }

    #[test]
    fn test_different_nonces() {
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let token = "test_token_12345";

        let encrypted1 = encrypt_token(token, key_hex).expect("Encryption 1 failed");
        let encrypted2 = encrypt_token(token, key_hex).expect("Encryption 2 failed");

        // Should be different due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same value
        let decrypted1 = decrypt_token(&encrypted1, key_hex).expect("Decryption 1 failed");
        let decrypted2 = decrypt_token(&encrypted2, key_hex).expect("Decryption 2 failed");

        assert_eq!(token, decrypted1);
        assert_eq!(token, decrypted2);
    }
}

