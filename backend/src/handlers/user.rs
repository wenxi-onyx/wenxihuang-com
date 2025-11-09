use axum::{Extension, Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::auth::UserInfo;
use crate::error::AuthError;
use crate::models::user::User;
use crate::services::encryption;
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

#[derive(Debug, Deserialize)]
pub struct SaveApiKeyRequest {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub provider: String,
    pub api_key_preview: String, // Only shows last 4 characters
    pub has_key: bool,
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
    if req.new_password.len() < 6 {
        return Err(AuthError::InvalidInput(
            "Password must be at least 6 characters".to_string(),
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

/// Get API key status for a provider (returns preview only)
pub async fn get_api_key(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Json<ApiKeyResponse>, AuthError> {
    // Validate provider
    if provider != "anthropic" {
        return Err(AuthError::InvalidInput(
            "Only 'anthropic' provider is supported".to_string(),
        ));
    }

    let result = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT encrypted_key FROM user_api_keys
        WHERE user_id = $1 AND provider = $2
        "#,
    )
    .bind(user.id)
    .bind(&provider)
    .fetch_optional(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;

    if let Some((encrypted_key,)) = result {
        // Decrypt to get the actual key
        let api_key = encryption::decrypt(&encrypted_key).map_err(|_| AuthError::DatabaseError)?;

        // Create preview (last 4 characters)
        let preview = if api_key.len() > 4 {
            format!("...{}", &api_key[api_key.len() - 4..])
        } else {
            "****".to_string()
        };

        Ok(Json(ApiKeyResponse {
            provider,
            api_key_preview: preview,
            has_key: true,
        }))
    } else {
        Ok(Json(ApiKeyResponse {
            provider,
            api_key_preview: String::new(),
            has_key: false,
        }))
    }
}

/// Save or update API key for a provider
pub async fn save_api_key(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(req): Json<SaveApiKeyRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Validate provider
    if req.provider != "anthropic" {
        return Err(AuthError::InvalidInput(
            "Only 'anthropic' provider is supported".to_string(),
        ));
    }

    // Validate API key format (basic validation)
    let api_key = req.api_key.trim();

    if api_key.is_empty() {
        return Err(AuthError::InvalidInput(
            "API key cannot be empty".to_string(),
        ));
    }

    // Anthropic API keys start with "sk-ant-" and are typically 100+ characters
    if !api_key.starts_with("sk-ant-") {
        return Err(AuthError::InvalidInput(
            "Invalid Anthropic API key format. Keys should start with 'sk-ant-'".to_string(),
        ));
    }

    if api_key.len() < 50 {
        return Err(AuthError::InvalidInput(
            "API key appears to be too short to be valid".to_string(),
        ));
    }

    // Encrypt the API key (use trimmed version)
    let encrypted_key = encryption::encrypt(api_key).map_err(|_| AuthError::DatabaseError)?;

    // Insert or update the API key
    sqlx::query(
        r#"
        INSERT INTO user_api_keys (user_id, provider, encrypted_key)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, provider)
        DO UPDATE SET encrypted_key = $3, updated_at = NOW()
        "#,
    )
    .bind(user.id)
    .bind(&req.provider)
    .bind(&encrypted_key)
    .execute(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;

    Ok(Json(serde_json::json!({
        "message": "API key saved successfully"
    })))
}

/// Delete API key for a provider
pub async fn delete_api_key(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Validate provider
    if provider != "anthropic" {
        return Err(AuthError::InvalidInput(
            "Only 'anthropic' provider is supported".to_string(),
        ));
    }

    sqlx::query(
        r#"
        DELETE FROM user_api_keys
        WHERE user_id = $1 AND provider = $2
        "#,
    )
    .bind(user.id)
    .bind(&provider)
    .execute(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;

    Ok(Json(serde_json::json!({
        "message": "API key deleted successfully"
    })))
}
