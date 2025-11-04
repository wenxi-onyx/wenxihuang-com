use axum::{Extension, Json, extract::State};
use serde::Deserialize;
use sqlx::PgPool;

use super::auth::UserInfo;
use crate::error::AuthError;
use crate::models::user::{User, UserRole};
use crate::services::password::hash_password;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
}

/// Create a new user (admin only)
/// Note: This endpoint should be protected by admin middleware
pub async fn create_user(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>, // This user is already verified as admin by middleware
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Validate username length
    if req.username.len() < 3 || req.username.len() > 20 {
        return Err(AuthError::InvalidInput(
            "Username must be 3-20 characters".to_string(),
        ));
    }

    // Validate password length
    if req.password.len() < 8 {
        return Err(AuthError::InvalidInput(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Validate first_name - reject empty strings or names longer than 50 chars
    if let Some(ref name) = req.first_name {
        if name.trim().is_empty() {
            return Err(AuthError::InvalidInput(
                "First name cannot be empty".to_string(),
            ));
        }
        if name.len() > 50 {
            return Err(AuthError::InvalidInput(
                "First name must be 50 characters or less".to_string(),
            ));
        }
    }

    // Validate last_name - reject empty strings or names longer than 50 chars
    if let Some(ref name) = req.last_name {
        if name.trim().is_empty() {
            return Err(AuthError::InvalidInput(
                "Last name cannot be empty".to_string(),
            ));
        }
        if name.len() > 50 {
            return Err(AuthError::InvalidInput(
                "Last name must be 50 characters or less".to_string(),
            ));
        }
    }

    // Check if username already exists
    if User::find_by_username(&pool, &req.username).await.is_ok() {
        return Err(AuthError::UsernameAlreadyExists);
    }

    // Hash password
    let password_hash = hash_password(&req.password)?;

    // Create user
    let user = User::create(
        &pool,
        &req.username,
        &password_hash,
        req.first_name.as_deref(),
        req.last_name.as_deref(),
        req.role,
    )
    .await
    .map_err(|_| AuthError::DatabaseError)?;

    let user_info: UserInfo = user.into();

    Ok(Json(serde_json::json!({
        "message": "User created successfully",
        "user": user_info
    })))
}
