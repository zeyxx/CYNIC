//! Introspection Port — pure contract for anomaly detection and cognitive diagnostics.
//!
//! Canonical owner: `cynic-intel` (SoC foundation §481).

use crate::introspection::{Alert, ForensicsSnapshot, SocThresholds};

/// Port for anomaly analysis and introspection diagnostics across the organism.
pub trait IntrospectionPort: std::fmt::Debug + Send + Sync {
    /// Classify a source identifier into its operational tier ("permanent", "cron", "session").
    fn classify_tier(&self, source_id: &str) -> &'static str;

    /// Check forensics evidence against calibrated thresholds and produce alerts.
    fn evaluate_soc_forensics(
        &self,
        thresholds: &SocThresholds,
        snapshot: &ForensicsSnapshot,
    ) -> Vec<Alert>;
}

/// Default pure in-memory implementation of the Introspection Port.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalIntrospectionEngine;

impl IntrospectionPort for LocalIntrospectionEngine {
    fn classify_tier(&self, source_id: &str) -> &'static str {
        crate::introspection::classify_source_tier(source_id)
    }

    fn evaluate_soc_forensics(
        &self,
        thresholds: &SocThresholds,
        snapshot: &ForensicsSnapshot,
    ) -> Vec<Alert> {
        crate::introspection::check_all(thresholds, snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection_engine_port() {
        let engine = LocalIntrospectionEngine;
        assert_eq!(engine.classify_tier("kernel"), "permanent");
        assert_eq!(engine.classify_tier("agent-hermes-1"), "session");

        let thresholds = SocThresholds::default();
        let snap = ForensicsSnapshot {
            boot_id: "boot-1".to_string(),
            gpu_faults: 3,
            kernel_traps: 0,
            service_cascades: 0,
            canary_disconnects: 0,
            watchdog_failures: 0,
            service_restart_rate: 0.0,
            dbus_timeouts: 0,
        };
        let alerts = engine.evaluate_soc_forensics(&thresholds, &snap);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, "soc_gpu_hang");
    }
}
