//! Local autonomy loop — observe → decide → act (kernel-optional).
//!
//! A node runs this loop whether the kernel is reachable or not. When the
//! kernel is alive, the node is "coordinated" (kernel directs). When the kernel
//! is dead, the node promotes to "autonomous" (local decisions, retain core
//! capabilities). This is the bottom-up federation: no SPOF, no hardcoded
//! failover target — just a local decision from observed evidence.

use serde::{Deserialize, Serialize};

pub const LOCAL_AUTONOMY_STATE_SCHEMA: &str = "cynic.local_autonomy_state.v1";
pub const LOCAL_AUTONOMY_DECISION_SCHEMA: &str = "cynic.local_autonomy_decision.v1";

/// The operating mode of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    /// Kernel is reachable and directing. Node follows kernel workloads.
    Coordinated,
    /// Kernel is unreachable. Node makes local decisions, retains core
    /// capabilities (identity, health, observe, judge).
    Autonomous,
    /// Neither coordinated nor autonomous — node cannot observe enough to
    /// decide. Degraded to minimum: identity + health only.
    Degraded,
}

/// What a node retains the right to do in a given mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedCapabilities {
    pub identity: bool,
    pub health: bool,
    pub observe: bool,
    pub judge: bool,
    pub build: bool,
    pub release: bool,
}

impl RetainedCapabilities {
    pub fn for_mode(mode: AutonomyMode) -> Self {
        match mode {
            AutonomyMode::Coordinated => Self {
                identity: true,
                health: true,
                observe: true,
                judge: true,
                build: true,
                release: true,
            },
            AutonomyMode::Autonomous => Self {
                identity: true,
                health: true,
                observe: true,
                judge: true,
                build: false,
                release: false,
            },
            AutonomyMode::Degraded => Self {
                identity: true,
                health: true,
                observe: false,
                judge: false,
                build: false,
                release: false,
            },
        }
    }
}

/// Observation input for the autonomy loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyObservation {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub kernel_reachable: bool,
    pub kernel_lease_valid: bool,
    pub local_os_healthy: bool,
    pub mesh_reachable_peers: usize,
    pub enrollment_proven: bool,
}

impl AutonomyObservation {
    pub fn unknown() -> Self {
        Self {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 0,
            kernel_reachable: false,
            kernel_lease_valid: false,
            local_os_healthy: false,
            mesh_reachable_peers: 0,
            enrollment_proven: false,
        }
    }
}

/// The autonomy decision (output of the loop).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyDecision {
    pub schema: String,
    pub mode: AutonomyMode,
    pub retained_capabilities: RetainedCapabilities,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

/// Pure rule: decide autonomy mode from observed evidence.
///
/// Rules (fail-closed):
/// - enrollment not proven → Degraded (cannot act without identity)
/// - kernel reachable + lease valid → Coordinated (kernel directs)
/// - kernel unreachable + OS healthy + mesh peers → Autonomous (local decisions)
/// - kernel unreachable + OS unhealthy → Degraded (cannot trust local judgment)
pub fn decide_autonomy(observation: &AutonomyObservation) -> AutonomyDecision {
    let mut blockers = vec![];

    if !observation.enrollment_proven {
        blockers.push("enrollment_unproved".to_string());
    }
    if !observation.local_os_healthy {
        blockers.push("local_os_unhealthy".to_string());
    }

    let mode = if !observation.enrollment_proven {
        AutonomyMode::Degraded
    } else if observation.kernel_reachable && observation.kernel_lease_valid {
        AutonomyMode::Coordinated
    } else if observation.local_os_healthy && observation.mesh_reachable_peers > 0 {
        AutonomyMode::Autonomous
    } else if observation.local_os_healthy {
        // OS healthy but no mesh peers — still autonomous (can observe/judge locally)
        AutonomyMode::Autonomous
    } else {
        AutonomyMode::Degraded
    };

    AutonomyDecision {
        schema: LOCAL_AUTONOMY_DECISION_SCHEMA.to_string(),
        mode,
        retained_capabilities: RetainedCapabilities::for_mode(mode),
        blockers,
        evidence_refs: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinated_when_kernel_reachable() {
        let obs = AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            kernel_reachable: true,
            kernel_lease_valid: true,
            local_os_healthy: true,
            mesh_reachable_peers: 3,
            enrollment_proven: true,
        };
        let decision = decide_autonomy(&obs);
        assert_eq!(decision.mode, AutonomyMode::Coordinated);
        assert!(decision.retained_capabilities.build);
    }

    #[test]
    fn autonomous_when_kernel_dead_but_os_healthy() {
        let obs = AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            kernel_reachable: false,
            kernel_lease_valid: false,
            local_os_healthy: true,
            mesh_reachable_peers: 2,
            enrollment_proven: true,
        };
        let decision = decide_autonomy(&obs);
        assert_eq!(decision.mode, AutonomyMode::Autonomous);
        assert!(decision.retained_capabilities.observe);
        assert!(!decision.retained_capabilities.build);
    }

    #[test]
    fn degraded_when_enrollment_unproved() {
        let obs = AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            kernel_reachable: true,
            kernel_lease_valid: true,
            local_os_healthy: true,
            mesh_reachable_peers: 3,
            enrollment_proven: false,
        };
        let decision = decide_autonomy(&obs);
        assert_eq!(decision.mode, AutonomyMode::Degraded);
        assert!(!decision.retained_capabilities.observe);
    }

    #[test]
    fn degraded_when_os_unhealthy() {
        let obs = AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            kernel_reachable: false,
            kernel_lease_valid: false,
            local_os_healthy: false,
            mesh_reachable_peers: 2,
            enrollment_proven: true,
        };
        let decision = decide_autonomy(&obs);
        assert_eq!(decision.mode, AutonomyMode::Degraded);
    }

    #[test]
    fn autonomous_without_mesh_peers_but_os_healthy() {
        let obs = AutonomyObservation {
            schema: LOCAL_AUTONOMY_STATE_SCHEMA.to_string(),
            observed_at_unix_s: 1,
            kernel_reachable: false,
            kernel_lease_valid: false,
            local_os_healthy: true,
            mesh_reachable_peers: 0,
            enrollment_proven: true,
        };
        let decision = decide_autonomy(&obs);
        assert_eq!(decision.mode, AutonomyMode::Autonomous);
        assert!(decision.retained_capabilities.observe);
    }

    #[test]
    fn schema_is_stable() {
        assert_eq!(LOCAL_AUTONOMY_STATE_SCHEMA, "cynic.local_autonomy_state.v1");
        assert_eq!(
            LOCAL_AUTONOMY_DECISION_SCHEMA,
            "cynic.local_autonomy_decision.v1"
        );
    }
}
