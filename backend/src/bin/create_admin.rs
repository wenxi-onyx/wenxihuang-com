use argon2::{
    Argon2, password_hash::PasswordHasher, password_hash::SaltString,
    password_hash::rand_core::OsRng,
};
use sqlx::PgPool;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: create_admin <password>");
        std::process::exit(1);
    }

    let password = &args[1];

    // Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    println!("🔐 Creating admin user...");
    println!("Password hash: {}", password_hash);

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/postgres".to_string());

    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;

    // Check if admin user already exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT username FROM users WHERE username = 'admin'")
            .fetch_optional(&pool)
            .await?;

    if existing.is_some() {
        println!("⚠️  Admin user already exists, updating password...");
        sqlx::query("UPDATE users SET password_hash = $1 WHERE username = 'admin'")
            .bind(&password_hash)
            .execute(&pool)
            .await?;
        println!("✅ Admin password updated successfully!");
    } else {
        println!("Creating new admin user...");
        sqlx::query(
            "INSERT INTO users (username, password_hash, role, first_name, last_name)
             VALUES ('admin', $1, 'admin', 'Admin', 'User')",
        )
        .bind(&password_hash)
        .execute(&pool)
        .await?;
        println!("✅ Admin user created successfully!");
    }

    println!("\nYou can now log in with:");
    println!("  Username: admin");
    println!("  Password: {}", password);

    Ok(())
}
