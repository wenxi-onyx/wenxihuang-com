use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

// Type alias for ELO configuration tuple: (k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period, starting_elo)
type EloConfigTuple = (f64, Option<f64>, Option<f64>, Option<i32>, f64);

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Season {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: DateTime<Utc>,
    pub starting_elo: f64,
    pub k_factor: f64,
    pub base_k_factor: Option<f64>,
    pub new_player_k_bonus: Option<f64>,
    pub new_player_bonus_period: Option<i32>,
    pub elo_version: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct PlayerSeasonStats {
    pub id: Uuid,
    pub player_id: Uuid,
    pub season_id: Uuid,
    pub current_elo: f64,
    pub games_played: i32,
    pub wins: i32,
    pub losses: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Get the currently active season
pub async fn get_active_season(pool: &PgPool) -> Result<Option<Season>, sqlx::Error> {
    sqlx::query_as::<_, Season>("SELECT * FROM seasons WHERE is_active = true LIMIT 1")
        .fetch_optional(pool)
        .await
}

/// Get a season by ID
pub async fn get_season_by_id(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<Option<Season>, sqlx::Error> {
    sqlx::query_as::<_, Season>("SELECT * FROM seasons WHERE id = $1")
        .bind(season_id)
        .fetch_optional(pool)
        .await
}

/// Get a season by name
pub async fn get_season_by_name(pool: &PgPool, name: &str) -> Result<Option<Season>, sqlx::Error> {
    sqlx::query_as::<_, Season>("SELECT * FROM seasons WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
}

/// Get all seasons ordered by start date (newest first)
pub async fn get_all_seasons(pool: &PgPool) -> Result<Vec<Season>, sqlx::Error> {
    sqlx::query_as::<_, Season>("SELECT * FROM seasons ORDER BY start_date DESC")
        .fetch_all(pool)
        .await
}

/// Create a new season and automatically activate it
/// This deactivates all previous seasons
/// If the season is inserted at a historical point, games will be reassigned and affected seasons recalculated
/// If player_ids is provided, only those players will be included; otherwise all active players will be included
#[allow(clippy::too_many_arguments)]
pub async fn create_season(
    pool: &PgPool,
    name: String,
    description: Option<String>,
    start_date: DateTime<Utc>,
    starting_elo: f64,
    k_factor: f64,
    base_k_factor: Option<f64>,
    new_player_k_bonus: Option<f64>,
    new_player_bonus_period: Option<i32>,
    elo_version: Option<String>,
    created_by: Uuid,
    player_ids: Option<Vec<Uuid>>,
) -> Result<Season, Box<dyn std::error::Error + Send + Sync>> {
    // Validate inputs before starting transaction
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Season name cannot be empty".into());
    }
    if trimmed_name.len() > 255 {
        return Err("Season name cannot exceed 255 characters".into());
    }

    if starting_elo < 0.0 {
        return Err("Starting ELO cannot be negative".into());
    }
    if starting_elo > 5000.0 {
        return Err("Starting ELO cannot exceed 5000".into());
    }

    if k_factor <= 0.0 {
        return Err("K-factor must be positive".into());
    }
    if k_factor > 100.0 {
        return Err("K-factor cannot exceed 100 (unreasonably high)".into());
    }

    if let Some(base_k) = base_k_factor {
        if base_k <= 0.0 {
            return Err("Base K-factor must be positive".into());
        }
        if base_k > 100.0 {
            return Err("Base K-factor cannot exceed 100".into());
        }
    }

    if let Some(bonus) = new_player_k_bonus {
        if bonus < 0.0 {
            return Err("New player K-bonus cannot be negative".into());
        }
        if bonus > 100.0 {
            return Err("New player K-bonus cannot exceed 100".into());
        }
    }

    if let Some(period) = new_player_bonus_period {
        if period < 0 {
            return Err("New player bonus period cannot be negative".into());
        }
        if period > 1000 {
            return Err("New player bonus period cannot exceed 1000 games".into());
        }
    }

    let mut tx = pool.begin().await?;

    // Deactivate all existing seasons
    sqlx::query("UPDATE seasons SET is_active = false")
        .execute(&mut *tx)
        .await?;

    // Create and activate the new season
    let season = sqlx::query_as::<_, Season>(
        "INSERT INTO seasons
         (name, description, start_date, starting_elo, k_factor,
          base_k_factor, new_player_k_bonus, new_player_bonus_period, elo_version, created_by, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true)
         RETURNING *",
    )
    .bind(trimmed_name)
    .bind(description)
    .bind(start_date)
    .bind(starting_elo)
    .bind(k_factor)
    .bind(base_k_factor)
    .bind(new_player_k_bonus)
    .bind(new_player_bonus_period)
    .bind(elo_version)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Initialize players for this season with starting ELO
    // If this fails, we need to clean up the season we just created
    let player_count = match if let Some(player_ids) = player_ids {
        // Initialize only specified players
        initialize_season_with_players(pool, season.id, &player_ids).await
    } else {
        // Initialize all active players
        initialize_season_players(pool, season.id).await
    } {
        Ok(count) => count,
        Err(e) => {
            tracing::error!(
                "Failed to initialize players for season '{}': {}",
                season.name,
                e
            );
            // Clean up: delete the season we just created
            if let Err(cleanup_err) = sqlx::query("DELETE FROM seasons WHERE id = $1")
                .bind(season.id)
                .execute(pool)
                .await
            {
                tracing::error!(
                    "Failed to clean up season after initialization error: {}",
                    cleanup_err
                );
            }
            return Err(format!("Failed to initialize players: {}", e).into());
        }
    };

    tracing::info!(
        "Initialized {} players for season '{}'",
        player_count,
        season.name
    );

    // Always reassign games to ensure they're in the correct season
    // This handles both historical insertions and cases where games were imported before season creation
    tracing::info!("Reassigning games to correct seasons");
    if let Err(e) = reassign_games_to_seasons(pool).await {
        tracing::error!(
            "Failed to reassign games for season '{}': {}",
            season.name,
            e
        );
        // Clean up: delete the season and player_seasons we just created
        if let Err(cleanup_err) = cleanup_season(pool, season.id).await {
            tracing::error!(
                "Failed to clean up season after reassignment error: {}",
                cleanup_err
            );
        }
        return Err(format!("Failed to reassign games: {}", e).into());
    }

    // Recalculate this season and any seasons that come after it
    tracing::info!(
        "Recalculating season '{}' and subsequent seasons",
        season.name
    );
    if let Err(e) = recalculate_seasons_from(pool, start_date).await {
        tracing::error!(
            "Failed to recalculate ELO for season '{}': {}",
            season.name,
            e
        );
        // Clean up: delete the season and related data we just created
        if let Err(cleanup_err) = cleanup_season(pool, season.id).await {
            tracing::error!(
                "Failed to clean up season after recalculation error: {}",
                cleanup_err
            );
        }
        return Err(format!("Failed to recalculate ELO: {}", e).into());
    }

    Ok(season)
}

/// Clean up a season by deleting it and all its associated data
/// Used for rollback when season creation partially fails
async fn cleanup_season(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tx = pool.begin().await?;

    // Delete player_seasons entries
    sqlx::query("DELETE FROM player_seasons WHERE season_id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Delete elo_history entries
    sqlx::query("DELETE FROM elo_history WHERE season_id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Delete the season itself
    sqlx::query("DELETE FROM seasons WHERE id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Activate a season (deactivates all others)
/// Note: Normally not needed as create_season auto-activates, but useful for switching back to old seasons
pub async fn activate_season(pool: &PgPool, season_id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Deactivate all seasons
    sqlx::query("UPDATE seasons SET is_active = false")
        .execute(&mut *tx)
        .await?;

    // Activate the specified season
    sqlx::query("UPDATE seasons SET is_active = true WHERE id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Update a season's ELO version
pub async fn update_season_elo_version(
    pool: &PgPool,
    season_id: Uuid,
    elo_version: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE seasons SET elo_version = $1 WHERE id = $2")
        .bind(elo_version)
        .bind(season_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Initialize player stats for a new season with specific player IDs
pub async fn initialize_season_with_players(
    pool: &PgPool,
    season_id: Uuid,
    player_ids: &[Uuid],
) -> Result<i32, sqlx::Error> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    let mut count = 0;
    for player_id in player_ids {
        // Check if player exists and is active
        let player_exists: Option<(bool,)> =
            sqlx::query_as("SELECT is_active FROM players WHERE id = $1")
                .bind(player_id)
                .fetch_optional(pool)
                .await?;

        if player_exists.is_none() {
            tracing::warn!("Skipping player {} - player not found", player_id);
            continue;
        }

        // Check if player_season already exists
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM player_seasons WHERE player_id = $1 AND season_id = $2")
                .bind(player_id)
                .bind(season_id)
                .fetch_optional(pool)
                .await?;

        if exists.is_none() {
            sqlx::query(
                "INSERT INTO player_seasons (player_id, season_id, current_elo, games_played, wins, losses)
                 VALUES ($1, $2, $3, 0, 0, 0)"
            )
            .bind(player_id)
            .bind(season_id)
            .bind(season.starting_elo)
            .execute(pool)
            .await?;
            count += 1;
        }
    }

    Ok(count)
}

/// Initialize player stats for a new season (creates player_seasons entries for all active players)
pub async fn initialize_season_players(pool: &PgPool, season_id: Uuid) -> Result<i32, sqlx::Error> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    // Get all active players
    let players: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM players WHERE is_active = true")
        .fetch_all(pool)
        .await?;

    let mut count = 0;
    for (player_id,) in players {
        // Check if player_season already exists
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM player_seasons WHERE player_id = $1 AND season_id = $2")
                .bind(player_id)
                .bind(season_id)
                .fetch_optional(pool)
                .await?;

        if exists.is_none() {
            sqlx::query(
                "INSERT INTO player_seasons (player_id, season_id, current_elo, games_played, wins, losses)
                 VALUES ($1, $2, $3, 0, 0, 0)"
            )
            .bind(player_id)
            .bind(season_id)
            .bind(season.starting_elo)
            .execute(pool)
            .await?;
            count += 1;
        }
    }

    Ok(count)
}

/// Get player stats for a specific season
#[allow(dead_code)]
pub async fn get_player_season_stats(
    pool: &PgPool,
    player_id: Uuid,
    season_id: Uuid,
) -> Result<Option<PlayerSeasonStats>, sqlx::Error> {
    sqlx::query_as::<_, PlayerSeasonStats>(
        "SELECT * FROM player_seasons WHERE player_id = $1 AND season_id = $2",
    )
    .bind(player_id)
    .bind(season_id)
    .fetch_optional(pool)
    .await
}

/// Get all players' stats for a specific season, ordered by ELO
/// Only returns players who are included in the season
pub async fn get_season_leaderboard(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<Vec<(Uuid, String, String, f64, i32, i32, i32, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.id, p.first_name, p.last_name, ps.current_elo, ps.games_played, ps.wins, ps.losses, p.is_active
         FROM player_seasons ps
         JOIN players p ON ps.player_id = p.id
         WHERE ps.season_id = $1 AND ps.is_included = true
         ORDER BY ps.current_elo DESC"
    )
    .bind(season_id)
    .fetch_all(pool)
    .await
}

/// Get all players in a season with their inclusion status
pub async fn get_season_players(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<Vec<(Uuid, String, String, bool, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.id, p.first_name, p.last_name, ps.is_included, p.is_active
         FROM player_seasons ps
         JOIN players p ON ps.player_id = p.id
         WHERE ps.season_id = $1
         ORDER BY p.first_name, p.last_name",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await
}

/// Add a player to a season (or update their inclusion status)
pub async fn add_player_to_season(
    pool: &PgPool,
    player_id: Uuid,
    season_id: Uuid,
) -> Result<(), sqlx::Error> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    // Check if player_season already exists
    let exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM player_seasons WHERE player_id = $1 AND season_id = $2")
            .bind(player_id)
            .bind(season_id)
            .fetch_optional(pool)
            .await?;

    if exists.is_some() {
        // Update existing record to set is_included = true
        sqlx::query(
            "UPDATE player_seasons SET is_included = true WHERE player_id = $1 AND season_id = $2",
        )
        .bind(player_id)
        .bind(season_id)
        .execute(pool)
        .await?;
    } else {
        // Insert new record
        sqlx::query(
            "INSERT INTO player_seasons (player_id, season_id, current_elo, games_played, wins, losses, is_included)
             VALUES ($1, $2, $3, 0, 0, 0, true)"
        )
        .bind(player_id)
        .bind(season_id)
        .bind(season.starting_elo)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Remove a player from a season (sets is_included = false)
pub async fn remove_player_from_season(
    pool: &PgPool,
    player_id: Uuid,
    season_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE player_seasons SET is_included = false WHERE player_id = $1 AND season_id = $2",
    )
    .bind(player_id)
    .bind(season_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all players not yet in a season (potential additions)
pub async fn get_available_players_for_season(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<Vec<(Uuid, String, String, bool)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT p.id, p.first_name, p.last_name, p.is_active
         FROM players p
         WHERE NOT EXISTS (
             SELECT 1 FROM player_seasons ps
             WHERE ps.player_id = p.id AND ps.season_id = $1
         )
         ORDER BY p.first_name, p.last_name",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await
}

/// Calculate dynamic K-factor based on player experience within a season
/// If base_k_factor is provided, uses dynamic calculation, otherwise returns static k_factor
fn calculate_dynamic_k_factor(
    k_factor: f64,
    base_k_factor: Option<f64>,
    new_player_k_bonus: Option<f64>,
    new_player_bonus_period: Option<i32>,
    games_played: i32,
) -> f64 {
    if let (Some(base_k), Some(bonus), Some(period)) =
        (base_k_factor, new_player_k_bonus, new_player_bonus_period)
        && period > 0
    {
        let decay = (-games_played as f64 / period as f64).exp();
        return base_k + (bonus * decay);
    }
    k_factor
}

/// Record a game result and update player_seasons stats
#[allow(dead_code)]
pub async fn record_game_result(
    pool: &PgPool,
    game_id: Uuid,
    season_id: Uuid,
    winner_id: Uuid,
    loser_id: Uuid,
    played_at: DateTime<Utc>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or("Season not found")?;

    // Determine elo_version string (max 50 chars for VARCHAR(50))
    let elo_version_string = season.elo_version.as_deref().unwrap_or_else(|| {
        // Use truncated season name if no elo_version
        if season.name.len() <= 50 {
            &season.name
        } else {
            &season.name[..50]
        }
    });

    // Get or create player_season stats
    let winner_stats = get_player_season_stats(pool, winner_id, season_id)
        .await?
        .ok_or("Winner not found in season")?;
    let loser_stats = get_player_season_stats(pool, loser_id, season_id)
        .await?
        .ok_or("Loser not found in season")?;

    let winner_elo_before = winner_stats.current_elo;
    let loser_elo_before = loser_stats.current_elo;

    // Calculate dynamic K-factors
    let winner_k = calculate_dynamic_k_factor(
        season.k_factor,
        season.base_k_factor,
        season.new_player_k_bonus,
        season.new_player_bonus_period,
        winner_stats.games_played,
    );
    let loser_k = calculate_dynamic_k_factor(
        season.k_factor,
        season.base_k_factor,
        season.new_player_k_bonus,
        season.new_player_bonus_period,
        loser_stats.games_played,
    );

    // Calculate ELO changes
    let expected_winner = 1.0 / (1.0 + 10_f64.powf((loser_elo_before - winner_elo_before) / 400.0));
    let expected_loser = 1.0 - expected_winner;

    let winner_change = winner_k * (1.0 - expected_winner);
    let loser_change = loser_k * (0.0 - expected_loser);

    let winner_elo_after = winner_elo_before + winner_change;
    let loser_elo_after = loser_elo_before + loser_change;

    // Start transaction
    let mut tx = pool.begin().await?;

    // Update winner stats
    sqlx::query(
        "UPDATE player_seasons
         SET current_elo = $1, games_played = games_played + 1, wins = wins + 1
         WHERE player_id = $2 AND season_id = $3",
    )
    .bind(winner_elo_after)
    .bind(winner_id)
    .bind(season_id)
    .execute(&mut *tx)
    .await?;

    // Update loser stats
    sqlx::query(
        "UPDATE player_seasons
         SET current_elo = $1, games_played = games_played + 1, losses = losses + 1
         WHERE player_id = $2 AND season_id = $3",
    )
    .bind(loser_elo_after)
    .bind(loser_id)
    .bind(season_id)
    .execute(&mut *tx)
    .await?;

    // Record ELO history for winner
    sqlx::query(
        "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(winner_id)
    .bind(game_id)
    .bind(winner_elo_before)
    .bind(winner_elo_after)
    .bind(elo_version_string)
    .bind(season_id)
    .bind(played_at)
    .execute(&mut *tx)
    .await?;

    // Record ELO history for loser
    sqlx::query(
        "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(loser_id)
    .bind(game_id)
    .bind(loser_elo_before)
    .bind(loser_elo_after)
    .bind(elo_version_string)
    .bind(season_id)
    .bind(played_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Recalculate all ELO for a specific season
/// Processes games grouped by match to maintain sequential ELO calculation within each match
pub async fn recalculate_season_elo(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or("Season not found")?;

    tracing::info!("Recalculating ELO for season: {}", season.name);

    // Determine elo_version string for recording (max 50 chars for VARCHAR(50))
    let elo_version_string = season.elo_version.as_deref().unwrap_or_else(|| {
        // Use truncated season name if no elo_version
        if season.name.len() <= 50 {
            &season.name
        } else {
            &season.name[..50]
        }
    });

    // Determine which ELO configuration to use
    let (k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period, starting_elo) =
        if let Some(ref elo_version) = season.elo_version {
            // Use ELO configuration from elo_configurations table
            tracing::info!("Using ELO configuration: {}", elo_version);
            let config: Option<EloConfigTuple> = sqlx::query_as(
                "SELECT k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period, starting_elo
                 FROM elo_configurations
                 WHERE version_name = $1"
            )
            .bind(elo_version)
            .fetch_optional(pool)
            .await?;

            match config {
                Some((k, base_k, bonus, period, start_elo)) => {
                    tracing::info!(
                        "Loaded ELO config: k_factor={}, base_k={:?}, bonus={:?}, period={:?}, starting_elo={}",
                        k,
                        base_k,
                        bonus,
                        period,
                        start_elo
                    );
                    (k, base_k, bonus, period, start_elo)
                }
                None => {
                    tracing::warn!(
                        "ELO configuration '{}' not found, falling back to season's own values",
                        elo_version
                    );
                    (
                        season.k_factor,
                        season.base_k_factor,
                        season.new_player_k_bonus,
                        season.new_player_bonus_period,
                        season.starting_elo,
                    )
                }
            }
        } else {
            // Use season's own ELO configuration
            tracing::info!("Using season's own ELO configuration");
            (
                season.k_factor,
                season.base_k_factor,
                season.new_player_k_bonus,
                season.new_player_bonus_period,
                season.starting_elo,
            )
        };

    // Get all matches for this season in chronological order
    let matches: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, player1_id, player2_id, submitted_at
         FROM matches
         WHERE season_id = $1
         ORDER BY submitted_at ASC",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await?;

    if matches.is_empty() {
        tracing::info!("No matches found for season {}", season.name);
        return Ok(());
    }

    tracing::info!("Found {} matches to recalculate", matches.len());

    // Initialize ELO and stats for all players in this season
    let mut player_elos: HashMap<Uuid, f64> = HashMap::new();
    let mut player_games_played: HashMap<Uuid, i32> = HashMap::new();
    let mut player_wins: HashMap<Uuid, i32> = HashMap::new();
    let mut player_losses: HashMap<Uuid, i32> = HashMap::new();

    // Get all players in this season
    let player_seasons: Vec<(Uuid,)> =
        sqlx::query_as("SELECT player_id FROM player_seasons WHERE season_id = $1")
            .bind(season_id)
            .fetch_all(pool)
            .await?;

    for (player_id,) in player_seasons {
        player_elos.insert(player_id, starting_elo);
        player_games_played.insert(player_id, 0);
        player_wins.insert(player_id, 0);
        player_losses.insert(player_id, 0);
    }

    // Start transaction
    let mut tx = pool.begin().await?;

    // Delete old ELO history for this season
    sqlx::query("DELETE FROM elo_history WHERE season_id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Process each match
    for (match_id, match_player1_id, match_player2_id, _submitted_at) in matches {
        // Get all games for this match in chronological order
        // Note: player1_id is ALWAYS the winner, player2_id is ALWAYS the loser
        let games: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, player1_id, player2_id, played_at
             FROM games
             WHERE match_id = $1
             ORDER BY played_at ASC",
        )
        .bind(match_id)
        .fetch_all(&mut *tx)
        .await?;

        if games.is_empty() {
            tracing::warn!("Match {} has no games, skipping", match_id);
            continue;
        }

        // Validate that both players are in this season
        let player1_elo_before = match player_elos.get(&match_player1_id) {
            Some(&elo) => elo,
            None => {
                tracing::warn!(
                    "Skipping match {}: player {} is not in season {}",
                    match_id,
                    match_player1_id,
                    season.name
                );
                continue;
            }
        };

        let player2_elo_before = match player_elos.get(&match_player2_id) {
            Some(&elo) => elo,
            None => {
                tracing::warn!(
                    "Skipping match {}: player {} is not in season {}",
                    match_id,
                    match_player2_id,
                    season.name
                );
                continue;
            }
        };

        // Get games played count for K-factor calculation
        let player1_games = player_games_played
            .get(&match_player1_id)
            .copied()
            .unwrap_or(0);
        let player2_games = player_games_played
            .get(&match_player2_id)
            .copied()
            .unwrap_or(0);

        // Calculate dynamic K-factors using the determined configuration
        let player1_k = calculate_dynamic_k_factor(
            k_factor,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
            player1_games,
        );
        let player2_k = calculate_dynamic_k_factor(
            k_factor,
            base_k_factor,
            new_player_k_bonus,
            new_player_bonus_period,
            player2_games,
        );

        // Build games vector for ELO calculation
        // For each game, determine which player won
        let game_winners: Vec<(Uuid, crate::services::elo::GameWinner)> = games
            .iter()
            .map(|(game_id, winner_id, _loser_id, _played_at)| {
                let winner = if winner_id == &match_player1_id {
                    crate::services::elo::GameWinner::Player1
                } else {
                    crate::services::elo::GameWinner::Player2
                };
                (*game_id, winner)
            })
            .collect();

        // Calculate sequential ELO changes for all games in this match
        let elo_changes = crate::services::elo::calculate_match_elo_changes(
            match_player1_id,
            match_player2_id,
            player1_elo_before,
            player2_elo_before,
            game_winners,
            player1_k,
            player2_k,
        );

        // Record ELO history and update stats for each game
        for (i, change) in elo_changes.iter().enumerate() {
            let played_at = games[i].3;

            // Record ELO history for player 1
            sqlx::query(
                "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(change.player1_id)
            .bind(change.game_id)
            .bind(change.player1_elo_before)
            .bind(change.player1_elo_after)
            .bind(elo_version_string)
            .bind(season_id)
            .bind(played_at)
            .execute(&mut *tx)
            .await?;

            // Record ELO history for player 2
            sqlx::query(
                "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(change.player2_id)
            .bind(change.game_id)
            .bind(change.player2_elo_before)
            .bind(change.player2_elo_after)
            .bind(elo_version_string)
            .bind(season_id)
            .bind(played_at)
            .execute(&mut *tx)
            .await?;

            // Update stats for this game
            // Determine who won this specific game
            let (winner_id, loser_id) = if games[i].1 == match_player1_id {
                (match_player1_id, match_player2_id)
            } else {
                (match_player2_id, match_player1_id)
            };

            *player_games_played.entry(winner_id).or_insert(0) += 1;
            *player_games_played.entry(loser_id).or_insert(0) += 1;
            *player_wins.entry(winner_id).or_insert(0) += 1;
            *player_losses.entry(loser_id).or_insert(0) += 1;
        }

        // Update in-memory ELOs with final values after processing all games in match
        if let Some(last_change) = elo_changes.last() {
            player_elos.insert(match_player1_id, last_change.player1_elo_after);
            player_elos.insert(match_player2_id, last_change.player2_elo_after);
        }
    }

    // Update player_seasons with final stats
    for (player_id, elo) in player_elos {
        let games = player_games_played.get(&player_id).copied().unwrap_or(0);
        let wins = player_wins.get(&player_id).copied().unwrap_or(0);
        let losses = player_losses.get(&player_id).copied().unwrap_or(0);

        sqlx::query(
            "UPDATE player_seasons
             SET current_elo = $1, games_played = $2, wins = $3, losses = $4
             WHERE player_id = $5 AND season_id = $6",
        )
        .bind(elo)
        .bind(games)
        .bind(wins)
        .bind(losses)
        .bind(player_id)
        .bind(season_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!("Successfully recalculated ELO for season {}", season.name);
    Ok(())
}

/// Reassign all matches and games to their correct seasons based on timestamps
/// Matches are assigned based on submitted_at, games inherit from their match
/// Uses efficient SQL-based approach for O(n log n) complexity
/// Records without a matching season are logged but not modified
pub async fn reassign_games_to_seasons(
    pool: &PgPool,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Reassigning all matches and games to correct seasons");

    // First, check for matches that have no matching season
    let orphaned_matches: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, submitted_at
         FROM matches
         WHERE NOT EXISTS (
             SELECT 1 FROM seasons s WHERE s.start_date <= matches.submitted_at
         )",
    )
    .fetch_all(pool)
    .await?;

    if !orphaned_matches.is_empty() {
        tracing::warn!(
            "Found {} matches with no matching season (submitted before earliest season):",
            orphaned_matches.len()
        );
        for (match_id, submitted_at) in &orphaned_matches {
            tracing::warn!("  Match {} submitted at {}", match_id, submitted_at);
        }
    }

    // Use SQL to efficiently reassign all matches in one query
    // For each match, find the season with the latest start_date <= match.submitted_at
    // Only update matches where a matching season exists (subquery returns non-NULL)
    let matches_result = sqlx::query(
        "UPDATE matches
         SET season_id = (
             SELECT s.id
             FROM seasons s
             WHERE s.start_date <= matches.submitted_at
             ORDER BY s.start_date DESC
             LIMIT 1
         )
         WHERE EXISTS (
             SELECT 1 FROM seasons s WHERE s.start_date <= matches.submitted_at
         )
         AND (
             season_id IS NULL
             OR season_id != (
                 SELECT s.id
                 FROM seasons s
                 WHERE s.start_date <= matches.submitted_at
                 ORDER BY s.start_date DESC
                 LIMIT 1
             )
         )",
    )
    .execute(pool)
    .await?;

    let matches_reassigned = matches_result.rows_affected() as i32;

    // Now update games to inherit the season_id from their parent match
    let games_result = sqlx::query(
        "UPDATE games
         SET season_id = m.season_id
         FROM matches m
         WHERE games.match_id = m.id
         AND (games.season_id IS NULL OR games.season_id != m.season_id)",
    )
    .execute(pool)
    .await?;

    let games_reassigned = games_result.rows_affected() as i32;

    tracing::info!(
        "Reassigned {} matches and {} games to their correct seasons",
        matches_reassigned,
        games_reassigned
    );
    Ok(matches_reassigned + games_reassigned)
}

/// Delete a season and all associated data
/// Reassigns games to other seasons and triggers recalculation of affected seasons
pub async fn delete_season(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or("Season not found")?;

    tracing::info!("Deleting season: {}", season.name);

    // Find the target season for reassignment BEFORE starting transaction
    let target_season = sqlx::query_as::<_, Season>(
        "SELECT * FROM seasons WHERE start_date < $1 ORDER BY start_date DESC LIMIT 1",
    )
    .bind(season.start_date)
    .fetch_optional(pool)
    .await?;

    // Start transaction - all modifications must be within this transaction
    let mut tx = pool.begin().await?;

    // Step 1: Reassign matches and games to the previous season (if exists)
    // This MUST happen before deleting the season to prevent orphaned records
    if let Some(target) = &target_season {
        tracing::info!(
            "Reassigning matches from season '{}' to '{}'",
            season.name,
            target.name
        );

        // Reassign matches
        let matches_updated = sqlx::query("UPDATE matches SET season_id = $1 WHERE season_id = $2")
            .bind(target.id)
            .bind(season_id)
            .execute(&mut *tx)
            .await?;

        tracing::info!("Reassigned {} matches", matches_updated.rows_affected());

        // Reassign games
        let games_updated = sqlx::query("UPDATE games SET season_id = $1 WHERE season_id = $2")
            .bind(target.id)
            .bind(season_id)
            .execute(&mut *tx)
            .await?;

        tracing::info!("Reassigned {} games", games_updated.rows_affected());
    } else {
        // No previous season exists - check if there are any matches
        let match_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM matches WHERE season_id = $1")
                .bind(season_id)
                .fetch_one(&mut *tx)
                .await?;

        if match_count.0 > 0 {
            return Err(format!(
                "Cannot delete season '{}': no previous season exists to reassign {} matches to",
                season.name, match_count.0
            )
            .into());
        }
    }

    // Step 2: Delete elo_history entries (will be recalculated after)
    sqlx::query("DELETE FROM elo_history WHERE season_id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Step 3: Delete player_seasons entries
    sqlx::query("DELETE FROM player_seasons WHERE season_id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Step 4: Delete the season itself (now safe since matches are reassigned)
    sqlx::query("DELETE FROM seasons WHERE id = $1")
        .bind(season_id)
        .execute(&mut *tx)
        .await?;

    // Commit all changes atomically
    tx.commit().await?;

    tracing::info!("Successfully deleted season: {}", season.name);

    // Step 5: Recalculate ELO for affected seasons (outside transaction is safe)
    let recalc_from = target_season
        .map(|s| s.start_date)
        .unwrap_or(season.start_date);

    tracing::info!("Recalculating all seasons from: {}", recalc_from);
    recalculate_seasons_from(pool, recalc_from).await?;

    Ok(())
}

/// Recalculate ELO for all seasons starting from a specific date
/// This is used after deleting or inserting a season to recalculate all affected seasons
pub async fn recalculate_seasons_from(
    pool: &PgPool,
    from_date: DateTime<Utc>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Recalculating seasons from date: {}", from_date);

    // Get all seasons starting from the specified date, ordered by start_date
    let seasons: Vec<Season> =
        sqlx::query_as("SELECT * FROM seasons WHERE start_date >= $1 ORDER BY start_date ASC")
            .bind(from_date)
            .fetch_all(pool)
            .await?;

    if seasons.is_empty() {
        tracing::info!("No seasons found to recalculate");
        return Ok(());
    }

    tracing::info!("Recalculating {} seasons", seasons.len());

    // Recalculate each season in order
    for season in seasons {
        tracing::info!("Recalculating season: {}", season.name);
        recalculate_season_elo(pool, season.id).await?;
    }

    tracing::info!("Successfully recalculated all affected seasons");
    Ok(())
}
