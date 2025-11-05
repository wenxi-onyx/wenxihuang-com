use sqlx::PgPool;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 Initializing wenxihuang.com table tennis database");
    println!("====================================================\n");

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5433/wenxihuang_backend".to_string()
    });

    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    println!("✓ Connected\n");

    // Step 1: Set up ELO configuration v3 as active
    println!("Step 1: Configuring ELO algorithm...");

    // Deactivate all existing configs
    sqlx::query("UPDATE elo_configurations SET is_active = false WHERE is_active = true")
        .execute(&pool)
        .await
        .ok();

    // Set v3 as active (it should already exist from migrations)
    let result =
        sqlx::query("UPDATE elo_configurations SET is_active = true WHERE version_name = 'v3'")
            .execute(&pool)
            .await?;

    if result.rows_affected() > 0 {
        println!("✓ ELO algorithm v3 (1000 base, varying K-factor) is now active\n");
    } else {
        println!("⚠ Warning: v3 config not found in database\n");
    }

    // Step 2: Import historic matches
    println!("Step 2: Importing historic match data...");

    // Get active ELO configuration
    let config: Option<(String, f64, f64)> = sqlx::query_as(
        "SELECT version_name, starting_elo, k_factor FROM elo_configurations WHERE is_active = true"
    )
    .fetch_optional(&pool)
    .await?;

    let (version_name, starting_elo, k_factor) = match config {
        Some(c) => c,
        None => {
            println!("✗ No active ELO configuration found. Skipping match import.");
            return Ok(());
        }
    };

    println!(
        "  Using: {}, Starting ELO: {}, K-factor: {}",
        version_name, starting_elo, k_factor
    );

    // Import matches using the import_matches logic
    // Read CSV file
    let csv_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Table Tennis Database - Ledger.csv".to_string());

    println!("  Reading matches from: {}", csv_path);

    let csv_content = std::fs::read_to_string(&csv_path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let mut matches = Vec::new();
    for result in reader.records() {
        let record = result?;
        if let (Some(time_str), Some(winner), Some(loser)) =
            (record.get(0), record.get(1), record.get(2))
        {
            // Parse timestamp (simple format for now)
            if let Ok(time) = chrono::NaiveDateTime::parse_from_str(time_str, "%m/%d/%y %I:%M %p") {
                let utc_time: chrono::DateTime<chrono::Utc> =
                    chrono::DateTime::from_naive_utc_and_offset(time, chrono::Utc);
                matches.push((utc_time, winner.to_string(), loser.to_string()));
            }
        }
    }

    matches.sort_by(|a, b| a.0.cmp(&b.0));
    println!("  Found {} matches to import", matches.len());

    // Get or create players
    let mut player_ids = std::collections::HashMap::new();
    let mut player_elos = std::collections::HashMap::new();

    for (_, winner, loser) in &matches {
        for name in [winner, loser] {
            if player_ids.contains_key(name) {
                continue;
            }

            // Parse name
            let parts: Vec<&str> = name.split_whitespace().collect();
            let (first_name, last_name) = if parts.len() >= 2 {
                (parts[0].to_string(), parts[1..].join(" "))
            } else {
                (name.to_string(), String::new())
            };

            // Check if player exists
            let existing: Option<(sqlx::types::Uuid, f64)> = sqlx::query_as(
                "SELECT id, current_elo FROM players WHERE first_name = $1 AND last_name = $2",
            )
            .bind(&first_name)
            .bind(&last_name)
            .fetch_optional(&pool)
            .await?;

            let (id, elo) = if let Some((id, elo)) = existing {
                (id, elo)
            } else {
                // Create new player
                let id: (sqlx::types::Uuid,) = sqlx::query_as(
                    "INSERT INTO players (first_name, last_name, current_elo) VALUES ($1, $2, $3) RETURNING id"
                )
                .bind(&first_name)
                .bind(&last_name)
                .bind(starting_elo)
                .fetch_one(&pool)
                .await?;
                (id.0, starting_elo)
            };

            player_ids.insert(name.clone(), id);
            player_elos.insert(name.clone(), elo);
        }
    }

    println!("  Processed {} players", player_ids.len());

    // Get All-Time season ID
    let all_time_season_id: (sqlx::types::Uuid,) =
        sqlx::query_as("SELECT id FROM seasons WHERE name = 'All-Time'")
            .fetch_one(&pool)
            .await?;
    let season_id = all_time_season_id.0;

    println!("  Using season: All-Time ({})", season_id);

    // Import matches
    let mut imported = 0;
    for (time, winner, loser) in matches {
        let winner_id = player_ids[&winner];
        let loser_id = player_ids[&loser];
        let winner_elo = player_elos[&winner];
        let loser_elo = player_elos[&loser];

        // Calculate ELO changes
        let expected_winner = 1.0 / (1.0 + 10_f64.powf((loser_elo - winner_elo) / 400.0));
        let winner_change = k_factor * (1.0 - expected_winner);
        let loser_change = k_factor * (0.0 - (1.0 - expected_winner));

        let new_winner_elo = winner_elo + winner_change;
        let new_loser_elo = loser_elo + loser_change;

        // Insert game
        let game_id: (sqlx::types::Uuid,) = sqlx::query_as(
            "INSERT INTO games (player1_id, player2_id, player1_score, player2_score, played_at, elo_version, season_id)
             VALUES ($1, $2, 1, 0, $3, $4, $5) RETURNING id"
        )
        .bind(winner_id)
        .bind(loser_id)
        .bind(time)
        .bind(&version_name)
        .bind(season_id)
        .fetch_one(&pool)
        .await?;

        // Insert ELO history
        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(winner_id)
        .bind(game_id.0)
        .bind(winner_elo)
        .bind(new_winner_elo)
        .bind(&version_name)
        .bind(season_id)
        .bind(time)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(loser_id)
        .bind(game_id.0)
        .bind(loser_elo)
        .bind(new_loser_elo)
        .bind(&version_name)
        .bind(season_id)
        .bind(time)
        .execute(&pool)
        .await?;

        // Update ELOs in memory
        player_elos.insert(winner, new_winner_elo);
        player_elos.insert(loser, new_loser_elo);

        imported += 1;
    }

    // Update final ELOs in database and create player_seasons entries
    for (name, elo) in &player_elos {
        let player_id = player_ids[name];

        // Update current ELO
        sqlx::query("UPDATE players SET current_elo = $1 WHERE id = $2")
            .bind(elo)
            .bind(player_id)
            .execute(&pool)
            .await?;

        // Get player's stats for the season
        let (games, wins): (i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*) as games,
                COUNT(*) FILTER (WHERE player1_id = $1) as wins
             FROM games
             WHERE (player1_id = $1 OR player2_id = $1) AND season_id = $2",
        )
        .bind(player_id)
        .bind(season_id)
        .fetch_one(&pool)
        .await?;

        // Create or update player_seasons entry
        sqlx::query(
            "INSERT INTO player_seasons (player_id, season_id, current_elo, games_played, wins, losses)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (player_id, season_id) DO UPDATE
             SET current_elo = $3, games_played = $4, wins = $5, losses = $6"
        )
        .bind(player_id)
        .bind(season_id)
        .bind(elo)
        .bind(games as i32)
        .bind(wins as i32)
        .bind((games - wins) as i32)
        .execute(&pool)
        .await?;
    }

    println!("✓ Imported {} matches\n", imported);

    println!("====================================================");
    println!("✅ Initialization complete!");
    println!("\nNOTE: Run './create_admin' to create an admin user after initialization");
    println!("\nFinal leaderboard:");

    let players: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT first_name, last_name, current_elo FROM players ORDER BY current_elo DESC LIMIT 10",
    )
    .fetch_all(&pool)
    .await?;

    for (i, (first, last, elo)) in players.iter().enumerate() {
        println!("  {}. {} {} - {:.1}", i + 1, first, last, elo);
    }

    Ok(())
}
