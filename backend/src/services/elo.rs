use chrono::DateTime;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

type EloConfigRow = (String, f64, f64, Option<f64>, Option<f64>, Option<i32>);

#[derive(Debug, Clone)]
pub struct EloConfig {
    pub version_name: String,
    pub k_factor: f64,
    pub starting_elo: f64,
    pub base_k_factor: Option<f64>,
    pub new_player_k_bonus: Option<f64>,
    pub new_player_bonus_period: Option<i32>,
}

#[derive(Debug)]
struct Game {
    id: Uuid,
    player1_id: Uuid,
    player2_id: Uuid,
    played_at: DateTime<chrono::Utc>,
}

/// Calculate dynamic K-factor based on player experience
/// Formula: K = base_k + (new_player_k_bonus * e^(-games_played / bonus_period))
fn calculate_dynamic_k_factor(config: &EloConfig, games_played: i32) -> f64 {
    // If dynamic K-factor is not configured, use static k_factor
    if let (Some(base_k), Some(bonus), Some(period)) = (
        config.base_k_factor,
        config.new_player_k_bonus,
        config.new_player_bonus_period,
    ) && period > 0
    {
        let decay = (-games_played as f64 / period as f64).exp();
        return base_k + (bonus * decay);
    }

    // Fallback to static k_factor
    config.k_factor
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

    // Initialize ELO and games played for all players
    let mut player_elos: HashMap<Uuid, f64> = HashMap::new();
    let mut player_games_played: HashMap<Uuid, i32> = HashMap::new();
    for (player_id,) in players {
        player_elos.insert(player_id, config.starting_elo);
        player_games_played.insert(player_id, 0);
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

        // Get games played for each player
        let winner_games = *player_games_played
            .get(&game.player1_id)
            .ok_or_else(|| format!("Player {} not found in games played map", game.player1_id))?;
        let loser_games = *player_games_played
            .get(&game.player2_id)
            .ok_or_else(|| format!("Player {} not found in games played map", game.player2_id))?;

        // Calculate dynamic K-factors for both players
        let winner_k = calculate_dynamic_k_factor(config, winner_games);
        let loser_k = calculate_dynamic_k_factor(config, loser_games);

        // Calculate expected scores
        let expected_winner =
            1.0 / (1.0 + 10_f64.powf((loser_elo_before - winner_elo_before) / 400.0));
        let expected_loser = 1.0 - expected_winner;

        // Calculate ELO changes with player-specific K-factors
        let winner_change = winner_k * (1.0 - expected_winner);
        let loser_change = loser_k * (0.0 - expected_loser);

        let winner_elo_after = winner_elo_before + winner_change;
        let loser_elo_after = loser_elo_before + loser_change;

        // Update in-memory ELO and games played
        player_elos.insert(game.player1_id, winner_elo_after);
        player_elos.insert(game.player2_id, loser_elo_after);
        player_games_played.insert(game.player1_id, winner_games + 1);
        player_games_played.insert(game.player2_id, loser_games + 1);

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
    let row: Option<EloConfigRow> = sqlx::query_as(
        "SELECT version_name, k_factor, starting_elo, base_k_factor, new_player_k_bonus, new_player_bonus_period
         FROM elo_configurations WHERE is_active = true"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(
            version_name,
            k_factor,
            starting_elo,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
        )| EloConfig {
            version_name,
            k_factor,
            starting_elo,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
        },
    ))
}

/// Get ELO configuration by version name
pub async fn get_config_by_version(
    pool: &PgPool,
    version: &str,
) -> Result<Option<EloConfig>, sqlx::Error> {
    let row: Option<EloConfigRow> = sqlx::query_as(
        "SELECT version_name, k_factor, starting_elo, base_k_factor, new_player_k_bonus, new_player_bonus_period
         FROM elo_configurations WHERE version_name = $1"
    )
    .bind(version)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(
            version_name,
            k_factor,
            starting_elo,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
        )| EloConfig {
            version_name,
            k_factor,
            starting_elo,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
        },
    ))
}

/// Enum to represent which player won a game
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum GameWinner {
    Player1,
    Player2,
}

/// Represents ELO changes for a single game within a match
#[derive(Debug, Clone)]
pub struct MatchEloChange {
    pub game_id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub player1_elo_before: f64,
    pub player1_elo_after: f64,
    pub player1_elo_change: f64,
    pub player2_elo_before: f64,
    pub player2_elo_after: f64,
    pub player2_elo_change: f64,
}

/// Calculate ELO changes for all games in a match sequentially
/// Each game uses the updated ELO from the previous game
pub fn calculate_match_elo_changes(
    player1_id: Uuid,
    player2_id: Uuid,
    player1_starting_elo: f64,
    player2_starting_elo: f64,
    games: Vec<(Uuid, GameWinner)>, // (game_id, winner)
    player1_k_factor: f64,
    player2_k_factor: f64,
) -> Vec<MatchEloChange> {
    let mut current_p1_elo = player1_starting_elo;
    let mut current_p2_elo = player2_starting_elo;
    let mut changes = Vec::new();

    for (game_id, winner) in games {
        // Calculate expected scores
        let expected_p1 = 1.0 / (1.0 + 10_f64.powf((current_p2_elo - current_p1_elo) / 400.0));
        let expected_p2 = 1.0 - expected_p1;

        // Determine actual scores based on winner
        let (p1_score, p2_score) = match winner {
            GameWinner::Player1 => (1.0, 0.0),
            GameWinner::Player2 => (0.0, 1.0),
        };

        // Calculate ELO changes
        let p1_change = player1_k_factor * (p1_score - expected_p1);
        let p2_change = player2_k_factor * (p2_score - expected_p2);

        let p1_after = current_p1_elo + p1_change;
        let p2_after = current_p2_elo + p2_change;

        changes.push(MatchEloChange {
            game_id,
            player1_id,
            player2_id,
            player1_elo_before: current_p1_elo,
            player1_elo_after: p1_after,
            player1_elo_change: p1_change,
            player2_elo_before: current_p2_elo,
            player2_elo_after: p2_after,
            player2_elo_change: p2_change,
        });

        // Update current ELOs for next game
        current_p1_elo = p1_after;
        current_p2_elo = p2_after;
    }

    changes
}
