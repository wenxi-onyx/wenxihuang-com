use axum::{Extension, Json, extract::State, http::HeaderMap};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::User;

#[derive(Debug, Serialize, FromRow)]
pub struct PlayerResponse {
    pub id: Uuid,
    pub name: String,
    pub current_elo: f64,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PlayerWithStatsResponse {
    pub id: Uuid,
    pub name: String,
    pub current_elo: f64,
    pub is_active: bool,
    pub games_played: i64,
    pub wins: i64,
    pub losses: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// List all players with their stats
pub async fn list_players(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<PlayerWithStatsResponse>>, AuthError> {
    let players: Vec<PlayerWithStatsResponse> = sqlx::query_as(
        "SELECT
            p.id,
            CONCAT(p.first_name, ' ', p.last_name) as name,
            p.current_elo,
            COALESCE(p.is_active, true) as is_active,
            COALESCE(COUNT(DISTINCT g.id), 0) as games_played,
            COALESCE(COUNT(DISTINCT CASE WHEN g.player1_id = p.id THEN g.id END), 0) as wins,
            COALESCE(COUNT(DISTINCT CASE WHEN g.player2_id = p.id THEN g.id END), 0) as losses,
            p.created_at,
            COALESCE(p.updated_at, p.created_at) as updated_at
         FROM players p
         LEFT JOIN games g ON (g.player1_id = p.id OR g.player2_id = p.id)
         GROUP BY p.id, p.first_name, p.last_name, p.current_elo, p.is_active, p.created_at, p.updated_at
         ORDER BY p.current_elo DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error listing players: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(players))
}

/// Get player ELO history (grouped by match)
#[derive(Debug, Serialize, FromRow)]
pub struct EloHistoryPoint {
    pub match_id: Uuid,
    pub elo_before: f64,
    pub elo_after: f64,
    pub elo_version: String,
    pub season_id: Uuid,
    pub season_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_player_history(
    State(pool): State<PgPool>,
    axum::extract::Path(player_id): axum::extract::Path<Uuid>,
) -> Result<Json<Vec<EloHistoryPoint>>, AuthError> {
    let history: Vec<EloHistoryPoint> = sqlx::query_as(
        "SELECT
            g.match_id,
            (array_agg(eh.elo_before ORDER BY g.played_at ASC))[1] as elo_before,
            (array_agg(eh.elo_after ORDER BY g.played_at DESC))[1] as elo_after,
            MAX(eh.elo_version) as elo_version,
            (array_agg(eh.season_id))[1] as season_id,
            MAX(s.name) as season_name,
            MAX(m.submitted_at) as created_at
         FROM elo_history eh
         JOIN games g ON eh.game_id = g.id
         JOIN matches m ON g.match_id = m.id
         JOIN seasons s ON eh.season_id = s.id
         WHERE eh.player_id = $1
         GROUP BY g.match_id
         ORDER BY MAX(m.submitted_at) ASC",
    )
    .bind(player_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player history: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(history))
}

/// Get all active players' ELO history
#[derive(Debug, Serialize)]
pub struct PlayerEloHistory {
    pub player_id: Uuid,
    pub player_name: String,
    pub history: Vec<EloHistoryPoint>,
}

pub async fn get_all_players_history(
    State(pool): State<PgPool>,
) -> Result<(HeaderMap, Json<Vec<PlayerEloHistory>>), AuthError> {
    // Single query to get all active players and their match-level history
    #[derive(sqlx::FromRow)]
    struct PlayerHistoryRow {
        player_id: Uuid,
        player_name: String,
        match_id: Option<Uuid>,
        elo_before: Option<f64>,
        elo_after: Option<f64>,
        elo_version: Option<String>,
        season_id: Option<Uuid>,
        season_name: Option<String>,
        created_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let rows: Vec<PlayerHistoryRow> = sqlx::query_as(
        "SELECT
            p.id as player_id,
            CONCAT(p.first_name, ' ', p.last_name) as player_name,
            g.match_id,
            (array_agg(eh.elo_before ORDER BY g.played_at ASC))[1] as elo_before,
            (array_agg(eh.elo_after ORDER BY g.played_at DESC))[1] as elo_after,
            MAX(eh.elo_version) as elo_version,
            (array_agg(eh.season_id))[1] as season_id,
            MAX(s.name) as season_name,
            MAX(m.submitted_at) as created_at
         FROM players p
         LEFT JOIN elo_history eh ON p.id = eh.player_id
         LEFT JOIN games g ON eh.game_id = g.id
         LEFT JOIN matches m ON g.match_id = m.id
         LEFT JOIN seasons s ON eh.season_id = s.id
         WHERE p.is_active = true
         GROUP BY p.id, p.first_name, p.last_name, g.match_id
         ORDER BY p.current_elo DESC, MAX(m.submitted_at) ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching players history: {}", e);
        AuthError::DatabaseError
    })?;

    // Group by player
    let mut result = Vec::new();
    let mut current_player_id: Option<Uuid> = None;
    let mut current_history = Vec::new();
    let mut current_player_name = String::new();

    for row in rows {
        if current_player_id != Some(row.player_id) {
            // Save previous player if exists
            if let Some(player_id) = current_player_id {
                result.push(PlayerEloHistory {
                    player_id,
                    player_name: std::mem::take(&mut current_player_name),
                    history: std::mem::take(&mut current_history),
                });
            }

            // Start new player
            current_player_id = Some(row.player_id);
            current_player_name = row.player_name;
        }

        // Add history point if it exists
        if let (
            Some(match_id),
            Some(elo_before),
            Some(elo_after),
            Some(elo_version),
            Some(season_id),
            Some(season_name),
            Some(created_at),
        ) = (
            row.match_id,
            row.elo_before,
            row.elo_after,
            row.elo_version,
            row.season_id,
            row.season_name,
            row.created_at,
        ) {
            current_history.push(EloHistoryPoint {
                match_id,
                elo_before,
                elo_after,
                elo_version,
                season_id,
                season_name,
                created_at,
            });
        }
    }

    // Don't forget the last player
    if let Some(player_id) = current_player_id {
        result.push(PlayerEloHistory {
            player_id,
            player_name: current_player_name,
            history: current_history,
        });
    }

    // Add cache headers
    let mut headers = HeaderMap::new();
    // Cache for 60 seconds, but allow stale data for 300 seconds while revalidating
    headers.insert(
        "Cache-Control",
        "public, max-age=60, stale-while-revalidate=300"
            .parse()
            .unwrap(),
    );

    Ok((headers, Json(result)))
}

/// Get matches for a specific player
#[derive(Debug, Serialize, FromRow)]
pub struct PlayerMatchResponse {
    pub match_id: Uuid,
    pub opponent_id: Uuid,
    pub opponent_name: String,
    pub player_games_won: i32,
    pub opponent_games_won: i32,
    pub result: String, // "W" or "L"
    pub season_name: String,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_player_matches(
    State(pool): State<PgPool>,
    axum::extract::Path(player_id): axum::extract::Path<Uuid>,
) -> Result<Json<Vec<PlayerMatchResponse>>, AuthError> {
    let matches = sqlx::query!(
        r#"
        SELECT
            m.id as match_id,
            m.player1_id,
            m.player2_id,
            p1.first_name as p1_first,
            p1.last_name as p1_last,
            p2.first_name as p2_first,
            p2.last_name as p2_last,
            COUNT(CASE WHEN g.player1_id = $1 THEN 1 END) as player_games_won,
            COUNT(CASE WHEN g.player1_id != $1 THEN 1 END) as opponent_games_won,
            s.name as season_name,
            m.submitted_at
        FROM matches m
        INNER JOIN players p1 ON m.player1_id = p1.id
        INNER JOIN players p2 ON m.player2_id = p2.id
        INNER JOIN seasons s ON m.season_id = s.id
        INNER JOIN games g ON g.match_id = m.id
        WHERE m.player1_id = $1 OR m.player2_id = $1
        GROUP BY m.id, m.player1_id, m.player2_id, p1.first_name, p1.last_name, p2.first_name, p2.last_name, s.name, m.submitted_at
        ORDER BY m.submitted_at DESC
        "#,
        player_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching player matches: {}", e);
        AuthError::DatabaseError
    })?;

    let result: Vec<PlayerMatchResponse> = matches
        .into_iter()
        .map(|row| {
            // Determine opponent based on which player is in the match
            let (opponent_id, opponent_first, opponent_last) = if row.player1_id == player_id {
                (row.player2_id, row.p2_first, row.p2_last)
            } else {
                (row.player1_id, row.p1_first, row.p1_last)
            };

            // Format opponent name safely
            let opponent_name = format!("{} {}", opponent_first.trim(), opponent_last.trim())
                .trim()
                .to_string();

            // Calculate result
            let player_wins = row.player_games_won.unwrap_or(0) as i32;
            let opponent_wins = row.opponent_games_won.unwrap_or(0) as i32;
            let result = if player_wins > opponent_wins {
                "W"
            } else {
                "L"
            };

            PlayerMatchResponse {
                match_id: row.match_id,
                opponent_id,
                opponent_name,
                player_games_won: player_wins,
                opponent_games_won: opponent_wins,
                result: result.to_string(),
                season_name: row.season_name,
                submitted_at: row.submitted_at,
            }
        })
        .collect();

    Ok(Json(result))
}

/// Toggle player active status (admin only)
pub async fn toggle_player_active(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    axum::extract::Path(player_id): axum::extract::Path<Uuid>,
) -> Result<Json<PlayerResponse>, AuthError> {
    // Toggle is_active
    let player: PlayerResponse = sqlx::query_as(
        "UPDATE players
         SET is_active = NOT is_active, updated_at = NOW()
         WHERE id = $1
         RETURNING id, CONCAT(first_name, ' ', last_name) as name, current_elo, is_active, created_at, updated_at"
    )
    .bind(player_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error toggling player active status: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(player))
}
