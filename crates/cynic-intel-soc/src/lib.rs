//! Introspection Subsystem — Cognitive Self-Observation & Anomaly Analysis (MAPE-K).
//!
//! Canonical domain owner: `cynic-intel` (SoC foundation §481).
//!
//! Ticks periodically to analyze internal metrics, sense health, and forensics
//! evidence, returning alerts ONLY when anomaly thresholds are violated.
//! Healthy system = zero alerts = silent.

pub mod rules;
pub mod soc;
pub mod types;

pub use rules::{RETIRED_SOURCES, classify_source_tier, is_permanent_source};
pub use soc::{
    ForensicsSnapshot, SocThresholds, check_all, detect_canary_disconnect, detect_dbus_timeout,
    detect_gpu_fault, detect_journald_watchdog, detect_service_cascade,
    detect_service_restart_rate, detect_softlockup,
};
pub use types::{Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD, SAVINGS_PCT_THRESHOLD};
