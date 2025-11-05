use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

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
    created_by: Uuid,
) -> Result<Season, Box<dyn std::error::Error + Send + Sync>> {
    let mut tx = pool.begin().await?;

    // Deactivate all existing seasons
    sqlx::query("UPDATE seasons SET is_active = false")
        .execute(&mut *tx)
        .await?;

    // Create and activate the new season
    let season = sqlx::query_as::<_, Season>(
        "INSERT INTO seasons
         (name, description, start_date, starting_elo, k_factor,
          base_k_factor, new_player_k_bonus, new_player_bonus_period, created_by, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true)
         RETURNING *",
    )
    .bind(name)
    .bind(description)
    .bind(start_date)
    .bind(starting_elo)
    .bind(k_factor)
    .bind(base_k_factor)
    .bind(new_player_k_bonus)
    .bind(new_player_bonus_period)
    .bind(created_by)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Initialize all active players for this season with starting ELO
    let player_count = initialize_season_players(pool, season.id).await?;
    tracing::info!(
        "Initialized {} players for season '{}'",
        player_count,
        season.name
    );

    // Check if this is the latest season (most common case)
    let latest_existing = sqlx::query_as::<_, Season>(
        "SELECT * FROM seasons WHERE id != $1 ORDER BY start_date DESC LIMIT 1",
    )
    .bind(season.id)
    .fetch_optional(pool)
    .await?;

    let is_latest = latest_existing
        .map(|s| season.start_date > s.start_date)
        .unwrap_or(true);

    if !is_latest {
        // Historical insertion: reassign games and recalculate affected seasons
        tracing::info!("Historical season insertion detected, reassigning games");
        reassign_games_to_seasons(pool).await?;
        recalculate_seasons_from(pool, start_date).await?;
    } else {
        tracing::info!("Latest season created, no reassignment needed");
    }

    Ok(season)
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
fn calculate_dynamic_k_factor(season: &Season, games_played: i32) -> f64 {
    if let (Some(base_k), Some(bonus), Some(period)) = (
        season.base_k_factor,
        season.new_player_k_bonus,
        season.new_player_bonus_period,
    ) && period > 0
    {
        let decay = (-games_played as f64 / period as f64).exp();
        return base_k + (bonus * decay);
    }
    season.k_factor
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
    let winner_k = calculate_dynamic_k_factor(&season, winner_stats.games_played);
    let loser_k = calculate_dynamic_k_factor(&season, loser_stats.games_played);

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
    .bind(&season.name) // Use season name as version
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
    .bind(&season.name)
    .bind(season_id)
    .bind(played_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Recalculate all ELO for a specific season
pub async fn recalculate_season_elo(
    pool: &PgPool,
    season_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let season = get_season_by_id(pool, season_id)
        .await?
        .ok_or("Season not found")?;

    tracing::info!("Recalculating ELO for season: {}", season.name);

    // Get all games for this season in chronological order
    let games: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, player1_id, player2_id, played_at
         FROM games
         WHERE season_id = $1
         ORDER BY played_at ASC",
    )
    .bind(season_id)
    .fetch_all(pool)
    .await?;

    if games.is_empty() {
        tracing::info!("No games found for season {}", season.name);
        return Ok(());
    }

    tracing::info!("Found {} games to recalculate", games.len());

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
        player_elos.insert(player_id, season.starting_elo);
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

    // Process each game
    for (game_id, winner_id, loser_id, played_at) in games {
        let winner_elo_before = player_elos
            .get(&winner_id)
            .copied()
            .unwrap_or(season.starting_elo);
        let loser_elo_before = player_elos
            .get(&loser_id)
            .copied()
            .unwrap_or(season.starting_elo);

        let winner_games = player_games_played.get(&winner_id).copied().unwrap_or(0);
        let loser_games = player_games_played.get(&loser_id).copied().unwrap_or(0);

        // Calculate dynamic K-factors
        let winner_k = calculate_dynamic_k_factor(&season, winner_games);
        let loser_k = calculate_dynamic_k_factor(&season, loser_games);

        // Calculate ELO changes
        let expected_winner =
            1.0 / (1.0 + 10_f64.powf((loser_elo_before - winner_elo_before) / 400.0));
        let expected_loser = 1.0 - expected_winner;

        let winner_change = winner_k * (1.0 - expected_winner);
        let loser_change = loser_k * (0.0 - expected_loser);

        let winner_elo_after = winner_elo_before + winner_change;
        let loser_elo_after = loser_elo_before + loser_change;

        // Update in-memory stats
        player_elos.insert(winner_id, winner_elo_after);
        player_elos.insert(loser_id, loser_elo_after);
        *player_games_played.entry(winner_id).or_insert(0) += 1;
        *player_games_played.entry(loser_id).or_insert(0) += 1;
        *player_wins.entry(winner_id).or_insert(0) += 1;
        *player_losses.entry(loser_id).or_insert(0) += 1;

        // Record ELO history
        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(winner_id)
        .bind(game_id)
        .bind(winner_elo_before)
        .bind(winner_elo_after)
        .bind(&season.name)
        .bind(season_id)
        .bind(played_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(loser_id)
        .bind(game_id)
        .bind(loser_elo_before)
        .bind(loser_elo_after)
        .bind(&season.name)
        .bind(season_id)
        .bind(played_at)
        .execute(&mut *tx)
        .await?;
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

/// Reassign all games to their correct seasons based on played_at timestamp
/// Games are assigned to the season with the latest start_date that is <= game.played_at
/// Uses efficient SQL-based approach for O(n log n) complexity
pub async fn reassign_games_to_seasons(
    pool: &PgPool,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Reassigning all games to correct seasons");

    // Use SQL to efficiently reassign all games in one query
    // For each game, find the season with the latest start_date <= game.played_at
    let result = sqlx::query(
        "UPDATE games
         SET season_id = (
             SELECT s.id
             FROM seasons s
             WHERE s.start_date <= games.played_at
             ORDER BY s.start_date DESC
             LIMIT 1
         )
         WHERE season_id IS NULL
            OR season_id != (
                SELECT s.id
                FROM seasons s
                WHERE s.start_date <= games.played_at
                ORDER BY s.start_date DESC
                LIMIT 1
            )",
    )
    .execute(pool)
    .await?;

    let reassigned_count = result.rows_affected() as i32;

    tracing::info!(
        "Reassigned {} games to their correct seasons",
        reassigned_count
    );
    Ok(reassigned_count)
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

    // Start transaction
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

    tracing::info!("Deleted season: {}", season.name);

    // Reassign all games to their correct seasons
    reassign_games_to_seasons(pool).await?;

    // Find the earliest affected season (the one before the deleted season)
    // Games from the deleted season get reassigned to it, so it needs recalculation
    let earliest_affected = sqlx::query_as::<_, Season>(
        "SELECT * FROM seasons WHERE start_date < $1 ORDER BY start_date DESC LIMIT 1",
    )
    .bind(season.start_date)
    .fetch_optional(pool)
    .await?;

    let recalc_from = earliest_affected
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
