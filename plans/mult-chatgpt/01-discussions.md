# Extension 01 - Discussions

**Builds on:** MVP (00-multiplayer-chatgpt-mvp.md)
**Next:** Extension 02 - Enhanced Status & Version Tracking

---

## What This Adds

Threaded discussions on comments, allowing multiple users to debate and refine feedback before the plan owner accepts/rejects.

**New Features:**
- Reply to comments with discussion messages
- View discussion threads inline
- Real-time discussion updates
- Discussion participants list

---

## Database Changes

### New Table: `comment_discussions`

```sql
-- Migration: backend/migrations/007_add_discussions.sql

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

**No changes to existing tables** - fully backward compatible.

---

## Backend Implementation

### 2.1 Update Models - `backend/src/models/plan.rs`

Add new struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommentDiscussion {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

// New DTOs
#[derive(Debug, Deserialize)]
pub struct AddDiscussionRequest {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DiscussionWithAuthor {
    #[serde(flatten)]
    pub discussion: CommentDiscussion,
    pub author_username: String,
}

// Update existing CommentWithAuthor to include discussions
#[derive(Debug, Serialize)]
pub struct CommentWithAuthor {
    #[serde(flatten)]
    pub comment: PlanComment,
    pub author_username: String,
    pub discussions: Vec<DiscussionWithAuthor>,  // Add this
}
```

---

### 2.2 Update Handlers - `backend/src/handlers/plans.rs`

Update `get_plan` handler to load discussions:

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

    // Get comments
    let comments_rows = sqlx::query!(
        r#"
        SELECT
            c.id, c.plan_id, c.user_id, c.content,
            c.start_line, c.end_line, c.selected_lines, c.plan_version,
            c.is_resolved, c.resolved_at, c.created_at, c.updated_at,
            u.username as author_username
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
                    is_resolved: row.is_resolved,
                    resolved_at: row.resolved_at,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
                author_username: row.author_username,
                discussions,
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

Add new handler for discussions:

```rust
// Add discussion message to comment
pub async fn add_discussion_message(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
    Json(req): Json<AddDiscussionRequest>,
) -> Result<Json<DiscussionWithAuthor>, (StatusCode, String)> {
    // Validate comment exists
    let comment_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM plan_comments WHERE id = $1)",
        comment_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(false);

    if !comment_exists {
        return Err((StatusCode::NOT_FOUND, "Comment not found".to_string()));
    }

    // Validate message not empty
    if req.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Message cannot be empty".to_string()));
    }

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

    // Get username
    let username = sqlx::query_scalar!(
        "SELECT username FROM users WHERE id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DiscussionWithAuthor {
        discussion: CommentDiscussion {
            id: discussion.id,
            comment_id: discussion.comment_id,
            user_id: discussion.user_id,
            message: discussion.message,
            created_at: discussion.created_at,
        },
        author_username: username,
    }))
}
```

---

### 2.3 Update Routes - `backend/src/main.rs`

Add new route:

```rust
// Authenticated plan routes
.route("/api/comments/:comment_id/discussions",
    post(plans::add_discussion_message)
        .layer(from_fn_with_state(pool.clone(), require_auth)))
```

---

## Frontend Implementation

### 3.1 Update API Client - `frontend/src/lib/api/plans.ts`

Add types and method:

```typescript
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

// Update CommentWithAuthor interface
export interface CommentWithAuthor extends PlanComment {
  author_username: string;
  discussions: DiscussionWithAuthor[];  // Add this
}

// Add to plansApi object
export const plansApi = {
  // ... existing methods ...

  async addDiscussion(commentId: string, message: string): Promise<DiscussionWithAuthor> {
    const response = await fetch(`${API_URL}/api/comments/${commentId}/discussions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ message }),
    });
    if (!response.ok) throw new Error('Failed to add discussion');
    return response.json();
  },
};
```

---

### 3.2 Update Comment Sidebar - `frontend/src/lib/components/CommentSidebar.svelte`

Add discussion UI to each comment:

```svelte
<script lang="ts">
  import { plansApi, type CommentWithAuthor, type AiIntegrationJob } from '$lib/api/plans';
  import { authStore } from '$lib/stores/auth';

  interface Props {
    comments: CommentWithAuthor[];
    isOwner: boolean;
    onaction: () => void;
  }

  let { comments, isOwner, onaction }: Props = $props();

  let activeComments = $derived(comments.filter((c) => !c.is_resolved));
  let resolvedComments = $derived(comments.filter((c) => c.is_resolved));
  let processingJobs = $state<Map<string, AiIntegrationJob>>(new Map());

  // Track which comments have discussion form open
  let discussionForms = $state<Map<string, string>>(new Map());

  async function handleAccept(commentId: string) {
    if (!confirm('Accept this comment? AI will integrate it into the plan.')) return;

    try {
      const result = await plansApi.acceptComment(commentId);
      pollJobStatus(result.job_id, commentId);
    } catch (e) {
      alert(e instanceof Error ? e.message : 'Failed to accept comment');
    }
  }

  async function pollJobStatus(jobId: string, commentId: string) {
    const job: AiIntegrationJob = await plansApi.getJobStatus(jobId);
    processingJobs.set(commentId, job);

    if (job.status === 'pending' || job.status === 'processing') {
      setTimeout(() => pollJobStatus(jobId, commentId), 2000);
    } else if (job.status === 'completed') {
      processingJobs.delete(commentId);
      onaction();
    } else if (job.status === 'failed') {
      alert(`AI integration failed: ${job.error_message}`);
      processingJobs.delete(commentId);
    }
  }

  async function handleReject(commentId: string) {
    if (!confirm('Reject this comment?')) return;

    try {
      await plansApi.rejectComment(commentId);
      onaction();
    } catch (e) {
      alert('Failed to reject comment');
    }
  }

  function toggleDiscussionForm(commentId: string) {
    if (discussionForms.has(commentId)) {
      discussionForms.delete(commentId);
    } else {
      discussionForms.set(commentId, '');
    }
    discussionForms = new Map(discussionForms); // Trigger reactivity
  }

  async function submitDiscussion(commentId: string) {
    const message = discussionForms.get(commentId);
    if (!message?.trim()) return;

    try {
      await plansApi.addDiscussion(commentId, message);
      discussionForms.delete(commentId);
      discussionForms = new Map(discussionForms);
      onaction(); // Reload to show new discussion
    } catch (e) {
      alert('Failed to add discussion message');
    }
  }
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

          <div class="border rounded p-4 bg-white dark:bg-gray-900">
            <div class="mb-2">
              <p class="text-sm font-semibold">@{comment.author_username}</p>
              <p class="text-xs text-gray-500">
                Lines {comment.start_line}-{comment.end_line}
              </p>
            </div>

            <p class="text-sm mb-3">{comment.content}</p>

            <!-- Discussion Thread -->
            {#if comment.discussions.length > 0}
              <div class="border-t pt-2 mt-2 space-y-2 bg-gray-50 dark:bg-gray-800 p-2 rounded">
                <p class="text-xs font-semibold text-gray-600 dark:text-gray-400">Discussion:</p>
                {#each comment.discussions as discussion}
                  <div class="text-xs">
                    <span class="font-semibold text-blue-600 dark:text-blue-400">
                      @{discussion.author_username}:
                    </span>
                    <span class="text-gray-800 dark:text-gray-200">{discussion.message}</span>
                    <span class="text-gray-400 text-[10px] ml-2">
                      {new Date(discussion.created_at).toLocaleString()}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}

            <!-- Discussion Form -->
            {#if showForm && $authStore.user}
              <div class="mt-2 border-t pt-2">
                <textarea
                  bind:value={discussionForms.get(comment.id)}
                  placeholder="Add to discussion..."
                  class="w-full text-xs border rounded px-2 py-1 mb-2"
                  rows="2"
                />
                <div class="flex gap-2">
                  <button
                    onclick={() => submitDiscussion(comment.id)}
                    class="text-xs bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded"
                  >
                    Send
                  </button>
                  <button
                    onclick={() => toggleDiscussionForm(comment.id)}
                    class="text-xs bg-gray-400 hover:bg-gray-500 text-white px-3 py-1 rounded"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            {:else if $authStore.user && !job}
              <button
                onclick={() => toggleDiscussionForm(comment.id)}
                class="text-xs text-blue-600 hover:underline mt-2"
              >
                💬 Reply
              </button>
            {/if}

            <!-- AI Processing Status -->
            {#if job}
              <div class="text-xs p-2 bg-blue-50 dark:bg-blue-900 rounded mt-2">
                {#if job.status === 'pending'}
                  ⏳ Queued for AI processing...
                {:else if job.status === 'processing'}
                  🤖 AI is integrating this feedback...
                {/if}
              </div>
            {:else if isOwner}
              <!-- Owner Actions -->
              <div class="flex gap-2 mt-3">
                <button
                  onclick={() => handleAccept(comment.id)}
                  class="text-xs bg-green-600 hover:bg-green-700 text-white px-3 py-1 rounded"
                >
                  ✓ Accept (AI Integrate)
                </button>
                <button
                  onclick={() => handleReject(comment.id)}
                  class="text-xs bg-red-600 hover:bg-red-700 text-white px-3 py-1 rounded"
                >
                  ✗ Reject
                </button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div>
      <h2 class="text-xl font-bold mb-4">Comments</h2>
      <p class="text-gray-600 text-sm">No active comments yet.</p>
    </div>
  {/if}

  <!-- Resolved Comments -->
  {#if resolvedComments.length > 0}
    <div class="opacity-60">
      <h2 class="text-lg font-semibold mb-3">Resolved ({resolvedComments.length})</h2>
      <div class="space-y-2">
        {#each resolvedComments.slice(0, 5) as comment}
          <div class="border rounded p-3 bg-gray-100 dark:bg-gray-800 text-sm">
            <p class="font-semibold">@{comment.author_username}</p>
            <p class="text-xs text-gray-600">{comment.content}</p>
            {#if comment.discussions.length > 0}
              <p class="text-[10px] text-gray-500 mt-1">
                {comment.discussions.length} discussion message{comment.discussions.length > 1 ? 's' : ''}
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

## Migration Steps

### From MVP to Extension 01

1. **Run migration:**
   ```bash
   cd backend
   sqlx migrate run
   ```

2. **Update backend code:**
   - Update `models/plan.rs` - Add discussion types
   - Update `handlers/plans.rs` - Update `get_plan`, add `add_discussion_message`
   - Update `main.rs` - Add discussion route

3. **Update frontend code:**
   - Update `lib/api/plans.ts` - Add discussion types and API method
   - Update `lib/components/CommentSidebar.svelte` - Add discussion UI

4. **Test:**
   - Create a comment
   - Add discussion messages
   - Verify discussions persist
   - Accept comment and verify discussions are preserved

---

## Testing Checklist

- [ ] Add discussion message to comment
- [ ] View discussion thread
- [ ] Multiple users can discuss
- [ ] Discussion messages ordered chronologically
- [ ] Discussions preserved when comment accepted
- [ ] Discussions preserved when comment rejected
- [ ] Cannot add discussion to non-existent comment
- [ ] Empty messages rejected
- [ ] Discussion UI updates without page reload

---

## Implementation Time

- Backend: 1 day
- Frontend: 1.5 days
- Testing: 0.5 days
- **Total: 3 days**

---

## Next Extension

👉 **Extension 02 - Enhanced Status & Version Tracking**

See: `02-enhanced-status.md`
