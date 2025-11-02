use base64::{Engine as _, engine::general_purpose};
use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::User;

pub fn generate_session_id() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    general_purpose::STANDARD.encode(bytes)
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    let session_id = generate_session_id();
    let expires_at = Utc::now() + Duration::days(30);

    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&session_id)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await?;

    Ok(session_id)
}

pub async fn validate_session(pool: &PgPool, session_id: &str) -> Result<User, AuthError> {
    // Check if session exists and is not expired
    let session = sqlx::query_as::<_, (Uuid, chrono::DateTime<Utc>)>(
        "SELECT user_id, expires_at FROM sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?
    .ok_or(AuthError::Unauthorized)?;

    // Check expiration
    if session.1 < Utc::now() {
        // Delete expired session
        delete_session(pool, session_id).await?;
        return Err(AuthError::SessionExpired);
    }

    // Update last_accessed
    sqlx::query("UPDATE sessions SET last_accessed = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|_| AuthError::DatabaseError)?;

    // Get user
    User::find_by_id(pool, session.0)
        .await
        .map_err(|_| AuthError::Unauthorized)
}

pub async fn delete_session(pool: &PgPool, session_id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|_| AuthError::DatabaseError)?;

    Ok(())
}

#[allow(dead_code)]
pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
