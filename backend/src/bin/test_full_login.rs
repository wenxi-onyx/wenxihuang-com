use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use sqlx::postgres::PgPoolOptions;

// Copy of User struct
#[derive(Debug, sqlx::FromRow)]
struct User {
    id: uuid::Uuid,
    username: String,
    password_hash: String,
    role: String, // Just use String for testing
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Create database connection pool (same as main.rs)
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✓ Connected to database");

    // Step 1: Find user by username (same query as User::find_by_username)
    let username = "wenxi";
    println!("\n1. Finding user with username: '{}'", username);

    let user: User = sqlx::query_as(
        "SELECT id, username, password_hash, role::text as role FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_one(&pool)
    .await?;

    println!("✓ Found user: {}", user.username);
    println!("  ID: {}", user.id);
    println!("  Role: {}", user.role);

    // Step 2: Verify password (same as verify_password function)
    let password = "1";
    println!("\n2. Verifying password: '{}'", password);

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| format!("Failed to parse hash: {:?}", e))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|e| format!("Password verification failed: {:?}", e))?;

    println!("✓ Password verified successfully!");

    println!("\n✓✓✓ Login flow completed successfully! ✓✓✓");

    Ok(())
}
