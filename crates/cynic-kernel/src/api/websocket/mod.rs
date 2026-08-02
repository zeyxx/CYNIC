//! Node WebSocket federation — bidirectional communication for node-kernel stimulus/verdict exchange.
//!
//! Supports both PUSH mode (kernel → node stimuli dispatch) and PULL mode (node requests work).
//! Nodes register with cryptographic identity (Ed25519 pubkey), verdicts are signed and verified.
//! Network partitions are resilient: nodes queue stimuli locally and deduplicate on reconnect.

use axum::{
    extract::{ConnectInfo, State, ws::WebSocketUpgrade},
    http::{StatusCode, header::HeaderMap},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::api::rest::AppState;

// ── NODE REGISTRATION & REGISTRY ────────────────────────────

/// Node registry: tracks connected nodes, public keys, heartbeat timestamps.
/// Periodically evicted on TTL (120s default heartbeat timeout).
#[derive(Debug)]
pub struct NodeRegistry {
    /// Map: node_id → (public_key_base64, registration_timestamp, last_heartbeat)
    nodes: RwLock<HashMap<String, NodeRecord>>,
    /// Node TTL (seconds): nodes evicted if no heartbeat within this window.
    ttl_secs: u64,
}

#[derive(Clone, Debug)]
pub struct NodeRecord {
    node_id: String,
    #[allow(dead_code)] // WHY: Used in Phase 3.4 (signature verification) and for audit logging
    public_key: String,
    #[allow(dead_code)] // WHY: Used for node identity reporting and diagnostics
    node_identity: String,
    #[allow(dead_code)] // WHY: Used for node age tracking and diagnostics
    registered_at: i64,
    last_heartbeat: i64,
}

impl NodeRegistry {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            ttl_secs,
        }
    }

    /// Register a new node. Returns node_id.
    pub async fn register(&self, public_key: String, node_identity: String) -> String {
        let now = chrono::Utc::now().timestamp();
        let node_id = format!(
            "{}-{}",
            node_identity,
            &crate::infra::crypto::generate_secure_id()[..8]
        );

        let record = NodeRecord {
            node_id: node_id.clone(),
            public_key,
            node_identity,
            registered_at: now,
            last_heartbeat: now,
        };

        let mut nodes = self.nodes.write().await;
        nodes.insert(node_id.clone(), record);

        info!(node_id = %node_id, "Node registered");
        node_id
    }

    /// Update heartbeat timestamp for a node.
    pub async fn heartbeat(&self, node_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let mut nodes = self.nodes.write().await;

        if let Some(record) = nodes.get_mut(node_id) {
            record.last_heartbeat = now;
            true
        } else {
            false
        }
    }

    /// Get node record if exists and not expired.
    pub async fn get(&self, node_id: &str) -> Option<NodeRecord> {
        let now = chrono::Utc::now().timestamp();
        let nodes = self.nodes.read().await;

        nodes.get(node_id).and_then(|record| {
            if now - record.last_heartbeat < self.ttl_secs as i64 {
                Some(record.clone())
            } else {
                None
            }
        })
    }

    /// Deregister a node (disconnect).
    pub async fn deregister(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);
        info!(node_id = %node_id, "Node deregistered");
    }

    /// Evict expired nodes (called periodically by health loop).
    pub async fn evict_expired(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut nodes = self.nodes.write().await;

        let before = nodes.len();
        nodes.retain(|_, record| now - record.last_heartbeat < self.ttl_secs as i64);
        let after = nodes.len();

        if before > after {
            info!(evicted = before - after, "Expired nodes evicted");
        }

        before - after
    }

    /// List all active nodes.
    pub async fn list_active(&self) -> Vec<String> {
        let now = chrono::Utc::now().timestamp();
        let nodes = self.nodes.read().await;

        nodes
            .iter()
            .filter(|(_, record)| now - record.last_heartbeat < self.ttl_secs as i64)
            .map(|(_, record)| record.node_id.clone())
            .collect()
    }
}

// ── MESSAGE FRAME TYPES ─────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
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

    /// Node → Kernel: No work available response
    #[serde(rename = "stimulus_unavailable")]
    StimulusUnavailable { retry_after_ms: u64 },

    /// Node → Kernel: Verdict with signature
    #[serde(rename = "verdict")]
    Verdict {
        stimulus_id: String,
        verdict: serde_json::Value,
        signature: String,
        node_public_key: String,
        timestamp: i64,
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

    // ── CYNIC WORKFLOW EXTENSIONS (BOTTOM-UP) ───────────────
    /// Dev → Kernel: Submit a new build/test task (via UDS)
    #[serde(rename = "post_task")]
    PostTask { patch: String, intent: String },

    /// Kernel → Node: Dispatch task to anvil organ
    #[serde(rename = "route_task")]
    RouteTask {
        task_id: String,
        patch: String,
        intent: String,
    },

    /// Node → Kernel, then Kernel → Dev: Real-time log stream
    #[serde(rename = "log_chunk")]
    LogChunk { task_id: String, stdout: String },

    /// Node → Kernel: Final result of the task
    #[serde(rename = "task_result")]
    TaskResult {
        task_id: String,
        status: String,
        artifact: Option<String>,
    },
}

// ── TASK BROKER ─────────────────────────────────────────────

use std::sync::OnceLock;
use tokio::sync::broadcast;

static TASK_BROKER: OnceLock<broadcast::Sender<NodeMessage>> = OnceLock::new();

pub fn get_task_broker() -> broadcast::Sender<NodeMessage> {
    TASK_BROKER
        .get_or_init(|| {
            let (tx, _) = broadcast::channel(1024);
            tx
        })
        .clone()
}

// ── WEBSOCKET HANDLER ───────────────────────────────────────

pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, (StatusCode, String)> {
    let public_key = headers
        .get("X-Node-Public-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Missing X-Node-Public-Key header".to_string(),
        ))?
        .to_string();
    let node_identity = headers
        .get("X-Node-Identity")
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Missing X-Node-Identity header".to_string(),
        ))?
        .to_string();
    let timestamp = headers
        .get("X-Cynic-Timestamp")
        .and_then(|value| value.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing workload signature timestamp".to_string(),
        ))?;
    let signature = headers
        .get("X-Cynic-Signature")
        .and_then(|value| value.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing workload signature".to_string(),
        ))?;
    let payload = crate::infra::crypto::build_rest_payload("GET", "/events/ws", &[]);
    crate::infra::crypto::verify_signature(&public_key, signature, timestamp, &payload).map_err(
        |error| {
            (
                StatusCode::UNAUTHORIZED,
                format!("WebSocket workload signature rejected: {error}"),
            )
        },
    )?;
    let principal = state.role_keys.resolve_signer(&public_key).ok_or((
        StatusCode::UNAUTHORIZED,
        "WebSocket signing key is not bound to a workload principal".to_string(),
    ))?;
    principal
        .require_agent(&node_identity)
        .map_err(|message| (StatusCode::FORBIDDEN, message.to_string()))?;
    if !matches!(
        principal.role,
        crate::api::rest::types::Role::Organ | crate::api::rest::types::Role::Internal
    ) {
        return Err((
            StatusCode::FORBIDDEN,
            "WebSocket federation requires organ or internal role".to_string(),
        ));
    }

    info!(
        client_addr = %addr,
        node_identity = %node_identity,
        "WebSocket connection request from node"
    );

    Ok(ws.on_upgrade(move |socket| {
        handle_node_connection(socket, state, public_key, node_identity, addr)
    }))
}

async fn handle_node_connection(
    socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
    public_key: String,
    node_identity: String,
    addr: SocketAddr,
) {
    let (mut sender, mut receiver) = socket.split();
    let node_registry = &state.node_registry;

    // Register node
    let node_id = node_registry
        .register(public_key.clone(), node_identity.clone())
        .await;
    let _ = addr; // Remote address used for diagnostics; kept for future telemetry

    // Send registration response
    let response = NodeMessage::RegisterResponse {
        node_id: node_id.clone(),
        kernel_version: "26.5.14".to_string(),
        max_stimulus_timeout: 60000,
        max_queue_size: 10000,
    };

    if let Ok(json) = serde_json::to_string(&response) {
        let _ = sender
            .send(axum::extract::ws::Message::Text(json.into()))
            .await;
    }

    // Subscribe to task routing broker
    let mut task_rx = get_task_broker().subscribe();

    // Heartbeat interval: send ping every 30s
    let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(30));

    // Message handling loop with heartbeat
    loop {
        tokio::select! {
            // Incoming from TASK BROKER (Kernel → Organ/Dev)
            Ok(broadcast_msg) = task_rx.recv() => {
                if let Ok(json) = serde_json::to_string(&broadcast_msg)
                    && sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
            }

            // Incoming message from node
            Some(Ok(msg)) = receiver.next() => {
                match msg {
                    axum::extract::ws::Message::Text(text) => {
                        if let Ok(frame) = serde_json::from_str::<NodeMessage>(&text) {
                            match frame {
                                NodeMessage::Pong { timestamp: _ } => {
                                    // Update heartbeat on pong
                                    let _ = node_registry.heartbeat(&node_id).await;
                                }
                                NodeMessage::StimulusRequest {
                                    domain: _,
                                    request_timeout_ms: _,
                                    last_processed_stimulus_id: _,
                                } => {
                                    // PULL mode: node requests stimulus
                                    // TODO: Fetch next stimulus from pipeline for this domain
                                    // For now, send unavailable
                                    let response = NodeMessage::StimulusUnavailable {
                                        retry_after_ms: 5000,
                                    };
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        let _ = sender.send(axum::extract::ws::Message::Text(json.into())).await;
                                    }
                                }
                                NodeMessage::Verdict {
                                    stimulus_id,
                                    verdict,
                                    signature,
                                    node_public_key,
                                    timestamp,
                                } => {
                                    let registered = node_registry.get(&node_id).await;
                                    let key_matches = registered
                                        .as_ref()
                                        .is_some_and(|record| record.public_key == node_public_key);
                                    let payload = websocket_verdict_payload(&stimulus_id, &verdict);
                                    let verified = key_matches
                                        && crate::infra::crypto::verify_signature(
                                            &node_public_key,
                                            &signature,
                                            &timestamp.to_string(),
                                            &payload,
                                        )
                                        .is_ok();
                                    let response = if verified {
                                        NodeMessage::Ack {
                                            stimulus_id,
                                            status: "signature_verified".to_string(),
                                            verdict_id: crate::infra::crypto::generate_secure_id(),
                                        }
                                    } else {
                                        NodeMessage::Error {
                                            error_code: "verdict_identity_proof_rejected".to_string(),
                                            stimulus_id: Some(stimulus_id),
                                            message: "verdict signature, freshness, or registered key binding is invalid".to_string(),
                                        }
                                    };
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        let _ = sender.send(axum::extract::ws::Message::Text(json.into())).await;
                                    }
                                }
                                NodeMessage::PostTask { patch, intent } => {
                                    // Generate a Task ID and broadcast the RouteTask
                                    let task_id = crate::infra::crypto::generate_secure_id();
                                    info!(task_id = %task_id, intent = %intent, "Routing new task from Dev to Organ-Anvil");
                                    let _ = get_task_broker().send(NodeMessage::RouteTask {
                                        task_id,
                                        patch,
                                        intent,
                                    });
                                }
                                NodeMessage::LogChunk { task_id, stdout } => {
                                    // Broadcast log stream so the UDS Dev client receives it
                                    let _ = get_task_broker().send(NodeMessage::LogChunk {
                                        task_id,
                                        stdout,
                                    });
                                }
                                NodeMessage::TaskResult { task_id, status, artifact } => {
                                    // Broadcast final result
                                    let _ = get_task_broker().send(NodeMessage::TaskResult {
                                        task_id,
                                        status,
                                        artifact,
                                    });
                                }
                                _ => {
                                    warn!(node_id = %node_id, "Unexpected message from node");
                                }
                            }
                        }
                    }
                    axum::extract::ws::Message::Close(_) => {
                        info!(node_id = %node_id, "Node closed WebSocket connection");
                        break;
                    }
                    _ => {
                        // Binary frames not supported
                    }
                }
            }

            // Heartbeat tick
            _ = heartbeat_interval.tick() => {
                let ping = NodeMessage::Ping {
                    timestamp: chrono::Utc::now().timestamp(),
                };
                if let Ok(json) = serde_json::to_string(&ping)
                    && sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err()
                {
                    break;
                }
            }

            // Connection closed from remote
            else => break,
        }
    }

    // Cleanup on disconnect
    node_registry.deregister(&node_id).await;
    info!(node_id = %node_id, "Node WebSocket handler closed");
}

fn websocket_verdict_payload(stimulus_id: &str, verdict: &serde_json::Value) -> Vec<u8> {
    let mut payload = b"WEBSOCKET_VERDICT|".to_vec();
    payload.extend_from_slice(stimulus_id.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(&serde_json::to_vec(verdict).unwrap_or_else(|_| b"null".to_vec()));
    payload
}
