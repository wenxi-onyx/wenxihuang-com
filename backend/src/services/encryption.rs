use crate::error::AppError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::env;

/// Encryption service for sensitive data (API keys)
///
/// This module provides AES-256-GCM encryption for storing sensitive user data.
///
/// # Security Considerations
///
/// - The encryption key is derived from the SESSION_SECRET environment variable
/// - If SESSION_SECRET is changed, all existing encrypted data becomes unreadable
/// - Always backup your SESSION_SECRET and keep it secure
/// - Never commit SESSION_SECRET to version control
///
/// # Key Rotation
///
/// If you need to rotate the SESSION_SECRET:
/// 1. Decrypt all API keys with the old secret
/// 2. Change the SESSION_SECRET
/// 3. Re-encrypt all API keys with the new secret
///
/// This is not handled automatically and requires manual intervention.
/// Get encryption key from environment variable SESSION_SECRET
/// We use SHA-256 to derive a 32-byte key from the session secret
fn get_encryption_key() -> Result<[u8; 32], AppError> {
    let session_secret = env::var("SESSION_SECRET").map_err(|_| {
        AppError::Internal("SESSION_SECRET environment variable not set".to_string())
    })?;

    let mut hasher = Sha256::new();
    hasher.update(session_secret.as_bytes());
    let result = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&result);

    Ok(key)
}

/// Encrypt a string using AES-256-GCM
/// Returns base64-encoded string in format: nonce:ciphertext
pub fn encrypt(plaintext: &str) -> Result<String, AppError> {
    let key_bytes = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to create cipher: {}", e)))?;

    // Generate random nonce
    let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt the plaintext
    let ciphertext = cipher
        .encrypt(&nonce_bytes, plaintext.as_bytes())
        .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

    // Combine nonce and ciphertext, then base64 encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(&combined))
}

/// Decrypt a string using AES-256-GCM
/// Expects base64-encoded string in format: nonce:ciphertext
pub fn decrypt(encrypted: &str) -> Result<String, AppError> {
    let key_bytes = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to create cipher: {}", e)))?;

    // Decode from base64
    let combined = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|e| AppError::Internal(format!("Failed to decode encrypted data: {}", e)))?;

    // Split into nonce and ciphertext
    if combined.len() < 12 {
        return Err(AppError::Internal("Invalid encrypted data".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce_array: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| AppError::Internal("Invalid nonce size".to_string()))?;
    let nonce = Nonce::from(nonce_array);

    // Decrypt
    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext).map_err(|e| {
        AppError::Internal(format!("Failed to convert decrypted data to string: {}", e))
    })
}
