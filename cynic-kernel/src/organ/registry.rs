// registry.rs — Catalog of known Dog backends: endpoint, capabilities, and live state.

use std::time::Instant;

// --- Newtype identifiers ---

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackendId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ClusterId(pub String);

// --- Capabilities ---

/// What the backend claims to support (from config / vendor docs).
#[derive(Debug, Clone, Default)]
pub struct DeclaredCapabilities {
    pub json: bool,
    pub thinking: bool,
    pub scoring: bool,
    pub embedding: bool,
    pub agent_reasoning: bool,
    pub grammar: bool,
}

/// What we have empirically measured this backend achieving.
/// K14: unknown = worst. Rates default to 0.0, latency to u32::MAX.
#[derive(Debug, Clone)]
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,
    pub scoring_in_range_rate: f64,
    /// K14: unknown latency = u32::MAX (worst case, not optimistic zero).
    pub mean_latency_ms: u32,
    pub tokens_per_second: f32,
}

impl Default for MeasuredCapabilities {
    fn default() -> Self {
        Self {
            json_valid_rate: 0.0,
            scoring_in_range_rate: 0.0,
            mean_latency_ms: u32::MAX, // K14: unknown = worst
            tokens_per_second: 0.0,
        }
    }
}

/// Minimum thresholds a backend must meet to be considered healthy enough for a cluster.
#[derive(Debug, Clone)]
pub struct CapabilityThreshold {
    pub min_json_valid_rate: f64,
    pub min_scoring_rate: f64,
    pub max_latency_ms: u32,
}

impl Default for CapabilityThreshold {
    fn default() -> Self {
        Self {
            min_json_valid_rate: 0.7,
            min_scoring_rate: 0.6,
            max_latency_ms: 5000,
        }
    }
}

// --- Backend health state ---

/// Live health classification for a backend.
/// K14: on poison/missing, the fallback is Degraded — never Healthy.
#[derive(Debug, Clone)]
pub enum BackendHealth {
    Healthy,
    Degraded { reason: String, since: Instant },
    Dead { reason: String, since: Instant },
}

// --- Remediation ---

/// How to attempt auto-recovery for a dead backend node.
#[derive(Debug, Clone)]
pub struct RemediationConfig {
    pub node: String,
    pub restart_command: String,
    pub max_retries: u32,
    pub cooldown_secs: u64,
}

// --- Backend ---

/// A single inference endpoint (one model on one node).
#[derive(Debug, Clone)]
pub struct Backend {
    pub id: BackendId,
    pub node_id: NodeId,
    pub endpoint: String,
    pub model: String,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
    pub timeout_secs: u64,
    pub remediation: Option<RemediationConfig>,
}

// --- Cluster routing ---

/// How to select among backends in a cluster.
#[derive(Debug, Clone)]
pub enum ClusterStrategy {
    RoundRobin,
    Failover,
    LowestLatency,
}

/// A named group of backends that serve the same logical role (e.g. "scoring", "embedding").
#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: ClusterId,
    pub required_json_rate: f64,
    pub backends: Vec<BackendId>,
    pub strategy: ClusterStrategy,
}

// --- Node ---

/// A physical or virtual machine that hosts one or more backends.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub address: String,
    pub backends: Vec<BackendId>,
}
