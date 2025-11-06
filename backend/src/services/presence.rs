use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: Uuid,
    pub username: String,
    pub page_path: String,
    pub cursor: Option<CursorPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PresenceMessage {
    Join { page_path: String },
    Leave,
    CursorMove { x: f64, y: f64 },
    PresenceUpdate { users: Vec<UserPresence> },
}

type Tx = mpsc::UnboundedSender<PresenceMessage>;
type SessionData = (Uuid, String, String, Tx);

#[derive(Clone)]
pub struct PresenceState {
    // Map of session_id -> (user_id, username, page_path, sender)
    sessions: Arc<RwLock<HashMap<Uuid, SessionData>>>,
    // Map of session_id -> cursor_position
    cursors: Arc<RwLock<HashMap<Uuid, CursorPosition>>>,
}

impl Default for PresenceState {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            cursors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn join(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        username: String,
        page_path: String,
        tx: Tx,
    ) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, (user_id, username, page_path, tx));
    }

    pub async fn leave(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);

        let mut cursors = self.cursors.write().await;
        cursors.remove(&session_id);
    }

    pub async fn update_cursor(&self, session_id: Uuid, x: f64, y: f64) {
        let mut cursors = self.cursors.write().await;
        cursors.insert(session_id, CursorPosition { x, y });
    }

    pub async fn get_page_users(&self, page_path: &str) -> Vec<UserPresence> {
        let sessions = self.sessions.read().await;
        let cursors = self.cursors.read().await;

        sessions
            .iter()
            .filter(|(_, (_, _, path, _))| path == page_path)
            .map(|(session_id, (user_id, username, path, _))| UserPresence {
                user_id: *user_id,
                username: username.clone(),
                page_path: path.clone(),
                cursor: cursors.get(session_id).cloned(),
            })
            .collect()
    }

    pub async fn broadcast_to_page(&self, page_path: &str, message: PresenceMessage) {
        let sessions = self.sessions.read().await;

        for (_, (_, _, path, tx)) in sessions.iter() {
            if path == page_path {
                let _ = tx.send(message.clone());
            }
        }
    }
}
