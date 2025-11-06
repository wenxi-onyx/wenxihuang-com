# Multiplayer ChatGPT - Comprehensive Implementation Plan

## Overview

This plan outlines the implementation of a collaborative plan review system where users can upload engineering plans (markdown files), share them publicly, receive comments with inline highlighting, and integrate feedback using the Anthropic API.

---

## 1. Database Schema Design

### 1.1 New Tables

#### `plans` Table
Stores markdown plan files and metadata.

```sql
CREATE TABLE plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    filename VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    is_public BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plans_user_id ON plans(user_id);
CREATE INDEX idx_plans_created_at ON plans(created_at DESC);
```

**Design Decisions:**
- Store content as TEXT (not BYTEA) since markdown is text-based and we need to search/index it
- `is_public` flag for future private plans support
- `user_id` references existing `users` table for ownership
- Indexes on user_id for "my plans" queries and created_at for sorting

---

#### `plan_versions` Table
Track plan revisions when AI updates are applied.

```sql
CREATE TYPE version_source AS ENUM ('manual', 'ai_comment', 'ai_discussion');

CREATE TABLE plan_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    source version_source NOT NULL,
    comment_id UUID REFERENCES plan_comments(id) ON DELETE SET NULL,
    summary TEXT,  -- AI-generated summary of changes
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(plan_id, version_number)
);

CREATE INDEX idx_plan_versions_plan_id ON plan_versions(plan_id);
```

**Design Decisions:**
- Full content snapshot per version (storage is cheap, recovery is critical)
- `version_number` starts at 1, increments per plan
- `source` tracks whether change was manual edit, AI from comment, or AI from discussion
- `comment_id` links to the comment that triggered the AI update (nullable)
- `summary` stores AI-generated explanation of what changed

---

#### `plan_comments` Table
Stores comments with text selection anchoring.

```sql
CREATE TYPE comment_status AS ENUM ('pending', 'accepted', 'rejected', 'debating');

CREATE TABLE plan_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,

    -- Text anchoring (using character offsets)
    start_offset INTEGER NOT NULL,
    end_offset INTEGER NOT NULL,
    selected_text TEXT NOT NULL,  -- Store for anchor recovery

    -- Plan version when comment was created
    plan_version INTEGER NOT NULL,

    status comment_status NOT NULL DEFAULT 'pending',
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plan_comments_plan_id ON plan_comments(plan_id);
CREATE INDEX idx_plan_comments_status ON plan_comments(status);
```

**Design Decisions:**
- Character offsets (`start_offset`, `end_offset`) for precise text anchoring
- Store `selected_text` as a fallback for re-anchoring if plan content changes
- `plan_version` tracks which version the comment was made on
- `status` flow: `pending` → `accepted`/`rejected`/`debating`
- `resolved_by` tracks who (plan owner) accepted/rejected

---

#### `comment_discussions` Table
Thread-like discussions on comments.

```sql
CREATE TABLE comment_discussions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    comment_id UUID NOT NULL REFERENCES plan_comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comment_discussions_comment_id ON comment_discussions(comment_id);
CREATE INDEX idx_comment_discussions_created_at ON comment_discussions(created_at);
```

**Design Decisions:**
- Simple append-only thread model
- No nested replies (flat discussion per comment)
- Chronological ordering via `created_at` index

---

### 1.2 Migration File

**File:** `backend/migrations/006_create_multiplayer_chatgpt_system.sql`

This will include all tables above plus triggers for `updated_at` auto-update (following existing pattern from table tennis tables).

---

## 2. Backend Architecture

### 2.1 New Models

**File:** `backend/src/models/plan.rs`

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub filename: String,
    pub content: String,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanVersion {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub version_number: i32,
    pub content: String,
    pub source: VersionSource,
    pub comment_id: Option<Uuid>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "version_source", rename_all = "snake_case")]
pub enum VersionSource {
    Manual,
    AiComment,
    AiDiscussion,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanComment {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub start_offset: i32,
    pub end_offset: i32,
    pub selected_text: String,
    pub plan_version: i32,
    pub status: CommentStatus,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "comment_status", rename_all = "snake_case")]
pub enum CommentStatus {
    Pending,
    Accepted,
    Rejected,
    Debating,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommentDiscussion {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

// DTOs (Data Transfer Objects)
#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub title: String,
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanRequest {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub start_offset: i32,
    pub end_offset: i32,
    pub selected_text: String,
}

#[derive(Debug, Deserialize)]
pub struct AddDiscussionMessageRequest {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AcceptCommentRequest {
    pub summary: Option<String>,  // Optional user-provided summary
}

// Response DTOs with joined data
#[derive(Debug, Serialize)]
pub struct PlanWithAuthor {
    #[serde(flatten)]
    pub plan: Plan,
    pub author_username: String,
    pub author_first_name: Option<String>,
    pub author_last_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentWithAuthorAndDiscussions {
    #[serde(flatten)]
    pub comment: PlanComment,
    pub author_username: String,
    pub discussions: Vec<DiscussionWithAuthor>,
}

#[derive(Debug, Serialize)]
pub struct DiscussionWithAuthor {
    #[serde(flatten)]
    pub discussion: CommentDiscussion,
    pub author_username: String,
}
```

Update `backend/src/models/mod.rs`:
```rust
pub mod user;
pub mod plan;  // Add this line
```

---

### 2.2 New Services

#### **File:** `backend/src/services/anthropic.rs`

Handles all Anthropic API interactions.

```rust
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    text: String,
}

pub async fn integrate_comment_into_plan(
    plan_content: &str,
    comment_content: &str,
    selected_text: &str,
) -> Result<String, String> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let prompt = format!(
        r#"You are an AI assistant helping to integrate feedback into an engineering plan.

ORIGINAL PLAN:
```markdown
{}
```

HIGHLIGHTED SECTION:
"{}"

FEEDBACK/COMMENT:
{}

INSTRUCTIONS:
Integrate this feedback into the plan. Modify the relevant section while keeping the rest of the plan intact. Return ONLY the updated markdown plan, with no additional commentary.
"#,
        plan_content, selected_text, comment_content
    );

    let request_body = AnthropicRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 8192,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error: {}", error_text));
    }

    let anthropic_response: AnthropicResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    anthropic_response
        .content
        .first()
        .map(|c| c.text.clone())
        .ok_or_else(|| "No content in response".to_string())
}

pub async fn integrate_discussion_into_plan(
    plan_content: &str,
    comment_content: &str,
    selected_text: &str,
    discussion_messages: Vec<(String, String)>,  // (username, message)
    summary: &str,
) -> Result<String, String> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let discussion_text = discussion_messages
        .iter()
        .map(|(user, msg)| format!("**{}:** {}", user, msg))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        r#"You are an AI assistant helping to integrate feedback into an engineering plan.

ORIGINAL PLAN:
```markdown
{}
```

HIGHLIGHTED SECTION:
"{}"

ORIGINAL COMMENT:
{}

DISCUSSION THREAD:
{}

SUMMARY OF DECISION:
{}

INSTRUCTIONS:
Based on the discussion and final summary, integrate the changes into the plan. Modify the relevant section while keeping the rest of the plan intact. Return ONLY the updated markdown plan, with no additional commentary.
"#,
        plan_content, selected_text, comment_content, discussion_text, summary
    );

    let request_body = AnthropicRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 8192,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error: {}", error_text));
    }

    let anthropic_response: AnthropicResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    anthropic_response
        .content
        .first()
        .map(|c| c.text.clone())
        .ok_or_else(|| "No content in response".to_string())
}
```

Update `backend/src/services/mod.rs`:
```rust
pub mod session;
pub mod password;
pub mod elo;
pub mod seasons;
pub mod jobs;
pub mod anthropic;  // Add this line
```

Add to `backend/Cargo.toml`:
```toml
reqwest = { version = "0.12", features = ["json"] }
```

---

#### **File:** `backend/src/services/plan_service.rs`

Business logic for plan operations.

```rust
use crate::models::plan::*;
use crate::services::anthropic;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_plan_version(
    pool: &PgPool,
    plan_id: Uuid,
    content: &str,
    source: VersionSource,
    comment_id: Option<Uuid>,
    summary: Option<String>,
) -> Result<PlanVersion, sqlx::Error> {
    // Get current max version number
    let current_version: Option<i32> = sqlx::query_scalar!(
        "SELECT MAX(version_number) FROM plan_versions WHERE plan_id = $1",
        plan_id
    )
    .fetch_one(pool)
    .await?;

    let new_version = current_version.unwrap_or(0) + 1;

    let version = sqlx::query_as!(
        PlanVersion,
        r#"
        INSERT INTO plan_versions (plan_id, version_number, content, source, comment_id, summary)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, plan_id, version_number, content,
                  source as "source: VersionSource",
                  comment_id, summary, created_at
        "#,
        plan_id,
        new_version,
        content,
        source as VersionSource,
        comment_id,
        summary
    )
    .fetch_one(pool)
    .await?;

    Ok(version)
}

pub async fn accept_comment_and_integrate(
    pool: &PgPool,
    comment_id: Uuid,
    user_id: Uuid,
) -> Result<Plan, String> {
    // Get comment with plan content
    let comment = sqlx::query_as!(
        PlanComment,
        r#"
        SELECT id, plan_id, user_id, content, start_offset, end_offset,
               selected_text, plan_version,
               status as "status: CommentStatus",
               resolved_at, resolved_by, created_at, updated_at
        FROM plan_comments
        WHERE id = $1
        "#,
        comment_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Comment not found: {}", e))?;

    // Get plan
    let plan = sqlx::query_as!(
        Plan,
        "SELECT * FROM plans WHERE id = $1",
        comment.plan_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Plan not found: {}", e))?;

    // Verify user is plan owner
    if plan.user_id != user_id {
        return Err("Only plan owner can accept comments".to_string());
    }

    // Call Anthropic API to integrate comment
    let updated_content = anthropic::integrate_comment_into_plan(
        &plan.content,
        &comment.content,
        &comment.selected_text,
    )
    .await?;

    // Update plan content
    sqlx::query!(
        "UPDATE plans SET content = $1, updated_at = NOW() WHERE id = $2",
        updated_content,
        plan.id
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update plan: {}", e))?;

    // Create version
    create_plan_version(
        pool,
        plan.id,
        &updated_content,
        VersionSource::AiComment,
        Some(comment_id),
        None,
    )
    .await
    .map_err(|e| format!("Failed to create version: {}", e))?;

    // Mark comment as accepted
    sqlx::query!(
        "UPDATE plan_comments SET status = 'accepted', resolved_at = NOW(), resolved_by = $1 WHERE id = $2",
        user_id,
        comment_id
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update comment: {}", e))?;

    // Return updated plan
    let updated_plan = sqlx::query_as!(
        Plan,
        "SELECT * FROM plans WHERE id = $1",
        plan.id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to fetch updated plan: {}", e))?;

    Ok(updated_plan)
}

pub async fn accept_discussion_and_integrate(
    pool: &PgPool,
    comment_id: Uuid,
    user_id: Uuid,
    summary: String,
) -> Result<Plan, String> {
    // Similar to above but calls integrate_discussion_into_plan
    // Fetch discussion messages and pass to API
    // Implementation details follow same pattern
    todo!("Implement discussion integration")
}
```

---

### 2.3 New Handlers

#### **File:** `backend/src/handlers/plans.rs`

REST API handlers for plan operations.

```rust
use crate::middleware::auth::User;
use crate::models::plan::*;
use crate::services::plan_service;
use axum::{
    extract::{Path, State, Multipart},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// List all public plans (paginated)
pub async fn list_plans(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<PlanWithAuthor>>, (StatusCode, String)> {
    let plans = sqlx::query_as!(
        PlanWithAuthor,
        r#"
        SELECT p.id, p.user_id, p.title, p.filename, p.content, p.is_public,
               p.created_at, p.updated_at,
               u.username as author_username,
               u.first_name as author_first_name,
               u.last_name as author_last_name
        FROM plans p
        JOIN users u ON p.user_id = u.id
        WHERE p.is_public = true
        ORDER BY p.created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(plans))
}

// Get plans by username (folder view)
pub async fn get_user_plans(
    State(pool): State<PgPool>,
    Path(username): Path<String>,
) -> Result<Json<Vec<Plan>>, (StatusCode, String)> {
    let plans = sqlx::query_as!(
        Plan,
        r#"
        SELECT p.*
        FROM plans p
        JOIN users u ON p.user_id = u.id
        WHERE u.username = $1 AND p.is_public = true
        ORDER BY p.created_at DESC
        "#,
        username
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(plans))
}

// Get single plan with comments
pub async fn get_plan(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let plan = sqlx::query_as!(
        Plan,
        "SELECT * FROM plans WHERE id = $1 AND is_public = true",
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    let comments = sqlx::query_as!(
        CommentWithAuthorAndDiscussions,
        r#"
        SELECT c.id, c.plan_id, c.user_id, c.content, c.start_offset, c.end_offset,
               c.selected_text, c.plan_version,
               c.status as "status!: CommentStatus",
               c.resolved_at, c.resolved_by, c.created_at, c.updated_at,
               u.username as author_username,
               ARRAY[]::UUID[] as "discussions!"
        FROM plan_comments c
        JOIN users u ON c.user_id = u.id
        WHERE c.plan_id = $1
        ORDER BY c.created_at DESC
        "#,
        plan_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "plan": plan,
        "comments": comments
    })))
}

// Upload/create plan (authenticated)
pub async fn create_plan(
    user: User,
    State(pool): State<PgPool>,
    Json(req): Json<CreatePlanRequest>,
) -> Result<Json<Plan>, (StatusCode, String)> {
    let plan = sqlx::query_as!(
        Plan,
        r#"
        INSERT INTO plans (user_id, title, filename, content)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        user.id,
        req.title,
        req.filename,
        req.content
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create initial version
    plan_service::create_plan_version(
        &pool,
        plan.id,
        &plan.content,
        VersionSource::Manual,
        None,
        Some("Initial version".to_string()),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(plan))
}

// Create comment on plan
pub async fn create_comment(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<PlanComment>, (StatusCode, String)> {
    // Get current plan version
    let plan = sqlx::query_as!(
        Plan,
        "SELECT * FROM plans WHERE id = $1",
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    let current_version: i32 = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(version_number), 1) FROM plan_versions WHERE plan_id = $1",
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(1);

    let comment = sqlx::query_as!(
        PlanComment,
        r#"
        INSERT INTO plan_comments (plan_id, user_id, content, start_offset, end_offset, selected_text, plan_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, plan_id, user_id, content, start_offset, end_offset,
                  selected_text, plan_version,
                  status as "status!: CommentStatus",
                  resolved_at, resolved_by, created_at, updated_at
        "#,
        plan_id,
        user.id,
        req.content,
        req.start_offset,
        req.end_offset,
        req.selected_text,
        current_version
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(comment))
}

// Accept comment (plan owner only) - triggers AI integration
pub async fn accept_comment(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let updated_plan = plan_service::accept_comment_and_integrate(&pool, comment_id, user.id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(json!({
        "message": "Comment accepted and integrated",
        "plan": updated_plan
    })))
}

// Reject comment (plan owner only)
pub async fn reject_comment(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Verify ownership via plan
    let comment = sqlx::query!(
        r#"
        SELECT c.id, p.user_id as plan_owner_id
        FROM plan_comments c
        JOIN plans p ON c.plan_id = p.id
        WHERE c.id = $1
        "#,
        comment_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Comment not found".to_string()))?;

    if comment.plan_owner_id != user.id {
        return Err((StatusCode::FORBIDDEN, "Only plan owner can reject comments".to_string()));
    }

    sqlx::query!(
        "UPDATE plan_comments SET status = 'rejected', resolved_at = NOW(), resolved_by = $1 WHERE id = $2",
        user.id,
        comment_id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "message": "Comment rejected"
    })))
}

// Add discussion message to comment
pub async fn add_discussion_message(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
    Json(req): Json<AddDiscussionMessageRequest>,
) -> Result<Json<CommentDiscussion>, (StatusCode, String)> {
    let discussion = sqlx::query_as!(
        CommentDiscussion,
        r#"
        INSERT INTO comment_discussions (comment_id, user_id, message)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        comment_id,
        user.id,
        req.message
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update comment status to 'debating' if still pending
    sqlx::query!(
        "UPDATE plan_comments SET status = 'debating' WHERE id = $1 AND status = 'pending'",
        comment_id
    )
    .execute(&pool)
    .await
    .ok();

    Ok(Json(discussion))
}
```

Update `backend/src/handlers/mod.rs`:
```rust
pub mod auth;
pub mod user;
pub mod admin;
pub mod players;
pub mod matches;
pub mod seasons;
pub mod elo;
pub mod plans;  // Add this line
```

---

### 2.4 Route Registration

Update `backend/src/main.rs` to add new routes:

```rust
// Add after existing imports
use crate::handlers::plans;

// In the router setup (after existing routes):
let app = Router::new()
    // ... existing routes ...

    // Public plan routes
    .route("/api/plans", get(plans::list_plans))
    .route("/api/plans/:plan_id", get(plans::get_plan))
    .route("/api/users/:username/plans", get(plans::get_user_plans))

    // Authenticated plan routes
    .route("/api/plans", post(plans::create_plan).layer(from_fn_with_state(pool.clone(), require_auth)))
    .route("/api/plans/:plan_id/comments", post(plans::create_comment).layer(from_fn_with_state(pool.clone(), require_auth)))
    .route("/api/comments/:comment_id/discussions", post(plans::add_discussion_message).layer(from_fn_with_state(pool.clone(), require_auth)))

    // Plan owner routes
    .route("/api/comments/:comment_id/accept", post(plans::accept_comment).layer(from_fn_with_state(pool.clone(), require_auth)))
    .route("/api/comments/:comment_id/reject", post(plans::reject_comment).layer(from_fn_with_state(pool.clone(), require_auth)))

    .with_state(pool);
```

---

### 2.5 Environment Variables

Add to `backend/.env.example`:
```
ANTHROPIC_API_KEY=your_anthropic_api_key_here
```

---

## 3. Frontend Architecture

### 3.1 New API Client Module

**File:** `frontend/src/lib/api/plans.ts`

```typescript
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export interface Plan {
  id: string;
  user_id: string;
  title: string;
  filename: string;
  content: string;
  is_public: boolean;
  created_at: string;
  updated_at: string;
}

export interface PlanWithAuthor extends Plan {
  author_username: string;
  author_first_name?: string;
  author_last_name?: string;
}

export interface PlanComment {
  id: string;
  plan_id: string;
  user_id: string;
  content: string;
  start_offset: number;
  end_offset: number;
  selected_text: string;
  plan_version: number;
  status: 'pending' | 'accepted' | 'rejected' | 'debating';
  resolved_at?: string;
  resolved_by?: string;
  created_at: string;
  updated_at: string;
}

export interface CommentWithAuthor extends PlanComment {
  author_username: string;
  discussions: DiscussionWithAuthor[];
}

export interface CommentDiscussion {
  id: string;
  comment_id: string;
  user_id: string;
  message: string;
  created_at: string;
}

export interface DiscussionWithAuthor extends CommentDiscussion {
  author_username: string;
}

export const plansApi = {
  async listPlans(): Promise<PlanWithAuthor[]> {
    const response = await fetch(`${API_URL}/api/plans`);
    if (!response.ok) throw new Error('Failed to fetch plans');
    return response.json();
  },

  async getUserPlans(username: string): Promise<Plan[]> {
    const response = await fetch(`${API_URL}/api/users/${username}/plans`);
    if (!response.ok) throw new Error('Failed to fetch user plans');
    return response.json();
  },

  async getPlan(planId: string): Promise<{ plan: Plan; comments: CommentWithAuthor[] }> {
    const response = await fetch(`${API_URL}/api/plans/${planId}`);
    if (!response.ok) throw new Error('Failed to fetch plan');
    return response.json();
  },

  async createPlan(title: string, filename: string, content: string): Promise<Plan> {
    const response = await fetch(`${API_URL}/api/plans`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ title, filename, content }),
    });
    if (!response.ok) throw new Error('Failed to create plan');
    return response.json();
  },

  async createComment(
    planId: string,
    content: string,
    startOffset: number,
    endOffset: number,
    selectedText: string
  ): Promise<PlanComment> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/comments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ content, start_offset: startOffset, end_offset: endOffset, selected_text: selectedText }),
    });
    if (!response.ok) throw new Error('Failed to create comment');
    return response.json();
  },

  async acceptComment(commentId: string): Promise<{ message: string; plan: Plan }> {
    const response = await fetch(`${API_URL}/api/comments/${commentId}/accept`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to accept comment');
    return response.json();
  },

  async rejectComment(commentId: string): Promise<{ message: string }> {
    const response = await fetch(`${API_URL}/api/comments/${commentId}/reject`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to reject comment');
    return response.json();
  },

  async addDiscussionMessage(commentId: string, message: string): Promise<CommentDiscussion> {
    const response = await fetch(`${API_URL}/api/comments/${commentId}/discussions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ message }),
    });
    if (!response.ok) throw new Error('Failed to add discussion message');
    return response.json();
  },
};
```

Update `frontend/src/lib/api/client.ts` to export this:
```typescript
export * from './plans';
```

---

### 3.2 New Routes

#### **3.2.1 Plans List Page**

**File:** `frontend/src/routes/plans/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { plansApi, type PlanWithAuthor } from '$lib/api/plans';
  import { goto } from '$app/navigation';

  let plans = $state<PlanWithAuthor[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      plans = await plansApi.listPlans();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load plans';
    } finally {
      loading = false;
    }
  });

  // Group plans by author
  const plansByAuthor = $derived(() => {
    const grouped = new Map<string, PlanWithAuthor[]>();
    plans.forEach(plan => {
      const author = plan.author_username;
      if (!grouped.has(author)) {
        grouped.set(author, []);
      }
      grouped.get(author)!.push(plan);
    });
    return grouped;
  });
</script>

<div class="container mx-auto px-4 py-8">
  <div class="flex justify-between items-center mb-8">
    <h1 class="text-3xl font-bold">Engineering Plans</h1>
    <button
      class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
      on:click={() => goto('/plans/upload')}
    >
      Upload Plan
    </button>
  </div>

  {#if loading}
    <p>Loading plans...</p>
  {:else if error}
    <p class="text-red-600">{error}</p>
  {:else}
    <div class="space-y-8">
      {#each [...plansByAuthor()] as [author, authorPlans]}
        <div class="border rounded-lg p-6">
          <h2 class="text-2xl font-semibold mb-4">@{author}</h2>
          <div class="grid gap-4">
            {#each authorPlans as plan}
              <a
                href="/plans/{plan.id}"
                class="block p-4 border rounded hover:bg-gray-50 dark:hover:bg-gray-800 transition"
              >
                <h3 class="text-lg font-medium">{plan.title}</h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">
                  {plan.filename} • {new Date(plan.created_at).toLocaleDateString()}
                </p>
              </a>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
```

---

#### **3.2.2 Plan Upload Page**

**File:** `frontend/src/routes/plans/upload/+page.svelte`

```svelte
<script lang="ts">
  import { plansApi } from '$lib/api/plans';
  import { goto } from '$app/navigation';
  import { authStore } from '$lib/stores/auth';

  let title = $state('');
  let filename = $state('');
  let content = $state('');
  let uploading = $state(false);
  let error = $state<string | null>(null);

  async function handleFileUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    if (!file.name.endsWith('.md')) {
      error = 'Only .md files are supported';
      return;
    }

    filename = file.name;
    title = file.name.replace('.md', '').replace(/[-_]/g, ' ');

    const reader = new FileReader();
    reader.onload = (e) => {
      content = e.target?.result as string;
    };
    reader.readAsText(file);
  }

  async function handleSubmit() {
    if (!title || !filename || !content) {
      error = 'All fields are required';
      return;
    }

    uploading = true;
    error = null;

    try {
      const plan = await plansApi.createPlan(title, filename, content);
      goto(`/plans/${plan.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to upload plan';
    } finally {
      uploading = false;
    }
  }
</script>

<div class="container mx-auto px-4 py-8 max-w-2xl">
  <h1 class="text-3xl font-bold mb-8">Upload Engineering Plan</h1>

  {#if !$authStore.user}
    <p class="text-red-600">You must be logged in to upload plans.</p>
  {:else}
    <form on:submit|preventDefault={handleSubmit} class="space-y-6">
      <div>
        <label class="block text-sm font-medium mb-2">Upload Markdown File</label>
        <input
          type="file"
          accept=".md"
          on:change={handleFileUpload}
          class="w-full border rounded px-3 py-2"
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-2">Title</label>
        <input
          type="text"
          bind:value={title}
          class="w-full border rounded px-3 py-2"
          required
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-2">Filename</label>
        <input
          type="text"
          bind:value={filename}
          class="w-full border rounded px-3 py-2"
          required
        />
      </div>

      <div>
        <label class="block text-sm font-medium mb-2">Content Preview</label>
        <textarea
          bind:value={content}
          class="w-full border rounded px-3 py-2 font-mono text-sm"
          rows="20"
          required
        />
      </div>

      {#if error}
        <p class="text-red-600">{error}</p>
      {/if}

      <button
        type="submit"
        disabled={uploading}
        class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white px-6 py-2 rounded"
      >
        {uploading ? 'Uploading...' : 'Upload Plan'}
      </button>
    </form>
  {/if}
</div>
```

---

#### **3.2.3 Plan Viewer Page with Comments**

**File:** `frontend/src/routes/plans/[id]/+page.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { plansApi, type Plan, type CommentWithAuthor } from '$lib/api/plans';
  import { authStore } from '$lib/stores/auth';
  import MarkdownViewer from '$lib/components/MarkdownViewer.svelte';
  import CommentThread from '$lib/components/CommentThread.svelte';

  let plan = $state<Plan | null>(null);
  let comments = $state<CommentWithAuthor[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let renderMode = $state<'rendered' | 'raw'>('rendered');
  let selectedText = $state<{ start: number; end: number; text: string } | null>(null);
  let showCommentForm = $state(false);
  let newCommentContent = $state('');

  const planId = $derived($page.params.id);
  const isOwner = $derived(plan && $authStore.user && plan.user_id === $authStore.user.id);

  // Group comments by status
  const pendingComments = $derived(comments.filter(c => c.status === 'pending' || c.status === 'debating'));
  const acceptedComments = $derived(comments.filter(c => c.status === 'accepted'));
  const rejectedComments = $derived(comments.filter(c => c.status === 'rejected'));

  onMount(async () => {
    await loadPlan();
  });

  async function loadPlan() {
    try {
      const data = await plansApi.getPlan(planId);
      plan = data.plan;
      comments = data.comments;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load plan';
    } finally {
      loading = false;
    }
  }

  function handleTextSelection() {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed) {
      selectedText = null;
      showCommentForm = false;
      return;
    }

    const text = selection.toString();
    const range = selection.getRangeAt(0);

    // Calculate character offsets relative to plan content
    const preSelectionRange = range.cloneRange();
    preSelectionRange.selectNodeContents(document.querySelector('.plan-content')!);
    preSelectionRange.setEnd(range.startContainer, range.startOffset);
    const start = preSelectionRange.toString().length;
    const end = start + text.length;

    selectedText = { start, end, text };
    showCommentForm = true;
  }

  async function submitComment() {
    if (!selectedText || !newCommentContent) return;

    try {
      await plansApi.createComment(
        planId,
        newCommentContent,
        selectedText.start,
        selectedText.end,
        selectedText.text
      );
      newCommentContent = '';
      selectedText = null;
      showCommentForm = false;
      await loadPlan();
    } catch (e) {
      alert('Failed to create comment');
    }
  }

  async function handleAcceptComment(commentId: string) {
    if (!confirm('Accept this comment? The plan will be updated using AI.')) return;

    try {
      await plansApi.acceptComment(commentId);
      await loadPlan();
    } catch (e) {
      alert('Failed to accept comment');
    }
  }

  async function handleRejectComment(commentId: string) {
    if (!confirm('Reject this comment?')) return;

    try {
      await plansApi.rejectComment(commentId);
      await loadPlan();
    } catch (e) {
      alert('Failed to reject comment');
    }
  }

  function downloadPlan() {
    if (!plan) return;
    const blob = new Blob([plan.content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = plan.filename;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="container mx-auto px-4 py-8">
  {#if loading}
    <p>Loading plan...</p>
  {:else if error}
    <p class="text-red-600">{error}</p>
  {:else if plan}
    <!-- Header -->
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-3xl font-bold">{plan.title}</h1>
        <p class="text-gray-600">{plan.filename}</p>
      </div>
      <div class="flex gap-2">
        <button
          on:click={downloadPlan}
          class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded"
        >
          Download
        </button>
        <button
          on:click={() => renderMode = renderMode === 'rendered' ? 'raw' : 'rendered'}
          class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
        >
          {renderMode === 'rendered' ? 'Show Raw' : 'Show Rendered'}
        </button>
      </div>
    </div>

    <div class="grid grid-cols-3 gap-6">
      <!-- Plan Content (2/3 width) -->
      <div class="col-span-2">
        <div
          class="plan-content border rounded p-6 bg-white dark:bg-gray-900"
          on:mouseup={handleTextSelection}
        >
          <MarkdownViewer content={plan.content} mode={renderMode} />
        </div>

        <!-- Comment Form -->
        {#if showCommentForm && selectedText && $authStore.user}
          <div class="mt-4 border rounded p-4 bg-blue-50 dark:bg-blue-900">
            <p class="text-sm mb-2">Selected: "{selectedText.text.substring(0, 50)}..."</p>
            <textarea
              bind:value={newCommentContent}
              placeholder="Add your comment..."
              class="w-full border rounded px-3 py-2 mb-2"
              rows="3"
            />
            <div class="flex gap-2">
              <button
                on:click={submitComment}
                class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
              >
                Submit Comment
              </button>
              <button
                on:click={() => showCommentForm = false}
                class="bg-gray-400 hover:bg-gray-500 text-white px-4 py-2 rounded"
              >
                Cancel
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Comments Sidebar (1/3 width) -->
      <div class="col-span-1">
        <!-- Active Comments -->
        <div class="space-y-4 mb-6">
          <h2 class="text-xl font-bold">Comments</h2>
          {#each pendingComments as comment}
            <CommentThread
              {comment}
              {isOwner}
              on:accept={() => handleAcceptComment(comment.id)}
              on:reject={() => handleRejectComment(comment.id)}
              on:discuss={() => loadPlan()}
            />
          {/each}
        </div>

        <!-- Rejected Comments -->
        {#if rejectedComments.length > 0}
          <div class="space-y-4 opacity-50">
            <h2 class="text-xl font-bold">Rejected</h2>
            {#each rejectedComments as comment}
              <div class="border rounded p-3 bg-gray-100 dark:bg-gray-800">
                <p class="text-sm">{comment.content}</p>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
```

---

### 3.3 New Components

#### **File:** `frontend/src/lib/components/MarkdownViewer.svelte`

```svelte
<script lang="ts">
  interface Props {
    content: string;
    mode: 'rendered' | 'raw';
  }

  let { content, mode }: Props = $props();

  // Simple markdown to HTML converter (or use a library like marked.js)
  function renderMarkdown(md: string): string {
    // This is a placeholder - in production, use a proper markdown library
    return md
      .replace(/^### (.+)$/gm, '<h3>$1</h3>')
      .replace(/^## (.+)$/gm, '<h2>$1</h2>')
      .replace(/^# (.+)$/gm, '<h1>$1</h1>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/\n/g, '<br>');
  }
</script>

{#if mode === 'rendered'}
  <div class="prose dark:prose-invert max-w-none">
    {@html renderMarkdown(content)}
  </div>
{:else}
  <pre class="font-mono text-sm whitespace-pre-wrap">{content}</pre>
{/if}

<style>
  .prose {
    @apply text-gray-900 dark:text-gray-100;
  }
  .prose h1 {
    @apply text-2xl font-bold mb-4;
  }
  .prose h2 {
    @apply text-xl font-bold mb-3;
  }
  .prose h3 {
    @apply text-lg font-semibold mb-2;
  }
</style>
```

**Note:** For production, install and use a proper markdown library:
```bash
cd frontend
npm install marked
```

Then update the component to use it.

---

#### **File:** `frontend/src/lib/components/CommentThread.svelte`

```svelte
<script lang="ts">
  import { plansApi, type CommentWithAuthor } from '$lib/api/plans';
  import { authStore } from '$lib/stores/auth';
  import { createEventDispatcher } from 'svelte';

  interface Props {
    comment: CommentWithAuthor;
    isOwner: boolean;
  }

  let { comment, isOwner }: Props = $props();
  const dispatch = createEventDispatcher();

  let showDiscussion = $state(false);
  let newMessage = $state('');

  async function addMessage() {
    if (!newMessage.trim()) return;

    try {
      await plansApi.addDiscussionMessage(comment.id, newMessage);
      newMessage = '';
      dispatch('discuss');
    } catch (e) {
      alert('Failed to add message');
    }
  }
</script>

<div class="border rounded p-4 {comment.status === 'debating' ? 'border-yellow-400' : ''}">
  <div class="mb-2">
    <p class="text-sm font-semibold">@{comment.author_username}</p>
    <p class="text-xs text-gray-500">on "{comment.selected_text.substring(0, 30)}..."</p>
  </div>

  <p class="text-sm mb-3">{comment.content}</p>

  <!-- Discussion Thread -->
  {#if comment.discussions.length > 0 || showDiscussion}
    <div class="border-t pt-2 mt-2 space-y-2">
      {#each comment.discussions as discussion}
        <div class="text-xs">
          <span class="font-semibold">{discussion.author_username}:</span>
          {discussion.message}
        </div>
      {/each}

      {#if $authStore.user}
        <div class="flex gap-2">
          <input
            type="text"
            bind:value={newMessage}
            placeholder="Add to discussion..."
            class="flex-1 text-xs border rounded px-2 py-1"
          />
          <button
            on:click={addMessage}
            class="text-xs bg-gray-600 text-white px-2 py-1 rounded"
          >
            Send
          </button>
        </div>
      {/if}
    </div>
  {:else if $authStore.user}
    <button
      on:click={() => showDiscussion = true}
      class="text-xs text-blue-600 hover:underline"
    >
      Discuss
    </button>
  {/if}

  <!-- Owner Actions -->
  {#if isOwner && (comment.status === 'pending' || comment.status === 'debating')}
    <div class="flex gap-2 mt-3">
      <button
        on:click={() => dispatch('accept')}
        class="text-xs bg-green-600 hover:bg-green-700 text-white px-3 py-1 rounded"
      >
        Accept (AI Integrate)
      </button>
      <button
        on:click={() => dispatch('reject')}
        class="text-xs bg-red-600 hover:bg-red-700 text-white px-3 py-1 rounded"
      >
        Reject
      </button>
    </div>
  {/if}
</div>
```

---

### 3.4 Navigation Update

Update `frontend/src/routes/+layout.svelte` to add "Plans" link to navigation:

```svelte
<!-- Add to nav menu -->
<a href="/plans" class="hover:text-blue-600">Plans</a>
```

---

## 4. Implementation Phases

### Phase 1: Database & Backend Foundation
1. Create migration file `006_create_multiplayer_chatgpt_system.sql`
2. Implement models in `models/plan.rs`
3. Create `services/anthropic.rs` (with mock responses initially)
4. Implement basic CRUD handlers in `handlers/plans.rs`
5. Register routes in `main.rs`
6. Test with curl/Postman

### Phase 2: Frontend Basics
1. Create API client (`lib/api/plans.ts`)
2. Implement plans list page (`/plans`)
3. Implement upload page (`/plans/upload`)
4. Basic plan viewer (`/plans/[id]`) with raw/rendered toggle
5. Test upload and viewing flow

### Phase 3: Comments & Highlighting
1. Implement text selection and offset calculation
2. Create comment submission UI
3. Build `CommentThread` component
4. Test comment creation and display
5. Implement comment status visualization (pending/accepted/rejected)

### Phase 4: Discussions
1. Implement discussion thread UI
2. Add real-time comment status updates
3. Build owner-only accept/reject controls
4. Test full discussion flow

### Phase 5: AI Integration
1. Integrate real Anthropic API
2. Implement `accept_comment_and_integrate` service
3. Implement `accept_discussion_and_integrate` service
4. Create plan versioning system
5. Test AI plan updates end-to-end

### Phase 6: Polish & Testing
1. Add loading states and error handling
2. Improve markdown rendering (use `marked` library)
3. Add comment anchoring resilience (handle plan changes)
4. Implement pagination for plans/comments
5. Add tests (backend unit tests, frontend integration tests)
6. Deploy to production

---

## 5. Key Technical Decisions

### 5.1 File Storage: Database vs S3
**Decision:** Store markdown content in PostgreSQL TEXT column

**Rationale:**
- Markdown files are text-based and compressible
- Easy full-text search and indexing
- Simplifies architecture (no S3 credentials, no external dependencies)
- Plans are typically < 100KB, manageable in Postgres
- Existing app already stores binary profile pics in DB

**Future Migration:** If plans grow beyond 1MB or exceed 10,000 plans, migrate to S3 with URL references in database.

---

### 5.2 Comment Anchoring: Character Offsets
**Decision:** Use character offsets (`start_offset`, `end_offset`) with `selected_text` fallback

**Rationale:**
- Precise and language-agnostic
- Works with any markdown structure
- `selected_text` allows re-anchoring if plan changes (fuzzy match)
- `plan_version` tracks which version comment was made on

**Alternative Considered:** Line + column numbers (rejected due to difficulty handling line changes)

---

### 5.3 Real-time Updates: Polling vs WebSockets
**Decision:** Poll on demand (no automatic updates)

**Rationale:**
- Simpler implementation for MVP
- Comments are not high-frequency (not a chat app)
- User can manually refresh to see new comments
- Avoids WebSocket complexity in SvelteKit + Axum

**Future Enhancement:** Add WebSocket support for live collaboration (Phase 7+)

---

### 5.4 Markdown Rendering
**Decision:** Use `marked` library on frontend

**Rationale:**
- Industry-standard, well-maintained
- Supports GitHub-flavored markdown
- Safe HTML sanitization built-in
- Works with SvelteKit SSR

**Installation:**
```bash
npm install marked
```

---

### 5.5 Authentication for Comments
**Decision:** Require login to comment

**Rationale:**
- Prevents spam
- Ties comments to user identity (accountability)
- Leverages existing session-based auth
- Public can still view plans without login

---

### 5.6 AI Integration Strategy
**Decision:** Synchronous API calls with user-facing loading states

**Rationale:**
- Anthropic API is fast (2-5 seconds for plan integration)
- Simpler UX than background jobs
- User gets immediate feedback
- No job polling required

**Future Enhancement:** Move to background jobs for very large plans (>50KB)

---

## 6. Security Considerations

1. **Authorization Checks:**
   - Only plan owner can accept/reject comments
   - All users can comment (if authenticated)
   - Verify ownership before AI integration

2. **Input Validation:**
   - Sanitize markdown content on render (use `marked` with sanitization)
   - Validate offset ranges against content length
   - Limit plan size (e.g., 1MB max)

3. **Rate Limiting:**
   - Limit Anthropic API calls per user (e.g., 10/day)
   - Prevent abuse of expensive AI operations

4. **API Key Security:**
   - Store `ANTHROPIC_API_KEY` in environment variables
   - Never expose in frontend
   - Use server-side proxy for all API calls

---

## 7. Future Enhancements (Post-MVP)

1. **Auto-sync from local filesystem:**
   - File watcher on user's machine
   - CLI tool to upload changes automatically

2. **Inline comment highlighting:**
   - Highlight commented sections in yellow/colored boxes
   - Click to jump to comment

3. **Comment resolution workflow:**
   - "Resolve" action separate from accept/reject
   - Track resolved vs unresolved comments

4. **Plan diffs:**
   - Show visual diff between versions
   - "What changed" summary from AI

5. **Collaboration features:**
   - @mentions in discussions
   - Email notifications for new comments

6. **Private plans:**
   - Toggle `is_public` flag
   - Share via private links

7. **Search:**
   - Full-text search across all plans
   - Filter by author, date, tags

---

## 8. Database Size Estimation

**Assumptions:**
- 100 users
- 10 plans per user (1,000 plans total)
- Average plan size: 20KB
- Average 5 comments per plan (5,000 comments)
- Average 3 discussion messages per comment (15,000 messages)

**Storage:**
- Plans: 1,000 × 20KB = 20MB
- Plan versions (3 versions avg): 60MB
- Comments: 5,000 × 500 bytes = 2.5MB
- Discussions: 15,000 × 200 bytes = 3MB
- **Total: ~86MB**

**Conclusion:** Very manageable for PostgreSQL. No need for external storage in Phase 1.

---

## 9. API Summary

### Public Endpoints
```
GET  /api/plans                      # List all public plans
GET  /api/plans/:id                  # Get plan with comments
GET  /api/users/:username/plans      # Get plans by user (folder view)
```

### Authenticated Endpoints
```
POST /api/plans                      # Upload new plan
POST /api/plans/:id/comments         # Add comment to plan
POST /api/comments/:id/discussions   # Add discussion message
```

### Owner-Only Endpoints
```
POST /api/comments/:id/accept        # Accept comment (triggers AI)
POST /api/comments/:id/reject        # Reject comment
```

---

## 10. Testing Strategy

### Backend Tests
- Unit tests for `anthropic.rs` (mock API responses)
- Integration tests for handlers (use test database)
- Test authorization (ensure only owner can accept/reject)

### Frontend Tests
- Component tests for `CommentThread`, `MarkdownViewer`
- E2E tests for upload → comment → accept flow
- Test text selection and offset calculation

### Manual Testing Checklist
- [ ] Upload plan via UI
- [ ] View plan in rendered and raw mode
- [ ] Select text and create comment
- [ ] Add discussion messages
- [ ] Accept comment and verify AI integration
- [ ] Reject comment and verify it moves to sidebar
- [ ] Download plan
- [ ] Test as non-owner (should not see accept/reject buttons)

---

## 11. Deployment Checklist

- [ ] Run migration `006_create_multiplayer_chatgpt_system.sql`
- [ ] Set `ANTHROPIC_API_KEY` environment variable in production
- [ ] Update CORS settings if frontend/backend on different domains
- [ ] Test file upload limits (nginx/reverse proxy settings)
- [ ] Monitor Anthropic API usage and costs
- [ ] Set up logging for AI integration errors
- [ ] Create database backups before going live

---

## Conclusion

This plan provides a complete roadmap for implementing "Multiplayer ChatGPT" collaboration on engineering plans. It integrates seamlessly with your existing Rust + Axum + SvelteKit architecture, following established patterns for auth, database migrations, and API design.

**Key Strengths:**
- Leverages existing auth system (no new user management)
- Uses PostgreSQL for simplicity (no S3 complexity in Phase 1)
- Clear separation of concerns (models, services, handlers)
- Incremental implementation (6 phases)
- Extensible for future features (real-time, auto-sync)

**Next Steps:**
1. Review and approve this plan
2. Start Phase 1: Database migration and backend foundation
3. Iterate based on feedback from early testing
