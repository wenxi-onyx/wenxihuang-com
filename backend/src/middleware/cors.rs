use axum::http::{
    Method,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use tower_http::cors::CorsLayer;

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "https://wenxihuang.com".parse().unwrap(),
            "https://www.wenxihuang.com".parse().unwrap(),
            "http://localhost:5173".parse().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
}
