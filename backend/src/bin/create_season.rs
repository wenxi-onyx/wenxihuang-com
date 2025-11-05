use chrono::Utc;
use sqlx::PgPool;
use std::env;
use std::error::Error;

use backend::services::seasons;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🏓 Season Creation Tool");
    println!("======================\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/postgres".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // Get season name from command line
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: create_season <season_name> [starting_elo] [k_factor]");
        println!("\nExamples:");
        println!("  create_season \"Spring 2025\"");
        println!("  create_season \"Summer 2025\" 1000 32");
        println!("\nAvailable seasons:");
        list_seasons(&pool).await?;
        return Ok(());
    }

    let season_name = &args[1];
    let starting_elo: f64 = if args.len() > 2 {
        args[2].parse()?
    } else {
        1000.0
    };
    let k_factor: f64 = if args.len() > 3 {
        args[3].parse()?
    } else {
        32.0
    };

    // Check if season already exists
    if let Some(existing) = seasons::get_season_by_name(&pool, season_name).await? {
        println!("❌ Season '{}' already exists!", season_name);
        println!(
            "   Created: {}",
            existing.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!("   Active: {}", existing.is_active);
        return Ok(());
    }

    // Create a dummy admin user ID for CLI operations
    // In production, you might want to create a system user for this
    let admin_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "SELECT id FROM users WHERE email LIKE '%admin%' LIMIT 1",
    )
    .fetch_optional(&pool)
    .await?
    .map(|(id,)| id)
    .unwrap_or_else(uuid::Uuid::nil);

    // Create the season
    println!("Creating season: {}", season_name);
    println!("  Starting ELO: {}", starting_elo);
    println!("  K-factor: {}", k_factor);
    println!("  Start date: {}", Utc::now().format("%Y-%m-%d %H:%M:%S"));

    let season = seasons::create_season(
        &pool,
        season_name.to_string(),
        Some(format!(
            "Created via CLI on {}",
            Utc::now().format("%Y-%m-%d")
        )),
        Utc::now(),
        starting_elo,
        k_factor,
        None, // No dynamic K-factor by default
        None,
        None,
        None, // elo_version
        admin_id,
        None, // Initialize all active players
    )
    .await?;

    println!("\n✅ Season created successfully!");
    println!("   ID: {}", season.id);
    println!("   Name: {}", season.name);

    // Initialize players
    println!("\nInitializing players for the season...");
    let player_count = seasons::initialize_season_players(&pool, season.id).await?;
    println!("✅ Initialized {} players", player_count);

    // Ask if user wants to activate this season
    println!("\nDo you want to activate this season now? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        seasons::activate_season(&pool, season.id).await?;
        println!("✅ Season '{}' is now active!", season_name);
    } else {
        println!("Season created but not activated. You can activate it later.");
    }

    println!("\n📊 Current Seasons:");
    list_seasons(&pool).await?;

    Ok(())
}

async fn list_seasons(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let seasons = seasons::get_all_seasons(pool).await?;

    if seasons.is_empty() {
        println!("  No seasons found");
        return Ok(());
    }

    for season in seasons {
        let status = if season.is_active {
            "ACTIVE"
        } else {
            "inactive"
        };

        println!(
            "  {} - {} (started: {}) [{}]",
            season.name,
            season.id,
            season.start_date.format("%Y-%m-%d"),
            status
        );
    }

    Ok(())
}
