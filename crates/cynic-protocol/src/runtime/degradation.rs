//! Explicit runtime degradation layers and retained capabilities.

use serde::Serialize;

pub const RUNTIME_DEGRADATION_SCHEMA: &str = "cynic.runtime_degradation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationLayer {
    FullyOperational,
    FunctionalDegraded,
    ReadOnly,
    BootstrapOnly,
    BlockedDisconnected,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDegradationObservation {
    pub kernel_live: bool,
    pub auth_ready: bool,
    pub storage_ok: bool,
    pub healthy_dogs: usize,
    pub probes_degraded: bool,
    pub scheduler_ready: bool,
    pub local_spool_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDegradationStatus {
    pub schema: &'static str,
    pub layer: DegradationLayer,
    pub status: &'static str,
    pub retained_capabilities: Vec<&'static str>,
    pub blocked_mutations: Vec<&'static str>,
    pub causes: Vec<&'static str>,
    pub observation: RuntimeDegradationObservation,
}

pub fn derive_degradation(observation: RuntimeDegradationObservation) -> RuntimeDegradationStatus {
    let mut causes = Vec::new();
    if !observation.kernel_live {
        causes.push("kernel_liveness_unobserved");
    }
    if !observation.auth_ready {
        causes.push("runtime_auth_unavailable");
    }
    if !observation.storage_ok {
        causes.push("durable_storage_unavailable");
    }
    if observation.healthy_dogs == 0 {
        causes.push("no_healthy_dogs");
    }
    if observation.probes_degraded {
        causes.push("substrate_probes_degraded");
    }
    if !observation.scheduler_ready {
        causes.push("scheduler_not_ready");
    }

    let layer = if !observation.kernel_live || !observation.auth_ready {
        DegradationLayer::BlockedDisconnected
    } else if observation.healthy_dogs == 0 {
        DegradationLayer::BootstrapOnly
    } else if !observation.storage_ok && !observation.local_spool_ready {
        DegradationLayer::ReadOnly
    } else if !observation.storage_ok || observation.probes_degraded || !observation.scheduler_ready
    {
        DegradationLayer::FunctionalDegraded
    } else {
        DegradationLayer::FullyOperational
    };

    let (status, retained_capabilities, blocked_mutations) = match layer {
        DegradationLayer::FullyOperational => (
            "operational",
            vec!["observe", "judge", "persist", "schedule", "publish"],
            Vec::new(),
        ),
        DegradationLayer::FunctionalDegraded => (
            "degraded",
            vec!["observe", "judge", "persist"],
            vec!["fleet_mutation", "release_promotion"],
        ),
        DegradationLayer::ReadOnly => (
            "read_only",
            vec!["observe", "health", "read_back"],
            vec!["persist", "schedule", "publish", "fleet_mutation"],
        ),
        DegradationLayer::BootstrapOnly => (
            "bootstrap_only",
            vec!["identity", "health", "bootstrap_observe"],
            vec!["judge", "persist", "schedule", "publish", "fleet_mutation"],
        ),
        DegradationLayer::BlockedDisconnected => (
            "blocked_disconnected",
            vec!["identity", "liveness"],
            vec![
                "observe",
                "judge",
                "persist",
                "schedule",
                "publish",
                "fleet_mutation",
            ],
        ),
    };
    RuntimeDegradationStatus {
        schema: RUNTIME_DEGRADATION_SCHEMA,
        layer,
        status,
        retained_capabilities,
        blocked_mutations,
        causes,
        observation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy() -> RuntimeDegradationObservation {
        RuntimeDegradationObservation {
            kernel_live: true,
            auth_ready: true,
            storage_ok: true,
            healthy_dogs: 3,
            probes_degraded: false,
            scheduler_ready: true,
            local_spool_ready: false,
        }
    }

    #[test]
    fn storage_failure_is_read_only_without_spool() {
        let mut observation = healthy();
        observation.storage_ok = false;
        let status = derive_degradation(observation);
        assert_eq!(status.layer, DegradationLayer::ReadOnly);
        assert!(status.blocked_mutations.contains(&"persist"));
    }

    #[test]
    fn missing_auth_is_disconnected() {
        let mut observation = healthy();
        observation.auth_ready = false;
        let status = derive_degradation(observation);
        assert_eq!(status.layer, DegradationLayer::BlockedDisconnected);
        assert!(!status.retained_capabilities.contains(&"judge"));
    }
}
