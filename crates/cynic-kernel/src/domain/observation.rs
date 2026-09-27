//! Observation port — typed sensory interface for external data stores.
//!
//! Each sense reads an external data store in read-only mode. The kernel
//! perceives the world only through implementations of `ObservationPort`.
//!
//! Renamed from `domain::organ` (OrganPort) — the "organ" terminology was a
//! fossil from the organism metaphor. This module is purely about observation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::fmt;
use std::time::Duration;

/// Health status of a sensor/observation source.
#[derive(Debug, Clone, PartialEq)]
pub enum ObservationHealth {
    Alive,
    Degraded { reason: String },
    Dead { reason: String },
}

/// Errors emitted by an `ObservationPort` implementation.
#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("observation unavailable: {0}")]
    Unavailable(String),
    #[error("observation read failed: {0}")]
    ReadFailed(String),
}

/// A single metric sampled from a data store.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metric {
    pub key: String,
    pub value: MetricValue,
    pub kind: MetricKind,
    pub unit: Option<String>,
}

/// Numeric metric value types.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MetricValue {
    I64(i64),
    F64(f64),
    Gauge(f64),
    Counter(u64),
    String(String),
}

/// Metric cardinality classification.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MetricKind {
    Counter,
    Gauge,
}

/// Point-in-time snapshot of a data store's observation.
///
/// `taken_at` must be parseable as RFC3339 for freshness computation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObservationSnapshot {
    pub taken_at: DateTime<Utc>,
    pub metrics: Vec<Metric>,
}

/// A sensory port — reads an external data store in read-only mode.
///
/// Implemented by readers such as `HermesXReader` (social perception) and
/// `RtkReader` (token history). Each implementation runs I/O inside
/// `spawn_blocking` to avoid stalling the async runtime.
#[async_trait]
pub trait ObservationPort: Send + Sync {
    /// Stable identifier for this observation source.
    fn name(&self) -> &'static str;

    /// Liveness check — does the source respond?
    async fn health(&self) -> ObservationHealth;

    /// How stale is the latest observation?
    async fn freshness(&self) -> Result<Duration, ObservationError>;

    /// Point-in-time snapshot of metrics.
    async fn snapshot(&self) -> Result<ObservationSnapshot, ObservationError>;

    /// Store an observation into this port's backing store (if writable).
    async fn store_observation(
        &self,
        _obs: crate::domain::storage::Observation,
    ) -> Result<(), ObservationError> {
        Err(ObservationError::Unavailable(
            "store_observation not supported by this port".to_string(),
        ))
    }
}
