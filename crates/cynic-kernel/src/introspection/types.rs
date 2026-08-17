//! Types and constants for the introspection subsystem.

/// Anomaly thresholds — conservative defaults.
pub const FAILURE_RATE_THRESHOLD: f64 = 0.20; // > 20% failure rate
pub const EMBED_FAILURE_THRESHOLD: f64 = 0.50; // > 50% embedding failures
/// phi^-2 = 0.382 — minimum acceptable savings rate (RTK gauge is 0-100).
pub const SAVINGS_PCT_THRESHOLD: f64 = 38.2;

/// Alert produced by introspection — serialized into /health "alerts" section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Alert {
    pub kind: &'static str,
    pub message: String,
    pub severity: &'static str, // "warning" | "critical"
}
