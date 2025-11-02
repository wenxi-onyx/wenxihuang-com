use axum::http::{
    Method,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue},
};
use tower_http::cors::CorsLayer;

pub fn cors_layer() -> CorsLayer {
    // Read CORS origins from environment, fallback to defaults
    let cors_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| {
            // Default origins for development and production
            "https://wenxihuang.com,https://www.wenxihuang.com,https://wenxihuang-frontend.fly.dev,http://localhost:5173".to_string()
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

    // If no valid origins were parsed, use a safe default (only allow production domain)
    if valid_origins.is_empty() {
        tracing::warn!("No valid CORS origins configured, using production-only defaults");
        CorsLayer::new()
            .allow_origin("https://wenxihuang.com".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(valid_origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
            .allow_credentials(true)
    }
}
