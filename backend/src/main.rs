use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true);

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .layer(cors);

    // Run it with hyper on 0.0.0.0:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({
        "message": "Welcome to wenxihuang.com API",
        "status": "online",
        "version": "0.1.0"
    }))
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
