//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Pure types and detection functions live in `cynic-intel-soc`.
//! Orchestration (engine, probe, rules) is kernel-local.

pub mod engine;
pub mod probe;
pub mod rules;
pub mod soc;
pub mod types;

#[cfg(test)]
mod tests;

// Re-exports for kernel-internal consumers (observe, health, agent_tasks).
pub use cynic_intel_soc::{
    Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD, ForensicsSnapshot, RETIRED_SOURCES,
    SAVINGS_PCT_THRESHOLD, SocThresholds, classify_source_tier, is_permanent_source,
};
pub use engine::analyze;
pub use engine::analyze_soc;
pub use rules::{analyze_state_log_trends, check_sense_health};
