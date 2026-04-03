// registry.rs — Catalog of known Dog backends: capabilities and live state.

use std::time::Instant;

// --- Newtype identifiers ---

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackendId(pub String);

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
/// K14: unknown = worst. Rate defaults to 0.0.
#[derive(Debug, Clone)]
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,
}

impl Default for MeasuredCapabilities {
    fn default() -> Self {
        Self {
            json_valid_rate: 0.0, // K14: unknown = pessimistic
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

// --- Backend ---

/// A single inference endpoint (one model on one node).
#[derive(Debug, Clone)]
pub struct Backend {
    pub id: BackendId,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
}
