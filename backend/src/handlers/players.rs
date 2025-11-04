use axum::{Extension, Json, extract::State};
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

/// Get player ELO history
#[derive(Debug, Serialize, FromRow)]
pub struct EloHistoryPoint {
    pub game_id: Uuid,
    pub elo_before: f64,
    pub elo_after: f64,
    pub elo_version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_player_history(
    State(pool): State<PgPool>,
    axum::extract::Path(player_id): axum::extract::Path<Uuid>,
) -> Result<Json<Vec<EloHistoryPoint>>, AuthError> {
    let history: Vec<EloHistoryPoint> = sqlx::query_as(
        "SELECT game_id, elo_before, elo_after, elo_version, created_at
         FROM elo_history
         WHERE player_id = $1
         ORDER BY created_at ASC",
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
