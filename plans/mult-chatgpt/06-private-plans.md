# Extension 06 - Private Plans & Sharing

**Builds on:** Extension 05 - Collaboration Features
**Next:** Extension 07 - Search & Auto-sync

---

## What This Adds

Private plans with shareable links and access control.

**New Features:**
- Toggle plan visibility (public/private)
- Share private plans via secret links
- Invite specific users to private plans
- View access logs
- Revoke access

---

## Database Changes

### Migration: `backend/migrations/010_add_private_plans.sql`

```sql
-- Add privacy fields to plans
ALTER TABLE plans
ADD COLUMN share_token VARCHAR(64) UNIQUE,
ADD COLUMN share_link_enabled BOOLEAN DEFAULT false;

-- Plan access table (for invited users)
CREATE TABLE plan_access (
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    granted_by UUID NOT NULL REFERENCES users(id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plan_id, user_id)
);

CREATE INDEX idx_plan_access_user_id ON plan_access(user_id);

-- Access logs
CREATE TABLE plan_access_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    accessed_via VARCHAR(50),  -- 'owner', 'invited', 'share_link'
    ip_address VARCHAR(45),
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plan_access_logs_plan_id ON plan_access_logs(plan_id);
CREATE INDEX idx_plan_access_logs_accessed_at ON plan_access_logs(accessed_at DESC);

-- Generate share tokens for existing plans
UPDATE plans
SET share_token = encode(gen_random_bytes(32), 'hex')
WHERE share_token IS NULL;
```

---

## Backend Implementation

### 2.1 Update Models - `backend/src/models/plan.rs`

```rust
// Update Plan struct
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub filename: String,
    pub content: String,
    pub is_public: bool,
    pub share_token: Option<String>,  // Added
    pub share_link_enabled: bool,  // Added
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// New models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanAccess {
    pub plan_id: Uuid,
    pub user_id: Uuid,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanAccessLog {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub user_id: Option<Uuid>,
    pub accessed_via: String,
    pub ip_address: Option<String>,
    pub accessed_at: DateTime<Utc>,
}

// DTOs
#[derive(Debug, Deserialize)]
pub struct UpdatePlanPrivacyRequest {
    pub is_public: Option<bool>,
    pub share_link_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GrantAccessRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct PlanAccessWithUser {
    #[serde(flatten)]
    pub access: PlanAccess,
    pub username: String,
    pub granted_by_username: String,
}

#[derive(Debug, Serialize)]
pub struct ShareLinkResponse {
    pub share_url: String,
    pub enabled: bool,
}
```

---

### 2.2 Create Access Control Service - `backend/src/services/access_control.rs`

```rust
use crate::models::plan::*;
use axum::http::HeaderMap;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn can_access_plan(
    pool: &PgPool,
    plan_id: Uuid,
    user_id: Option<Uuid>,
    share_token: Option<&str>,
) -> Result<bool, String> {
    // Get plan
    let plan = sqlx::query_as!(Plan, "SELECT * FROM plans WHERE id = $1", plan_id)
        .fetch_one(pool)
        .await
        .map_err(|_| "Plan not found".to_string())?;

    // Public plans - everyone can access
    if plan.is_public {
        return Ok(true);
    }

    // Owner can always access
    if let Some(uid) = user_id {
        if plan.user_id == uid {
            return Ok(true);
        }

        // Check if user has been granted access
        let has_access = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM plan_access WHERE plan_id = $1 AND user_id = $2)",
            plan_id,
            uid
        )
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or(false);

        if has_access {
            return Ok(true);
        }
    }

    // Check share link
    if let Some(token) = share_token {
        if plan.share_link_enabled && plan.share_token.as_deref() == Some(token) {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn log_access(
    pool: &PgPool,
    plan_id: Uuid,
    user_id: Option<Uuid>,
    accessed_via: &str,
    headers: &HeaderMap,
) -> Result<(), String> {
    // Extract IP address from headers
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
        })
        .map(|s| s.to_string());

    sqlx::query!(
        r#"
        INSERT INTO plan_access_logs (plan_id, user_id, accessed_via, ip_address)
        VALUES ($1, $2, $3, $4)
        "#,
        plan_id,
        user_id,
        accessed_via,
        ip_address
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn regenerate_share_token(pool: &PgPool, plan_id: Uuid) -> Result<String, String> {
    let new_token = format!("{:x}", md5::compute(uuid::Uuid::new_v4().to_string()));

    sqlx::query!(
        "UPDATE plans SET share_token = $1 WHERE id = $2",
        new_token,
        plan_id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(new_token)
}
```

Add to `Cargo.toml`:
```toml
md5 = "0.7"
```

Update `backend/src/services/mod.rs`:
```rust
pub mod session;
pub mod password;
pub mod elo;
pub mod seasons;
pub mod jobs;
pub mod anthropic;
pub mod plan_service;
pub mod notification_service;
pub mod access_control;
```

---

### 2.3 Update Plan Handlers - `backend/src/handlers/plans.rs`

Add access control middleware:

```rust
use crate::services::access_control;
use axum::http::HeaderMap;

// Update get_plan to check access
pub async fn get_plan(
    headers: HeaderMap,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    user: Option<User>,  // Make user optional
) -> Result<Json<PlanDetailResponse>, (StatusCode, String)> {
    // Extract share token from query params
    let share_token = headers
        .get("x-share-token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check access
    let can_access = access_control::can_access_plan(
        &pool,
        plan_id,
        user.as_ref().map(|u| u.id),
        share_token.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if !can_access {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Determine access method
    let accessed_via = if let Some(ref u) = user {
        let is_owner = sqlx::query_scalar!(
            "SELECT user_id = $1 FROM plans WHERE id = $2",
            u.id,
            plan_id
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if is_owner {
            "owner"
        } else {
            "invited"
        }
    } else {
        "share_link"
    };

    // Log access
    let _ = access_control::log_access(
        &pool,
        plan_id,
        user.as_ref().map(|u| u.id),
        accessed_via,
        &headers,
    )
    .await;

    // Fetch plan (remove is_public filter)
    let plan = sqlx::query_as!(Plan, "SELECT * FROM plans WHERE id = $1", plan_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    // ... rest of existing code ...
}
```

Add new handlers:

```rust
// Update plan privacy settings
pub async fn update_plan_privacy(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    Json(req): Json<UpdatePlanPrivacyRequest>,
) -> Result<Json<Plan>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can change privacy".to_string()));
    }

    // Update fields
    if let Some(is_public) = req.is_public {
        sqlx::query!("UPDATE plans SET is_public = $1 WHERE id = $2", is_public, plan_id)
            .execute(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(share_link_enabled) = req.share_link_enabled {
        sqlx::query!(
            "UPDATE plans SET share_link_enabled = $1 WHERE id = $2",
            share_link_enabled,
            plan_id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Fetch updated plan
    let plan = sqlx::query_as!(Plan, "SELECT * FROM plans WHERE id = $1", plan_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    Ok(Json(plan))
}

// Get share link
pub async fn get_share_link(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<ShareLinkResponse>, (StatusCode, String)> {
    let plan = sqlx::query_as!(Plan, "SELECT * FROM plans WHERE id = $1 AND user_id = $2", plan_id, user.id)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    let share_url = format!(
        "{}/plans/{}?token={}",
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string()),
        plan_id,
        plan.share_token.unwrap_or_default()
    );

    Ok(Json(ShareLinkResponse {
        share_url,
        enabled: plan.share_link_enabled,
    }))
}

// Regenerate share link
pub async fn regenerate_share_link(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<ShareLinkResponse>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can regenerate link".to_string()));
    }

    let new_token = access_control::regenerate_share_token(&pool, plan_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let share_url = format!(
        "{}/plans/{}?token={}",
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string()),
        plan_id,
        new_token
    );

    Ok(Json(ShareLinkResponse {
        share_url,
        enabled: true,
    }))
}

// Grant access to user
pub async fn grant_access(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    Json(req): Json<GrantAccessRequest>,
) -> Result<Json<PlanAccessWithUser>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can grant access".to_string()));
    }

    // Get user ID from username
    let target_user_id: Uuid = sqlx::query_scalar!(
        "SELECT id FROM users WHERE username = $1",
        req.username
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Grant access
    let access = sqlx::query_as!(
        PlanAccess,
        r#"
        INSERT INTO plan_access (plan_id, user_id, granted_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (plan_id, user_id) DO NOTHING
        RETURNING *
        "#,
        plan_id,
        target_user_id,
        user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PlanAccessWithUser {
        access,
        username: req.username,
        granted_by_username: user.username,
    }))
}

// Revoke access
pub async fn revoke_access(
    user: User,
    State(pool): State<PgPool>,
    Path((plan_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can revoke access".to_string()));
    }

    sqlx::query!(
        "DELETE FROM plan_access WHERE plan_id = $1 AND user_id = $2",
        plan_id,
        target_user_id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Access revoked" })))
}

// List users with access
pub async fn list_plan_access(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlanAccessWithUser>>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can view access list".to_string()));
    }

    let accesses = sqlx::query!(
        r#"
        SELECT
            pa.plan_id, pa.user_id, pa.granted_by, pa.granted_at,
            u.username,
            g.username as granted_by_username
        FROM plan_access pa
        JOIN users u ON pa.user_id = u.id
        JOIN users g ON pa.granted_by = g.id
        WHERE pa.plan_id = $1
        ORDER BY pa.granted_at DESC
        "#,
        plan_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result = accesses
        .into_iter()
        .map(|row| PlanAccessWithUser {
            access: PlanAccess {
                plan_id: row.plan_id,
                user_id: row.user_id,
                granted_by: row.granted_by,
                granted_at: row.granted_at,
            },
            username: row.username,
            granted_by_username: row.granted_by_username,
        })
        .collect();

    Ok(Json(result))
}

// View access logs
pub async fn view_access_logs(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlanAccessLog>>, (StatusCode, String)> {
    // Verify ownership
    let is_owner = sqlx::query_scalar!(
        "SELECT user_id = $1 FROM plans WHERE id = $2",
        user.id,
        plan_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?
    .unwrap_or(false);

    if !is_owner {
        return Err((StatusCode::FORBIDDEN, "Only owner can view access logs".to_string()));
    }

    let logs = sqlx::query_as!(
        PlanAccessLog,
        "SELECT * FROM plan_access_logs WHERE plan_id = $1 ORDER BY accessed_at DESC LIMIT 100",
        plan_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}
```

---

### 2.4 Update Routes - `backend/src/main.rs`

```rust
// Plan privacy (owner only)
.route("/api/plans/:plan_id/privacy", put(plans::update_plan_privacy)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/share-link", get(plans::get_share_link)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/share-link/regenerate", post(plans::regenerate_share_link)
    .layer(from_fn_with_state(pool.clone(), require_auth)))

// Plan access management
.route("/api/plans/:plan_id/access", get(plans::list_plan_access)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/access", post(plans::grant_access)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/access/:user_id", delete(plans::revoke_access)
    .layer(from_fn_with_state(pool.clone(), require_auth)))

// Access logs
.route("/api/plans/:plan_id/access-logs", get(plans::view_access_logs)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
```

---

## Frontend Implementation

### 3.1 Update API Client - `frontend/src/lib/api/plans.ts`

```typescript
export interface ShareLinkResponse {
  share_url: string;
  enabled: boolean;
}

export interface PlanAccessWithUser {
  plan_id: string;
  user_id: string;
  granted_by: string;
  granted_at: string;
  username: string;
  granted_by_username: string;
}

export const plansApi = {
  // ... existing methods ...

  async updatePrivacy(planId: string, settings: {
    is_public?: boolean;
    share_link_enabled?: boolean;
  }): Promise<Plan> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/privacy`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(settings),
    });
    if (!response.ok) throw new Error('Failed to update privacy');
    return response.json();
  },

  async getShareLink(planId: string): Promise<ShareLinkResponse> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/share-link`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to get share link');
    return response.json();
  },

  async regenerateShareLink(planId: string): Promise<ShareLinkResponse> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/share-link/regenerate`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to regenerate link');
    return response.json();
  },

  async grantAccess(planId: string, username: string): Promise<PlanAccessWithUser> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/access`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ username }),
    });
    if (!response.ok) throw new Error('Failed to grant access');
    return response.json();
  },

  async revokeAccess(planId: string, userId: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/access/${userId}`, {
      method: 'DELETE',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to revoke access');
  },

  async listAccess(planId: string): Promise<PlanAccessWithUser[]> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/access`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to list access');
    return response.json();
  },
};
```

---

### 3.2 Create Privacy Settings Component - `frontend/src/lib/components/PlanPrivacySettings.svelte`

```svelte
<script lang="ts">
  import { plansApi, type Plan, type ShareLinkResponse, type PlanAccessWithUser } from '$lib/api/plans';

  interface Props {
    plan: Plan;
    onupdate: () => void;
  }

  let { plan, onupdate }: Props = $props();

  let shareLink = $state<ShareLinkResponse | null>(null);
  let accessList = $state<PlanAccessWithUser[]>([]);
  let newUsername = $state('');
  let loading = $state(false);
  let showSettings = $state(false);

  async function togglePublic() {
    try {
      await plansApi.updatePrivacy(plan.id, { is_public: !plan.is_public });
      onupdate();
    } catch (e) {
      alert('Failed to update visibility');
    }
  }

  async function toggleShareLink() {
    try {
      await plansApi.updatePrivacy(plan.id, { share_link_enabled: !plan.share_link_enabled });
      onupdate();
      if (!plan.share_link_enabled) {
        await loadShareLink();
      }
    } catch (e) {
      alert('Failed to toggle share link');
    }
  }

  async function loadShareLink() {
    try {
      shareLink = await plansApi.getShareLink(plan.id);
    } catch (e) {
      console.error('Failed to load share link', e);
    }
  }

  async function regenerateLink() {
    if (!confirm('Regenerate share link? The old link will stop working.')) return;

    try {
      shareLink = await plansApi.regenerateShareLink(plan.id);
    } catch (e) {
      alert('Failed to regenerate link');
    }
  }

  async function copyShareLink() {
    if (!shareLink) return;
    try {
      await navigator.clipboard.writeText(shareLink.share_url);
      alert('Link copied to clipboard!');
    } catch (e) {
      alert('Failed to copy link');
    }
  }

  async function loadAccessList() {
    try {
      accessList = await plansApi.listAccess(plan.id);
    } catch (e) {
      console.error('Failed to load access list', e);
    }
  }

  async function grantAccess() {
    if (!newUsername.trim()) return;

    loading = true;
    try {
      await plansApi.grantAccess(plan.id, newUsername.trim());
      newUsername = '';
      await loadAccessList();
    } catch (e) {
      alert('Failed to grant access. User may not exist.');
    } finally {
      loading = false;
    }
  }

  async function revokeAccess(userId: string) {
    if (!confirm('Revoke access for this user?')) return;

    try {
      await plansApi.revokeAccess(plan.id, userId);
      await loadAccessList();
    } catch (e) {
      alert('Failed to revoke access');
    }
  }

  async function openSettings() {
    showSettings = true;
    if (!plan.is_public && plan.share_link_enabled) {
      await loadShareLink();
    }
    if (!plan.is_public) {
      await loadAccessList();
    }
  }
</script>

<div class="border rounded p-4 bg-white dark:bg-gray-900">
  <button
    onclick={openSettings}
    class="w-full flex items-center justify-between text-left"
  >
    <h3 class="font-semibold">🔒 Privacy Settings</h3>
    <span class="text-sm">{showSettings ? '▼' : '▶'}</span>
  </button>

  {#if showSettings}
    <div class="mt-4 space-y-4">
      <!-- Public/Private Toggle -->
      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium">Visibility</p>
          <p class="text-sm text-gray-600">
            {plan.is_public ? 'Public - Anyone can view' : 'Private - Only you and invited users'}
          </p>
        </div>
        <button
          onclick={togglePublic}
          class="px-4 py-2 rounded text-sm font-medium"
          class:bg-green-600={plan.is_public}
          class:bg-gray-600={!plan.is_public}
          class:text-white={true}
        >
          {plan.is_public ? 'Public' : 'Private'}
        </button>
      </div>

      {#if !plan.is_public}
        <!-- Share Link -->
        <div class="border-t pt-4">
          <div class="flex items-center justify-between mb-2">
            <p class="font-medium">Share Link</p>
            <label class="flex items-center gap-2">
              <input
                type="checkbox"
                checked={plan.share_link_enabled}
                onchange={toggleShareLink}
                class="rounded"
              />
              <span class="text-sm">Enabled</span>
            </label>
          </div>

          {#if plan.share_link_enabled && shareLink}
            <div class="bg-gray-50 dark:bg-gray-800 p-3 rounded">
              <p class="text-xs font-mono break-all mb-2">{shareLink.share_url}</p>
              <div class="flex gap-2">
                <button
                  onclick={copyShareLink}
                  class="text-sm bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded"
                >
                  📋 Copy
                </button>
                <button
                  onclick={regenerateLink}
                  class="text-sm bg-gray-600 hover:bg-gray-700 text-white px-3 py-1 rounded"
                >
                  🔄 Regenerate
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Invited Users -->
        <div class="border-t pt-4">
          <p class="font-medium mb-2">Invited Users</p>

          <div class="flex gap-2 mb-3">
            <input
              type="text"
              bind:value={newUsername}
              placeholder="Enter username"
              class="flex-1 border rounded px-3 py-2 text-sm"
            />
            <button
              onclick={grantAccess}
              disabled={loading || !newUsername.trim()}
              class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white px-4 py-2 rounded text-sm"
            >
              {loading ? 'Adding...' : 'Add'}
            </button>
          </div>

          {#if accessList.length > 0}
            <div class="space-y-2">
              {#each accessList as access}
                <div class="flex items-center justify-between bg-gray-50 dark:bg-gray-800 p-2 rounded">
                  <div>
                    <p class="text-sm font-medium">@{access.username}</p>
                    <p class="text-xs text-gray-600">
                      Added by @{access.granted_by_username} on {new Date(access.granted_at).toLocaleDateString()}
                    </p>
                  </div>
                  <button
                    onclick={() => revokeAccess(access.user_id)}
                    class="text-sm text-red-600 hover:underline"
                  >
                    Revoke
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-gray-600">No invited users yet</p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
```

---

### 3.3 Add to Plan Viewer

```svelte
<script lang="ts">
  // ... existing imports ...
  import PlanPrivacySettings from '$lib/components/PlanPrivacySettings.svelte';

  // ... existing code ...
</script>

<!-- Add after Version History -->
{#if isOwner}
  <div class="mb-6">
    <PlanPrivacySettings plan={data.plan} onupdate={loadPlan} />
  </div>
{/if}
```

---

## Testing Checklist

- [ ] Toggle plan public/private
- [ ] Private plan not accessible to other users
- [ ] Enable share link
- [ ] Access plan via share link
- [ ] Regenerate share link - old link stops working
- [ ] Grant access to user by username
- [ ] Invited user can access private plan
- [ ] Revoke access - user can no longer access
- [ ] Access logs show all visitors
- [ ] Owner always has access

---

## Implementation Time

- Backend: 2 days
- Frontend: 1.5 days
- Testing: 0.5 days
- **Total: 4 days**

---

## Next Extension

👉 **Extension 07 - Search & Auto-sync**

See: `07-search-and-autosync.md`
