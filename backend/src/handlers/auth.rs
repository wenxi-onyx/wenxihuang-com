use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::Duration;
use tower_cookies::{Cookie, Cookies};

use crate::error::AuthError;
use crate::models::user::{User, UserRole};
use crate::services::password::{hash_password, verify_password};
use crate::services::session::{create_session, delete_session, validate_session};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: uuid::Uuid,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
        }
    }
}

/// Login handler
pub async fn login(
    State(pool): State<PgPool>,
    cookies: Cookies,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    // Find user by username
    let user = User::find_by_username(&pool, &req.username)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

    // Verify password
    verify_password(&req.password, &user.password_hash)?;

    // Create session (30 days)
    let session_id = create_session(&pool, user.id)
        .await
        .map_err(|_| AuthError::DatabaseError)?;

    // Create secure cookie (30 days)
    let mut cookie = Cookie::new("session_id", session_id);
    cookie.set_http_only(true);
    // Only require HTTPS in production
    cookie.set_secure(cfg!(not(debug_assertions)));
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_max_age(Duration::days(30));
    cookie.set_path("/");

    cookies.add(cookie);

    Ok(Json(AuthResponse { user: user.into() }))
}

/// Logout handler
pub async fn logout(
    State(pool): State<PgPool>,
    cookies: Cookies,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Extract session ID from cookie
    if let Some(cookie) = cookies.get("session_id") {
        let session_id = cookie.value();

        // Delete session from database
        delete_session(&pool, session_id).await?;
    }

    // Remove cookie
    let mut cookie = Cookie::new("session_id", "");
    cookie.set_http_only(true);
    // Only require HTTPS in production
    cookie.set_secure(cfg!(not(debug_assertions)));
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_max_age(Duration::ZERO);
    cookie.set_path("/");

    cookies.add(cookie);

    Ok(Json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}

/// Get current user handler
pub async fn me(
    State(pool): State<PgPool>,
    cookies: Cookies,
) -> Result<Json<AuthResponse>, AuthError> {
    // Extract session from cookie
    let cookie = cookies.get("session_id").ok_or(AuthError::Unauthorized)?;
    let session_id = cookie.value().to_string();

    // Validate session and get user
    let user = validate_session(&pool, &session_id).await?;

    Ok(Json(AuthResponse { user: user.into() }))
}

/// Register new user (admin only)
pub async fn register(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    // Check if username already exists
    if User::find_by_username(&pool, &req.username).await.is_ok() {
        return Err(AuthError::UsernameAlreadyExists);
    }

    // Hash password
    let password_hash = hash_password(&req.password)?;

    // Create user
    let user = User::create(&pool, &req.username, &password_hash, None, None, req.role)
        .await
        .map_err(|_| AuthError::DatabaseError)?;

    Ok(Json(AuthResponse { user: user.into() }))
}
