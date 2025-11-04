use axum::{Extension, Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::auth::UserInfo;
use crate::error::AuthError;
use crate::models::user::User;
use crate::services::password::{hash_password, verify_password};

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub user: UserInfo,
}

/// Get current user's profile
pub async fn get_profile(
    Extension(user): Extension<User>,
) -> Result<Json<ProfileResponse>, AuthError> {
    Ok(Json(ProfileResponse { user: user.into() }))
}

/// Update current user's profile (username, first_name, last_name)
pub async fn update_profile(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, AuthError> {
    // Validate username length
    if req.username.len() < 3 || req.username.len() > 20 {
        return Err(AuthError::InvalidInput(
            "Username must be 3-20 characters".to_string(),
        ));
    }

    // Check if username is already taken (but allow if it's the same as current username)
    if req.username != user.username && User::find_by_username(&pool, &req.username).await.is_ok() {
        return Err(AuthError::UsernameAlreadyExists);
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

    // Update user profile
    let updated_user = User::update_profile(
        &pool,
        user.id,
        &req.username,
        req.first_name.as_deref(),
        req.last_name.as_deref(),
    )
    .await
    .map_err(|_| AuthError::DatabaseError)?;

    Ok(Json(ProfileResponse {
        user: updated_user.into(),
    }))
}

/// Change current user's password
pub async fn change_password(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Verify current password
    verify_password(&req.current_password, &user.password_hash)?;

    // Validate new password
    if req.new_password.len() < 8 {
        return Err(AuthError::InvalidInput(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash new password
    let new_password_hash = hash_password(&req.new_password)?;

    // Update password
    User::update_password(&pool, user.id, &new_password_hash)
        .await
        .map_err(|_| AuthError::DatabaseError)?;

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}
