//! Sensory observation domain contracts.
//!
//! Provides the universal, typed observation interface for external sensory
//! inputs and local tool data stores (e.g. RTK token metabolism, RSS feeds).
//!
//! Replaces untyped key-value metric bags (`Vec<Metric>`) with schema-versioned,
//! content-addressed `SensorySnapshot` payloads.
//!
//! Authority: AGENTS.md §Structural Invariant 7 ("Adapters are disposable; ports
//! and proofs are durable. Policy is portable; substrate is injected.")

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const SENSORY_SNAPSHOT_SCHEMA: &str = "cynic.sensory_snapshot.v1";

/// Universal port implemented by all sensory input observers.
#[async_trait::async_trait]
pub trait SensoryPort: Send + Sync {
    /// Stable identifier of this sense (e.g. "rtk", "rss_feed", "host_substrate").
    fn sense_id(&self) -> &'static str;

    /// Health status of the sensor.
    async fn health(&self) -> SenseHealth;

    /// Age / freshness of the most recent data point.
    async fn freshness(&self) -> Result<Duration, SenseError>;

    /// Capture an immutable, content-addressed sensory snapshot.
    async fn capture_snapshot(&self) -> Result<SensorySnapshot, SenseError>;
}

/// Immutable, content-addressed snapshot of sensory perception.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorySnapshot {
    pub schema: String,
    pub sense_id: String,
    pub taken_at_unix_s: u64,
    pub freshness_seconds: u64,
    pub payload: SensoryPayload,
    pub evidence_digest: String,
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
}

impl SensorySnapshot {
    pub fn new(
        sense_id: impl Into<String>,
        taken_at_unix_s: u64,
        freshness_seconds: u64,
        payload: SensoryPayload,
    ) -> Self {
        let sense_str = sense_id.into();
        let digest = crate::digest::digest_json(&serde_json::json!({
            "sense_id": &sense_str,
            "taken_at_unix_s": taken_at_unix_s,
            "payload": &payload,
        }))
        .unwrap_or_else(|_| format!("sha3-512:{}", sense_str));

        Self {
            schema: SENSORY_SNAPSHOT_SCHEMA.to_string(),
            sense_id: sense_str,
            taken_at_unix_s,
            freshness_seconds,
            payload,
            evidence_digest: digest,
            residual_unknowns: Vec::new(),
        }
    }
}

/// Typed payloads for distinct sensory families.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "family")]
pub enum SensoryPayload {
    /// Token metabolism and CLI optimization statistics (RTK).
    TokenMetabolism(TokenMetabolismPayload),
    /// Host hardware and OS substrate metrics.
    HostSubstrate(HostSubstrateSensoryPayload),
    /// Ingested web/API feed items.
    FeedIngestion(FeedIngestionSensoryPayload),
    /// Extensible custom sensory payload.
    Custom {
        kind: String,
        data: serde_json::Value,
    },
}

/// Token savings and metabolism metrics produced by RTK proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenMetabolismPayload {
    pub total_commands: u64,
    pub total_input_tokens: u64,
    pub total_saved_tokens: u64,
    pub savings_percent: f64,
    pub active_scope: Option<String>,
}

/// Host substrate summary observed through sensory port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostSubstrateSensoryPayload {
    pub cpu_usage_percent: f64,
    pub mem_available_bytes: u64,
    pub disk_available_bytes: u64,
}

/// Feed ingestion summary observed through sensory port.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedIngestionSensoryPayload {
    pub feed_id: String,
    pub items_count: usize,
    pub last_item_published_unix_s: u64,
}

/// Health and liveness status of a sensor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SenseHealth {
    Alive,
    Degraded { reason: String },
    Dead { reason: String },
}

impl SenseHealth {
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Alive)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }

    pub fn is_dead(&self) -> bool {
        matches!(self, Self::Dead { .. })
    }
}

/// Error encountered while reading or capturing sensory data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum SenseError {
    #[error("sensor unavailable: {0}")]
    Unavailable(String),
    #[error("sensor read failed: {0}")]
    ReadFailed(String),
    #[error("sensor payload malformed: {0}")]
    Malformed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensory_snapshot_schema_and_digest_are_deterministic() {
        let payload = SensoryPayload::TokenMetabolism(TokenMetabolismPayload {
            total_commands: 2276,
            total_input_tokens: 1_900_000,
            total_saved_tokens: 1_450_000,
            savings_percent: 76.3,
            active_scope: Some("CYNIC".to_string()),
        });

        let snapshot = SensorySnapshot::new("rtk", 1724000000, 10, payload);

        assert_eq!(snapshot.schema, SENSORY_SNAPSHOT_SCHEMA);
        assert!(snapshot.evidence_digest.starts_with("sha3-512:"));

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("token_metabolism"));
        assert!(json.contains("2276"));
    }

    #[test]
    fn sense_health_predicates() {
        let alive = SenseHealth::Alive;
        let degraded = SenseHealth::Degraded {
            reason: "stale cache".to_string(),
        };
        let dead = SenseHealth::Dead {
            reason: "db not found".to_string(),
        };

        assert!(alive.is_alive());
        assert!(!alive.is_degraded());
        assert!(degraded.is_degraded());
        assert!(dead.is_dead());
    }
}
