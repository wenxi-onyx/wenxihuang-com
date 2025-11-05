use axum::{
    Json, Router,
    routing::{delete, get, patch, post, put},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;

mod error;
mod handlers;
mod middleware;
mod models;
mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing with better visibility
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_line_number(true)
        .with_file(false)
        .compact()
        .init();

    tracing::info!("=== wenxihuang.com Backend Starting ===");

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

    // Run database migrations automatically on startup
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run migrations: {}", e);
            e
        })?;
    tracing::info!("Database migrations completed successfully");

    // Check if any admin users exist, create one if not
    tracing::info!("Checking for admin users...");
    let admin_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check for admin users: {}", e);
            e
        })?;

    if admin_count.0 == 0 {
        tracing::info!("No admin users found, creating default admin...");

        // Get admin password from environment or use default
        let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| {
            tracing::warn!("ADMIN_PASSWORD not set, using default password 'admin'");
            tracing::warn!("⚠️  SECURITY WARNING: Please change the admin password immediately!");
            "admin".to_string()
        });

        // Hash the password
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(admin_password.as_bytes(), &salt)
            .expect("Failed to hash password")
            .to_string();

        // Create admin user
        sqlx::query(
            "INSERT INTO users (username, password_hash, first_name, last_name, role)
             VALUES ($1, $2, $3, $4, 'admin')",
        )
        .bind("admin")
        .bind(password_hash)
        .bind("Admin")
        .bind("User")
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create admin user: {}", e);
            e
        })?;

        tracing::info!("✓ Created default admin user (username: admin)");
        if std::env::var("ADMIN_PASSWORD").is_err() {
            tracing::warn!("⚠️  Using default password - please change it immediately!");
        }
    } else {
        tracing::info!("✓ Admin user exists");
    }

    tracing::info!("Setting up routes...");

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
        .route("/matches", post(handlers::matches::create_match))
        .route_layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            self::middleware::auth::require_auth,
        ));

    // Admin routes (admin users only)
    let admin_routes = Router::new()
        .route("/users", post(handlers::admin::create_user))
        .route(
            "/elo-configurations",
            post(handlers::elo::create_elo_config),
        )
        .route("/elo-configurations", get(handlers::elo::list_elo_configs))
        .route(
            "/elo-configurations/{version_name}",
            put(handlers::elo::update_elo_config),
        )
        .route(
            "/elo-configurations/{version_name}",
            delete(handlers::elo::delete_elo_config),
        )
        .route(
            "/elo-configurations/{version_name}/activate",
            post(handlers::elo::activate_elo_config),
        )
        .route(
            "/elo-configurations/{version_name}/recalculate",
            post(handlers::elo::recalculate_elo),
        )
        .route("/jobs/{job_id}", get(handlers::elo::get_job_status))
        .route(
            "/players/{player_id}/toggle-active",
            post(handlers::players::toggle_player_active),
        )
        // Season management routes
        .route("/seasons", post(handlers::seasons::create_season))
        .route(
            "/seasons/{season_id}/activate",
            post(handlers::seasons::activate_season),
        )
        .route(
            "/seasons/{season_id}/recalculate",
            post(handlers::seasons::recalculate_season),
        )
        .route(
            "/seasons/{season_id}/elo-version",
            patch(handlers::seasons::update_season_elo_version),
        )
        .route(
            "/seasons/{season_id}",
            delete(handlers::seasons::delete_season),
        )
        // Season player management routes
        .route(
            "/seasons/{season_id}/players",
            get(handlers::seasons::get_season_players),
        )
        .route(
            "/seasons/{season_id}/available-players",
            get(handlers::seasons::get_available_players),
        )
        .route(
            "/seasons/{season_id}/players/add",
            post(handlers::seasons::add_player_to_season),
        )
        .route(
            "/seasons/{season_id}/players/remove",
            post(handlers::seasons::remove_player_from_season),
        )
        // Match management routes
        .route(
            "/matches/{match_id}",
            delete(handlers::matches::delete_match),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            self::middleware::auth::require_admin,
        ));

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/players", get(handlers::players::list_players))
        .route(
            "/players/{player_id}/history",
            get(handlers::players::get_player_history),
        )
        // Season routes
        .route("/seasons", get(handlers::seasons::list_seasons))
        .route("/seasons/active", get(handlers::seasons::get_active_season))
        .route(
            "/seasons/active/players",
            get(handlers::seasons::get_active_season_players),
        )
        .route("/seasons/{season_id}", get(handlers::seasons::get_season))
        .route(
            "/seasons/{season_id}/leaderboard",
            get(handlers::seasons::get_season_leaderboard),
        )
        // Match routes
        .route("/matches", get(handlers::matches::list_matches));

    tracing::info!("Routes configured successfully");

    // Build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/api/auth", auth_routes)
        .nest("/api/user", user_routes)
        .nest("/api/admin", admin_routes)
        .nest("/api", public_routes)
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
        .layer(CookieManagerLayer::new())
        .layer(self::middleware::cors::cors_layer());

    tracing::info!("Application layers configured");

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🚀 Starting server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        tracing::error!("❌ Failed to bind to {}: {}", addr, e);
        e
    })?;

    tracing::info!("✓ Server listening on http://{}", addr);
    tracing::info!("✓ Health check available at http://{}/health", addr);
    tracing::info!("✓ API available at http://{}/api", addr);

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
