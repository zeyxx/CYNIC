//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Re-exports pure introspection types and rules from `cynic-intel` (SoC foundation §481).

pub mod engine;
pub mod probe;
pub mod rules;
pub mod soc;
pub mod types;

#[cfg(test)]
mod tests;

pub use cynic_intel::introspection::{
    Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD, RETIRED_SOURCES, SAVINGS_PCT_THRESHOLD,
    classify_source_tier, is_permanent_source,
};
pub use engine::analyze;
pub use engine::analyze_soc;
pub use rules::{analyze_state_log_trends, check_sense_health};
