use chrono::DateTime;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EloConfig {
    pub version_name: String,
    pub k_factor: f64,
    pub starting_elo: f64,
}

#[derive(Debug)]
struct Game {
    id: Uuid,
    player1_id: Uuid,
    player2_id: Uuid,
    played_at: DateTime<chrono::Utc>,
}

fn calculate_elo_change(winner_elo: f64, loser_elo: f64, k_factor: f64) -> (f64, f64) {
    let expected_winner = 1.0 / (1.0 + 10_f64.powf((loser_elo - winner_elo) / 400.0));
    let expected_loser = 1.0 - expected_winner;

    let winner_change = k_factor * (1.0 - expected_winner);
    let loser_change = k_factor * (0.0 - expected_loser);

    (winner_change, loser_change)
}

/// Recalculate all ELO ratings using the specified configuration
pub async fn recalculate_all_elo(
    pool: &PgPool,
    config: &EloConfig,
    job_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting ELO recalculation with config: {:?}", config);

    // Get all players
    let players: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM players")
        .fetch_all(pool)
        .await?;

    tracing::info!("Found {} players", players.len());

    // Initialize ELO for all players
    let mut player_elos: HashMap<Uuid, f64> = HashMap::new();
    for (player_id,) in players {
        player_elos.insert(player_id, config.starting_elo);
    }

    // Get all games in chronological order
    let games: Vec<Game> = sqlx::query_as::<_, (Uuid, Uuid, Uuid, DateTime<chrono::Utc>)>(
        "SELECT id, player1_id, player2_id, played_at FROM games ORDER BY played_at ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id, p1, p2, played_at)| Game {
        id,
        player1_id: p1,
        player2_id: p2,
        played_at,
    })
    .collect();

    tracing::info!("Found {} games to process", games.len());

    // Start transaction for atomic update
    let mut tx = pool.begin().await?;

    // Delete old ELO history for this version
    sqlx::query("DELETE FROM elo_history WHERE elo_version = $1")
        .bind(&config.version_name)
        .execute(&mut *tx)
        .await?;

    tracing::info!(
        "Cleared existing ELO history for version '{}'",
        config.version_name
    );

    // Process each game
    for (i, game) in games.iter().enumerate() {
        let winner_elo_before = *player_elos
            .get(&game.player1_id)
            .ok_or_else(|| format!("Player {} not found in ELO map", game.player1_id))?;
        let loser_elo_before = *player_elos
            .get(&game.player2_id)
            .ok_or_else(|| format!("Player {} not found in ELO map", game.player2_id))?;

        // Calculate ELO changes
        let (winner_change, loser_change) =
            calculate_elo_change(winner_elo_before, loser_elo_before, config.k_factor);

        let winner_elo_after = winner_elo_before + winner_change;
        let loser_elo_after = loser_elo_before + loser_change;

        // Update in-memory ELO
        player_elos.insert(game.player1_id, winner_elo_after);
        player_elos.insert(game.player2_id, loser_elo_after);

        // Update game's ELO version
        sqlx::query("UPDATE games SET elo_version = $1 WHERE id = $2")
            .bind(&config.version_name)
            .bind(game.id)
            .execute(&mut *tx)
            .await?;

        // Insert ELO history for winner
        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(game.player1_id)
        .bind(game.id)
        .bind(winner_elo_before)
        .bind(winner_elo_after)
        .bind(&config.version_name)
        .bind(game.played_at)
        .execute(&mut *tx)
        .await?;

        // Insert ELO history for loser
        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(game.player2_id)
        .bind(game.id)
        .bind(loser_elo_before)
        .bind(loser_elo_after)
        .bind(&config.version_name)
        .bind(game.played_at)
        .execute(&mut *tx)
        .await?;

        if (i + 1) % 100 == 0 {
            tracing::info!("Processed {}/{} games", i + 1, games.len());

            // Update job progress if job_id is provided
            if let Some(jid) = job_id
                && let Err(e) = crate::services::jobs::update_job_progress(
                    pool,
                    jid,
                    (i + 1) as i32,
                    games.len() as i32,
                )
                .await
            {
                tracing::warn!("Failed to update job progress: {}", e);
            }
        }
    }

    // Update all players' current ELO
    for (player_id, elo) in player_elos.iter() {
        sqlx::query("UPDATE players SET current_elo = $1 WHERE id = $2")
            .bind(elo)
            .bind(player_id)
            .execute(&mut *tx)
            .await?;
    }

    // Commit transaction
    tx.commit().await?;

    tracing::info!("Successfully recalculated ELO for {} games", games.len());

    Ok(())
}

/// Get the active ELO configuration
#[allow(dead_code)]
pub async fn get_active_config(pool: &PgPool) -> Result<Option<EloConfig>, sqlx::Error> {
    let row: Option<(String, f64, f64)> = sqlx::query_as(
        "SELECT version_name, k_factor, starting_elo FROM elo_configurations WHERE is_active = true"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(version_name, k_factor, starting_elo)| EloConfig {
        version_name,
        k_factor,
        starting_elo,
    }))
}

/// Get ELO configuration by version name
pub async fn get_config_by_version(
    pool: &PgPool,
    version: &str,
) -> Result<Option<EloConfig>, sqlx::Error> {
    let row: Option<(String, f64, f64)> = sqlx::query_as(
        "SELECT version_name, k_factor, starting_elo FROM elo_configurations WHERE version_name = $1"
    )
    .bind(version)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(version_name, k_factor, starting_elo)| EloConfig {
        version_name,
        k_factor,
        starting_elo,
    }))
}
