use chrono::{DateTime, NaiveDateTime, Utc};
use csv::ReaderBuilder;
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use uuid::Uuid;

// Import the shared ELO service
use backend::services::elo::get_active_config;

#[derive(Debug)]
struct Match {
    time: DateTime<Utc>,
    winner: String,
    loser: String,
}

#[derive(Debug, Clone)]
struct Player {
    id: Uuid,
    #[allow(dead_code)]
    first_name: String,
    #[allow(dead_code)]
    last_name: String,
    elo: f64,
}

fn parse_player_name(name: &str) -> (String, String) {
    // Parse names like "W Huang", "Y Sun", "R Bhagat"
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        let first_name = parts[0].to_string();
        let last_name = parts[1..].join(" ");
        (first_name, last_name)
    } else {
        // Fallback for single names
        (name.to_string(), String::new())
    }
}

fn parse_timestamp(time_str: &str) -> Result<DateTime<Utc>, Box<dyn Error>> {
    // Try multiple date formats
    let formats = vec![
        "%m/%d/%y %I:%M %p", // 6/11/25 11:03 AM
        "%m/%d/%Y %H:%M:%S", // 10/16/2025 17:45:00
        "%m/%d/%Y",          // 11/03/2025
    ];

    for format in formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(time_str, format) {
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
        // Try parsing as just date
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(time_str, format) {
            let naive_dt = naive_date.and_hms_opt(12, 0, 0).unwrap();
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }

    Err(format!("Could not parse timestamp: {}", time_str).into())
}

fn calculate_elo_change(winner_elo: f64, loser_elo: f64, k_factor: f64) -> (f64, f64) {
    let expected_winner = 1.0 / (1.0 + 10_f64.powf((loser_elo - winner_elo) / 400.0));
    let expected_loser = 1.0 - expected_winner;

    let winner_change = k_factor * (1.0 - expected_winner);
    let loser_change = k_factor * (0.0 - expected_loser);

    (winner_change, loser_change)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🏓 Table Tennis Match Importer");
    println!("================================\n");

    // Get CSV path from command line or use default
    let csv_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Table Tennis Database - Ledger.csv".to_string());

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5433/wenxihuang".to_string());

    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;

    // Get active ELO configuration
    println!("Loading ELO configuration...");
    let config = get_active_config(&pool)
        .await?
        .ok_or("No active ELO configuration found. Please create one first.")?;

    println!("Using ELO Configuration:");
    println!("  Version: {}", config.version_name);
    println!("  K-Factor: {}", config.k_factor);
    println!("  Starting ELO: {}\n", config.starting_elo);

    // Read CSV file
    println!("Reading CSV file: {}...", csv_path);
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(csv_path)?;

    let mut matches: Vec<Match> = Vec::new();

    for result in reader.records() {
        let record = result?;
        let time_str = record.get(0).unwrap_or("");
        let winner = record.get(1).unwrap_or("").to_string();
        let loser = record.get(2).unwrap_or("").to_string();

        match parse_timestamp(time_str) {
            Ok(time) => {
                matches.push(Match {
                    time,
                    winner,
                    loser,
                });
            }
            Err(e) => {
                println!(
                    "⚠️  Skipping record with invalid timestamp: {} - {}",
                    time_str, e
                );
            }
        }
    }

    // Sort matches by time (oldest first)
    matches.sort_by(|a, b| a.time.cmp(&b.time));

    println!("Found {} valid matches\n", matches.len());

    // Create or find all unique players
    println!("Creating players...");
    let mut player_map: HashMap<String, Player> = HashMap::new();
    let mut unique_names = std::collections::HashSet::new();

    for m in &matches {
        unique_names.insert(m.winner.clone());
        unique_names.insert(m.loser.clone());
    }

    for name in unique_names {
        let (first_name, last_name) = parse_player_name(&name);

        // Check if player already exists
        let existing: Option<(Uuid, String, String, f64)> = sqlx::query_as(
            "SELECT id, first_name, last_name, current_elo FROM players WHERE first_name = $1 AND last_name = $2"
        )
        .bind(&first_name)
        .bind(&last_name)
        .fetch_optional(&pool)
        .await?;

        let player = if let Some((id, first, last, elo)) = existing {
            println!(
                "  Found existing player: {} {} (ELO: {:.0})",
                first, last, elo
            );
            Player {
                id,
                first_name: first,
                last_name: last,
                elo,
            }
        } else {
            // Create new player with starting ELO from config
            let id: (Uuid,) = sqlx::query_as(
                "INSERT INTO players (first_name, last_name, current_elo) VALUES ($1, $2, $3) RETURNING id"
            )
            .bind(&first_name)
            .bind(&last_name)
            .bind(config.starting_elo)
            .fetch_one(&pool)
            .await?;

            println!(
                "  Created new player: {} {} (Starting ELO: {:.0})",
                first_name, last_name, config.starting_elo
            );
            Player {
                id: id.0,
                first_name: first_name.clone(),
                last_name: last_name.clone(),
                elo: config.starting_elo,
            }
        };

        player_map.insert(name, player);
    }

    println!("\n{} players ready\n", player_map.len());

    // Import matches and update ELO
    println!("Importing matches and calculating ELO...");
    let mut imported = 0;
    let mut skipped = 0;

    for (i, m) in matches.iter().enumerate() {
        // Get ELO values before (read-only access)
        let winner_elo_before = player_map.get(&m.winner).unwrap().elo;
        let loser_elo_before = player_map.get(&m.loser).unwrap().elo;

        // Get player IDs for duplicate check
        let winner_id = player_map.get(&m.winner).unwrap().id;
        let loser_id = player_map.get(&m.loser).unwrap().id;

        // Check if this match already exists (idempotency check)
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM games WHERE player1_id = $1 AND player2_id = $2 AND played_at = $3",
        )
        .bind(winner_id)
        .bind(loser_id)
        .bind(m.time)
        .fetch_optional(&pool)
        .await?;

        if existing.is_some() {
            skipped += 1;
            if (i + 1) % 50 == 0 {
                println!(
                    "  Processed {}/{} matches ({} imported, {} skipped)...",
                    i + 1,
                    matches.len(),
                    imported,
                    skipped
                );
            }
            continue; // Skip duplicate
        }

        // Calculate ELO changes
        let (winner_change, loser_change) =
            calculate_elo_change(winner_elo_before, loser_elo_before, config.k_factor);

        // Update ELOs (separate mutable accesses)
        let winner = player_map.get_mut(&m.winner).unwrap();
        winner.elo += winner_change;
        let winner_elo_after = winner.elo;

        let loser = player_map.get_mut(&m.loser).unwrap();
        loser.elo += loser_change;
        let loser_elo_after = loser.elo;

        // Insert game (winner is player1, loser is player2, score is 1-0 for win/loss tracking)
        let game_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO games (player1_id, player2_id, player1_score, player2_score, played_at, elo_version)
             VALUES ($1, $2, 1, 0, $3, $4) RETURNING id"
        )
        .bind(winner_id)
        .bind(loser_id)
        .bind(m.time)
        .bind(&config.version_name)
        .fetch_one(&pool)
        .await?;

        // Insert ELO history for both players
        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(winner_id)
        .bind(game_id.0)
        .bind(winner_elo_before)
        .bind(winner_elo_after)
        .bind(&config.version_name)
        .bind(m.time)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(loser_id)
        .bind(game_id.0)
        .bind(loser_elo_before)
        .bind(loser_elo_after)
        .bind(&config.version_name)
        .bind(m.time)
        .execute(&pool)
        .await?;

        imported += 1;

        if (i + 1) % 50 == 0 {
            println!(
                "  Processed {}/{} matches ({} imported, {} skipped)...",
                i + 1,
                matches.len(),
                imported,
                skipped
            );
        }
    }

    println!(
        "\n✅ Successfully imported {} matches ({} skipped as duplicates)!",
        imported, skipped
    );

    // Update final ELO ratings for all players
    println!("\nUpdating final ELO ratings...");
    for (_name, player) in player_map.iter() {
        sqlx::query("UPDATE players SET current_elo = $1 WHERE id = $2")
            .bind(player.elo)
            .bind(player.id)
            .execute(&pool)
            .await?;
    }

    println!("\n🎉 Import complete!");
    println!("\nFinal ELO Ratings:");
    println!("==================");

    let players: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT first_name, last_name, current_elo FROM players ORDER BY current_elo DESC",
    )
    .fetch_all(&pool)
    .await?;

    for (i, (first, last, elo)) in players.iter().enumerate() {
        println!("{}. {} {} - {:.0}", i + 1, first, last, elo);
    }

    Ok(())
}
