use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Create database connection pool
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    // Get the password hash from the database
    let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE email = $1")
        .bind("wenxi")
        .fetch_one(&pool)
        .await?;

    let password_hash = row.0;
    println!("Password hash from database: {}", password_hash);

    // Try to verify password "1"
    let password = "1";
    let parsed_hash = match PasswordHash::new(&password_hash) {
        Ok(h) => h,
        Err(e) => {
            println!("Failed to parse hash: {:?}", e);
            return Ok(());
        }
    };

    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => println!("Password '{}' is CORRECT!", password),
        Err(e) => println!("Password '{}' is INCORRECT: {:?}", password, e),
    }

    Ok(())
}
