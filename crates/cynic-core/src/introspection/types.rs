//! Types and constants for the introspection subsystem.
//!
//! Re-exports canonical types from `cynic-intel` (SoC foundation §481).

pub use cynic_intel_soc::{
    Alert, EMBED_FAILURE_THRESHOLD, FAILURE_RATE_THRESHOLD, RETIRED_SOURCES, SAVINGS_PCT_THRESHOLD,
    classify_source_tier, is_permanent_source,
};
