use sqlx::PgPool;
use std::env;
use std::error::Error;

// Import the shared ELO service
use backend::services::elo::{get_active_config, get_config_by_version, recalculate_all_elo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🔄 ELO Recalculation Tool");
    println!("========================\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/postgres".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // Get ELO configuration from command line or use active config
    let version_name = env::args().nth(1);

    let config = if let Some(version) = version_name {
        // Use specified version
        match get_config_by_version(&pool, &version).await? {
            Some(cfg) => cfg,
            None => {
                println!("❌ ELO configuration '{}' not found!", version);
                println!("\nAvailable configurations:");
                let configs: Vec<(String, f64, f64, Option<String>)> = sqlx::query_as(
                    "SELECT version_name, k_factor, starting_elo, description FROM elo_configurations ORDER BY created_at"
                )
                .fetch_all(&pool)
                .await?;

                for (name, k, start, desc) in configs {
                    let desc_str = desc.unwrap_or_else(|| "No description".to_string());
                    println!("  - {} (K={}, Start={}) - {}", name, k, start, desc_str);
                }
                return Ok(());
            }
        }
    } else {
        // Use active configuration
        match get_active_config(&pool).await? {
            Some(cfg) => cfg,
            None => {
                println!("❌ No active ELO configuration found!");
                println!("Please create one or specify a version name.");
                return Ok(());
            }
        }
    };

    println!("Using ELO Configuration:");
    println!("  Version: {}", config.version_name);
    println!("  K-Factor: {}", config.k_factor);
    println!("  Starting ELO: {}\n", config.starting_elo);

    // Recalculate using shared service (no job tracking for CLI)
    recalculate_all_elo(&pool, &config, None).await?;

    println!("\n✅ Successfully recalculated ELO!");

    // Show final rankings
    println!("\n📊 Final ELO Rankings ({})", config.version_name);
    println!("==================");

    let rankings: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT first_name, last_name, current_elo FROM players ORDER BY current_elo DESC",
    )
    .fetch_all(&pool)
    .await?;

    for (i, (first, last, elo)) in rankings.iter().enumerate() {
        println!("{}. {} {} - {:.0}", i + 1, first, last, elo);
    }

    Ok(())
}
