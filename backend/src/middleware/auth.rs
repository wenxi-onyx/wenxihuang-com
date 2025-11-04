use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use tower_cookies::Cookies;

use crate::error::AuthError;
use crate::models::user::{User, UserRole};
use crate::services::session::validate_session;

/// Middleware to require authentication
pub async fn require_auth(
    State(pool): State<PgPool>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract session ID from cookie
    let cookie = cookies.get("session_id").ok_or(AuthError::Unauthorized)?;
    let session_id = cookie.value().to_string();

    // Validate session and get user
    let user = validate_session(&pool, &session_id).await?;

    // Attach user to request extensions
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Middleware to require admin role
pub async fn require_admin(
    State(pool): State<PgPool>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // First authenticate
    let cookie = cookies.get("session_id").ok_or(AuthError::Unauthorized)?;
    let session_id = cookie.value().to_string();

    let user = validate_session(&pool, &session_id).await?;

    // Check if user is admin
    if !matches!(user.role, UserRole::Admin) {
        return Err(AuthError::Forbidden);
    }

    // Attach user to request extensions
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Helper function to extract user from request extensions
#[allow(dead_code)]
pub fn get_user_from_request(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}
