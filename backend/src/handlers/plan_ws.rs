use axum::{
    Extension,
    extract::{
        ConnectInfo, Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::plan::CommentWithAuthor;
use crate::services::plan_broadcast::PlanBroadcastState;
use crate::services::session::validate_session;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlanMessage {
    CommentAdded {
        plan_id: String,
        comment: CommentWithAuthor,
    },
    CommentUpdated {
        plan_id: String,
        comment: CommentWithAuthor,
    },
    CommentDeleted {
        plan_id: String,
        comment_id: String,
    },
}

pub async fn plan_websocket_handler(
    Path(plan_id): Path<Uuid>,
    State(pool): State<PgPool>,
    Extension(broadcast_state): Extension<PlanBroadcastState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    cookies: Cookies,
    ws: WebSocketUpgrade,
) -> Result<Response, AuthError> {
    tracing::info!(
        "Plan WebSocket upgrade request for plan: {} from {}",
        plan_id,
        addr
    );

    let client_ip = addr.ip();
    if let Err(err) = broadcast_state.check_connection_limit(client_ip).await {
        tracing::warn!(
            "Rate limit exceeded for IP {} on plan {}: {}",
            client_ip,
            plan_id,
            err
        );
        return Err(AuthError::Unauthorized);
    }

    // Validate plan exists and check access permissions
    let plan = sqlx::query_as::<_, (Uuid, bool, Uuid)>(
        r#"
        SELECT id, is_public, owner_id
        FROM plans
        WHERE id = $1
        "#,
    )
    .bind(plan_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking plan: {}", e);
        AuthError::Unauthorized
    })?
    .ok_or_else(|| {
        tracing::warn!(
            "WebSocket connection attempted for non-existent plan: {}",
            plan_id
        );
        AuthError::Unauthorized
    })?;

    let (_, is_public, owner_id) = plan;

    // Authentication is optional for public plans
    let user = if let Some(cookie) = cookies.get("session_id") {
        let session_id = cookie.value().to_string();
        (validate_session(&pool, &session_id).await).ok()
    } else {
        None
    };

    // For private plans, require authentication and ownership
    if !is_public {
        let user = user.ok_or_else(|| {
            tracing::warn!(
                "Unauthenticated WebSocket connection attempted for private plan: {}",
                plan_id
            );
            AuthError::Unauthorized
        })?;

        if user.id != owner_id {
            tracing::warn!(
                "User {} attempted WebSocket connection to plan {} owned by {}",
                user.id,
                plan_id,
                owner_id
            );
            return Err(AuthError::Unauthorized);
        }
    }

    Ok(
        ws.on_upgrade(move |socket| {
            handle_plan_socket(socket, plan_id, broadcast_state, client_ip)
        }),
    )
}

async fn handle_plan_socket(
    socket: WebSocket,
    plan_id: Uuid,
    broadcast_state: PlanBroadcastState,
    client_ip: std::net::IpAddr,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<PlanMessage>();

    let plan_id_str = plan_id.to_string();

    broadcast_state.subscribe(&plan_id_str, tx.clone()).await;

    let plan_id_for_send = plan_id_str.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && sender.send(Message::Text(json.into())).await.is_err()
            {
                tracing::debug!(
                    "Failed to send message to client for plan: {}",
                    plan_id_for_send
                );
                break;
            }
        }
    });

    // Handle incoming WebSocket messages
    // Note: Axum automatically responds to ping with pong
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Ping(_)) => {
                    tracing::debug!("Received ping");
                }
                Ok(Message::Pong(_)) => {
                    tracing::debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client requested close");
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => {
            tracing::debug!("Send task completed for plan: {}", plan_id);
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            tracing::debug!("Receive task completed for plan: {}", plan_id);
            send_task.abort();
        },
    }

    // Cleanup
    broadcast_state.unsubscribe(&plan_id_str).await;
    broadcast_state.release_connection(client_ip).await;

    tracing::info!("Client {} disconnected from plan: {}", client_ip, plan_id);
}
