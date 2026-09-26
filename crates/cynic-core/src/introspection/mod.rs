//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Pure types and detection functions live in `cynic-intel-soc`.
//! Orchestration (engine, probe, rules) is kernel-local.

pub mod probe;
pub mod soc;
pub mod types;

// Re-exports for kernel-internal consumers (observe, health, agent_tasks).
pub use cynic_intel_soc::{
    Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD, ForensicsSnapshot, RETIRED_SOURCES,
    SAVINGS_PCT_THRESHOLD, SocThresholds, classify_source_tier, is_permanent_source,
};
