//! Backend health and capabilities domain model.
//!
//! Canonical owner: cynic-inference (SoC rule: foundation §481).

use std::time::Instant;

/// Newtype identifier for an inference backend.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackendId(pub String);

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
#[derive(Debug, Clone)]
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,
}

impl Default for MeasuredCapabilities {
    fn default() -> Self {
        Self {
            json_valid_rate: 0.0,
        }
    }
}

/// Live health classification for a backend.
#[derive(Debug, Clone)]
pub enum BackendHealth {
    Healthy,
    Degraded { reason: String, since: Instant },
    Dead { reason: String, since: Instant },
}

/// A single inference endpoint (one model on one node).
#[derive(Debug, Clone)]
pub struct Backend {
    pub id: BackendId,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
}
