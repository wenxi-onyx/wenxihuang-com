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

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Internal(String),
    NotFound(String),
    BadRequest(String),
    Forbidden(String),
    FileSizeTooLarge(String),
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::FileSizeTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

// Convert from sqlx::Error to AppError
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e.to_string())
    }
}
