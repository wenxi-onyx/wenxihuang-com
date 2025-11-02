use axum::{
    Json, Router,
    routing::{get, post},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_cookies::CookieManagerLayer;

mod error;
mod handlers;
mod middleware;
mod models;
mod services;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create database connection pool
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Auth routes
    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route(
            "/logout",
            post(handlers::auth::logout).route_layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                self::middleware::auth::require_auth,
            )),
        )
        .route(
            "/me",
            get(handlers::auth::me).route_layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                self::middleware::auth::require_auth,
            )),
        );

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/api/auth", auth_routes)
        .with_state(pool)
        .layer(CookieManagerLayer::new())
        .layer(self::middleware::cors::cors_layer());

    // Run it on 0.0.0.0:8080
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
