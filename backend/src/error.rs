use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AuthError {
    InvalidCredentials,
    Unauthorized,
    Forbidden,
    SessionExpired,
    DatabaseError,
    HashingError,
    UsernameAlreadyExists,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid username or password")
            }
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Authentication required"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "Insufficient permissions"),
            AuthError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired"),
            AuthError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AuthError::HashingError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Password hashing error")
            }
            AuthError::UsernameAlreadyExists => (StatusCode::CONFLICT, "Username already taken"),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
