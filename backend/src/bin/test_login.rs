use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Create database connection pool
    let database_url = std::env::var("DATABASE_URL")?;
    println!("Connecting to: {}", database_url);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to database successfully!");

    // Try to find user by email
    let email = "wenxi";
    println!("\nLooking for user with email: '{}'", email);

    let result: Result<(String, String), sqlx::Error> =
        sqlx::query_as("SELECT email, password_hash FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&pool)
            .await;

    match result {
        Ok((found_email, hash)) => {
            println!("Found user!");
            println!("Email: '{}'", found_email);
            println!("Hash preview: {}...", &hash[..50]);
        }
        Err(e) => {
            println!("ERROR: Could not find user: {:?}", e);
        }
    }

    Ok(())
}
