# Extension 02 - Enhanced Status & Version Tracking

**Builds on:** Extension 01 - Discussions
**Next:** Extension 03 - Inline Highlighting

---

## What This Adds

Enhanced comment status tracking and detailed version metadata to understand plan evolution better.

**New Features:**
- Comment status: `pending`, `debating`, `accepted`, `rejected`
- Track who resolved comments
- Track version source (manual, ai_comment, ai_discussion)
- AI-generated version summaries

---

## Database Changes

### Migration: `backend/migrations/008_enhanced_status.sql`

```sql
-- Create status enum
CREATE TYPE comment_status AS ENUM ('pending', 'debating', 'accepted', 'rejected');

-- Create version source enum
CREATE TYPE version_source AS ENUM ('manual', 'ai_comment', 'ai_discussion');

-- Add new columns to plan_comments
ALTER TABLE plan_comments
ADD COLUMN status comment_status,
ADD COLUMN resolved_by UUID REFERENCES users(id);

-- Migrate existing data (is_resolved = true -> 'accepted', false -> 'pending')
UPDATE plan_comments
SET status = CASE
    WHEN is_resolved THEN 'accepted'::comment_status
    ELSE 'pending'::comment_status
END;

-- Set status as required
ALTER TABLE plan_comments
ALTER COLUMN status SET NOT NULL,
ALTER COLUMN status SET DEFAULT 'pending';

-- Drop old boolean column
ALTER TABLE plan_comments
DROP COLUMN is_resolved;

-- Add new columns to plan_versions
ALTER TABLE plan_versions
ADD COLUMN source version_source NOT NULL DEFAULT 'manual',
ADD COLUMN summary TEXT;

-- Create index on comment status
CREATE INDEX idx_plan_comments_status ON plan_comments(status);
```

---

## Backend Implementation

### 2.1 Update Models - `backend/src/models/plan.rs`

```rust
// Add enums
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "comment_status", rename_all = "snake_case")]
pub enum CommentStatus {
    Pending,
    Debating,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "version_source", rename_all = "snake_case")]
pub enum VersionSource {
    Manual,
    AiComment,
    AiDiscussion,
}

// Update PlanComment struct
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanComment {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub start_line: i32,
    pub end_line: i32,
    pub selected_lines: String,
    pub plan_version: i32,
    pub status: CommentStatus,  // Changed from is_resolved
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,  // Added
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Update PlanVersion struct
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanVersion {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub version_number: i32,
    pub content: String,
    pub comment_id: Option<Uuid>,
    pub created_by: Uuid,
    pub source: VersionSource,  // Added
    pub summary: Option<String>,  // Added
    pub created_at: DateTime<Utc>,
}

// Add response DTO with resolver info
#[derive(Debug, Serialize)]
pub struct CommentWithAuthor {
    #[serde(flatten)]
    pub comment: PlanComment,
    pub author_username: String,
    pub discussions: Vec<DiscussionWithAuthor>,
    pub resolved_by_username: Option<String>,  // Added
}
```

---

### 2.2 Update Plan Service - `backend/src/services/plan_service.rs`

Update `create_plan_version` to accept source and summary:

```rust
pub async fn create_plan_version(
    pool: &PgPool,
    plan_id: Uuid,
    content: &str,
    comment_id: Option<Uuid>,
    created_by: Uuid,
    source: VersionSource,  // Added
    summary: Option<String>,  // Added
) -> Result<PlanVersion, sqlx::Error> {
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
        INSERT INTO plan_versions (plan_id, version_number, content, comment_id, created_by, source, summary)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, plan_id, version_number, content, comment_id, created_by,
                  source as "source: VersionSource", summary, created_at
        "#,
        plan_id,
        new_version,
        content,
        comment_id,
        created_by,
        source as VersionSource,
        summary
    )
    .fetch_one(pool)
    .await?;

    Ok(version)
}
```

Update `process_ai_integration` to set status and track resolver:

```rust
pub async fn process_ai_integration(
    pool: &PgPool,
    job_id: Uuid,
) -> Result<(), String> {
    // Update status to processing
    sqlx::query!(
        r#"UPDATE ai_integration_jobs SET status = 'processing' WHERE id = $1"#,
        job_id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Get job details with comment and plan
    let job = sqlx::query!(
        r#"
        SELECT
            j.id, j.comment_id, j.plan_id, j.user_id,
            c.content as comment_content,
            c.start_line, c.end_line, c.selected_lines,
            p.content as plan_content
        FROM ai_integration_jobs j
        JOIN plan_comments c ON j.comment_id = c.id
        JOIN plans p ON j.plan_id = p.id
        WHERE j.id = $1
        "#,
        job_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Job not found: {}", e))?;

    // Check if comment has discussions
    let discussion_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM comment_discussions WHERE comment_id = $1",
        job.comment_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?
    .unwrap_or(0);

    let has_discussions = discussion_count > 0;

    // Validate plan size
    if job.plan_content.len() > MAX_PLAN_SIZE {
        let err = "Plan too large for AI processing".to_string();
        mark_job_failed(pool, job_id, &err).await?;
        return Err(err);
    }

    // Call Anthropic API
    let updated_content = match anthropic::integrate_comment_into_plan(
        &job.plan_content,
        &job.comment_content,
        &job.selected_lines,
        job.start_line,
        job.end_line,
    )
    .await
    {
        Ok(content) => content,
        Err(e) => {
            mark_job_failed(pool, job_id, &e).await?;
            return Err(e);
        }
    };

    // Generate summary
    let summary = format!(
        "AI integrated feedback on lines {}-{}: {}",
        job.start_line,
        job.end_line,
        if job.comment_content.len() > 60 {
            format!("{}...", &job.comment_content[..60])
        } else {
            job.comment_content.clone()
        }
    );

    // Begin transaction for atomic update
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Update plan
    sqlx::query!(
        "UPDATE plans SET content = $1, updated_at = NOW() WHERE id = $2",
        updated_content,
        job.plan_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        let _ = tx.rollback();
        format!("Failed to update plan: {}", e)
    })?;

    // Create version with source tracking
    let current_version: Option<i32> = sqlx::query_scalar!(
        "SELECT MAX(version_number) FROM plan_versions WHERE plan_id = $1",
        job.plan_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        let _ = tx.rollback();
        e.to_string()
    })?;

    let new_version = current_version.unwrap_or(0) + 1;
    let source = if has_discussions {
        VersionSource::AiDiscussion
    } else {
        VersionSource::AiComment
    };

    sqlx::query!(
        r#"
        INSERT INTO plan_versions (plan_id, version_number, content, comment_id, created_by, source, summary)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        job.plan_id,
        new_version,
        updated_content,
        job.comment_id,
        job.user_id,
        source as VersionSource,
        summary
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        let _ = tx.rollback();
        format!("Failed to create version: {}", e)
    })?;

    // Update comment status to 'accepted' and set resolved_by
    sqlx::query!(
        r#"UPDATE plan_comments
           SET status = 'accepted', resolved_at = NOW(), resolved_by = $1
           WHERE id = $2"#,
        job.user_id,
        job.comment_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        let _ = tx.rollback();
        format!("Failed to update comment: {}", e)
    })?;

    // Mark job complete
    sqlx::query!(
        r#"UPDATE ai_integration_jobs SET status = 'completed', completed_at = NOW() WHERE id = $1"#,
        job_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        let _ = tx.rollback();
        format!("Failed to mark job complete: {}", e)
    })?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(())
}
```

---

### 2.3 Update Handlers - `backend/src/handlers/plans.rs`

Update `get_plan` to fetch resolver username:

```rust
pub async fn get_plan(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<PlanDetailResponse>, (StatusCode, String)> {
    let plan = sqlx::query_as!(
        Plan,
        "SELECT * FROM plans WHERE id = $1 AND is_public = true",
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

    // Get comments with resolver info
    let comments_rows = sqlx::query!(
        r#"
        SELECT
            c.id, c.plan_id, c.user_id, c.content,
            c.start_line, c.end_line, c.selected_lines, c.plan_version,
            c.status as "status: CommentStatus",
            c.resolved_at, c.resolved_by, c.created_at, c.updated_at,
            u.username as author_username,
            r.username as resolved_by_username
        FROM plan_comments c
        JOIN users u ON c.user_id = u.id
        LEFT JOIN users r ON c.resolved_by = r.id
        WHERE c.plan_id = $1
        ORDER BY c.created_at DESC
        "#,
        plan_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get all discussions for these comments
    let comment_ids: Vec<Uuid> = comments_rows.iter().map(|c| c.id).collect();

    let discussions_rows = if !comment_ids.is_empty() {
        sqlx::query!(
            r#"
            SELECT
                d.id, d.comment_id, d.user_id, d.message, d.created_at,
                u.username as author_username
            FROM comment_discussions d
            JOIN users u ON d.user_id = u.id
            WHERE d.comment_id = ANY($1)
            ORDER BY d.created_at ASC
            "#,
            &comment_ids
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        vec![]
    };

    // Group discussions by comment_id
    let mut discussions_map: std::collections::HashMap<Uuid, Vec<DiscussionWithAuthor>> =
        std::collections::HashMap::new();

    for row in discussions_rows {
        let discussion = DiscussionWithAuthor {
            discussion: CommentDiscussion {
                id: row.id,
                comment_id: row.comment_id,
                user_id: row.user_id,
                message: row.message,
                created_at: row.created_at,
            },
            author_username: row.author_username,
        };

        discussions_map
            .entry(row.comment_id)
            .or_insert_with(Vec::new)
            .push(discussion);
    }

    // Build comments with discussions
    let comments: Vec<CommentWithAuthor> = comments_rows
        .into_iter()
        .map(|row| {
            let discussions = discussions_map
                .get(&row.id)
                .cloned()
                .unwrap_or_default();

            CommentWithAuthor {
                comment: PlanComment {
                    id: row.id,
                    plan_id: row.plan_id,
                    user_id: row.user_id,
                    content: row.content,
                    start_line: row.start_line,
                    end_line: row.end_line,
                    selected_lines: row.selected_lines,
                    plan_version: row.plan_version,
                    status: row.status,
                    resolved_at: row.resolved_at,
                    resolved_by: row.resolved_by,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
                author_username: row.author_username,
                discussions,
                resolved_by_username: row.resolved_by_username,
            }
        })
        .collect();

    Ok(Json(PlanDetailResponse {
        plan,
        comments,
        current_version,
    }))
}
```

Update `reject_comment`:

```rust
pub async fn reject_comment(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Verify ownership
    let result = sqlx::query!(
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

    if result.plan_owner_id != user.id {
        return Err((
            StatusCode::FORBIDDEN,
            "Only plan owner can reject comments".to_string(),
        ));
    }

    sqlx::query!(
        r#"UPDATE plan_comments
           SET status = 'rejected', resolved_at = NOW(), resolved_by = $1
           WHERE id = $2"#,
        user.id,
        comment_id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Comment rejected" })))
}
```

Update `add_discussion_message` to set status to 'debating':

```rust
pub async fn add_discussion_message(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
    Json(req): Json<AddDiscussionRequest>,
) -> Result<Json<DiscussionWithAuthor>, (StatusCode, String)> {
    // ... existing validation ...

    let discussion = sqlx::query!(
        r#"
        INSERT INTO comment_discussions (comment_id, user_id, message)
        VALUES ($1, $2, $3)
        RETURNING id, comment_id, user_id, message, created_at
        "#,
        comment_id,
        user.id,
        req.message.trim()
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update comment status to 'debating' if still pending
    sqlx::query!(
        r#"UPDATE plan_comments
           SET status = 'debating'
           WHERE id = $1 AND status = 'pending'"#,
        comment_id
    )
    .execute(&pool)
    .await
    .ok(); // Ignore error if already debating

    // ... rest of function ...
}
```

Add new endpoint to view version history:

```rust
pub async fn get_plan_versions(
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlanVersion>>, (StatusCode, String)> {
    let versions = sqlx::query_as!(
        PlanVersion,
        r#"
        SELECT
            id, plan_id, version_number, content, comment_id, created_by,
            source as "source: VersionSource", summary, created_at
        FROM plan_versions
        WHERE plan_id = $1
        ORDER BY version_number DESC
        "#,
        plan_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(versions))
}
```

---

### 2.4 Update Routes - `backend/src/main.rs`

Add version history route:

```rust
// Public plan routes
.route("/api/plans/:plan_id/versions", get(plans::get_plan_versions))
```

Update all calls to `create_plan_version`:

```rust
// In create_plan handler:
plan_service::create_plan_version(
    &pool,
    plan.id,
    &plan.content,
    None,
    user.id,
    VersionSource::Manual,  // Added
    Some("Initial version".to_string()),  // Added
)
```

---

## Frontend Implementation

### 3.1 Update API Client - `frontend/src/lib/api/plans.ts`

```typescript
export type CommentStatus = 'pending' | 'debating' | 'accepted' | 'rejected';
export type VersionSource = 'manual' | 'ai_comment' | 'ai_discussion';

export interface PlanComment {
  id: string;
  plan_id: string;
  user_id: string;
  content: string;
  start_line: number;
  end_line: number;
  selected_lines: string;
  plan_version: number;
  status: CommentStatus;  // Changed from is_resolved
  resolved_at?: string;
  resolved_by?: string;  // Added
  created_at: string;
  updated_at: string;
}

export interface CommentWithAuthor extends PlanComment {
  author_username: string;
  discussions: DiscussionWithAuthor[];
  resolved_by_username?: string;  // Added
}

export interface PlanVersion {
  id: string;
  plan_id: string;
  version_number: number;
  content: string;
  comment_id?: string;
  created_by: string;
  source: VersionSource;  // Added
  summary?: string;  // Added
  created_at: string;
}

export const plansApi = {
  // ... existing methods ...

  async getPlanVersions(planId: string): Promise<PlanVersion[]> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/versions`);
    if (!response.ok) throw new Error('Failed to fetch versions');
    return response.json();
  },
};
```

---

### 3.2 Update Comment Sidebar - `frontend/src/lib/components/CommentSidebar.svelte`

Update filters to use new status:

```svelte
<script lang="ts">
  // ... existing code ...

  let activeComments = $derived(
    comments.filter((c) => c.status === 'pending' || c.status === 'debating')
  );
  let acceptedComments = $derived(comments.filter((c) => c.status === 'accepted'));
  let rejectedComments = $derived(comments.filter((c) => c.status === 'rejected'));
</script>

<div class="space-y-6">
  <!-- Active Comments -->
  {#if activeComments.length > 0}
    <div>
      <h2 class="text-xl font-bold mb-4">Active Comments ({activeComments.length})</h2>
      <div class="space-y-4">
        {#each activeComments as comment}
          {@const job = processingJobs.get(comment.id)}
          {@const showForm = discussionForms.has(comment.id)}

          <div class="border rounded p-4 bg-white dark:bg-gray-900"
               class:border-yellow-400={comment.status === 'debating'}>

            <!-- Status Badge -->
            <div class="flex items-center gap-2 mb-2">
              <p class="text-sm font-semibold">@{comment.author_username}</p>
              {#if comment.status === 'debating'}
                <span class="text-xs bg-yellow-100 text-yellow-800 px-2 py-0.5 rounded">
                  💬 Debating
                </span>
              {/if}
            </div>

            <!-- ... rest of comment rendering ... -->
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Accepted Comments (with who accepted them) -->
  {#if acceptedComments.length > 0}
    <div class="opacity-75">
      <h2 class="text-lg font-semibold mb-3 text-green-700">
        ✓ Accepted ({acceptedComments.length})
      </h2>
      <div class="space-y-2">
        {#each acceptedComments.slice(0, 5) as comment}
          <div class="border border-green-200 rounded p-3 bg-green-50 dark:bg-green-900 text-sm">
            <p class="font-semibold">@{comment.author_username}</p>
            <p class="text-xs text-gray-600">{comment.content}</p>
            {#if comment.resolved_by_username}
              <p class="text-[10px] text-gray-500 mt-1">
                Accepted by @{comment.resolved_by_username}
              </p>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Rejected Comments -->
  {#if rejectedComments.length > 0}
    <div class="opacity-60">
      <h2 class="text-lg font-semibold mb-3 text-red-700">
        ✗ Rejected ({rejectedComments.length})
      </h2>
      <div class="space-y-2">
        {#each rejectedComments.slice(0, 5) as comment}
          <div class="border rounded p-3 bg-gray-100 dark:bg-gray-800 text-sm">
            <p class="font-semibold">@{comment.author_username}</p>
            <p class="text-xs text-gray-600">{comment.content}</p>
            {#if comment.resolved_by_username}
              <p class="text-[10px] text-gray-500 mt-1">
                Rejected by @{comment.resolved_by_username}
              </p>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
```

---

### 3.3 Add Version History Component - `frontend/src/lib/components/VersionHistory.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { plansApi, type PlanVersion } from '$lib/api/plans';

  interface Props {
    planId: string;
  }

  let { planId }: Props = $props();

  let versions = $state<PlanVersion[]>([]);
  let loading = $state(true);
  let expanded = $state(false);

  onMount(async () => {
    await loadVersions();
  });

  async function loadVersions() {
    try {
      versions = await plansApi.getPlanVersions(planId);
    } catch (e) {
      console.error('Failed to load versions', e);
    } finally {
      loading = false;
    }
  }

  function getSourceIcon(source: string): string {
    switch (source) {
      case 'manual': return '✏️';
      case 'ai_comment': return '🤖';
      case 'ai_discussion': return '💬🤖';
      default: return '📄';
    }
  }

  function getSourceLabel(source: string): string {
    switch (source) {
      case 'manual': return 'Manual edit';
      case 'ai_comment': return 'AI integration';
      case 'ai_discussion': return 'AI after discussion';
      default: return source;
    }
  }
</script>

<div class="border rounded p-4 bg-gray-50 dark:bg-gray-900">
  <button
    onclick={() => expanded = !expanded}
    class="w-full flex items-center justify-between text-left"
  >
    <h3 class="font-semibold">Version History ({versions.length})</h3>
    <span class="text-sm">{expanded ? '▼' : '▶'}</span>
  </button>

  {#if expanded}
    <div class="mt-4 space-y-2">
      {#if loading}
        <p class="text-sm text-gray-600">Loading versions...</p>
      {:else if versions.length === 0}
        <p class="text-sm text-gray-600">No version history yet</p>
      {:else}
        {#each versions as version}
          <div class="border rounded p-3 bg-white dark:bg-gray-800 text-sm">
            <div class="flex items-center justify-between mb-1">
              <span class="font-semibold">v{version.version_number}</span>
              <span class="text-xs text-gray-500">
                {new Date(version.created_at).toLocaleString()}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs text-gray-600">
              <span>{getSourceIcon(version.source)}</span>
              <span>{getSourceLabel(version.source)}</span>
            </div>
            {#if version.summary}
              <p class="text-xs text-gray-700 dark:text-gray-300 mt-2">
                {version.summary}
              </p>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>
```

---

### 3.4 Update Plan Viewer - `frontend/src/routes/plans/[id]/+page.svelte`

Add version history:

```svelte
<script lang="ts">
  // ... existing imports ...
  import VersionHistory from '$lib/components/VersionHistory.svelte';

  // ... existing code ...
</script>

<div class="container mx-auto px-4 py-8">
  {#if loading}
    <p>Loading plan...</p>
  {:else if error}
    <p class="text-red-600">{error}</p>
  {:else if data}
    <!-- Header with version history -->
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-3xl font-bold">{data.plan.title}</h1>
        <p class="text-gray-600">{data.plan.filename}</p>
      </div>
      <button
        onclick={downloadPlan}
        class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded"
      >
        Download
      </button>
    </div>

    <!-- Version History -->
    <div class="mb-6">
      <VersionHistory planId={data.plan.id} />
    </div>

    <!-- Rest of the page ... -->
  {/if}
</div>
```

---

## Migration Steps

### From Extension 01 to Extension 02

1. **Run migration:**
   ```bash
   cd backend
   sqlx migrate run
   ```

2. **Update backend:**
   - Update `models/plan.rs` - Add enums and update structs
   - Update `services/plan_service.rs` - Update function signatures
   - Update `handlers/plans.rs` - Update queries and add version endpoint
   - Update `main.rs` - Add version route

3. **Update frontend:**
   - Update `lib/api/plans.ts` - Add new types
   - Update `CommentSidebar.svelte` - Handle new statuses
   - Create `VersionHistory.svelte`
   - Update plan viewer

4. **Test:**
   - Verify existing data migrated correctly
   - Create comment and accept - check status is 'accepted'
   - Add discussion - check status changes to 'debating'
   - View version history - verify sources tracked correctly

---

## Testing Checklist

- [ ] Comment status shows correctly (pending/debating/accepted/rejected)
- [ ] Discussion changes status to 'debating'
- [ ] Accept sets status to 'accepted' and tracks resolver
- [ ] Reject sets status to 'rejected' and tracks resolver
- [ ] Version source tracked correctly (manual/ai_comment/ai_discussion)
- [ ] Version summaries generated
- [ ] Version history displays correctly
- [ ] Resolved comments show who resolved them
- [ ] Existing data migrated without errors

---

## Implementation Time

- Backend: 1.5 days
- Frontend: 1.5 days
- Testing & Migration: 1 day
- **Total: 4 days**

---

## Next Extension

👉 **Extension 03 - Inline Highlighting**

See: `03-inline-highlighting.md`
