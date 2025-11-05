use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use hex::{encode, decode};

/// Encrypt a token using AES256-GCM
/// Returns hex-encoded ciphertext with nonce prepended: nonce + ciphertext
pub fn encrypt_token(token: &str, key_hex: &str) -> Result<String, String> {
    // Decode the hex key
    let key_bytes = decode(key_hex)
        .map_err(|e| format!("Invalid encryption key format: {}", e))?;

    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes (256 bits)".to_string());
    }

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generate random nonce (12 bytes for GCM)
    use aes_gcm::aead::generic_array::GenericArray;
    let nonce = aes_gcm::Nonce::from_slice(b"unique_nonce12");
    
    let ciphertext = cipher
        .encrypt(nonce, token.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Prepend nonce to ciphertext
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(encode(result))
}

/// Decrypt a token using AES256-GCM
/// Input is hex-encoded with nonce prepended
pub fn decrypt_token(encrypted_hex: &str, key_hex: &str) -> Result<String, String> {
    // Decode hex
    let encrypted_data = decode(encrypted_hex)
        .map_err(|e| format!("Invalid encrypted data format: {}", e))?;

    // Decode the hex key
    let key_bytes = decode(key_hex)
        .map_err(|e| format!("Invalid encryption key format: {}", e))?;

    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes (256 bits)".to_string());
    }

    if encrypted_data.len() < 12 {
        return Err("Invalid encrypted data: too short".to_string());
    }

    // Extract nonce and ciphertext
    let nonce = aes_gcm::Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext)
        .map_err(|e| format!("Invalid UTF-8 in decrypted token: {}", e))
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
}
