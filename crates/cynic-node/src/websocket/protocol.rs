use serde::{Deserialize, Serialize};

/// Node→Kernel or Kernel→Node message in WebSocket protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeMessage {
    #[serde(rename = "register")]
    Register {
        node_public_key: String,
        node_identity: String,
    },
    #[serde(rename = "register_response")]
    RegisterResponse {
        node_id: String,
        kernel_version: String,
        max_stimulus_timeout: u64,
        max_queue_size: usize,
    },
    #[serde(rename = "stimulus")]
    Stimulus {
        stimulus_id: String,
        domain: String,
        content: serde_json::Value,
        context: serde_json::Value,
        timeout_ms: u64,
        idempotent_key: String,
    },
    #[serde(rename = "stimulus_request")]
    StimulusRequest {
        domain: String,
        request_timeout_ms: u64,
        last_processed_stimulus_id: Option<String>,
    },
    #[serde(rename = "stimulus_unavailable")]
    StimulusUnavailable { retry_after_ms: u64 },
    #[serde(rename = "verdict")]
    Verdict {
        stimulus_id: String,
        verdict: serde_json::Value,
        signature: String,
        node_public_key: String,
<<<<<<< HEAD
        timestamp: i64,
=======
>>>>>>> github/main
    },
    #[serde(rename = "ack")]
    Ack {
        stimulus_id: String,
        status: String,
        verdict_id: String,
    },
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },
    #[serde(rename = "backpressure")]
    Backpressure {
        queue_usage_percent: u8,
        resume_after_ms: u64,
    },
    #[serde(rename = "eviction")]
    Eviction { reason: String, message: String },
    #[serde(rename = "error")]
    Error {
        error_code: String,
        stimulus_id: Option<String>,
        message: String,
    },
}

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
