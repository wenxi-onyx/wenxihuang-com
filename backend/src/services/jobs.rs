use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar")]
pub enum JobStatus {
    #[sqlx(rename = "pending")]
    Pending,
    #[sqlx(rename = "running")]
    Running,
    #[sqlx(rename = "completed")]
    Completed,
    #[sqlx(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: String,
    pub status: JobStatus,
    pub progress: i32,
    pub total_items: Option<i32>,
    pub processed_items: i32,
    pub result_data: Option<serde_json::Value>,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Create a new job
pub async fn create_job(
    pool: &PgPool,
    job_type: &str,
    created_by: Option<Uuid>,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO jobs (job_type, status, created_by)
         VALUES ($1, 'pending', $2)
         RETURNING id",
    )
    .bind(job_type)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// Update job status
pub async fn update_job_status(
    pool: &PgPool,
    job_id: Uuid,
    status: JobStatus,
    result_data: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();

    match status {
        JobStatus::Running => {
            sqlx::query("UPDATE jobs SET status = 'running', started_at = $1 WHERE id = $2")
                .bind(now)
                .bind(job_id)
                .execute(pool)
                .await?;
        }
        JobStatus::Completed | JobStatus::Failed => {
            sqlx::query(
                "UPDATE jobs SET status = $1, completed_at = $2, progress = 100, result_data = $3 WHERE id = $4"
            )
            .bind(match status {
                JobStatus::Completed => "completed",
                JobStatus::Failed => "failed",
                _ => unreachable!(),
            })
            .bind(now)
            .bind(result_data)
            .bind(job_id)
            .execute(pool)
            .await?;
        }
        _ => {
            sqlx::query("UPDATE jobs SET status = $1 WHERE id = $2")
                .bind(match status {
                    JobStatus::Pending => "pending",
                    _ => unreachable!(),
                })
                .bind(job_id)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

/// Update job progress
pub async fn update_job_progress(
    pool: &PgPool,
    job_id: Uuid,
    processed_items: i32,
    total_items: i32,
) -> Result<(), sqlx::Error> {
    let progress = ((processed_items as f64 / total_items as f64) * 100.0) as i32;

    sqlx::query(
        "UPDATE jobs SET processed_items = $1, total_items = $2, progress = $3 WHERE id = $4",
    )
    .bind(processed_items)
    .bind(total_items)
    .bind(progress)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get job by ID
pub async fn get_job(pool: &PgPool, job_id: Uuid) -> Result<Option<Job>, sqlx::Error> {
    let job = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            i32,
            Option<i32>,
            i32,
            Option<serde_json::Value>,
            Option<Uuid>,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        "SELECT id, job_type, status, progress, total_items, processed_items,
                result_data, created_by, created_at, started_at, completed_at
         FROM jobs WHERE id = $1",
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?
    .map(
        |(
            id,
            job_type,
            status,
            progress,
            total_items,
            processed_items,
            result_data,
            created_by,
            created_at,
            started_at,
            completed_at,
        )| {
            let status = match status.as_str() {
                "pending" => JobStatus::Pending,
                "running" => JobStatus::Running,
                "completed" => JobStatus::Completed,
                "failed" => JobStatus::Failed,
                _ => JobStatus::Pending,
            };

            Job {
                id,
                job_type,
                status,
                progress,
                total_items,
                processed_items,
                result_data,
                created_by,
                created_at,
                started_at,
                completed_at,
            }
        },
    );

    Ok(job)
}
