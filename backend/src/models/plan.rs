use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Plan {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub content_hash: String,
    pub owner_id: Uuid,
    pub is_public: bool,
    pub current_version: i32,
    pub file_size_bytes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlanVersion {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub version_number: i32,
    pub content: String,
    pub content_hash: String,
    pub change_description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlanComment {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub plan_version: i32,
    pub author_id: Uuid,
    pub line_start: i32,
    pub line_end: i32,
    pub comment_text: String,
    pub is_resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub resolution_action: Option<String>, // 'accepted' or 'rejected'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiIntegrationJob {
    pub id: Uuid,
    pub job_id: Uuid,
    pub comment_id: Uuid,
    pub plan_id: Uuid,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_cost_usd: Option<rust_decimal::Decimal>,
    pub model_used: Option<String>,
    pub ai_response: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub encrypted_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// DTOs for API requests/responses

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub title: String,
    pub content: String,
    pub is_public: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub line_start: i32,
    pub line_end: i32,
    pub comment_text: String,
}

#[derive(Debug, Serialize)]
pub struct PlanWithComments {
    pub plan: Plan,
    pub comments: Vec<CommentWithAuthor>,
    pub owner_username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithAuthor {
    #[serde(flatten)]
    pub comment: PlanComment,
    pub author_username: String,
    pub author_first_name: Option<String>,
    pub author_last_name: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlanListItem {
    pub id: Uuid,
    pub title: String,
    pub owner_id: Uuid,
    pub owner_username: String,
    pub current_version: i32,
    pub comment_count: i64,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AcceptCommentResponse {
    pub job_id: Uuid,
    pub message: String,
}
