//! Build metrics for a single crate: build, sync and test timings.
//!
//! Pure data. Lives in the build bounded context. Accounting conversion
//! (ResourceUnit) is deliberately absent: cynic-build must not depend on
//! cynic-finance. A conversion belongs to the consumer that needs both.

use serde::{Deserialize, Serialize};

/// Build metrics for a single crate — records build, sync, and test timings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildMetrics {
    /// Name of the crate measured.
    pub crate_name: String,
    /// Build time in milliseconds.
    pub build_time_ms: u64,
    /// Sync time in milliseconds.
    pub sync_time_ms: u64,
    /// Test time in milliseconds.
    pub test_time_ms: u64,
    /// Whether the build cache was hit.
    pub cache_hit: bool,
}

impl BuildMetrics {
    /// Creates a new `BuildMetrics` instance.
    pub fn new(
        crate_name: impl Into<String>,
        build_time_ms: u64,
        sync_time_ms: u64,
        test_time_ms: u64,
        cache_hit: bool,
    ) -> Self {
        Self {
            crate_name: crate_name.into(),
            build_time_ms,
            sync_time_ms,
            test_time_ms,
            cache_hit,
        }
    }

    /// Returns the total time (build + sync + test) in milliseconds.
    pub fn total_time_ms(&self) -> u64 {
        self.build_time_ms + self.sync_time_ms + self.test_time_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metrics_new() {
        let m = BuildMetrics::new("cynic-build", 1500, 200, 800, false);
        assert_eq!(m.crate_name, "cynic-build");
        assert_eq!(m.build_time_ms, 1500);
        assert_eq!(m.sync_time_ms, 200);
        assert_eq!(m.test_time_ms, 800);
        assert!(!m.cache_hit);
    }

    #[test]
    fn build_metrics_total_time_ms() {
        let m = BuildMetrics::new("cynic-core", 1000, 500, 300, true);
        assert_eq!(m.total_time_ms(), 1800);
    }

    #[test]
    fn build_metrics_cache_hit_flag() {
        let m = BuildMetrics::new("cynic-proto", 0, 50, 0, true);
        assert!(m.cache_hit);
        assert_eq!(m.total_time_ms(), 50);
    }

    #[test]
    fn build_metrics_serialization_roundtrip() {
        let m = BuildMetrics::new("cynic-test", 100, 20, 30, true);
        let json = serde_json::to_string(&m).expect("serialize");
        let deserialized: BuildMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, deserialized);
    }
}
