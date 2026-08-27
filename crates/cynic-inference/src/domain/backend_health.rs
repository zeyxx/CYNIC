//! backend_health.rs — Catalog of known backend endpoints: capabilities, identity, and live state.
//!
//! Extracted from cynic-kernel organ registry to consolidate all backend health
//! and endpoint typing under cynic-inference domain authority (Slice A / Slice 1).

use std::time::Instant;

// --- Newtype identifiers ---

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackendId(pub String);

impl std::fmt::Display for BackendId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Capabilities ---

/// What the backend claims to support (from config / vendor docs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum BackendHealth {
    Healthy,
    Degraded { reason: String, since: Instant },
    Dead { reason: String, since: Instant },
}

impl BackendHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }

    pub fn is_dead(&self) -> bool {
        matches!(self, Self::Dead { .. })
    }
}

// --- Backend ---

/// A single inference endpoint (one model on one node).
#[derive(Debug, Clone, PartialEq)]
pub struct Backend {
    pub id: BackendId,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_id_display() {
        let id = BackendId("dog-01".to_string());
        assert_eq!(format!("{id}"), "dog-01");
    }

    #[test]
    fn test_measured_capabilities_default_pessimistic() {
        let caps = MeasuredCapabilities::default();
        assert_eq!(caps.json_valid_rate, 0.0);
    }

    #[test]
    fn test_backend_health_predicates() {
        let healthy = BackendHealth::Healthy;
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
        assert!(!healthy.is_dead());

        let degraded = BackendHealth::Degraded {
            reason: "timeout".to_string(),
            since: Instant::now(),
        };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());
        assert!(!degraded.is_dead());
    }
}
