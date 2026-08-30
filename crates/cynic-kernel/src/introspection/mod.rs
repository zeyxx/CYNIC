//! MAPE-K Analyze loop — self-observation for ANOMALIES ONLY.
//!
//! Ticks every 5 minutes. Observes internal metrics and returns alerts
//! when thresholds are violated. Healthy system = zero alerts = silent.
//!
//! IMPORTANT: introspection MUST NOT call pipeline::run(). Feeding health
//! alerts through the judge creates a self-amplifying feedback loop.

pub mod engine;
pub mod probe;
pub mod rules;
pub mod soc;
pub mod types;

#[cfg(test)]
mod tests;

pub use engine::analyze;
pub use rules::{
    analyze_state_log_trends, check_sense_health, classify_source_tier, is_permanent_source,
};
pub use types::Alert;
