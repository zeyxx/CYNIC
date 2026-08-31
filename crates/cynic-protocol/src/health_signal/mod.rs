//! Typed health signals, anomalies, and autonomic remediation contracts.
//!
//! Replaces brittle string heuristics with typed, schema-validated signals
//! for closed-loop organism homeostasis.

pub mod reconcile;
pub mod types;

pub use reconcile::reconcile_health_signal;
pub use types::*;
