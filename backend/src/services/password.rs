use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::error::AuthError;

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::HashingError)?
        .to_string();

    Ok(password_hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, password_hash: &str) -> Result<(), AuthError> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|_| AuthError::InvalidCredentials)?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)
}
