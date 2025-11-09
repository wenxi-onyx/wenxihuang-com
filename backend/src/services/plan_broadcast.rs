use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::handlers::plan_ws::PlanMessage;

const WS_CONNECTIONS_PER_IP: usize = 10;
const WS_CONNECTION_WINDOW_SECONDS: i64 = 60;

#[derive(Clone)]
struct ConnectionInfo {
    count: usize,
    window_start: DateTime<Utc>,
}

#[derive(Clone)]
pub struct PlanBroadcastState {
    subscribers: Arc<RwLock<HashMap<String, Vec<UnboundedSender<PlanMessage>>>>>,
    connection_counts: Arc<RwLock<HashMap<IpAddr, ConnectionInfo>>>,
}

impl PlanBroadcastState {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            connection_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if IP can create new WebSocket connection (connection limiting)
    pub async fn check_connection_limit(&self, ip: IpAddr) -> Result<(), String> {
        let now = Utc::now();
        let mut counts = self.connection_counts.write().await;

        counts.retain(|_, info| {
            now.signed_duration_since(info.window_start).num_seconds()
                < WS_CONNECTION_WINDOW_SECONDS
        });

        let info = counts.entry(ip).or_insert(ConnectionInfo {
            count: 0,
            window_start: now,
        });

        if now.signed_duration_since(info.window_start).num_seconds()
            >= WS_CONNECTION_WINDOW_SECONDS
        {
            info.count = 0;
            info.window_start = now;
        }

        if info.count >= WS_CONNECTIONS_PER_IP {
            return Err(format!(
                "WebSocket connection limit exceeded. Maximum {} connections per {} seconds.",
                WS_CONNECTIONS_PER_IP, WS_CONNECTION_WINDOW_SECONDS
            ));
        }

        info.count += 1;
        Ok(())
    }

    pub async fn release_connection(&self, ip: IpAddr) {
        let mut counts = self.connection_counts.write().await;
        if let Some(info) = counts.get_mut(&ip) {
            if info.count > 0 {
                info.count -= 1;
            }
            if info.count == 0 {
                counts.remove(&ip);
            }
        }
    }

    pub async fn subscribe(&self, plan_id: &str, tx: UnboundedSender<PlanMessage>) {
        let mut subs = self.subscribers.write().await;
        subs.entry(plan_id.to_string())
            .or_insert_with(Vec::new)
            .push(tx);
        tracing::debug!("Client subscribed to plan: {}", plan_id);
    }

    pub async fn unsubscribe(&self, plan_id: &str) {
        let mut subs = self.subscribers.write().await;
        if let Some(plan_subs) = subs.get_mut(plan_id) {
            plan_subs.retain(|tx| !tx.is_closed());
            if plan_subs.is_empty() {
                subs.remove(plan_id);
            }
        }
        tracing::debug!("Client unsubscribed from plan: {}", plan_id);
    }

    pub async fn broadcast(&self, plan_id: &str, message: PlanMessage) {
        let mut subs = self.subscribers.write().await;

        if let Some(plan_subs) = subs.get_mut(plan_id) {
            let mut sent_count = 0;
            let initial_count = plan_subs.len();

            plan_subs.retain(|tx| match tx.send(message.clone()) {
                Ok(_) => {
                    sent_count += 1;
                    true
                }
                Err(_) => {
                    tracing::debug!("Removing dead subscriber for plan: {}", plan_id);
                    false
                }
            });

            if plan_subs.is_empty() {
                subs.remove(plan_id);
                tracing::debug!("No subscribers left for plan: {}, removing entry", plan_id);
            }

            if initial_count != sent_count {
                tracing::debug!(
                    "Broadcasted to {}/{} clients for plan: {} ({} dead)",
                    sent_count,
                    initial_count,
                    plan_id,
                    initial_count - sent_count
                );
            } else {
                tracing::debug!(
                    "Broadcasted to {} clients for plan: {}",
                    sent_count,
                    plan_id
                );
            }
        }
    }
}

impl Default for PlanBroadcastState {
    fn default() -> Self {
        Self::new()
    }
}
