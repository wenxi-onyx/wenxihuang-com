use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    Unauthorized,
    Forbidden,
    SessionExpired,
    DatabaseError,
    HashingError,
    UsernameAlreadyExists,
    InvalidInput(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            ),
            AuthError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
            ),
            AuthError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Insufficient permissions".to_string(),
            ),
            AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired".to_string()),
            AuthError::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            AuthError::HashingError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing error".to_string(),
            ),
            AuthError::UsernameAlreadyExists => {
                (StatusCode::CONFLICT, "Username already taken".to_string())
            }
            AuthError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
