use crate::error::AppError;
use crate::models::plan::{
    AcceptCommentResponse, CommentWithAuthor, CreateCommentRequest, CreatePlanRequest, Plan,
    PlanComment, PlanListItem, PlanWithComments,
};
use crate::models::user::User;
use crate::services::{ai_integration, encryption, jobs};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 1_048_576; // 1MB

/// Upload a new plan
pub async fn upload_plan(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Json(request): Json<CreatePlanRequest>,
) -> Result<Json<Plan>, AppError> {
    let user_id = user.id;
    // Validate file size
    let file_size = request.content.len();
    if file_size > MAX_FILE_SIZE {
        return Err(AppError::FileSizeTooLarge(format!(
            "File size ({} bytes) exceeds maximum allowed size (1MB)",
            file_size
        )));
    }

    // Validate title
    if request.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".to_string()));
    }

    if request.title.len() > 500 {
        return Err(AppError::BadRequest(
            "Title cannot exceed 500 characters".to_string(),
        ));
    }

    // Calculate content hash
    let mut hasher = Sha256::new();
    hasher.update(request.content.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());

    let is_public = request.is_public.unwrap_or(true);

    // Check for duplicate content from same user
    let existing = sqlx::query_as::<_, (Uuid,)>(
        r#"
        SELECT id FROM plans
        WHERE content_hash = $1 AND owner_id = $2
        LIMIT 1
        "#,
    )
    .bind(&content_hash)
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if let Some((existing_id,)) = existing {
        return Err(AppError::BadRequest(format!(
            "A plan with identical content already exists (ID: {})",
            existing_id
        )));
    }

    // Start a transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Insert the plan
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        INSERT INTO plans (title, content, content_hash, owner_id, is_public, file_size_bytes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&request.title)
    .bind(&request.content)
    .bind(&content_hash)
    .bind(user_id)
    .bind(is_public)
    .bind(file_size as i32)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create the first version
    sqlx::query(
        r#"
        INSERT INTO plan_versions (plan_id, version_number, content, content_hash, created_by, change_description)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(plan.id)
    .bind(1)
    .bind(&request.content)
    .bind(&content_hash)
    .bind(user_id)
    .bind("Initial version")
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(plan))
}

/// List all public plans
pub async fn list_plans(State(pool): State<PgPool>) -> Result<Json<Vec<PlanListItem>>, AppError> {
    let plans = sqlx::query_as::<_, PlanListItem>(
        r#"
        SELECT
            p.id,
            p.title,
            p.owner_id,
            u.username as owner_username,
            p.current_version,
            p.is_public,
            p.created_at,
            p.updated_at,
            COUNT(pc.id) as comment_count
        FROM plans p
        INNER JOIN users u ON p.owner_id = u.id
        LEFT JOIN plan_comments pc ON p.id = pc.plan_id
        WHERE p.is_public = true
        GROUP BY p.id, u.username
        ORDER BY p.created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(Json(plans))
}

/// Get a specific plan with its comments
pub async fn get_plan(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<PlanWithComments>, AppError> {
    // Get the plan
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        SELECT * FROM plans WHERE id = $1 AND is_public = true
        "#,
    )
    .bind(plan_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Plan {} not found or not public", plan_id)))?;

    // Get the owner username
    let owner = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT username FROM users WHERE id = $1
        "#,
    )
    .bind(plan.owner_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Get comments with author info
    let comments = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            i32,
            Uuid,
            i32,
            i32,
            String,
            bool,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<Uuid>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            pc.id, pc.plan_id, pc.plan_version, pc.author_id,
            pc.line_start, pc.line_end, pc.comment_text, pc.is_resolved,
            pc.resolved_at, pc.resolved_by, pc.resolution_action,
            pc.created_at, pc.updated_at,
            u.username, u.first_name, u.last_name
        FROM plan_comments pc
        INNER JOIN users u ON pc.author_id = u.id
        WHERE pc.plan_id = $1
        ORDER BY pc.line_start ASC, pc.created_at ASC
        "#,
    )
    .bind(plan_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let comments_with_authors: Vec<CommentWithAuthor> = comments
        .into_iter()
        .map(
            |(
                id,
                plan_id,
                plan_version,
                author_id,
                line_start,
                line_end,
                comment_text,
                is_resolved,
                resolved_at,
                resolved_by,
                resolution_action,
                created_at,
                updated_at,
                username,
                first_name,
                last_name,
            )| {
                CommentWithAuthor {
                    comment: PlanComment {
                        id,
                        plan_id,
                        plan_version,
                        author_id,
                        line_start,
                        line_end,
                        comment_text,
                        is_resolved,
                        resolved_at,
                        resolved_by,
                        resolution_action,
                        created_at,
                        updated_at,
                    },
                    author_username: username,
                    author_first_name: first_name,
                    author_last_name: last_name,
                }
            },
        )
        .collect();

    Ok(Json(PlanWithComments {
        plan,
        comments: comments_with_authors,
        owner_username: owner.0,
    }))
}

/// Download plan content with proper headers
pub async fn download_plan(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Response, AppError> {
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        SELECT * FROM plans WHERE id = $1 AND is_public = true
        "#,
    )
    .bind(plan_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Plan {} not found", plan_id)))?;

    // Sanitize filename for Content-Disposition header
    let safe_filename = plan
        .title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}.md\"", safe_filename),
            ),
        ],
        plan.content,
    )
        .into_response())
}

/// Create a comment on a plan
pub async fn create_comment(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Extension(broadcast_state): Extension<crate::services::plan_broadcast::PlanBroadcastState>,
    Path(plan_id): Path<Uuid>,
    Json(request): Json<CreateCommentRequest>,
) -> Result<Json<PlanComment>, AppError> {
    let user_id = user.id;
    // Validate line numbers
    if request.line_start < 1 || request.line_end < request.line_start {
        return Err(AppError::BadRequest("Invalid line range".to_string()));
    }

    // Validate comment text
    if request.comment_text.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Comment text cannot be empty".to_string(),
        ));
    }

    // Get the plan to ensure it exists and get current version
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        SELECT * FROM plans WHERE id = $1
        "#,
    )
    .bind(plan_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Plan {} not found", plan_id)))?;

    // Validate line numbers against plan content
    let line_count = plan.content.lines().count() as i32;
    if request.line_end > line_count {
        return Err(AppError::BadRequest(format!(
            "Line number {} exceeds plan length ({})",
            request.line_end, line_count
        )));
    }

    // Insert the comment
    let comment = sqlx::query_as::<_, PlanComment>(
        r#"
        INSERT INTO plan_comments (plan_id, plan_version, author_id, line_start, line_end, comment_text)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(plan_id)
    .bind(plan.current_version)
    .bind(user_id)
    .bind(request.line_start)
    .bind(request.line_end)
    .bind(&request.comment_text)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Use existing User extension data instead of fetching from database again
    let comment_with_author = CommentWithAuthor {
        comment: comment.clone(),
        author_username: user.username.clone(),
        author_first_name: user.first_name.clone(),
        author_last_name: user.last_name.clone(),
    };

    // Broadcast WebSocket update
    broadcast_state
        .broadcast(
            &plan_id.to_string(),
            crate::handlers::plan_ws::PlanMessage::CommentAdded {
                plan_id: plan_id.to_string(),
                comment: comment_with_author,
            },
        )
        .await;

    Ok(Json(comment))
}

/// Accept a comment and trigger AI integration
pub async fn accept_comment(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Extension(broadcast_state): Extension<crate::services::plan_broadcast::PlanBroadcastState>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<AcceptCommentResponse>, AppError> {
    let user_id = user.id;
    // Get the comment
    let comment = sqlx::query_as::<_, PlanComment>(
        r#"
        SELECT * FROM plan_comments WHERE id = $1
        "#,
    )
    .bind(comment_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Comment {} not found", comment_id)))?;

    // Get the plan
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        SELECT * FROM plans WHERE id = $1
        "#,
    )
    .bind(comment.plan_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Check if user is the plan owner BEFORE checking rate limit
    if plan.owner_id != user_id {
        return Err(AppError::Forbidden(format!(
            "Only the plan owner can accept comments on plan {}",
            plan.id
        )));
    }

    // Check if comment is already resolved
    if comment.is_resolved {
        return Err(AppError::BadRequest(format!(
            "Comment {} is already resolved",
            comment_id
        )));
    }

    // Create a job for AI integration
    let job_id = jobs::create_job(&pool, "ai_integration", Some(user_id)).await?;

    // Mark comment as resolved and fetch author info in a single query
    let result = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            i32,
            Uuid,
            i32,
            i32,
            String,
            bool,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<Uuid>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        UPDATE plan_comments pc
        SET is_resolved = true,
            resolved_at = NOW(),
            resolved_by = $1,
            resolution_action = 'accepted'
        FROM users u
        WHERE pc.id = $2 AND pc.author_id = u.id
        RETURNING pc.id, pc.plan_id, pc.plan_version, pc.author_id,
                  pc.line_start, pc.line_end, pc.comment_text, pc.is_resolved,
                  pc.resolved_at, pc.resolved_by, pc.resolution_action,
                  pc.created_at, pc.updated_at,
                  u.username, u.first_name, u.last_name
        "#,
    )
    .bind(user_id)
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let updated_comment = PlanComment {
        id: result.0,
        plan_id: result.1,
        plan_version: result.2,
        author_id: result.3,
        line_start: result.4,
        line_end: result.5,
        comment_text: result.6,
        is_resolved: result.7,
        resolved_at: result.8,
        resolved_by: result.9,
        resolution_action: result.10,
        created_at: result.11,
        updated_at: result.12,
    };

    let comment_with_author = CommentWithAuthor {
        comment: updated_comment,
        author_username: result.13,
        author_first_name: result.14,
        author_last_name: result.15,
    };

    // Broadcast WebSocket update
    let plan_id_str = plan.id.to_string();
    broadcast_state
        .broadcast(
            &plan_id_str,
            crate::handlers::plan_ws::PlanMessage::CommentUpdated {
                plan_id: plan_id_str.clone(),
                comment: comment_with_author,
            },
        )
        .await;

    // Get user's API key before spawning task
    let api_key_result = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT encrypted_key FROM user_api_keys
        WHERE user_id = $1 AND provider = 'anthropic'
        "#,
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let encrypted_key = api_key_result.map(|(key,)| key).ok_or_else(|| {
        AppError::BadRequest(
            "No Anthropic API key found. Please add your API key in Settings.".to_string(),
        )
    })?;

    let api_key = encryption::decrypt(&encrypted_key)
        .map_err(|_| AppError::Internal("Failed to decrypt API key".to_string()))?;

    // Spawn async task to process AI integration
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let result =
            process_ai_integration(&pool_clone, job_id, comment_id, &plan, &comment, &api_key)
                .await;

        if let Err(e) = result {
            tracing::error!("AI integration job {} failed: {:?}", job_id, e);
            let _ = jobs::mark_job_failed(&pool_clone, job_id, &format!("{:?}", e)).await;
        }
    });

    Ok(Json(AcceptCommentResponse {
        job_id,
        message: "AI integration job started".to_string(),
    }))
}

/// Reject a comment
pub async fn reject_comment(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    Extension(broadcast_state): Extension<crate::services::plan_broadcast::PlanBroadcastState>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = user.id;
    // Get the comment
    let comment = sqlx::query_as::<_, PlanComment>(
        r#"
        SELECT * FROM plan_comments WHERE id = $1
        "#,
    )
    .bind(comment_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::NotFound(format!("Comment {} not found", comment_id)))?;

    // Get the plan
    let plan = sqlx::query_as::<_, Plan>(
        r#"
        SELECT * FROM plans WHERE id = $1
        "#,
    )
    .bind(comment.plan_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Check if user is the plan owner
    if plan.owner_id != user_id {
        return Err(AppError::Forbidden(format!(
            "Only the plan owner can reject comments on plan {}",
            plan.id
        )));
    }

    // Check if comment is already resolved
    if comment.is_resolved {
        return Err(AppError::BadRequest(format!(
            "Comment {} is already resolved",
            comment_id
        )));
    }

    // Mark comment as resolved with rejected action and fetch author info in a single query
    let result = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            i32,
            Uuid,
            i32,
            i32,
            String,
            bool,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<Uuid>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        UPDATE plan_comments pc
        SET is_resolved = true,
            resolved_at = NOW(),
            resolved_by = $1,
            resolution_action = 'rejected'
        FROM users u
        WHERE pc.id = $2 AND pc.author_id = u.id
        RETURNING pc.id, pc.plan_id, pc.plan_version, pc.author_id,
                  pc.line_start, pc.line_end, pc.comment_text, pc.is_resolved,
                  pc.resolved_at, pc.resolved_by, pc.resolution_action,
                  pc.created_at, pc.updated_at,
                  u.username, u.first_name, u.last_name
        "#,
    )
    .bind(user_id)
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let updated_comment = PlanComment {
        id: result.0,
        plan_id: result.1,
        plan_version: result.2,
        author_id: result.3,
        line_start: result.4,
        line_end: result.5,
        comment_text: result.6,
        is_resolved: result.7,
        resolved_at: result.8,
        resolved_by: result.9,
        resolution_action: result.10,
        created_at: result.11,
        updated_at: result.12,
    };

    let comment_with_author = CommentWithAuthor {
        comment: updated_comment,
        author_username: result.13,
        author_first_name: result.14,
        author_last_name: result.15,
    };

    // Broadcast WebSocket update
    let plan_id_str = plan.id.to_string();
    broadcast_state
        .broadcast(
            &plan_id_str,
            crate::handlers::plan_ws::PlanMessage::CommentUpdated {
                plan_id: plan_id_str.clone(),
                comment: comment_with_author,
            },
        )
        .await;

    Ok(Json(json!({
        "message": "Comment rejected successfully"
    })))
}

/// Process AI integration in the background
async fn process_ai_integration(
    pool: &PgPool,
    job_id: Uuid,
    comment_id: Uuid,
    plan: &Plan,
    comment: &PlanComment,
    api_key: &str,
) -> Result<(), AppError> {
    // Update job status to running
    jobs::update_job_progress(pool, job_id, "running", 10, None).await?;

    // Call AI service
    let ai_response = ai_integration::generate_plan_changes(
        api_key,
        &plan.content,
        &comment.comment_text,
        comment.line_start,
        comment.line_end,
    )
    .await?;

    jobs::update_job_progress(pool, job_id, "running", 50, None).await?;

    // Apply the changes
    let new_content = ai_integration::apply_changes_to_plan(
        &plan.content,
        &ai_response.text,
        comment.line_start,
        comment.line_end,
    );

    // Calculate new hash
    let mut hasher = Sha256::new();
    hasher.update(new_content.as_bytes());
    let new_hash = format!("{:x}", hasher.finalize());

    jobs::update_job_progress(pool, job_id, "running", 75, None).await?;

    // Start transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Update the plan
    let new_version = plan.current_version + 1;
    sqlx::query(
        r#"
        UPDATE plans
        SET content = $1,
            content_hash = $2,
            current_version = $3,
            file_size_bytes = $4
        WHERE id = $5
        "#,
    )
    .bind(&new_content)
    .bind(&new_hash)
    .bind(new_version)
    .bind(new_content.len() as i32)
    .bind(plan.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create new version record
    sqlx::query(
        r#"
        INSERT INTO plan_versions (plan_id, version_number, content, content_hash, created_by, change_description)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(plan.id)
    .bind(new_version)
    .bind(&new_content)
    .bind(&new_hash)
    .bind(plan.owner_id)
    .bind(format!("AI-generated changes for comment: {}", comment.comment_text.chars().take(100).collect::<String>()))
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Store AI integration record
    sqlx::query(
        r#"
        INSERT INTO ai_integration_jobs (job_id, comment_id, plan_id, prompt_tokens, completion_tokens, model_used, ai_response)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(job_id)
    .bind(comment_id)
    .bind(plan.id)
    .bind(ai_response.prompt_tokens)
    .bind(ai_response.completion_tokens)
    .bind(&ai_response.model_used)
    .bind(&ai_response.text)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Mark job as completed
    jobs::mark_job_completed(
        pool,
        job_id,
        Some(json!({
            "new_version": new_version,
            "tokens_used": ai_response.prompt_tokens + ai_response.completion_tokens
        })),
    )
    .await?;

    Ok(())
}
