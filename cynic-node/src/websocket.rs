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

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

// ── Message Types ──────────────────────────────────────────────────────────

/// Node→Kernel or Kernel→Node message in WebSocket protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeMessage {
    /// Node → Kernel: Registration with public key
    #[serde(rename = "register")]
    Register {
        node_public_key: String,
        node_identity: String,
    },

    /// Kernel → Node: Registration response with assigned node_id
    #[serde(rename = "register_response")]
    RegisterResponse {
        node_id: String,
        kernel_version: String,
        max_stimulus_timeout: u64,
        max_queue_size: usize,
    },

    /// Kernel → Node: Stimulus (PUSH mode)
    #[serde(rename = "stimulus")]
    Stimulus {
        stimulus_id: String,
        domain: String,
        content: serde_json::Value,
        context: serde_json::Value,
        timeout_ms: u64,
        idempotent_key: String,
    },

    /// Node → Kernel: Stimulus request (PULL mode)
    #[serde(rename = "stimulus_request")]
    StimulusRequest {
        domain: String,
        request_timeout_ms: u64,
        last_processed_stimulus_id: Option<String>,
    },

    /// Kernel → Node: No work available response
    #[serde(rename = "stimulus_unavailable")]
    StimulusUnavailable { retry_after_ms: u64 },

    /// Node → Kernel: Verdict with signature
    #[serde(rename = "verdict")]
    Verdict {
        stimulus_id: String,
        verdict: serde_json::Value,
        signature: String,
        node_public_key: String,
    },

    /// Kernel → Node: Acknowledgment of verdict
    #[serde(rename = "ack")]
    Ack {
        stimulus_id: String,
        status: String,
        verdict_id: String,
    },

    /// Heartbeat ping/pong
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },

    #[serde(rename = "pong")]
    Pong { timestamp: i64 },

    /// Kernel → Node: Backpressure (node should pause requests)
    #[serde(rename = "backpressure")]
    Backpressure {
        queue_usage_percent: u8,
        resume_after_ms: u64,
    },

    /// Kernel → Node: Node eviction (TTL expired)
    #[serde(rename = "eviction")]
    Eviction { reason: String, message: String },

    /// Error frame (both directions)
    #[serde(rename = "error")]
    Error {
        error_code: String,
        stimulus_id: Option<String>,
        message: String,
    },
}

// ── Pending Queue Items ────────────────────────────────────────────────────

/// Stimulus stored in local queue during network partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingStimulus {
    pub stimulus_id: String,
    pub stimulus: NodeMessage,
    pub received_at: i64,
}

/// Verdict queued for delivery to kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingVerdict {
    pub stimulus_id: String,
    pub verdict: serde_json::Value,
    pub signature: String,
    pub queued_at: i64,
}

// ── WebSocket Client ───────────────────────────────────────────────────────

/// Manages WebSocket connection to kernel with offline queue and dedup.
pub struct WebSocketClient {
    /// Kernel WebSocket URL (e.g., "ws://100.64.0.1:3030/node/ws")
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
        let stimuli_file = self.queue_dir.join("pending_stimuli.json");
        let verdicts_file = self.queue_dir.join("pending_verdicts.json");

        if stimuli_file.exists() {
            let data = std::fs::read_to_string(&stimuli_file)?;
            let stimuli: Vec<PendingStimulus> = serde_json::from_str(&data)?;
            let mut queue = self.pending_stimuli.write().await;
            queue.extend(stimuli);
            info!(count = queue.len(), "Loaded pending stimuli from disk");
        }

        if verdicts_file.exists() {
            let data = std::fs::read_to_string(&verdicts_file)?;
            let verdicts: Vec<PendingVerdict> = serde_json::from_str(&data)?;
            let mut queue = self.pending_verdicts.write().await;
            queue.extend(verdicts);
            info!(count = queue.len(), "Loaded pending verdicts from disk");
        }

        Ok(())
    }

    /// Persist pending queues to disk.
    pub async fn save_persistent_queues(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.queue_dir)?;

        let stimuli_file = self.queue_dir.join("pending_stimuli.json");
        let stimuli = self.pending_stimuli.read().await;
        let stimuli_vec: Vec<_> = stimuli.iter().cloned().collect();
        std::fs::write(&stimuli_file, serde_json::to_string(&stimuli_vec)?)?;

        let verdicts_file = self.queue_dir.join("pending_verdicts.json");
        let verdicts = self.pending_verdicts.read().await;
        let verdicts_vec: Vec<_> = verdicts.iter().cloned().collect();
        std::fs::write(&verdicts_file, serde_json::to_string(&verdicts_vec)?)?;

        info!("Saved persistent queues to disk");
        Ok(())
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
        // Connect to kernel with authentication headers
        debug!("Connecting to kernel at {}", self.kernel_url);

        // For now, connect with URI only. Custom headers for WebSocket auth require tungstenite ClientRequest.
        // TODO: Implement proper header-based auth via tungstenite::client::ClientRequest when kernel WebSocket
        //       endpoint is finalized. Currently, kernel does not require WebSocket upgrade auth headers.
        let (ws_stream, _) = connect_async(&self.kernel_url).await.map_err(|e| {
            error!("WebSocket connection failed: {}", e);
            e.to_string()
        })?;
        info!("Connected to kernel at {}", self.kernel_url);

        // Send registration
        let register_msg = NodeMessage::Register {
            node_public_key: self.node_public_key.clone(),
            node_identity: self.node_identity.clone(),
        };
        let register_json = serde_json::to_string(&register_msg).map_err(|e| e.to_string())?;
        let (mut write, mut read) = ws_stream.split();
        write
            .send(Message::Text(register_json))
            .await
            .map_err(|e| e.to_string())?;
        debug!("Sent registration to kernel");

        // Wait for RegisterResponse
        let register_response = loop {
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
                            break (node_id, max_stimulus_timeout, max_queue_size);
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
        };

        let (_node_id, _max_stimulus_timeout, _max_queue_size) = register_response;

        // Multiplex: heartbeat, incoming messages, and queue draining
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Heartbeat every 30s
                _ = heartbeat_interval.tick() => {
                    let now = chrono::Utc::now().timestamp();
                    let ping = NodeMessage::Ping {
                        timestamp: now,
                    };
                    let ping_json = serde_json::to_string(&ping)
                        .map_err(|e| e.to_string())?;
                    write.send(Message::Text(ping_json)).await
                        .map_err(|e| e.to_string())?;
                    debug!("Sent heartbeat ping");
                }

                // Receive messages from kernel
                msg_result = read.next() => {
                    if let Some(result) = msg_result {
                        match result.map_err(|e| e.to_string())? {
                            Message::Text(text) => {
                                if let Ok(msg) = serde_json::from_str::<NodeMessage>(&text) {
                                    match msg {
                                        NodeMessage::Stimulus {
                                            stimulus_id,
                                            domain,
                                            content,
                                            context,
                                            timeout_ms,
                                            idempotent_key,
                                        } => {
                                            // Check dedup cache
                                            if self.should_skip_stimulus(&stimulus_id).await {
                                                debug!(stimulus_id = %stimulus_id, "Skipping duplicate stimulus");
                                                // Send ack anyway for idempotence
                                                let ack = NodeMessage::Ack {
                                                    stimulus_id: stimulus_id.clone(),
                                                    status: "already_processed".to_string(),
                                                    verdict_id: format!("verd_{stimulus_id}"),
                                                };
                                                let ack_json = serde_json::to_string(&ack)
                                                    .map_err(|e| e.to_string())?;
                                                write.send(Message::Text(ack_json)).await
                                                    .map_err(|e| e.to_string())?;
                                            } else {
                                                // Mark as processed in dedup cache
                                                self.mark_stimulus_processed(stimulus_id.clone()).await;
                                                info!(
                                                    stimulus_id = %stimulus_id,
                                                    domain = %domain,
                                                    timeout_ms = timeout_ms,
                                                    "Received stimulus from kernel"
                                                );

                                                // TODO: Phase 3.2c — Route to local judgment engine
                                                // For now, queue it locally
                                                let pending = PendingStimulus {
                                                    stimulus_id,
                                                    stimulus: NodeMessage::Stimulus {
                                                        stimulus_id: "".to_string(), // filled in above
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
                                            // TODO: Phase 3.2c — Pause stimulus requests
                                        }
                                        NodeMessage::Eviction { reason, message } => {
                                            error!(
                                                reason = %reason,
                                                message = %message,
                                                "Kernel evicted node, reconnecting..."
                                            );
                                            return Err(format!("Evicted by kernel: {message}"));
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

                // Drain pending verdicts
                _ = tokio::time::sleep(Duration::from_millis(100)), if self.pending_verdict_count().await > 0 => {
                    while let Some(pending) = self.dequeue_verdict().await {
                        let verdict = NodeMessage::Verdict {
                            stimulus_id: pending.stimulus_id.clone(),
                            verdict: pending.verdict.clone(),
                            signature: pending.signature.clone(),
                            node_public_key: self.node_public_key.clone(),
                        };
                        let verdict_json = serde_json::to_string(&verdict)
                            .map_err(|e| e.to_string())?;
                        write.send(Message::Text(verdict_json)).await
                            .map_err(|e| e.to_string())?;
                        info!(stimulus_id = %pending.stimulus_id, "Sent pending verdict to kernel");
                    }
                }
            }
        }
    }
}
