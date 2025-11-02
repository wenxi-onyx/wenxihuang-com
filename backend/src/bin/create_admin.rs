use sqlx::postgres::PgPoolOptions;
use std::io::{self, Write};

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

    // Get username and password from command line args or prompt
    let args: Vec<String> = std::env::args().collect();

    let username = if args.len() > 1 {
        args[1].clone()
    } else {
        print!("Enter admin username: ");
        io::stdout().flush()?;
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        username.trim().to_string()
    };

    let password = if args.len() > 2 {
        args[2].clone()
    } else {
        print!("Enter admin password: ");
        io::stdout().flush()?;
        rpassword::read_password()?
    };

    // Hash password
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?
        .to_string();

    // Create admin user
    sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ($1, $2, 'admin')")
        .bind(username)
        .bind(password_hash)
        .execute(&pool)
        .await?;

    println!("Admin user created successfully!");

    Ok(())
}
