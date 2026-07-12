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
mod protocol;
mod queue_store;

use futures_util::Sink;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use http::Request;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

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
    /// Assigned node_id by kernel (set on RegisterResponse)
    node_id: Arc<RwLock<Option<String>>>,
    /// Local stimulus queue (persisted on disk)
    pending_stimuli: Arc<RwLock<VecDeque<PendingStimulus>>>,
    /// Verdict queue (persisted on disk)
    pending_verdicts: Arc<RwLock<VecDeque<PendingVerdict>>>,
    /// Dedup cache: stimulus_id → timestamp (3600s window)
    dedup_cache: Arc<RwLock<HashMap<String, i64>>>,
    /// Directory for queue persistence
    queue_dir: PathBuf,
}

impl WebSocketClient {
    /// Create a new WebSocket client (not yet connected).
    pub fn new(
        kernel_url: String,
        api_key: String,
        node_public_key: String,
        node_identity: String,
        queue_dir: PathBuf,
    ) -> Self {
        Self {
            kernel_url,
            api_key,
            node_public_key,
            node_identity,
            node_id: Arc::new(RwLock::new(None)),
            pending_stimuli: Arc::new(RwLock::new(VecDeque::new())),
            pending_verdicts: Arc::new(RwLock::new(VecDeque::new())),
            dedup_cache: Arc::new(RwLock::new(HashMap::new())),
            queue_dir,
        }
    }

    /// Load pending stimuli and verdicts from disk.
    pub async fn load_persistent_queues(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (stimuli, verdicts) = queue_store::load_queues(&self.queue_dir)?;
        let mut stimuli_queue = self.pending_stimuli.write().await;
        *stimuli_queue = stimuli;
        let mut verdicts_queue = self.pending_verdicts.write().await;
        *verdicts_queue = verdicts;
        Ok(())
    }

    /// Persist pending queues to disk.
    pub async fn save_persistent_queues(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stimuli = self.pending_stimuli.read().await;
        let verdicts = self.pending_verdicts.read().await;
        queue_store::save_queues(&self.queue_dir, &stimuli, &verdicts)
    }

    /// Check if stimulus is in dedup cache (within 3600s window).
    pub async fn should_skip_stimulus(&self, stimulus_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let cache = self.dedup_cache.read().await;

        if let Some(timestamp) = cache.get(stimulus_id) {
            now - *timestamp < 3600
        } else {
            false
        }
    }

    /// Add stimulus to dedup cache.
    pub async fn mark_stimulus_processed(&self, stimulus_id: String) {
        let now = chrono::Utc::now().timestamp();
        let mut cache = self.dedup_cache.write().await;
        cache.insert(stimulus_id, now);

        // Cleanup old entries (>3600s ago)
        cache.retain(|_, &mut ts| now - ts < 3600);
    }

    /// Queue a stimulus locally (used during offline mode or partition).
    pub async fn queue_stimulus(&self, stimulus: PendingStimulus) {
        let mut queue = self.pending_stimuli.write().await;
        queue.push_back(stimulus);
        let _ = self.save_persistent_queues().await;
    }

    /// Queue a verdict for delivery (used during offline mode or partition).
    pub async fn queue_verdict(&self, verdict: PendingVerdict) {
        let mut queue = self.pending_verdicts.write().await;
        queue.push_back(verdict);
        let _ = self.save_persistent_queues().await;
    }

    /// Dequeue next stimulus (FIFO).
    pub async fn dequeue_stimulus(&self) -> Option<PendingStimulus> {
        let mut queue = self.pending_stimuli.write().await;
        let stimulus = queue.pop_front();
        if stimulus.is_some() {
            let _ = self.save_persistent_queues().await;
        }
        stimulus
    }

    /// Dequeue next verdict (FIFO).
    pub async fn dequeue_verdict(&self) -> Option<PendingVerdict> {
        let mut queue = self.pending_verdicts.write().await;
        let verdict = queue.pop_front();
        if verdict.is_some() {
            let _ = self.save_persistent_queues().await;
        }
        verdict
    }

    /// Get current node_id (assigned by kernel).
    pub async fn get_node_id(&self) -> Option<String> {
        self.node_id.read().await.clone()
    }

    /// Set node_id (called on RegisterResponse).
    pub async fn set_node_id(&self, id: String) {
        *self.node_id.write().await = Some(id);
    }

    /// Get current stimulus queue length.
    pub async fn pending_stimulus_count(&self) -> usize {
        self.pending_stimuli.read().await.len()
    }

    /// Get current verdict queue length.
    pub async fn pending_verdict_count(&self) -> usize {
        self.pending_verdicts.read().await.len()
    }
}

// ── Main Connection Loop ──────────────────────────────────────────────────

impl WebSocketClient {
    /// Main async loop: connect to kernel, register, multiplex messages/heartbeats/queues.
    ///
    /// Exits only on explicit close or unrecoverable error. Reconnects with exponential backoff
    /// on network partition. Queues stimuli/verdicts locally during offline partition.
    /// On reconnect, flushes queues with dedup via stimulus_id.
    pub async fn run(&self) -> Result<(), String> {
        let mut backoff_ms = 100u64;
        let max_backoff_ms = 30000u64; // 30s max backoff

        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    // Clean shutdown or kernel eviction
                    info!("WebSocket connection closed by kernel");
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        backoff_ms = backoff_ms,
                        "WebSocket connection failed, reconnecting..."
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                }
            }
        }
    }

    /// Connect to kernel and run the main multiplexing loop.
    async fn connect_and_run(&self) -> Result<(), String> {
        debug!("Connecting to kernel at {}", self.kernel_url);

        let request = self.build_request()?;

        let (ws_stream, _) = connect_async(request).await.map_err(|e| e.to_string())?;
        info!("Connected to kernel at {}", self.kernel_url);

        let (mut write, mut read) = ws_stream.split();
        self.register_session(&mut write, &mut read).await?;

        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    self.send_ping(&mut write).await?;
                }

                msg_result = read.next() => {
                    if let Some(result) = msg_result {
                        match result.map_err(|e| e.to_string())? {
                            Message::Text(text) => {
                                if let Some(err) = self.handle_incoming_text(&mut write, &text).await? {
                                    return err;
                                }
                            }
                            Message::Close(_) => {
                                info!("Kernel closed WebSocket connection");
                                return Ok(());
                            }
                            _ => {}
                        }
                    } else {
                        warn!("WebSocket stream ended");
                        return Err("WebSocket stream closed".into());
                    }
                }

                _ = tokio::time::sleep(Duration::from_millis(100)), if self.pending_verdict_count().await > 0 => {
                    self.flush_pending_verdicts(&mut write).await?;
                }
            }
        }
    }

    fn build_request(&self) -> Result<Request<()>, String> {
        Request::builder()
            .uri(&self.kernel_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Node-Public-Key", &self.node_public_key)
            .header("X-Node-Identity", &self.node_identity)
            .body(())
            .map_err(|e| format!("Failed to build WebSocket request: {e}"))
    }

    async fn register_session(
        &self,
        write: &mut WsWriter,
        read: &mut WsReader,
    ) -> Result<(), String> {
        let register_msg = NodeMessage::Register {
            node_public_key: self.node_public_key.clone(),
            node_identity: self.node_identity.clone(),
        };
        self.send_message(write, &register_msg).await?;
        debug!("Sent registration to kernel");

        loop {
            if let Some(msg_result) = read.next().await {
                match msg_result.map_err(|e| e.to_string())? {
                    Message::Text(text) => {
                        if let Ok(NodeMessage::RegisterResponse {
                            node_id,
                            kernel_version,
                            max_stimulus_timeout,
                            max_queue_size,
                        }) = serde_json::from_str(&text)
                        {
                            self.set_node_id(node_id.clone()).await;
                            info!(
                                node_id = %node_id,
                                kernel_version = %kernel_version,
                                max_stimulus_timeout = max_stimulus_timeout,
                                max_queue_size = max_queue_size,
                                "Registered with kernel"
                            );
                            return Ok(());
                        }
                    }
                    Message::Close(_) => {
                        return Err("Kernel closed connection during registration".into());
                    }
                    _ => {}
                }
            } else {
                return Err("WebSocket stream ended during registration".into());
            }
        }
    }

    async fn send_ping(&self, write: &mut WsWriter) -> Result<(), String> {
        let ping = NodeMessage::Ping {
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.send_message(write, &ping).await?;
        debug!("Sent heartbeat ping");
        Ok(())
    }

    async fn handle_incoming_text(
        &self,
        write: &mut WsWriter,
        text: &str,
    ) -> Result<Option<Result<(), String>>, String> {
        if let Ok(msg) = serde_json::from_str::<NodeMessage>(text) {
            match msg {
                NodeMessage::Stimulus {
                    stimulus_id,
                    domain,
                    content,
                    context,
                    timeout_ms,
                    idempotent_key,
                } => {
                    self.handle_stimulus(
                        write,
                        stimulus_id,
                        domain,
                        content,
                        context,
                        timeout_ms,
                        idempotent_key,
                    )
                    .await?;
                }
                NodeMessage::Pong { timestamp: _ } => {
                    debug!("Received heartbeat pong");
                }
                NodeMessage::Backpressure {
                    queue_usage_percent,
                    resume_after_ms,
                } => {
                    warn!(
                        queue_usage_percent = queue_usage_percent,
                        resume_after_ms = resume_after_ms,
                        "Kernel backpressure applied, pausing stimulus requests"
                    );
                }
                NodeMessage::Eviction { reason, message } => {
                    error!(
                        reason = %reason,
                        message = %message,
                        "Kernel evicted node, reconnecting..."
                    );
                    return Ok(Some(Err(format!("Evicted by kernel: {message}"))));
                }
                NodeMessage::Error {
                    error_code,
                    stimulus_id,
                    message,
                } => {
                    error!(
                        error_code = %error_code,
                        stimulus_id = ?stimulus_id,
                        message = %message,
                        "Kernel error"
                    );
                }
                _ => {
                    debug!("Received unexpected message type: {:?}", msg);
                }
            }
        }
        Ok(None)
    }

    async fn handle_stimulus(
        &self,
        write: &mut WsWriter,
        stimulus_id: String,
        domain: String,
        content: serde_json::Value,
        context: serde_json::Value,
        timeout_ms: u64,
        idempotent_key: String,
    ) -> Result<(), String> {
        if self.should_skip_stimulus(&stimulus_id).await {
            debug!(stimulus_id = %stimulus_id, "Skipping duplicate stimulus");
            let ack = NodeMessage::Ack {
                stimulus_id: stimulus_id.clone(),
                status: "already_processed".to_string(),
                verdict_id: format!("verd_{stimulus_id}"),
            };
            self.send_message(write, &ack).await?;
            return Ok(());
        }

        self.mark_stimulus_processed(stimulus_id.clone()).await;
        info!(
            stimulus_id = %stimulus_id,
            domain = %domain,
            timeout_ms = timeout_ms,
            "Received stimulus from kernel"
        );

        if !domain_dispatch::spawn_native_handler(
            domain.as_str(),
            stimulus_id.clone(),
            content.clone(),
            self.pending_verdicts.clone(),
        ) {
            let pending = PendingStimulus {
                stimulus_id: stimulus_id.clone(),
                stimulus: NodeMessage::Stimulus {
                    stimulus_id,
                    domain,
                    content,
                    context,
                    timeout_ms,
                    idempotent_key,
                },
                received_at: chrono::Utc::now().timestamp(),
            };
            self.queue_stimulus(pending).await;
        }
        Ok(())
    }

    async fn flush_pending_verdicts(&self, write: &mut WsWriter) -> Result<(), String> {
        while let Some(pending) = self.dequeue_verdict().await {
            let verdict = NodeMessage::Verdict {
                stimulus_id: pending.stimulus_id.clone(),
                verdict: pending.verdict.clone(),
                signature: pending.signature.clone(),
                node_public_key: self.node_public_key.clone(),
            };
            self.send_message(write, &verdict).await?;
            info!(stimulus_id = %pending.stimulus_id, "Sent pending verdict to kernel");
        }
        Ok(())
    }

    async fn send_message<S>(&self, write: &mut S, msg: &NodeMessage) -> Result<(), String>
    where
        S: Sink<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let json = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        write
            .send(Message::Text(json))
            .await
            .map_err(|e| e.to_string())
    }
}
