use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::user::User;
use crate::services::presence::{PresenceMessage, PresenceState};
use crate::services::session::validate_session;

pub async fn websocket_handler(
    State(pool): State<PgPool>,
    Extension(presence_state): Extension<PresenceState>,
    cookies: Cookies,
    ws: WebSocketUpgrade,
) -> Result<Response, AuthError> {
    tracing::info!("WebSocket upgrade request received");

    // Extract and validate session from cookie
    let cookie = cookies.get("session_id").ok_or_else(|| {
        tracing::error!("No session cookie found for WebSocket connection");
        AuthError::Unauthorized
    })?;
    let session_id = cookie.value().to_string();

    tracing::info!("Validating session for WebSocket: {}", session_id);

    // Validate session and get user
    let user = validate_session(&pool, &session_id).await.map_err(|e| {
        tracing::error!("Session validation failed for WebSocket: {:?}", e);
        e
    })?;

    tracing::info!(
        "WebSocket authenticated for user: {} ({})",
        user.username,
        user.id
    );

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, presence_state, user)))
}

async fn handle_socket(socket: WebSocket, presence_state: PresenceState, user: User) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<PresenceMessage>();

    let session_id = Uuid::new_v4();
    let user_id = user.id;
    let username = user.username.clone();

    // Spawn a task to forward messages from rx to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg)
                && sender.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    });

    // Current page path for this session - shared between recv_task and cleanup
    let current_page = std::sync::Arc::new(tokio::sync::RwLock::new(Option::<String>::None));

    // Clone Arc for recv_task
    let current_page_clone = current_page.clone();
    let presence_state_clone = presence_state.clone();

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(msg) = serde_json::from_str::<PresenceMessage>(&text) {
                match msg {
                    PresenceMessage::Join { page_path } => {
                        // Leave previous page if any
                        {
                            let prev_page = current_page_clone.read().await;
                            if let Some(prev_page) = prev_page.as_ref() {
                                presence_state_clone
                                    .broadcast_to_page(
                                        prev_page,
                                        PresenceMessage::PresenceUpdate {
                                            users: presence_state_clone
                                                .get_page_users(prev_page)
                                                .await,
                                        },
                                    )
                                    .await;
                            }
                        }

                        // Join new page
                        presence_state_clone
                            .join(
                                session_id,
                                user_id,
                                username.clone(),
                                page_path.clone(),
                                tx.clone(),
                            )
                            .await;

                        // Update current page
                        *current_page_clone.write().await = Some(page_path.clone());

                        // Broadcast updated presence to all users on this page
                        presence_state_clone
                            .broadcast_to_page(
                                &page_path,
                                PresenceMessage::PresenceUpdate {
                                    users: presence_state_clone.get_page_users(&page_path).await,
                                },
                            )
                            .await;
                    }
                    PresenceMessage::CursorMove { x, y } => {
                        let page_path_opt = current_page_clone.read().await;
                        if let Some(page_path) = page_path_opt.as_ref() {
                            presence_state_clone.update_cursor(session_id, x, y).await;

                            // Broadcast updated presence to all users on this page
                            presence_state_clone
                                .broadcast_to_page(
                                    page_path,
                                    PresenceMessage::PresenceUpdate {
                                        users: presence_state_clone.get_page_users(page_path).await,
                                    },
                                )
                                .await;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Ensure cleanup always happens, even if tasks were aborted
    let page_path_opt = current_page.read().await;
    if let Some(page_path) = page_path_opt.as_ref() {
        presence_state.leave(session_id).await;
        presence_state
            .broadcast_to_page(
                page_path,
                PresenceMessage::PresenceUpdate {
                    users: presence_state.get_page_users(page_path).await,
                },
            )
            .await;
        tracing::debug!("User {} disconnected from page {}", user_id, page_path);
    }
}
