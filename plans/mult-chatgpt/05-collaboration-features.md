# Extension 05 - Collaboration Features (@mentions & Notifications)

**Builds on:** Extension 04 - Plan Diffs
**Next:** Extension 06 - Private Plans

---

## What This Adds

Enhanced collaboration with @mentions and notification system.

**New Features:**
- @mention users in comments and discussions
- Email notifications for mentions
- Email notifications for new comments on your plans
- In-app notification center
- Notification preferences
- Subscribe/unsubscribe from plans

---

## Database Changes

### Migration: `backend/migrations/009_add_notifications.sql`

```sql
-- Notification types
CREATE TYPE notification_type AS ENUM (
    'mention',
    'comment_on_plan',
    'comment_accepted',
    'comment_rejected',
    'discussion_reply'
);

-- Notifications table
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type notification_type NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    link VARCHAR(500),  -- URL to navigate to
    is_read BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_is_read ON notifications(is_read);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);

-- Mentions table (for tracking @mentions)
CREATE TABLE mentions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mentioned_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mentioner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment_id UUID REFERENCES plan_comments(id) ON DELETE CASCADE,
    discussion_id UUID REFERENCES comment_discussions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (comment_id IS NOT NULL AND discussion_id IS NULL) OR
        (comment_id IS NULL AND discussion_id IS NOT NULL)
    )
);

CREATE INDEX idx_mentions_mentioned_user_id ON mentions(mentioned_user_id);

-- Plan subscriptions (users watching plans)
CREATE TABLE plan_subscriptions (
    plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscribed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plan_id, user_id)
);

CREATE INDEX idx_plan_subscriptions_user_id ON plan_subscriptions(user_id);

-- Notification preferences
CREATE TABLE notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    email_on_mention BOOLEAN DEFAULT true,
    email_on_comment BOOLEAN DEFAULT true,
    email_on_acceptance BOOLEAN DEFAULT true,
    email_digest BOOLEAN DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Backend Implementation

### 2.1 Update Models - `backend/src/models/notification.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
pub enum NotificationType {
    Mention,
    CommentOnPlan,
    CommentAccepted,
    CommentRejected,
    DiscussionReply,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub link: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Mention {
    pub id: Uuid,
    pub mentioned_user_id: Uuid,
    pub mentioner_user_id: Uuid,
    pub comment_id: Option<Uuid>,
    pub discussion_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlanSubscription {
    pub plan_id: Uuid,
    pub user_id: Uuid,
    pub subscribed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationPreferences {
    pub user_id: Uuid,
    pub email_on_mention: bool,
    pub email_on_comment: bool,
    pub email_on_acceptance: bool,
    pub email_digest: bool,
    pub updated_at: DateTime<Utc>,
}

// DTOs
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub email_on_mention: Option<bool>,
    pub email_on_comment: Option<bool>,
    pub email_on_acceptance: Option<bool>,
    pub email_digest: Option<bool>,
}
```

Update `backend/src/models/mod.rs`:
```rust
pub mod user;
pub mod plan;
pub mod notification;
```

---

### 2.2 Create Notification Service - `backend/src/services/notification_service.rs`

```rust
use crate::models::notification::*;
use sqlx::PgPool;
use uuid::Uuid;
use regex::Regex;

pub async fn create_notification(
    pool: &PgPool,
    user_id: Uuid,
    notification_type: NotificationType,
    title: String,
    message: String,
    link: Option<String>,
) -> Result<Notification, sqlx::Error> {
    let notification = sqlx::query_as!(
        Notification,
        r#"
        INSERT INTO notifications (user_id, type, title, message, link)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, type as "notification_type: NotificationType",
                  title, message, link, is_read, created_at
        "#,
        user_id,
        notification_type as NotificationType,
        title,
        message,
        link
    )
    .fetch_one(pool)
    .await?;

    Ok(notification)
}

pub async fn extract_and_create_mentions(
    pool: &PgPool,
    text: &str,
    mentioner_user_id: Uuid,
    comment_id: Option<Uuid>,
    discussion_id: Option<Uuid>,
    plan_id: Uuid,
) -> Result<Vec<Mention>, String> {
    // Extract @mentions using regex
    let mention_regex = Regex::new(r"@(\w+)").unwrap();
    let mut mentioned_usernames: Vec<String> = vec![];

    for cap in mention_regex.captures_iter(text) {
        if let Some(username) = cap.get(1) {
            mentioned_usernames.push(username.as_str().to_string());
        }
    }

    if mentioned_usernames.is_empty() {
        return Ok(vec![]);
    }

    // Get user IDs for mentioned usernames
    let user_ids: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM users WHERE username = ANY($1)",
        &mentioned_usernames
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut mentions = vec![];

    for user_id in user_ids {
        // Don't mention yourself
        if user_id == mentioner_user_id {
            continue;
        }

        // Create mention record
        let mention = sqlx::query_as!(
            Mention,
            r#"
            INSERT INTO mentions (mentioned_user_id, mentioner_user_id, comment_id, discussion_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id, mentioned_user_id, mentioner_user_id, comment_id, discussion_id, created_at
            "#,
            user_id,
            mentioner_user_id,
            comment_id,
            discussion_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        // Get mentioner username
        let mentioner_username: String = sqlx::query_scalar!(
            "SELECT username FROM users WHERE id = $1",
            mentioner_user_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        // Create notification
        let link = if let Some(cid) = comment_id {
            Some(format!("/plans/{}#comment-{}", plan_id, cid))
        } else if discussion_id.is_some() {
            Some(format!("/plans/{}", plan_id))
        } else {
            None
        };

        create_notification(
            pool,
            user_id,
            NotificationType::Mention,
            format!("@{} mentioned you", mentioner_username),
            text.chars().take(200).collect::<String>(),
            link,
        )
        .await
        .map_err(|e| e.to_string())?;

        mentions.push(mention);
    }

    Ok(mentions)
}

pub async fn notify_plan_owner_of_comment(
    pool: &PgPool,
    plan_id: Uuid,
    comment_id: Uuid,
    commenter_username: &str,
    plan_title: &str,
) -> Result<(), String> {
    // Get plan owner
    let owner_id: Uuid = sqlx::query_scalar!("SELECT user_id FROM plans WHERE id = $1", plan_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Check preferences
    let prefs = get_or_create_preferences(pool, owner_id).await?;

    if !prefs.email_on_comment {
        return Ok(());
    }

    // Create notification
    create_notification(
        pool,
        owner_id,
        NotificationType::CommentOnPlan,
        format!("New comment on '{}'", plan_title),
        format!("@{} commented on your plan", commenter_username),
        Some(format!("/plans/{}#comment-{}", plan_id, comment_id)),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn notify_subscribers(
    pool: &PgPool,
    plan_id: Uuid,
    exclude_user_id: Uuid,
    title: String,
    message: String,
) -> Result<(), String> {
    let subscribers = sqlx::query_scalar!(
        "SELECT user_id FROM plan_subscriptions WHERE plan_id = $1 AND user_id != $2",
        plan_id,
        exclude_user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for user_id in subscribers {
        create_notification(
            pool,
            user_id,
            NotificationType::CommentOnPlan,
            title.clone(),
            message.clone(),
            Some(format!("/plans/{}", plan_id)),
        )
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn get_or_create_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<NotificationPreferences, String> {
    let prefs = sqlx::query_as!(
        NotificationPreferences,
        "SELECT * FROM notification_preferences WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(prefs) = prefs {
        Ok(prefs)
    } else {
        // Create default preferences
        let new_prefs = sqlx::query_as!(
            NotificationPreferences,
            r#"
            INSERT INTO notification_preferences (user_id)
            VALUES ($1)
            RETURNING *
            "#,
            user_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(new_prefs)
    }
}
```

Add to `Cargo.toml`:
```toml
regex = "1.10"
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
```

---

### 2.3 Update Handlers - `backend/src/handlers/notifications.rs`

```rust
use crate::middleware::auth::User;
use crate::models::notification::*;
use crate::services::notification_service;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// Get user's notifications
pub async fn get_notifications(
    user: User,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Notification>>, (StatusCode, String)> {
    let notifications = sqlx::query_as!(
        Notification,
        r#"
        SELECT id, user_id, type as "notification_type: NotificationType",
               title, message, link, is_read, created_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
        user.id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(notifications))
}

// Mark notification as read
pub async fn mark_notification_read(
    user: User,
    State(pool): State<PgPool>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    sqlx::query!(
        "UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2",
        notification_id,
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Notification marked as read" })))
}

// Mark all notifications as read
pub async fn mark_all_notifications_read(
    user: User,
    State(pool): State<PgPool>,
) -> Result<Json<Value>, (StatusCode, String)> {
    sqlx::query!(
        "UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false",
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "All notifications marked as read" })))
}

// Get unread count
pub async fn get_unread_count(
    user: User,
    State(pool): State<PgPool>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false",
        user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(0);

    Ok(Json(json!({ "unread_count": count })))
}

// Get notification preferences
pub async fn get_preferences(
    user: User,
    State(pool): State<PgPool>,
) -> Result<Json<NotificationPreferences>, (StatusCode, String)> {
    let prefs = notification_service::get_or_create_preferences(&pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(prefs))
}

// Update notification preferences
pub async fn update_preferences(
    user: User,
    State(pool): State<PgPool>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<Json<NotificationPreferences>, (StatusCode, String)> {
    // Ensure preferences exist
    notification_service::get_or_create_preferences(&pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Update only provided fields
    if let Some(email_on_mention) = req.email_on_mention {
        sqlx::query!(
            "UPDATE notification_preferences SET email_on_mention = $1 WHERE user_id = $2",
            email_on_mention,
            user.id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(email_on_comment) = req.email_on_comment {
        sqlx::query!(
            "UPDATE notification_preferences SET email_on_comment = $1 WHERE user_id = $2",
            email_on_comment,
            user.id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(email_on_acceptance) = req.email_on_acceptance {
        sqlx::query!(
            "UPDATE notification_preferences SET email_on_acceptance = $1 WHERE user_id = $2",
            email_on_acceptance,
            user.id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(email_digest) = req.email_digest {
        sqlx::query!(
            "UPDATE notification_preferences SET email_digest = $1 WHERE user_id = $2",
            email_digest,
            user.id
        )
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Fetch updated preferences
    let prefs = notification_service::get_or_create_preferences(&pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(prefs))
}

// Subscribe to plan
pub async fn subscribe_to_plan(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    sqlx::query!(
        "INSERT INTO plan_subscriptions (plan_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        plan_id,
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Subscribed to plan" })))
}

// Unsubscribe from plan
pub async fn unsubscribe_from_plan(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    sqlx::query!(
        "DELETE FROM plan_subscriptions WHERE plan_id = $1 AND user_id = $2",
        plan_id,
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Unsubscribed from plan" })))
}

// Check if user is subscribed
pub async fn check_subscription(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let is_subscribed = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM plan_subscriptions WHERE plan_id = $1 AND user_id = $2)",
        plan_id,
        user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(false);

    Ok(Json(json!({ "is_subscribed": is_subscribed })))
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
pub mod plans;
pub mod notifications;
```

---

### 2.4 Update Plan Handlers to Create Notifications

In `backend/src/handlers/plans.rs`, update `create_comment`:

```rust
pub async fn create_comment(
    user: User,
    State(pool): State<PgPool>,
    Path(plan_id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<PlanComment>, (StatusCode, String)> {
    // ... existing validation ...

    let comment = sqlx::query_as!(
        PlanComment,
        r#"
        INSERT INTO plan_comments
            (plan_id, user_id, content, start_line, end_line, selected_lines, plan_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, plan_id, user_id, content, start_line, end_line, selected_lines,
                  plan_version, status as "status: CommentStatus",
                  resolved_at, resolved_by, created_at, updated_at
        "#,
        plan_id,
        user.id,
        req.content,
        req.start_line,
        req.end_line,
        req.selected_lines,
        current_version
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Extract mentions
    let _ = notification_service::extract_and_create_mentions(
        &pool,
        &req.content,
        user.id,
        Some(comment.id),
        None,
        plan_id,
    )
    .await;

    // Notify plan owner (async, don't block)
    let pool_clone = pool.clone();
    let comment_id = comment.id;
    let username = user.username.clone();
    let plan_title = plan.title.clone();

    tokio::spawn(async move {
        let _ = notification_service::notify_plan_owner_of_comment(
            &pool_clone,
            plan_id,
            comment_id,
            &username,
            &plan_title,
        )
        .await;
    });

    Ok(Json(comment))
}
```

Update `add_discussion_message`:

```rust
pub async fn add_discussion_message(
    user: User,
    State(pool): State<PgPool>,
    Path(comment_id): Path<Uuid>,
    Json(req): Json<AddDiscussionRequest>,
) -> Result<Json<DiscussionWithAuthor>, (StatusCode, String)> {
    // ... existing code ...

    // Extract mentions
    let comment = sqlx::query!("SELECT plan_id FROM plan_comments WHERE id = $1", comment_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Comment not found".to_string()))?;

    let _ = notification_service::extract_and_create_mentions(
        &pool,
        &req.message,
        user.id,
        None,
        Some(discussion.id),
        comment.plan_id,
    )
    .await;

    // ... rest of function ...
}
```

---

### 2.5 Update Routes - `backend/src/main.rs`

```rust
use crate::handlers::notifications;

// Authenticated notification routes
.route("/api/notifications", get(notifications::get_notifications)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/notifications/unread-count", get(notifications::get_unread_count)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/notifications/:id/read", post(notifications::mark_notification_read)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/notifications/mark-all-read", post(notifications::mark_all_notifications_read)
    .layer(from_fn_with_state(pool.clone(), require_auth)))

// Notification preferences
.route("/api/notifications/preferences", get(notifications::get_preferences)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/notifications/preferences", put(notifications::update_preferences)
    .layer(from_fn_with_state(pool.clone(), require_auth)))

// Plan subscriptions
.route("/api/plans/:plan_id/subscribe", post(notifications::subscribe_to_plan)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/unsubscribe", post(notifications::unsubscribe_from_plan)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
.route("/api/plans/:plan_id/subscription", get(notifications::check_subscription)
    .layer(from_fn_with_state(pool.clone(), require_auth)))
```

---

## Frontend Implementation

### 3.1 Update API Client - `frontend/src/lib/api/notifications.ts`

```typescript
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export type NotificationType =
  | 'mention'
  | 'comment_on_plan'
  | 'comment_accepted'
  | 'comment_rejected'
  | 'discussion_reply';

export interface Notification {
  id: string;
  user_id: string;
  notification_type: NotificationType;
  title: string;
  message: string;
  link?: string;
  is_read: boolean;
  created_at: string;
}

export interface NotificationPreferences {
  user_id: string;
  email_on_mention: boolean;
  email_on_comment: boolean;
  email_on_acceptance: boolean;
  email_digest: boolean;
  updated_at: string;
}

export const notificationsApi = {
  async getNotifications(): Promise<Notification[]> {
    const response = await fetch(`${API_URL}/api/notifications`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to fetch notifications');
    return response.json();
  },

  async getUnreadCount(): Promise<number> {
    const response = await fetch(`${API_URL}/api/notifications/unread-count`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to fetch unread count');
    const data = await response.json();
    return data.unread_count;
  },

  async markAsRead(notificationId: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/notifications/${notificationId}/read`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to mark notification as read');
  },

  async markAllAsRead(): Promise<void> {
    const response = await fetch(`${API_URL}/api/notifications/mark-all-read`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to mark all as read');
  },

  async getPreferences(): Promise<NotificationPreferences> {
    const response = await fetch(`${API_URL}/api/notifications/preferences`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to fetch preferences');
    return response.json();
  },

  async updatePreferences(prefs: Partial<NotificationPreferences>): Promise<NotificationPreferences> {
    const response = await fetch(`${API_URL}/api/notifications/preferences`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(prefs),
    });
    if (!response.ok) throw new Error('Failed to update preferences');
    return response.json();
  },

  async subscribeToPlan(planId: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/subscribe`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to subscribe');
  },

  async unsubscribeFromPlan(planId: string): Promise<void> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/unsubscribe`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to unsubscribe');
  },

  async checkSubscription(planId: string): Promise<boolean> {
    const response = await fetch(`${API_URL}/api/plans/${planId}/subscription`, {
      credentials: 'include',
    });
    if (!response.ok) throw new Error('Failed to check subscription');
    const data = await response.json();
    return data.is_subscribed;
  },
};
```

Export from `frontend/src/lib/api/client.ts`:
```typescript
export * from './plans';
export * from './notifications';
```

---

### 3.2 Create Notification Bell Component - `frontend/src/lib/components/NotificationBell.svelte`

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { notificationsApi, type Notification } from '$lib/api/notifications';

  let notifications = $state<Notification[]>([]);
  let unreadCount = $state(0);
  let showDropdown = $state(false);
  let loading = $state(false);

  onMount(async () => {
    await loadUnreadCount();
    // Poll every 30 seconds
    const interval = setInterval(loadUnreadCount, 30000);
    return () => clearInterval(interval);
  });

  async function loadUnreadCount() {
    try {
      unreadCount = await notificationsApi.getUnreadCount();
    } catch (e) {
      console.error('Failed to load unread count', e);
    }
  }

  async function loadNotifications() {
    loading = true;
    try {
      notifications = await notificationsApi.getNotifications();
    } catch (e) {
      console.error('Failed to load notifications', e);
    } finally {
      loading = false;
    }
  }

  async function handleClick() {
    showDropdown = !showDropdown;
    if (showDropdown && notifications.length === 0) {
      await loadNotifications();
    }
  }

  async function markAsRead(notification: Notification) {
    try {
      await notificationsApi.markAsRead(notification.id);
      notifications = notifications.map((n) =>
        n.id === notification.id ? { ...n, is_read: true } : n
      );
      unreadCount = Math.max(0, unreadCount - 1);

      if (notification.link) {
        goto(notification.link);
        showDropdown = false;
      }
    } catch (e) {
      console.error('Failed to mark as read', e);
    }
  }

  async function markAllAsRead() {
    try {
      await notificationsApi.markAllAsRead();
      notifications = notifications.map((n) => ({ ...n, is_read: true }));
      unreadCount = 0;
    } catch (e) {
      console.error('Failed to mark all as read', e);
    }
  }

  function getNotificationIcon(type: string): string {
    switch (type) {
      case 'mention':
        return '@';
      case 'comment_on_plan':
        return '💬';
      case 'comment_accepted':
        return '✅';
      case 'comment_rejected':
        return '❌';
      case 'discussion_reply':
        return '💭';
      default:
        return '🔔';
    }
  }
</script>

<div class="relative">
  <!-- Bell Button -->
  <button
    onclick={handleClick}
    class="relative p-2 rounded-full hover:bg-gray-200 dark:hover:bg-gray-700 transition"
  >
    <span class="text-2xl">🔔</span>
    {#if unreadCount > 0}
      <span
        class="absolute top-0 right-0 bg-red-500 text-white text-xs rounded-full w-5 h-5 flex items-center justify-center"
      >
        {unreadCount > 9 ? '9+' : unreadCount}
      </span>
    {/if}
  </button>

  <!-- Dropdown -->
  {#if showDropdown}
    <div
      class="absolute right-0 mt-2 w-96 bg-white dark:bg-gray-800 border rounded-lg shadow-lg z-50"
    >
      <div class="border-b p-3 flex justify-between items-center">
        <h3 class="font-semibold">Notifications</h3>
        {#if unreadCount > 0}
          <button
            onclick={markAllAsRead}
            class="text-xs text-blue-600 hover:underline"
          >
            Mark all read
          </button>
        {/if}
      </div>

      <div class="max-h-96 overflow-y-auto">
        {#if loading}
          <p class="p-4 text-sm text-gray-600">Loading...</p>
        {:else if notifications.length === 0}
          <p class="p-4 text-sm text-gray-600">No notifications</p>
        {:else}
          {#each notifications as notification}
            <button
              onclick={() => markAsRead(notification)}
              class="w-full text-left p-3 hover:bg-gray-50 dark:hover:bg-gray-700 transition border-b"
              class:bg-blue-50={!notification.is_read}
              class:dark:bg-blue-900={!notification.is_read}
            >
              <div class="flex items-start gap-2">
                <span class="text-lg">{getNotificationIcon(notification.notification_type)}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold truncate">
                    {notification.title}
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400 truncate">
                    {notification.message}
                  </p>
                  <p class="text-xs text-gray-500 mt-1">
                    {new Date(notification.created_at).toLocaleString()}
                  </p>
                </div>
                {#if !notification.is_read}
                  <div class="w-2 h-2 bg-blue-500 rounded-full flex-shrink-0"></div>
                {/if}
              </div>
            </button>
          {/each}
        {/if}
      </div>

      <div class="border-t p-2">
        <button
          onclick={() => {
            goto('/notifications');
            showDropdown = false;
          }}
          class="w-full text-center text-sm text-blue-600 hover:underline py-2"
        >
          View all notifications
        </button>
      </div>
    </div>
  {/if}
</div>

<!-- Click outside to close -->
{#if showDropdown}
  <button
    class="fixed inset-0 z-40"
    onclick={() => (showDropdown = false)}
    aria-label="Close notifications"
  ></button>
{/if}
```

---

### 3.3 Add to Layout - `frontend/src/routes/+layout.svelte`

```svelte
<script lang="ts">
  import { authStore } from '$lib/stores/auth';
  import NotificationBell from '$lib/components/NotificationBell.svelte';
  // ... other imports
</script>

<nav>
  <a href="/">Home</a>
  <a href="/plans">Plans</a>

  {#if $authStore.user}
    <NotificationBell />
    <span>@{$authStore.user.username}</span>
    <button onclick={handleLogout}>Logout</button>
  {:else}
    <a href="/login">Login</a>
  {/if}
</nav>
```

---

### 3.4 Create Mention Input Component - `frontend/src/lib/components/MentionInput.svelte`

```svelte
<script lang="ts">
  interface Props {
    value: string;
    placeholder?: string;
    rows?: number;
    onchange?: (value: string) => void;
  }

  let { value = $bindable(), placeholder = '', rows = 3 }: Props = $props();

  let textarea: HTMLTextAreaElement;

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    value = target.value;
  }

  function highlightMentions(text: string): string {
    return text.replace(
      /@(\w+)/g,
      '<span class="text-blue-600 dark:text-blue-400 font-semibold">@$1</span>'
    );
  }
</script>

<div class="relative">
  <textarea
    bind:this={textarea}
    {value}
    {placeholder}
    {rows}
    oninput={handleInput}
    class="w-full border rounded px-3 py-2 text-sm font-mono"
  />
  <p class="text-xs text-gray-500 mt-1">
    💡 Type @username to mention someone
  </p>
</div>
```

---

## Testing Checklist

- [ ] @mention user in comment - they receive notification
- [ ] @mention user in discussion - they receive notification
- [ ] New comment on plan - owner receives notification
- [ ] Accept comment - commenter receives notification
- [ ] Reject comment - commenter receives notification
- [ ] Notification bell shows unread count
- [ ] Click notification marks as read and navigates
- [ ] Mark all as read works
- [ ] Subscribe to plan
- [ ] Unsubscribe from plan
- [ ] Update notification preferences
- [ ] Email preferences respected (if email implemented)

---

## Implementation Time

- Backend: 2 days
- Frontend: 2 days
- Testing: 1 day
- **Total: 5 days**

---

## Next Extension

👉 **Extension 06 - Private Plans & Sharing**

See: `06-private-plans.md`
