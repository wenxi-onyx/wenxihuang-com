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

/// Helper function to create a session cookie with consistent settings
fn create_session_cookie(value: String, max_age: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::new("session_id", value);
    cookie.set_http_only(true);
    cookie.set_path("/");

    // Use SameSite::Lax for Safari compatibility
    // Since frontend and backend share the same root domain (wenxihuang.com),
    // SameSite::Lax works and is more compatible with Safari than SameSite::None
    if cfg!(debug_assertions) {
        cookie.set_secure(false);
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    } else {
        cookie.set_secure(true);
        cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
        // Set domain to allow cookie sharing between wenxihuang.com and api.wenxihuang.com
        cookie.set_domain("wenxihuang.com");
    }
    cookie.set_max_age(max_age);
    cookie
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

    // Create and add secure cookie (30 days)
    let cookie = create_session_cookie(session_id, Duration::days(30));
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

    // Remove cookie by setting max age to zero
    let cookie = create_session_cookie("".to_string(), Duration::ZERO);
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
