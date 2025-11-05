use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::{User, UserRole};
use crate::services::elo::{KFactorConfig, calculate_elo_change};
use crate::services::seasons;

/// Helper function to format player name, handling NULL values properly
fn format_player_name(first_name: String, last_name: String) -> String {
    let first = first_name.trim();
    let last = last_name.trim();

    if first.is_empty() && last.is_empty() {
        "Unknown Player".to_string()
    } else if first.is_empty() {
        last.to_string()
    } else if last.is_empty() {
        first.to_string()
    } else {
        format!("{} {}", first, last)
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub player1_score: i32,
    pub player2_score: i32,
    pub played_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub player1_score: i32,
    pub player2_score: i32,
    pub season_id: Uuid,
    pub played_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub message: String,
    pub game: GameResponse,
}

/// Create a new game (match)
/// Requires authentication (user or admin role)
pub async fn create_game(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateGameRequest>,
) -> Result<(StatusCode, Json<CreateGameResponse>), AuthError> {
    tracing::info!(
        "User {} creating game: {:?} vs {:?}",
        user.username,
        payload.player1_id,
        payload.player2_id
    );

    // Validate input
    if payload.player1_id == payload.player2_id {
        return Err(AuthError::InvalidInput(
            "Players must be different".to_string(),
        ));
    }

    if payload.player1_score < 0 || payload.player2_score < 0 {
        return Err(AuthError::InvalidInput(
            "Scores cannot be negative".to_string(),
        ));
    }

    if payload.player1_score == payload.player2_score {
        return Err(AuthError::InvalidInput("Game cannot be a tie".to_string()));
    }

    // Ensure player1 is always the winner by swapping if needed
    let (player1_id, player2_id, player1_score, player2_score) =
        if payload.player1_score > payload.player2_score {
            (
                payload.player1_id,
                payload.player2_id,
                payload.player1_score,
                payload.player2_score,
            )
        } else {
            // Swap so player1 is the winner
            (
                payload.player2_id,
                payload.player1_id,
                payload.player2_score,
                payload.player1_score,
            )
        };

    // Get the active season
    let active_season = sqlx::query!(
        r#"
        SELECT id, name, starting_elo, k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period
        FROM seasons
        WHERE is_active = true
        LIMIT 1
        "#
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching active season: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| {
        tracing::error!("No active season found");
        AuthError::InvalidInput("No active season found".to_string())
    })?;

    let season_id = active_season.id;

    // Verify both players exist and are active
    let player1 = sqlx::query!(
        r#"
        SELECT id, first_name, last_name, is_active
        FROM players
        WHERE id = $1
        "#,
        player1_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player1: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| AuthError::InvalidInput("Player 1 not found".to_string()))?;

    let player2 = sqlx::query!(
        r#"
        SELECT id, first_name, last_name, is_active
        FROM players
        WHERE id = $1
        "#,
        player2_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player2: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| AuthError::InvalidInput("Player 2 not found".to_string()))?;

    if !player1.is_active {
        return Err(AuthError::InvalidInput(format!(
            "Player {} {} is not active",
            player1.first_name, player1.last_name
        )));
    }

    if !player2.is_active {
        return Err(AuthError::InvalidInput(format!(
            "Player {} {} is not active",
            player2.first_name, player2.last_name
        )));
    }

    // Start a transaction early to prevent race conditions
    let mut tx = pool.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {}", e);
        AuthError::DatabaseError
    })?;

    // Verify both players are in the active season and lock rows to prevent concurrent updates
    // Using FOR UPDATE to lock the rows for this transaction
    let player1_season = sqlx::query!(
        r#"
        SELECT current_elo, games_played, is_included
        FROM player_seasons
        WHERE player_id = $1 AND season_id = $2
        FOR UPDATE
        "#,
        player1_id,
        season_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player1 season: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| {
        AuthError::InvalidInput(format!(
            "Player {} {} is not in the active season",
            player1.first_name, player1.last_name
        ))
    })?;

    let player2_season = sqlx::query!(
        r#"
        SELECT current_elo, games_played, is_included
        FROM player_seasons
        WHERE player_id = $1 AND season_id = $2
        FOR UPDATE
        "#,
        player2_id,
        season_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player2 season: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| {
        AuthError::InvalidInput(format!(
            "Player {} {} is not in the active season",
            player2.first_name, player2.last_name
        ))
    })?;

    if !player1_season.is_included {
        return Err(AuthError::InvalidInput(format!(
            "Player {} {} is not included in the active season",
            player1.first_name, player1.last_name
        )));
    }

    if !player2_season.is_included {
        return Err(AuthError::InvalidInput(format!(
            "Player {} {} is not included in the active season",
            player2.first_name, player2.last_name
        )));
    }

    // Determine winner (player1 is always the winner after the swap above)
    let player1_won = true;

    // Calculate ELO changes
    let k_config = KFactorConfig {
        k_factor: active_season.k_factor,
        base_k_factor: active_season.base_k_factor,
        new_player_k_bonus: active_season.new_player_k_bonus,
        new_player_bonus_period: active_season.new_player_bonus_period,
    };

    let (player1_elo_change, player2_elo_change) = calculate_elo_change(
        player1_season.current_elo,
        player2_season.current_elo,
        player1_won,
        &k_config,
        player1_season.games_played,
        player2_season.games_played,
    );

    let player1_new_elo = player1_season.current_elo + player1_elo_change;
    let player2_new_elo = player2_season.current_elo + player2_elo_change;

    // Transaction already started above with row-level locks

    // Insert the game
    let played_at = payload.played_at.unwrap_or_else(chrono::Utc::now);
    let game = sqlx::query!(
        r#"
        INSERT INTO games (player1_id, player2_id, player1_score, player2_score, season_id, played_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, player1_id, player2_id, player1_score, player2_score, season_id, played_at
        "#,
        player1_id,
        player2_id,
        player1_score,
        player2_score,
        season_id,
        played_at
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating game: {}", e);
        AuthError::DatabaseError
    })?;

    // Insert ELO history for player 1
    sqlx::query!(
        r#"
        INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        player1_id,
        game.id,
        player1_season.current_elo,
        player1_new_elo,
        active_season.name,
        season_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating elo_history for player1: {}", e);
        AuthError::DatabaseError
    })?;

    // Insert ELO history for player 2
    sqlx::query!(
        r#"
        INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        player2_id,
        game.id,
        player2_season.current_elo,
        player2_new_elo,
        active_season.name,
        season_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating elo_history for player2: {}", e);
        AuthError::DatabaseError
    })?;

    // Update player_seasons for player 1
    sqlx::query!(
        r#"
        UPDATE player_seasons
        SET current_elo = $1,
            games_played = games_played + 1,
            wins = wins + $2,
            losses = losses + $3
        WHERE player_id = $4 AND season_id = $5
        "#,
        player1_new_elo,
        if player1_won { 1 } else { 0 },
        if player1_won { 0 } else { 1 },
        player1_id,
        season_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating player1 season stats: {}", e);
        AuthError::DatabaseError
    })?;

    // Update player_seasons for player 2
    sqlx::query!(
        r#"
        UPDATE player_seasons
        SET current_elo = $1,
            games_played = games_played + 1,
            wins = wins + $2,
            losses = losses + $3
        WHERE player_id = $4 AND season_id = $5
        "#,
        player2_new_elo,
        if player1_won { 0 } else { 1 },
        if player1_won { 1 } else { 0 },
        player2_id,
        season_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating player2 season stats: {}", e);
        AuthError::DatabaseError
    })?;

    // Update global current_elo for both players
    sqlx::query!(
        r#"
        UPDATE players
        SET current_elo = $1
        WHERE id = $2
        "#,
        player1_new_elo,
        player1_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating player1 current_elo: {}", e);
        AuthError::DatabaseError
    })?;

    sqlx::query!(
        r#"
        UPDATE players
        SET current_elo = $1
        WHERE id = $2
        "#,
        player2_new_elo,
        player2_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating player2 current_elo: {}", e);
        AuthError::DatabaseError
    })?;

    // Commit the transaction
    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        AuthError::DatabaseError
    })?;

    tracing::info!(
        "Game created successfully: {} ({}) vs {} ({}), score: {}-{}",
        player1.first_name,
        player1_new_elo,
        player2.first_name,
        player2_new_elo,
        player1_score,
        player2_score
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateGameResponse {
            message: "Game created successfully".to_string(),
            game: GameResponse {
                id: game.id,
                player1_id: game.player1_id,
                player2_id: game.player2_id,
                player1_score: game.player1_score,
                player2_score: game.player2_score,
                season_id: game.season_id,
                played_at: game.played_at,
            },
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ListGamesParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 {
    1
}
fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct GameWithDetails {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player1_name: String,
    pub player1_score: i32,
    pub player1_elo_before: f64,
    pub player1_elo_after: f64,
    pub player1_elo_change: f64,
    pub player2_id: Uuid,
    pub player2_name: String,
    pub player2_score: i32,
    pub player2_elo_before: f64,
    pub player2_elo_after: f64,
    pub player2_elo_change: f64,
    pub season_id: Uuid,
    pub season_name: String,
    pub played_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListGamesResponse {
    pub games: Vec<GameWithDetails>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGameRequest {
    pub player1_score: i32,
    pub player2_score: i32,
    pub played_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// List all games with player names and ELO changes (with pagination)
/// Public endpoint (no auth required)
pub async fn list_games(
    State(pool): State<PgPool>,
    Query(params): Query<ListGamesParams>,
) -> Result<Json<ListGamesResponse>, AuthError> {
    // Validate and sanitize pagination parameters
    let limit = params.limit.clamp(1, 100); // Max 100 per page
    let page = params.page.max(1);
    let offset = (page - 1) * limit;

    // Get total count
    let total_result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM games g
        INNER JOIN elo_history eh1 ON g.id = eh1.game_id
            AND eh1.player_id = g.player1_id
            AND eh1.season_id = g.season_id
        INNER JOIN elo_history eh2 ON g.id = eh2.game_id
            AND eh2.player_id = g.player2_id
            AND eh2.season_id = g.season_id
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error counting games: {}", e);
        AuthError::DatabaseError
    })?;

    let total = total_result.count.unwrap_or(0);
    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    // Get paginated games
    let games = sqlx::query!(
        r#"
        SELECT
            g.id,
            g.player1_id,
            g.player2_id,
            g.player1_score,
            g.player2_score,
            g.season_id,
            g.played_at,
            p1.first_name as player1_first_name,
            p1.last_name as player1_last_name,
            p2.first_name as player2_first_name,
            p2.last_name as player2_last_name,
            s.name as season_name,
            eh1.elo_before as player1_elo_before,
            eh1.elo_after as player1_elo_after,
            eh2.elo_before as player2_elo_before,
            eh2.elo_after as player2_elo_after
        FROM games g
        INNER JOIN players p1 ON g.player1_id = p1.id
        INNER JOIN players p2 ON g.player2_id = p2.id
        INNER JOIN seasons s ON g.season_id = s.id
        INNER JOIN elo_history eh1 ON g.id = eh1.game_id
            AND eh1.player_id = g.player1_id
            AND eh1.season_id = g.season_id
        INNER JOIN elo_history eh2 ON g.id = eh2.game_id
            AND eh2.player_id = g.player2_id
            AND eh2.season_id = g.season_id
        ORDER BY g.played_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching games: {}", e);
        AuthError::DatabaseError
    })?;

    let games_with_details = games
        .into_iter()
        .map(|game| GameWithDetails {
            id: game.id,
            player1_id: game.player1_id,
            player1_name: format_player_name(game.player1_first_name, game.player1_last_name),
            player1_score: game.player1_score,
            player1_elo_before: game.player1_elo_before,
            player1_elo_after: game.player1_elo_after,
            player1_elo_change: game.player1_elo_after - game.player1_elo_before,
            player2_id: game.player2_id,
            player2_name: format_player_name(game.player2_first_name, game.player2_last_name),
            player2_score: game.player2_score,
            player2_elo_before: game.player2_elo_before,
            player2_elo_after: game.player2_elo_after,
            player2_elo_change: game.player2_elo_after - game.player2_elo_before,
            season_id: game.season_id,
            season_name: game.season_name,
            played_at: game.played_at,
        })
        .collect();

    Ok(Json(ListGamesResponse {
        games: games_with_details,
        total,
        page,
        limit,
        total_pages,
    }))
}

/// Delete a game
/// Requires admin authentication
/// This will delete the game and recalculate the entire season's ELO ratings
pub async fn delete_game(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if user is admin
    if !matches!(user.role, UserRole::Admin) {
        return Err(AuthError::Forbidden);
    }

    tracing::info!("Admin {} deleting game: {}", user.username, game_id);

    // Get the game to find its season
    let game = sqlx::query!(
        r#"
        SELECT season_id
        FROM games
        WHERE id = $1
        "#,
        game_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching game: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| AuthError::InvalidInput("Game not found".to_string()))?;

    // Start a transaction
    let mut tx = pool.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {}", e);
        AuthError::DatabaseError
    })?;

    // Delete elo_history entries for this game
    sqlx::query!(
        r#"
        DELETE FROM elo_history
        WHERE game_id = $1
        "#,
        game_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error deleting elo_history: {}", e);
        AuthError::DatabaseError
    })?;

    // Delete the game
    sqlx::query!(
        r#"
        DELETE FROM games
        WHERE id = $1
        "#,
        game_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error deleting game: {}", e);
        AuthError::DatabaseError
    })?;

    // Commit the transaction
    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        AuthError::DatabaseError
    })?;

    // Recalculate the season
    seasons::recalculate_season_elo(&pool, game.season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to recalculate season: {}", e);
            AuthError::DatabaseError
        })?;

    tracing::info!("Game {} deleted successfully, season recalculated", game_id);

    Ok(Json(serde_json::json!({
        "message": "Game deleted successfully"
    })))
}

/// Update a game
/// Requires admin authentication
/// This will update the game scores/date and recalculate the entire season's ELO ratings
pub async fn update_game(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Path(game_id): Path<Uuid>,
    Json(payload): Json<UpdateGameRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if user is admin
    if !matches!(user.role, UserRole::Admin) {
        return Err(AuthError::Forbidden);
    }

    tracing::info!("Admin {} updating game: {}", user.username, game_id);

    // Validate input
    if payload.player1_score < 0 || payload.player2_score < 0 {
        return Err(AuthError::InvalidInput(
            "Scores cannot be negative".to_string(),
        ));
    }

    if payload.player1_score == payload.player2_score {
        return Err(AuthError::InvalidInput("Game cannot be a tie".to_string()));
    }

    // Get the game to verify it exists and get its season and players
    let game = sqlx::query!(
        r#"
        SELECT id, player1_id, player2_id, season_id, played_at
        FROM games
        WHERE id = $1
        "#,
        game_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching game: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| AuthError::InvalidInput("Game not found".to_string()))?;

    // Ensure player1 is always the winner by swapping if needed
    // This maintains data consistency with create_game behavior
    let (final_player1_id, final_player2_id, final_player1_score, final_player2_score) =
        if payload.player1_score > payload.player2_score {
            // Current player1 wins with new scores - no swap needed
            (
                game.player1_id,
                game.player2_id,
                payload.player1_score,
                payload.player2_score,
            )
        } else {
            // Current player2 wins with new scores - swap players
            tracing::info!(
                "Swapping players for game {} (player2 is now winner)",
                game_id
            );
            (
                game.player2_id,
                game.player1_id,
                payload.player2_score,
                payload.player1_score,
            )
        };

    // Update the game
    let played_at = payload.played_at.unwrap_or(game.played_at);

    sqlx::query!(
        r#"
        UPDATE games
        SET player1_id = $1, player2_id = $2, player1_score = $3, player2_score = $4, played_at = $5
        WHERE id = $6
        "#,
        final_player1_id,
        final_player2_id,
        final_player1_score,
        final_player2_score,
        played_at,
        game_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating game: {}", e);
        AuthError::DatabaseError
    })?;

    // Recalculate the season (this will recalculate all ELO history)
    seasons::recalculate_season_elo(&pool, game.season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to recalculate season: {}", e);
            AuthError::DatabaseError
        })?;

    tracing::info!("Game {} updated successfully, season recalculated", game_id);

    Ok(Json(serde_json::json!({
        "message": "Game updated successfully"
    })))
}
