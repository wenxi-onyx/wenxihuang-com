use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::User;
use crate::services::seasons;

// Validation constants
const MAX_SEASON_NAME_LENGTH: usize = 100;
const MAX_DESCRIPTION_LENGTH: usize = 500;
const MIN_K_FACTOR: f64 = 1.0;
const MAX_K_FACTOR: f64 = 100.0;
const MIN_STARTING_ELO: f64 = 100.0;
const MAX_STARTING_ELO: f64 = 3000.0;

#[derive(Debug, Deserialize)]
pub struct CreateSeasonRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: DateTime<Utc>,
    pub starting_elo: f64,
    pub k_factor: f64,
    pub base_k_factor: Option<f64>,
    pub new_player_k_bonus: Option<f64>,
    pub new_player_bonus_period: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SeasonResponse {
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

impl From<seasons::Season> for SeasonResponse {
    fn from(s: seasons::Season) -> Self {
        Self {
            id: s.id,
            name: s.name,
            description: s.description,
            start_date: s.start_date,
            starting_elo: s.starting_elo,
            k_factor: s.k_factor,
            base_k_factor: s.base_k_factor,
            new_player_k_bonus: s.new_player_k_bonus,
            new_player_bonus_period: s.new_player_bonus_period,
            is_active: s.is_active,
            created_at: s.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlayerSeasonStatsResponse {
    pub player_id: Uuid,
    pub player_name: String,
    pub current_elo: f64,
    pub games_played: i32,
    pub wins: i32,
    pub losses: i32,
    pub win_rate: f64,
    pub is_active: bool,
}

/// Get all seasons
pub async fn list_seasons(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<SeasonResponse>>, AuthError> {
    let seasons = seasons::get_all_seasons(&pool).await.map_err(|e| {
        tracing::error!("Failed to fetch seasons: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(
        seasons.into_iter().map(SeasonResponse::from).collect(),
    ))
}

/// Get active season
pub async fn get_active_season(
    State(pool): State<PgPool>,
) -> Result<Json<Option<SeasonResponse>>, AuthError> {
    let season = seasons::get_active_season(&pool).await.map_err(|e| {
        tracing::error!("Failed to fetch active season: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(season.map(SeasonResponse::from)))
}

/// Get a specific season by ID
pub async fn get_season(
    State(pool): State<PgPool>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<SeasonResponse>, AuthError> {
    let season = seasons::get_season_by_id(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season: {}", e);
            AuthError::DatabaseError
        })?
        .ok_or(AuthError::InvalidInput("Season not found".to_string()))?;

    Ok(Json(SeasonResponse::from(season)))
}

/// Create a new season (admin only)
/// Automatically activates the new season and deactivates previous ones
pub async fn create_season(
    State(pool): State<PgPool>,
    Extension(admin_user): Extension<User>,
    Json(req): Json<CreateSeasonRequest>,
) -> Result<Json<SeasonResponse>, AuthError> {
    // Validate season name
    if req.name.is_empty() || req.name.len() > MAX_SEASON_NAME_LENGTH {
        return Err(AuthError::InvalidInput(format!(
            "Season name must be 1-{} characters",
            MAX_SEASON_NAME_LENGTH
        )));
    }

    // Validate K-factor
    if req.k_factor < MIN_K_FACTOR || req.k_factor > MAX_K_FACTOR {
        return Err(AuthError::InvalidInput(format!(
            "K-factor must be between {} and {}",
            MIN_K_FACTOR, MAX_K_FACTOR
        )));
    }

    // Validate starting ELO
    if req.starting_elo < MIN_STARTING_ELO || req.starting_elo > MAX_STARTING_ELO {
        return Err(AuthError::InvalidInput(format!(
            "Starting ELO must be between {} and {}",
            MIN_STARTING_ELO, MAX_STARTING_ELO
        )));
    }

    // Validate description length
    if let Some(ref desc) = req.description
        && desc.len() > MAX_DESCRIPTION_LENGTH
    {
        return Err(AuthError::InvalidInput(format!(
            "Description must be {} characters or less",
            MAX_DESCRIPTION_LENGTH
        )));
    }

    // Validate dynamic K-factor fields
    if let Some(base_k) = req.base_k_factor
        && !(MIN_K_FACTOR..=MAX_K_FACTOR).contains(&base_k)
    {
        return Err(AuthError::InvalidInput(format!(
            "Base K-factor must be between {} and {}",
            MIN_K_FACTOR, MAX_K_FACTOR
        )));
    }

    if let Some(bonus) = req.new_player_k_bonus
        && !(0.0..=MAX_K_FACTOR).contains(&bonus)
    {
        return Err(AuthError::InvalidInput(format!(
            "New player K bonus must be between 0 and {}",
            MAX_K_FACTOR
        )));
    }

    if let Some(period) = req.new_player_bonus_period
        && period <= 0
    {
        return Err(AuthError::InvalidInput(
            "New player bonus period must be positive".to_string(),
        ));
    }

    // Check if season name already exists
    if seasons::get_season_by_name(&pool, &req.name)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check season name: {}", e);
            AuthError::DatabaseError
        })?
        .is_some()
    {
        return Err(AuthError::InvalidInput(
            "Season name already exists".to_string(),
        ));
    }

    // Create season (automatically activates it, initializes players, and recalculates if historical)
    let season = seasons::create_season(
        &pool,
        req.name,
        req.description,
        req.start_date,
        req.starting_elo,
        req.k_factor,
        req.base_k_factor,
        req.new_player_k_bonus,
        req.new_player_bonus_period,
        admin_user.id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create season: {}", e);
        AuthError::DatabaseError
    })?;

    tracing::info!("Successfully created season '{}'", season.name);

    Ok(Json(SeasonResponse::from(season)))
}

/// Activate a season (admin only)
/// Note: Creating a new season automatically activates it.
/// This endpoint is useful for switching back to a previous season.
pub async fn activate_season(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if season exists
    let season = seasons::get_season_by_id(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season: {}", e);
            AuthError::DatabaseError
        })?
        .ok_or(AuthError::InvalidInput("Season not found".to_string()))?;

    seasons::activate_season(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to activate season: {}", e);
            AuthError::DatabaseError
        })?;

    Ok(Json(serde_json::json!({
        "message": format!("Season '{}' activated", season.name)
    })))
}

/// Get leaderboard for a specific season
pub async fn get_season_leaderboard(
    State(pool): State<PgPool>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<Vec<PlayerSeasonStatsResponse>>, AuthError> {
    let leaderboard = seasons::get_season_leaderboard(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season leaderboard: {}", e);
            AuthError::DatabaseError
        })?;

    let response: Vec<PlayerSeasonStatsResponse> = leaderboard
        .into_iter()
        .map(
            |(id, first_name, last_name, elo, games, wins, losses, is_active)| {
                let win_rate = if games > 0 {
                    (wins as f64 / games as f64) * 100.0
                } else {
                    0.0
                };

                PlayerSeasonStatsResponse {
                    player_id: id,
                    player_name: format!("{} {}", first_name, last_name),
                    current_elo: elo,
                    games_played: games,
                    wins,
                    losses,
                    win_rate,
                    is_active,
                }
            },
        )
        .collect();

    Ok(Json(response))
}

/// Recalculate ELO for a season (admin only)
pub async fn recalculate_season(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let season = seasons::get_season_by_id(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season: {}", e);
            AuthError::DatabaseError
        })?
        .ok_or(AuthError::InvalidInput("Season not found".to_string()))?;

    // Spawn background task for recalculation
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = seasons::recalculate_season_elo(&pool_clone, season_id).await {
            tracing::error!("Failed to recalculate season ELO: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "message": format!("Started ELO recalculation for season '{}'", season.name)
    })))
}

/// Delete a season (admin only)
/// This will delete the season and all associated data, reassign games to other seasons,
/// and recalculate all affected seasons
pub async fn delete_season(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AuthError> {
    let season = seasons::get_season_by_id(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season: {}", e);
            AuthError::DatabaseError
        })?
        .ok_or(AuthError::InvalidInput("Season not found".to_string()))?;

    // Spawn background task for deletion
    let pool_clone = pool.clone();
    let season_name = season.name.clone();
    tokio::spawn(async move {
        if let Err(e) = seasons::delete_season(&pool_clone, season_id).await {
            tracing::error!("Failed to delete season: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "message": format!("Started deletion of season '{}'. Games will be reassigned and affected seasons recalculated.", season_name)
    })))
}

#[derive(Debug, Serialize)]
pub struct SeasonPlayerResponse {
    pub player_id: Uuid,
    pub player_name: String,
    pub is_included: bool,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct ManageSeasonPlayerRequest {
    pub player_id: Uuid,
}

/// Get all players in a season (admin only)
pub async fn get_season_players(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<Vec<SeasonPlayerResponse>>, AuthError> {
    let players = seasons::get_season_players(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch season players: {}", e);
            AuthError::DatabaseError
        })?;

    let response: Vec<SeasonPlayerResponse> = players
        .into_iter()
        .map(
            |(id, first_name, last_name, is_included, is_active)| SeasonPlayerResponse {
                player_id: id,
                player_name: format!("{} {}", first_name, last_name),
                is_included,
                is_active,
            },
        )
        .collect();

    Ok(Json(response))
}

/// Get available players for a season (admin only)
pub async fn get_available_players(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
) -> Result<Json<Vec<SeasonPlayerResponse>>, AuthError> {
    let players = seasons::get_available_players_for_season(&pool, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch available players: {}", e);
            AuthError::DatabaseError
        })?;

    let response: Vec<SeasonPlayerResponse> = players
        .into_iter()
        .map(|(id, first_name, last_name, is_active)| {
            SeasonPlayerResponse {
                player_id: id,
                player_name: format!("{} {}", first_name, last_name),
                is_included: false, // These are not in the season yet
                is_active,
            }
        })
        .collect();

    Ok(Json(response))
}

/// Add a player to a season (admin only)
pub async fn add_player_to_season(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
    Json(req): Json<ManageSeasonPlayerRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    seasons::add_player_to_season(&pool, req.player_id, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add player to season: {}", e);
            AuthError::DatabaseError
        })?;

    Ok(Json(serde_json::json!({
        "message": "Player added to season successfully"
    })))
}

/// Remove a player from a season (admin only)
pub async fn remove_player_from_season(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    Path(season_id): Path<Uuid>,
    Json(req): Json<ManageSeasonPlayerRequest>,
) -> Result<Json<serde_json::Value>, AuthError> {
    seasons::remove_player_from_season(&pool, req.player_id, season_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove player from season: {}", e);
            AuthError::DatabaseError
        })?;

    Ok(Json(serde_json::json!({
        "message": "Player removed from season successfully"
    })))
}
