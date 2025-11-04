use axum::{Extension, Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::error::AuthError;
use crate::models::user::User;
use crate::services::elo::{get_config_by_version, recalculate_all_elo};
use crate::services::jobs::{JobStatus, create_job, get_job, update_job_status};

// Validation constants
const MAX_VERSION_NAME_LENGTH: usize = 50;
const MIN_K_FACTOR: f64 = 1.0;
const MAX_K_FACTOR: f64 = 100.0;
const MIN_STARTING_ELO: f64 = 100.0;
const MAX_STARTING_ELO: f64 = 3000.0;
const MAX_DESCRIPTION_LENGTH: usize = 500;

#[derive(Debug, Deserialize)]
pub struct CreateEloConfigRequest {
    pub version_name: String,
    pub k_factor: f64,
    pub starting_elo: f64,
    pub base_k_factor: Option<f64>,
    pub new_player_k_bonus: Option<f64>,
    pub new_player_bonus_period: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EloConfigResponse {
    pub id: uuid::Uuid,
    pub version_name: String,
    pub k_factor: f64,
    pub starting_elo: f64,
    pub base_k_factor: Option<f64>,
    pub new_player_k_bonus: Option<f64>,
    pub new_player_bonus_period: Option<i32>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create a new ELO configuration (admin only)
pub async fn create_elo_config(
    State(pool): State<PgPool>,
    Extension(admin_user): Extension<User>,
    Json(req): Json<CreateEloConfigRequest>,
) -> Result<Json<EloConfigResponse>, AuthError> {
    // Validate version name
    if req.version_name.is_empty() || req.version_name.len() > MAX_VERSION_NAME_LENGTH {
        return Err(AuthError::InvalidInput(format!(
            "Version name must be 1-{} characters",
            MAX_VERSION_NAME_LENGTH
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

    // Check if version name already exists
    let exists: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM elo_configurations WHERE version_name = $1")
            .bind(&req.version_name)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking version name: {}", e);
                AuthError::DatabaseError
            })?;

    if exists.is_some() {
        return Err(AuthError::InvalidInput(
            "Version name already exists".to_string(),
        ));
    }

    // Create configuration
    let config: EloConfigResponse = sqlx::query_as(
        "INSERT INTO elo_configurations
         (version_name, k_factor, starting_elo, base_k_factor,
          new_player_k_bonus, new_player_bonus_period, description, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, version_name, k_factor, starting_elo, base_k_factor,
                   new_player_k_bonus, new_player_bonus_period, description,
                   is_active, created_at",
    )
    .bind(&req.version_name)
    .bind(req.k_factor)
    .bind(req.starting_elo)
    .bind(req.base_k_factor)
    .bind(req.new_player_k_bonus)
    .bind(req.new_player_bonus_period)
    .bind(&req.description)
    .bind(admin_user.id)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error creating ELO configuration: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(config))
}

/// List all ELO configurations (admin only)
pub async fn list_elo_configs(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<EloConfigResponse>>, AuthError> {
    let configs: Vec<EloConfigResponse> = sqlx::query_as(
        "SELECT id, version_name, k_factor, starting_elo, base_k_factor,
                new_player_k_bonus, new_player_bonus_period, description,
                is_active, created_at
         FROM elo_configurations
         ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error listing ELO configurations: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(configs))
}

/// Set a configuration as active (admin only)
pub async fn activate_elo_config(
    State(pool): State<PgPool>,
    axum::extract::Path(version_name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if config exists
    let exists: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM elo_configurations WHERE version_name = $1")
            .bind(&version_name)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking configuration existence: {}", e);
                AuthError::DatabaseError
            })?;

    if exists.is_none() {
        return Err(AuthError::InvalidInput(
            "Configuration not found".to_string(),
        ));
    }

    // Start transaction
    let mut tx = pool.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        AuthError::DatabaseError
    })?;

    // Deactivate all configs
    sqlx::query("UPDATE elo_configurations SET is_active = false")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to deactivate configurations: {}", e);
            AuthError::DatabaseError
        })?;

    // Activate this config
    sqlx::query("UPDATE elo_configurations SET is_active = true WHERE version_name = $1")
        .bind(&version_name)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to activate configuration '{}': {}", version_name, e);
            AuthError::DatabaseError
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(serde_json::json!({
        "message": format!("Configuration '{}' activated", version_name)
    })))
}

/// Update an ELO configuration (admin only)
pub async fn update_elo_config(
    State(pool): State<PgPool>,
    Extension(_admin_user): Extension<User>,
    axum::extract::Path(version_name): axum::extract::Path<String>,
    Json(req): Json<CreateEloConfigRequest>,
) -> Result<Json<EloConfigResponse>, AuthError> {
    // Validate inputs (same as create)
    if req.k_factor < MIN_K_FACTOR || req.k_factor > MAX_K_FACTOR {
        return Err(AuthError::InvalidInput(format!(
            "K-factor must be between {} and {}",
            MIN_K_FACTOR, MAX_K_FACTOR
        )));
    }

    if req.starting_elo < MIN_STARTING_ELO || req.starting_elo > MAX_STARTING_ELO {
        return Err(AuthError::InvalidInput(format!(
            "Starting ELO must be between {} and {}",
            MIN_STARTING_ELO, MAX_STARTING_ELO
        )));
    }

    if let Some(ref desc) = req.description
        && desc.len() > MAX_DESCRIPTION_LENGTH
    {
        return Err(AuthError::InvalidInput(format!(
            "Description must be {} characters or less",
            MAX_DESCRIPTION_LENGTH
        )));
    }

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

    // Check if config exists
    let exists: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM elo_configurations WHERE version_name = $1")
            .bind(&version_name)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking configuration: {}", e);
                AuthError::DatabaseError
            })?;

    if exists.is_none() {
        return Err(AuthError::InvalidInput(
            "Configuration not found".to_string(),
        ));
    }

    // Cannot update active configuration
    let is_active: Option<(bool,)> =
        sqlx::query_as("SELECT is_active FROM elo_configurations WHERE version_name = $1")
            .bind(&version_name)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking active status: {}", e);
                AuthError::DatabaseError
            })?;

    if let Some((true,)) = is_active {
        return Err(AuthError::InvalidInput(
            "Cannot update active configuration. Deactivate it first.".to_string(),
        ));
    }

    // Update configuration
    let config: EloConfigResponse = sqlx::query_as(
        "UPDATE elo_configurations
         SET k_factor = $2, starting_elo = $3, base_k_factor = $4,
             new_player_k_bonus = $5, new_player_bonus_period = $6,
             description = $7
         WHERE version_name = $1
         RETURNING id, version_name, k_factor, starting_elo, base_k_factor,
                   new_player_k_bonus, new_player_bonus_period, description,
                   is_active, created_at",
    )
    .bind(&version_name)
    .bind(req.k_factor)
    .bind(req.starting_elo)
    .bind(req.base_k_factor)
    .bind(req.new_player_k_bonus)
    .bind(req.new_player_bonus_period)
    .bind(&req.description)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating ELO configuration: {}", e);
        AuthError::DatabaseError
    })?;

    Ok(Json(config))
}

/// Delete an ELO configuration (admin only)
pub async fn delete_elo_config(
    State(pool): State<PgPool>,
    axum::extract::Path(version_name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Check if config is active
    let is_active: Option<(bool,)> =
        sqlx::query_as("SELECT is_active FROM elo_configurations WHERE version_name = $1")
            .bind(&version_name)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking active status: {}", e);
                AuthError::DatabaseError
            })?;

    if let Some((true,)) = is_active {
        return Err(AuthError::InvalidInput(
            "Cannot delete active configuration. Deactivate it first.".to_string(),
        ));
    }

    // Delete configuration
    let result = sqlx::query("DELETE FROM elo_configurations WHERE version_name = $1")
        .bind(&version_name)
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting configuration: {}", e);
            AuthError::DatabaseError
        })?;

    if result.rows_affected() == 0 {
        return Err(AuthError::InvalidInput(
            "Configuration not found".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "message": format!("Configuration '{}' deleted", version_name)
    })))
}

/// Recalculate ELO with a specific configuration (admin only)
/// This spawns a background task and returns a job ID for tracking progress
pub async fn recalculate_elo(
    State(pool): State<PgPool>,
    Extension(admin_user): Extension<User>,
    axum::extract::Path(version_name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AuthError> {
    // Get configuration
    let config = get_config_by_version(&pool, &version_name)
        .await
        .map_err(|e| {
            tracing::error!(
                "Database error fetching configuration '{}': {}",
                version_name,
                e
            );
            AuthError::DatabaseError
        })?
        .ok_or_else(|| AuthError::InvalidInput("Configuration not found".to_string()))?;

    // Create a job for tracking
    let job_id = create_job(&pool, "elo_recalculation", Some(admin_user.id))
        .await
        .map_err(|e| {
            tracing::error!("Failed to create job: {}", e);
            AuthError::DatabaseError
        })?;

    tracing::info!(
        "Created recalculation job {} for version '{}'",
        job_id,
        version_name
    );

    // Spawn background task
    let pool_clone = pool.clone();
    let version_clone = version_name.clone();
    tokio::spawn(async move {
        tracing::info!("Starting background ELO recalculation for job {}", job_id);

        // Mark job as running
        if let Err(e) = update_job_status(&pool_clone, job_id, JobStatus::Running, None).await {
            tracing::error!("Failed to update job status to running: {}", e);
            return;
        }

        // Perform recalculation
        match recalculate_all_elo(&pool_clone, &config, Some(job_id)).await {
            Ok(_) => {
                tracing::info!(
                    "Successfully completed ELO recalculation for job {}",
                    job_id
                );
                let result = serde_json::json!({
                    "version": version_clone,
                    "message": "Recalculation completed successfully"
                });
                if let Err(e) =
                    update_job_status(&pool_clone, job_id, JobStatus::Completed, Some(result)).await
                {
                    tracing::error!("Failed to update job status to completed: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("ELO recalculation failed for job {}: {}", job_id, e);
                let error_result = serde_json::json!({
                    "error": format!("Recalculation failed: {}", e)
                });
                if let Err(e) =
                    update_job_status(&pool_clone, job_id, JobStatus::Failed, Some(error_result))
                        .await
                {
                    tracing::error!("Failed to update job status to failed: {}", e);
                }
            }
        }
    });

    Ok(Json(serde_json::json!({
        "message": format!("Started ELO recalculation for version '{}'", version_name),
        "job_id": job_id,
        "version": version_name
    })))
}

/// Get job status by ID (admin only)
pub async fn get_job_status(
    State(pool): State<PgPool>,
    axum::extract::Path(job_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<crate::services::jobs::Job>, AuthError> {
    let job = get_job(&pool, job_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching job {}: {}", job_id, e);
            AuthError::DatabaseError
        })?
        .ok_or_else(|| AuthError::InvalidInput("Job not found".to_string()))?;

    Ok(Json(job))
}
