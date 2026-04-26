//! OrganPort — domain contract for organism sensory organs.
//!
//! Each organ owns its data store (SQLite, files, etc). The kernel reads
//! organ data in read-only mode via this trait. Designed for N organs
//! with N functions and N states — zero kernel changes per new organ.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;

#[async_trait]
pub trait OrganPort: Send + Sync {
    /// Stable organ identity (e.g. "rtk", "hermes-x").
    fn name(&self) -> &str;

    /// Organ liveness. Degraded/Dead carry a reason string.
    async fn health(&self) -> OrganHealth;

    /// Age of the organ's most recent meaningful data.
    /// Semantics are organ-defined: RTK = last command timestamp,
    /// Hermes = last tweet captured. Not the same as snapshot time.
    async fn freshness(&self) -> Result<Duration, OrganError>;

    /// Timestamped bag of typed metrics. Caller can compute rates
    /// from consecutive Counter snapshots via delta/Δt.
    async fn snapshot(&self) -> Result<OrganSnapshot, OrganError>;
}

#[derive(Debug)]
pub struct OrganSnapshot {
    pub taken_at: DateTime<Utc>,
    pub metrics: Vec<Metric>,
}

#[derive(Debug)]
pub struct Metric {
    pub key: String,
    pub value: MetricValue,
    pub kind: MetricKind,
    pub unit: Option<String>,
}

#[derive(Debug)]
pub enum MetricValue {
    F64(f64),
    I64(i64),
    Str(String),
    Bool(bool),
}

/// Counter = monotonically increasing (delta/time = rate).
/// Gauge = point-in-time snapshot (current value).
#[derive(Debug)]
pub enum MetricKind {
    Counter,
    Gauge,
}

#[derive(Debug)]
pub enum OrganHealth {
    Alive,
    Degraded { reason: String },
    Dead { reason: String },
}

#[derive(Debug)]
pub enum OrganError {
    /// Data source not found (DB missing, file absent).
    Unavailable(String),
    /// Data source found but read failed (IO, parse, lock timeout).
    ReadFailed(String),
}

impl std::fmt::Display for OrganError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(msg) => write!(f, "organ unavailable: {msg}"),
            Self::ReadFailed(msg) => write!(f, "organ read failed: {msg}"),
        }
    }
}

impl std::error::Error for OrganError {}
