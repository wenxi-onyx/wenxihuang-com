use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::{User, UserRole};
use crate::services::elo::{GameWinner, calculate_match_elo_changes};
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
pub struct CreateMatchRequest {
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub games: Vec<GameWinner>,
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct GameDetail {
    pub game_number: i32,
    pub winner: String, // "Player1" or "Player2"
    pub player1_elo_before: f64,
    pub player1_elo_after: f64,
    pub player1_elo_change: f64,
    pub player2_elo_before: f64,
    pub player2_elo_after: f64,
    pub player2_elo_change: f64,
    pub played_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MatchWithDetails {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player1_name: String,
    pub player1_games_won: i32,
    pub player1_elo_before: f64,
    pub player1_elo_after: f64,
    pub player1_elo_change: f64,
    pub player2_id: Uuid,
    pub player2_name: String,
    pub player2_games_won: i32,
    pub player2_elo_before: f64,
    pub player2_elo_after: f64,
    pub player2_elo_change: f64,
    pub season_id: Uuid,
    pub season_name: String,
    pub total_games: i32,
    pub submitted_at: DateTime<Utc>,
    pub games: Vec<GameDetail>,
}

#[derive(Debug, Serialize)]
pub struct CreateMatchResponse {
    pub message: String,
    pub match_data: MatchWithDetails,
}

/// Create a new match with multiple games
/// Requires authentication (user or admin role)
pub async fn create_match(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateMatchRequest>,
) -> Result<(StatusCode, Json<CreateMatchResponse>), AuthError> {
    tracing::info!(
        "User {} creating match: {:?} vs {:?} ({} games)",
        user.username,
        payload.player1_id,
        payload.player2_id,
        payload.games.len()
    );

    // Validate input
    if payload.player1_id == payload.player2_id {
        return Err(AuthError::InvalidInput(
            "Players must be different".to_string(),
        ));
    }

    if payload.games.is_empty() {
        return Err(AuthError::InvalidInput(
            "Match must have at least one game".to_string(),
        ));
    }

    // Get the active season
    let active_season = sqlx::query!(
        r#"
        SELECT id, name, starting_elo, k_factor, base_k_factor, new_player_k_bonus, new_player_bonus_period, elo_version
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
        payload.player1_id
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
        payload.player2_id
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

    // Verify both players are in the active season and lock rows
    let player1_season = sqlx::query!(
        r#"
        SELECT current_elo, games_played, is_included
        FROM player_seasons
        WHERE player_id = $1 AND season_id = $2
        FOR UPDATE
        "#,
        payload.player1_id,
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
        payload.player2_id,
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

    // Calculate game timestamps (work backward from submitted_at, 5 min intervals)
    let submitted_at = payload.submitted_at.unwrap_or_else(Utc::now);
    let num_games = payload.games.len() as i32;

    // Create match record
    let match_record = sqlx::query!(
        r#"
        INSERT INTO matches (player1_id, player2_id, submitted_at, season_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, player1_id, player2_id, submitted_at, season_id, created_at
        "#,
        payload.player1_id,
        payload.player2_id,
        submitted_at,
        season_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating match: {}", e);
        AuthError::DatabaseError
    })?;

    // Create game records with UUIDs for ELO calculation
    let mut game_ids_with_winners = Vec::new();
    let mut game_details = Vec::new();

    for (i, winner) in payload.games.iter().enumerate() {
        // Calculate timestamp: last game is at submitted_at, work backward
        let minutes_back = (num_games - 1 - i as i32) * 5;
        let game_played_at = submitted_at - Duration::minutes(minutes_back as i64);

        // Determine which player won this game
        let (game_player1_id, game_player2_id) = match winner {
            GameWinner::Player1 => (payload.player1_id, payload.player2_id),
            GameWinner::Player2 => (payload.player2_id, payload.player1_id),
        };

        // Create game record (player1 is always winner)
        let game = sqlx::query!(
            r#"
            INSERT INTO games (match_id, player1_id, player2_id, played_at, season_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            match_record.id,
            game_player1_id,
            game_player2_id,
            game_played_at,
            season_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating game: {}", e);
            AuthError::DatabaseError
        })?;

        game_ids_with_winners.push((game.id, *winner));
    }

    // Calculate dynamic K-factors (similar to games.rs)
    let calculate_k_factor = |games_played: i32| -> f64 {
        match (
            active_season.base_k_factor,
            active_season.new_player_k_bonus,
            active_season.new_player_bonus_period,
        ) {
            (Some(base_k), Some(bonus), Some(period)) if period > 0 => {
                let decay = (-(games_played as f64) / (period as f64)).exp();
                base_k + (bonus * decay)
            }
            _ => active_season.k_factor,
        }
    };

    let player1_k = calculate_k_factor(player1_season.games_played);
    let player2_k = calculate_k_factor(player2_season.games_played);

    // Calculate ELO changes for all games sequentially
    let elo_changes = calculate_match_elo_changes(
        payload.player1_id,
        payload.player2_id,
        player1_season.current_elo,
        player2_season.current_elo,
        game_ids_with_winners.clone(),
        player1_k,
        player2_k,
    );

    // Insert ELO history records and build game details
    for (i, change) in elo_changes.iter().enumerate() {
        let (_game_id, winner) = &game_ids_with_winners[i];
        let game_played_at =
            submitted_at - Duration::minutes(((num_games - 1 - i as i32) * 5) as i64);

        // Insert ELO history for player 1
        sqlx::query!(
            r#"
            INSERT INTO elo_history (player_id, game_id, elo_before, elo_after, elo_version, season_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            payload.player1_id,
            change.game_id,
            change.player1_elo_before,
            change.player1_elo_after,
            active_season.elo_version,
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
            payload.player2_id,
            change.game_id,
            change.player2_elo_before,
            change.player2_elo_after,
            active_season.elo_version,
            season_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating elo_history for player2: {}", e);
            AuthError::DatabaseError
        })?;

        // Build game detail for response
        game_details.push(GameDetail {
            game_number: (i + 1) as i32,
            winner: match winner {
                GameWinner::Player1 => "Player1".to_string(),
                GameWinner::Player2 => "Player2".to_string(),
            },
            player1_elo_before: change.player1_elo_before,
            player1_elo_after: change.player1_elo_after,
            player1_elo_change: change.player1_elo_change,
            player2_elo_before: change.player2_elo_before,
            player2_elo_after: change.player2_elo_after,
            player2_elo_change: change.player2_elo_change,
            played_at: game_played_at,
        });
    }

    // Get first and last ELO changes for the match
    let first_change = elo_changes.first().unwrap();
    let last_change = elo_changes.last().unwrap();

    let player1_elo_before = first_change.player1_elo_before;
    let player1_elo_after = last_change.player1_elo_after;
    let player2_elo_before = first_change.player2_elo_before;
    let player2_elo_after = last_change.player2_elo_after;

    // Count wins for each player
    let player1_games_won = payload
        .games
        .iter()
        .filter(|w| matches!(w, GameWinner::Player1))
        .count() as i32;
    let player2_games_won = payload
        .games
        .iter()
        .filter(|w| matches!(w, GameWinner::Player2))
        .count() as i32;

    // Update player_seasons for both players
    sqlx::query!(
        r#"
        UPDATE player_seasons
        SET current_elo = $1,
            games_played = games_played + $2,
            wins = wins + $3,
            losses = losses + $4
        WHERE player_id = $5 AND season_id = $6
        "#,
        player1_elo_after,
        num_games,
        player1_games_won,
        player2_games_won,
        payload.player1_id,
        season_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating player1 season stats: {}", e);
        AuthError::DatabaseError
    })?;

    sqlx::query!(
        r#"
        UPDATE player_seasons
        SET current_elo = $1,
            games_played = games_played + $2,
            wins = wins + $3,
            losses = losses + $4
        WHERE player_id = $5 AND season_id = $6
        "#,
        player2_elo_after,
        num_games,
        player2_games_won,
        player1_games_won,
        payload.player2_id,
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
        player1_elo_after,
        payload.player1_id
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
        player2_elo_after,
        payload.player2_id
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
        "Match created successfully: {} vs {}, {} games ({}-{})",
        format_player_name(player1.first_name.clone(), player1.last_name.clone()),
        format_player_name(player2.first_name.clone(), player2.last_name.clone()),
        num_games,
        player1_games_won,
        player2_games_won
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateMatchResponse {
            message: "Match created successfully".to_string(),
            match_data: MatchWithDetails {
                id: match_record.id,
                player1_id: payload.player1_id,
                player1_name: format_player_name(player1.first_name, player1.last_name),
                player1_games_won,
                player1_elo_before,
                player1_elo_after,
                player1_elo_change: player1_elo_after - player1_elo_before,
                player2_id: payload.player2_id,
                player2_name: format_player_name(player2.first_name, player2.last_name),
                player2_games_won,
                player2_elo_before,
                player2_elo_after,
                player2_elo_change: player2_elo_after - player2_elo_before,
                season_id,
                season_name: active_season.name,
                total_games: num_games,
                submitted_at,
                games: game_details,
            },
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ListMatchesParams {
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
pub struct ListMatchesResponse {
    pub matches: Vec<MatchWithDetails>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

/// List all matches with player names and ELO changes (with pagination)
/// Public endpoint (no auth required)
pub async fn list_matches(
    State(pool): State<PgPool>,
    Query(params): Query<ListMatchesParams>,
) -> Result<Json<ListMatchesResponse>, AuthError> {
    // Validate and sanitize pagination parameters
    let limit = params.limit.clamp(1, 100); // Max 100 per page
    let page = params.page.max(1);
    let offset = (page - 1) * limit;

    // Get total count
    let total_result = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM matches
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error counting matches: {}", e);
        AuthError::DatabaseError
    })?;

    let total = total_result.count.unwrap_or(0);
    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    // Get paginated matches
    let matches = sqlx::query!(
        r#"
        SELECT
            m.id,
            m.player1_id,
            m.player2_id,
            m.season_id,
            m.submitted_at,
            p1.first_name as player1_first_name,
            p1.last_name as player1_last_name,
            p2.first_name as player2_first_name,
            p2.last_name as player2_last_name,
            s.name as season_name
        FROM matches m
        INNER JOIN players p1 ON m.player1_id = p1.id
        INNER JOIN players p2 ON m.player2_id = p2.id
        INNER JOIN seasons s ON m.season_id = s.id
        ORDER BY m.submitted_at DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching matches: {}", e);
        AuthError::DatabaseError
    })?;

    let mut matches_with_details = Vec::new();

    for match_row in matches {
        // Get all games for this match with ELO history
        let games = sqlx::query!(
            r#"
            SELECT
                g.id,
                g.player1_id,
                g.player2_id,
                g.played_at,
                eh1.elo_before as player1_elo_before,
                eh1.elo_after as player1_elo_after,
                eh2.elo_before as player2_elo_before,
                eh2.elo_after as player2_elo_after
            FROM games g
            INNER JOIN elo_history eh1 ON g.id = eh1.game_id
                AND eh1.player_id = $1
                AND eh1.season_id = g.season_id
            INNER JOIN elo_history eh2 ON g.id = eh2.game_id
                AND eh2.player_id = $2
                AND eh2.season_id = g.season_id
            WHERE g.match_id = $3
            ORDER BY g.played_at ASC
            "#,
            match_row.player1_id,
            match_row.player2_id,
            match_row.id
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching games for match: {}", e);
            AuthError::DatabaseError
        })?;

        if games.is_empty() {
            continue; // Skip matches with no games
        }

        // Calculate match-level stats
        let first_game = games.first().unwrap();
        let last_game = games.last().unwrap();

        let player1_elo_before = first_game.player1_elo_before;
        let player1_elo_after = last_game.player1_elo_after;
        let player2_elo_before = first_game.player2_elo_before;
        let player2_elo_after = last_game.player2_elo_after;

        // Count wins for each player (player1 of each game is the winner)
        let player1_games_won = games
            .iter()
            .filter(|g| g.player1_id == match_row.player1_id)
            .count() as i32;
        let player2_games_won = games
            .iter()
            .filter(|g| g.player1_id == match_row.player2_id)
            .count() as i32;

        // Build game details
        let game_details: Vec<GameDetail> = games
            .iter()
            .enumerate()
            .map(|(i, game)| {
                let winner = if game.player1_id == match_row.player1_id {
                    "Player1"
                } else {
                    "Player2"
                };

                GameDetail {
                    game_number: (i + 1) as i32,
                    winner: winner.to_string(),
                    player1_elo_before: game.player1_elo_before,
                    player1_elo_after: game.player1_elo_after,
                    player1_elo_change: game.player1_elo_after - game.player1_elo_before,
                    player2_elo_before: game.player2_elo_before,
                    player2_elo_after: game.player2_elo_after,
                    player2_elo_change: game.player2_elo_after - game.player2_elo_before,
                    played_at: game.played_at,
                }
            })
            .collect();

        matches_with_details.push(MatchWithDetails {
            id: match_row.id,
            player1_id: match_row.player1_id,
            player1_name: format_player_name(
                match_row.player1_first_name,
                match_row.player1_last_name,
            ),
            player1_games_won,
            player1_elo_before,
            player1_elo_after,
            player1_elo_change: player1_elo_after - player1_elo_before,
            player2_id: match_row.player2_id,
            player2_name: format_player_name(
                match_row.player2_first_name,
                match_row.player2_last_name,
            ),
            player2_games_won,
            player2_elo_before,
            player2_elo_after,
            player2_elo_change: player2_elo_after - player2_elo_before,
            season_id: match_row.season_id,
            season_name: match_row.season_name,
            total_games: games.len() as i32,
            submitted_at: match_row.submitted_at,
            games: game_details,
        });
    }

    Ok(Json(ListMatchesResponse {
        matches: matches_with_details,
        total,
        page,
        limit,
        total_pages,
    }))
}

/// Delete a match
/// Requires admin authentication
/// This will delete the match (cascades to games and elo_history) and recalculate the entire season's ELO ratings
pub async fn delete_match(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Path(match_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if user is admin
    if !matches!(user.role, UserRole::Admin) {
        return Err(AuthError::Forbidden);
    }

    tracing::info!("Admin {} deleting match: {}", user.username, match_id);

    // Get the match to find its season
    let match_record = sqlx::query!(
        r#"
        SELECT season_id
        FROM matches
        WHERE id = $1
        "#,
        match_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching match: {}", e);
        AuthError::DatabaseError
    })?
    .ok_or_else(|| AuthError::InvalidInput("Match not found".to_string()))?;

    // Delete the match (cascades to games via ON DELETE CASCADE)
    // Note: elo_history is cleared during season recalculation below
    sqlx::query!(
        r#"
        DELETE FROM matches
        WHERE id = $1
        "#,
        match_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error deleting match: {}", e);
        AuthError::DatabaseError
    })?;

    // Recalculate the season
    seasons::recalculate_season_elo(&pool, match_record.season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to recalculate season: {}", e);
            AuthError::DatabaseError
        })?;

    tracing::info!(
        "Match {} deleted successfully, season recalculated",
        match_id
    );

    Ok(Json(serde_json::json!({
        "message": "Match deleted successfully"
    })))
}
