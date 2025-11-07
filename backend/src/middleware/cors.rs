use axum::http::{
    Method,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue},
};
use tower_http::cors::CorsLayer;

pub fn cors_layer() -> CorsLayer {
    // Read CORS origins from environment, fallback to defaults
    let cors_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            // Default origins for development and production
            "https://wenxihuang.com,https://www.wenxihuang.com,https://wenxihuang-frontend.fly.dev,http://localhost:5173,http://127.0.0.1:5173".to_string()
        });

    // Parse and filter valid origins
    let valid_origins: Vec<HeaderValue> = cors_origins
        .split(',')
        .filter_map(|origin| {
            let trimmed = origin.trim();
            match trimmed.parse::<HeaderValue>() {
                Ok(header) => {
                    tracing::info!("Allowing CORS origin: {}", trimmed);
                    Some(header)
                }
                Err(e) => {
                    tracing::warn!("Invalid CORS origin '{}': {}", trimmed, e);
                    None
                }
            }
        })
        .collect();

    // WebSocket-specific headers
    let ws_headers = [
        AUTHORIZATION,
        CONTENT_TYPE,
        HeaderName::from_static("sec-websocket-protocol"),
        HeaderName::from_static("sec-websocket-extensions"),
    ];

    // If no valid origins were parsed, use a safe default (only allow production domain)
    if valid_origins.is_empty() {
        tracing::warn!("No valid CORS origins configured, using production-only defaults");
        CorsLayer::new()
            .allow_origin("https://wenxihuang.com".parse::<HeaderValue>().unwrap())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(ws_headers)
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(valid_origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(ws_headers)
            .allow_credentials(true)
    }
}
