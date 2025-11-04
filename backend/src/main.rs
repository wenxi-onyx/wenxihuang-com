use axum::{
    Json, Router,
    routing::{get, post, put},
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Validate required environment variables
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let session_secret = std::env::var("SESSION_SECRET").expect("SESSION_SECRET must be set");

    // Validate session secret length (should be at least 32 bytes)
    if session_secret.len() < 32 {
        panic!("SESSION_SECRET must be at least 32 characters long");
    }

    tracing::info!("Environment variables loaded successfully");

    // Create database connection pool with better settings for production
    let pool = PgPoolOptions::new()
        .max_connections(20) // Increased from 5 for better concurrency
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {}", e);
            e
        })?;

    tracing::info!("Database connection pool created successfully");

    // Auth routes
    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route(
            "/register",
            post(handlers::auth::register).route_layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                self::middleware::auth::require_admin,
            )),
        )
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

    // User routes (authenticated users)
    let user_routes = Router::new()
        .route("/profile", get(handlers::user::get_profile))
        .route("/profile", put(handlers::user::update_profile))
        .route("/change-password", post(handlers::user::change_password))
        .route_layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            self::middleware::auth::require_auth,
        ));

    // Admin routes (admin users only)
    let admin_routes = Router::new()
        .route("/users", post(handlers::admin::create_user))
        .route_layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            self::middleware::auth::require_admin,
        ));

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/api/auth", auth_routes)
        .nest("/api/user", user_routes)
        .nest("/api/admin", admin_routes)
        .with_state(pool)
        .layer(CookieManagerLayer::new())
        .layer(self::middleware::cors::cors_layer());

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        tracing::error!("Failed to bind to {}: {}", addr, e);
        e
    })?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await.map_err(|e| {
        tracing::error!("Server error: {}", e);
        e
    })?;

    Ok(())
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
