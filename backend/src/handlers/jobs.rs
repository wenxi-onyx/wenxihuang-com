use crate::error::AppError;
use crate::models::user::User;
use crate::services::jobs;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

/// Get job status (for job creator only)
pub async fn get_job_status(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<jobs::Job>, AppError> {
    let user_id = user.id;
    let job = jobs::get_job(&pool, job_id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", job_id)))?;

    // Check if user is the job creator
    if job.created_by != Some(user_id) {
        return Err(AppError::Forbidden(format!(
            "You do not have permission to view job {}",
            job_id
        )));
    }

    Ok(Json(job))
}
