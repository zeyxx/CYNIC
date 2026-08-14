//! WebSocket client for node-kernel bidirectional communication (Phase 3).
//!
//! Handles PUSH mode (kernel dispatches stimuli) and PULL mode (node requests work).
//! Maintains local queue during network partitions and deduplicates on reconnect.
//!
//! WHY: This module is not yet integrated with the main watch() loop. Until Phase 3.2
//! integration is complete, the structures and methods appear unused. When wired into
//! main.rs, these allows will be removed.
// WHY: Module not yet wired into main.rs — see Phase 3.2 comment above.
#![allow(dead_code, unreachable_pub)]

mod domain_dispatch;
mod inbox;
mod protocol;
mod queue_runtime;
mod session;
#[cfg(test)]
mod tests;
mod wire;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::ports::runtime_federation_queue_store_port::RuntimeFederationQueueStorePort;

pub use protocol::{NodeMessage, PendingStimulus, PendingVerdict};

type WsWriter = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;
type WsReader = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

// ── WebSocket Client ───────────────────────────────────────────────────────

/// Manages WebSocket connection to kernel with offline queue and dedup.
pub struct WebSocketClient {
    /// Kernel WebSocket URL (e.g., "ws://<TAILSCALE_CORE>:3030/events/ws")
    kernel_url: String,
    /// API key for Bearer token authentication
    api_key: String,
    /// Node's Ed25519 public key (base64)
    node_public_key: String,
    /// Node identity (name/id)
    node_identity: String,
    /// Persistence port for local queues
    queue_store: Arc<dyn RuntimeFederationQueueStorePort>,
    /// Assigned node_id by kernel (set on RegisterResponse)
    node_id: Arc<Mutex<Option<String>>>,
    /// Local stimulus queue (persisted on disk)
    pending_stimuli: Arc<Mutex<VecDeque<PendingStimulus>>>,
    /// Verdict queue (persisted on disk)
    pending_verdicts: Arc<Mutex<VecDeque<PendingVerdict>>>,
    /// Dedup cache: stimulus_id → timestamp (3600s window)
    dedup_cache: Arc<Mutex<HashMap<String, i64>>>,
}

impl WebSocketClient {
    pub fn new(
        kernel_url: String,
        api_key: String,
        node_public_key: String,
        node_identity: String,
        queue_store: Arc<dyn RuntimeFederationQueueStorePort>,
    ) -> Self {
        Self {
            kernel_url,
            api_key,
            node_public_key,
            node_identity,
            queue_store,
            node_id: Arc::new(Mutex::new(None)),
            pending_stimuli: Arc::new(Mutex::new(VecDeque::new())),
            pending_verdicts: Arc::new(Mutex::new(VecDeque::new())),
            dedup_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
