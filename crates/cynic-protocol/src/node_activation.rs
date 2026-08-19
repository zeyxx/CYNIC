//! Node activation — the convergence point of deployment_epoch,
//! bootstrap_federation, and runtime_takeover_activation.
//!
//! Pure decision: given the declared intent (deployment_epoch), the observed
//! bootstrap progress (bootstrap_federation), and the takeover contract
//! (runtime_takeover_activation), produce a single authoritative verdict:
//! Blocked, Activatable, or Active.
//!
//! From AGENTS.md Runtime Harness section:
//!   lab → conformance → staging → canary → production
//!
//! Activation is the gate between "declared" and "live". No silent promotion.

use crate::deployment_epoch::{DeploymentEpoch, DeploymentPhase, DeploymentPhaseObservation};
use crate::bootstrap_federation::{
    FederatedBootstrapBinding, FederatedBootstrapObservation, FederatedBootstrapPhase,
};
use crate::runtime_takeover_activation::{
    RuntimeTakeoverActivationObservation, RuntimeTakeoverActivationRequest,
};

/// The three possible activation states for a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeActivationVerdict {
    /// Node is live and serving traffic.
    Active {
        node_id: String,
        deployment_epoch_id: String,
        /// Authority that granted activation.
        authority: ActivationAuthority,
    },
    /// Node can be activated — all preconditions met.
    Activatable {
        node_id: String,
        deployment_epoch_id: String,
        /// What authority must grant activation.
        required_authority: ActivationAuthority,
        /// The next action to take.
        next_action: ActivationAction,
    },
    /// Node cannot be activated — blockers present.
    Blocked {
        node_id: String,
        deployment_epoch_id: Option<String>,
        blockers: Vec<String>,
    },
}

/// Authority levels for activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationAuthority {
    /// Test projection — lab only.
    TestProjection,
    /// Conformance gate — staging.
    ConformanceGate,
    /// Release candidate — canary.
    ReleaseCandidate,
    /// Fleet canary — production.
    FleetCanary,
    /// Runtime authority — full production.
    RuntimeAuthority,
}

/// The action required to advance activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationAction {
    /// Bootstrap the node (first time).
    Bootstrap,
    /// Advance the deployment epoch.
    AdvanceDeployment {
        from: DeploymentPhase,
        to: DeploymentPhase,
    },
    /// Execute the takeover activation.
    ExecuteTakeover {
        fencing_token: u64,
    },
    /// No action needed (already active).
    None,
}

/// Pure decision: compose deployment_epoch + bootstrap_federation + takeover
/// into a single activation verdict.
pub fn decide_node_activation(
    deployment_epoch: Option<&DeploymentEpoch>,
    bootstrap_observation: Option<&FederatedBootstrapObservation>,
    takeover_request: Option<&RuntimeTakeoverActivationRequest>,
    takeover_observation: Option<&RuntimeTakeoverActivationObservation>,
    current_deployment_phase: Option<DeploymentPhase>,
) -> NodeActivationVerdict {
    // Determine node_id from whichever source is available.
    let node_id = bootstrap_observation
        .map(|o| o.node_id.clone())
        .or_else(|| deployment_epoch.map(|e| e.target_scope.clone()))
        .or_else(|| takeover_request.map(|r| r.node_id.clone()))
        .unwrap_or_else(|| "unknown".to_string());

    let epoch_id = deployment_epoch.map(|e| e.epoch_id.clone());

    // 1. Check takeover activation first — if a takeover is in progress,
    //    its blockers take precedence.
    if let (Some(request), Some(observed)) = (takeover_request, takeover_observation) {
        let mut blockers = Vec::new();

        if request.node_id != observed.node_id {
            blockers.push("takeover_activation_identity_mismatch".into());
        }
        if request.active_unit != observed.active_unit {
            blockers.push("takeover_activation_unit_read_back_mismatch".into());
        }
        if request.fencing_token == 0 || request.fencing_token <= request.prior_fencing_token {
            blockers.push("takeover_activation_fencing_invalid".into());
        }
        if observed.observed_fencing_token != request.fencing_token {
            blockers.push("takeover_activation_fencing_read_back_mismatch".into());
        }
        if !observed.process_active {
            blockers.push("takeover_activation_process_inactive".into());
        }
        if !observed.identity_verified {
            blockers.push("takeover_activation_identity_unproved".into());
        }

        if !blockers.is_empty() {
            return NodeActivationVerdict::Blocked {
                node_id,
                deployment_epoch_id: epoch_id,
                blockers,
            };
        }

        // Takeover checks pass — node is active.
        return NodeActivationVerdict::Active {
            node_id,
            deployment_epoch_id: epoch_id.unwrap_or_default(),
            authority: ActivationAuthority::RuntimeAuthority,
        };
    }

    // 2. Check bootstrap federation — if bootstrap is incomplete, node is blocked.
    if let Some(bootstrap) = bootstrap_observation {
        if bootstrap.phase != FederatedBootstrapPhase::Complete {
            let mut blockers = bootstrap.blockers.clone();
            if blockers.is_empty() {
                blockers.push(format!(
                    "federated_bootstrap_phase_incomplete:{:?}",
                    bootstrap.phase
                ));
            }
            return NodeActivationVerdict::Blocked {
                node_id,
                deployment_epoch_id: epoch_id,
                blockers,
            };
        }
    }

    // 3. Check deployment epoch — if deployment is not complete, node may be activatable.
    if let Some(epoch) = deployment_epoch {
        match current_deployment_phase {
            Some(DeploymentPhase::Verifying) | Some(DeploymentPhase::Completed) => {
                // Deployment complete — node is active.
                return NodeActivationVerdict::Active {
                    node_id,
                    deployment_epoch_id: epoch.epoch_id.clone(),
                    authority: authority_for_scope(&epoch.target_scope),
                };
            }
            Some(phase) => {
                // Deployment in progress — activatable if phase is ready.
                return NodeActivationVerdict::Activatable {
                    node_id,
                    deployment_epoch_id: epoch.epoch_id.clone(),
                    required_authority: authority_for_scope(&epoch.target_scope),
                    next_action: ActivationAction::AdvanceDeployment {
                        from: phase,
                        to: next_deployment_phase(phase),
                    },
                };
            }
            None => {
                return NodeActivationVerdict::Blocked {
                    node_id,
                    deployment_epoch_id: Some(epoch.epoch_id.clone()),
                    blockers: vec!["deployment_phase_unknown".into()],
                };
            }
        }
    }

    // 4. No deployment epoch, no bootstrap, no takeover — unknown state.
    NodeActivationVerdict::Blocked {
        node_id,
        deployment_epoch_id: epoch_id,
        blockers: vec!["no_deployment_epoch_or_bootstrap_observation".into()],
    }
}

/// Map target scope to activation authority.
fn authority_for_scope(scope: &str) -> ActivationAuthority {
    match scope {
        "lab" => ActivationAuthority::TestProjection,
        "conformance" => ActivationAuthority::ConformanceGate,
        "staging" => ActivationAuthority::ReleaseCandidate,
        "canary" => ActivationAuthority::FleetCanary,
        "fleet" | "production" => ActivationAuthority::RuntimeAuthority,
        _ => ActivationAuthority::RuntimeAuthority,
    }
}

/// Get the next deployment phase.
fn next_deployment_phase(current: DeploymentPhase) -> DeploymentPhase {
    use DeploymentPhase::*;
    match current {
        Proposed => Admitted,
        Admitted => Building,
        Building => Built,
        Built => Staging,
        Staging => Staged,
        Staged => Canary,
        Canary => RollingOut,
        RollingOut => Verifying,
        Verifying => Completed,
        Completed | Failed => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment_epoch::derive_deployment_epoch;

    #[test]
    fn active_when_deployment_completed() {
        let epoch = derive_deployment_epoch("commit-1", "sha256:policy", "fleet", 1).unwrap();
        let verdict = decide_node_activation(
            Some(&epoch),
            None,
            None,
            None,
            Some(DeploymentPhase::Completed),
        );
        assert!(matches!(&verdict,
            NodeActivationVerdict::Active { authority: ActivationAuthority::RuntimeAuthority, .. }
        ));
    }

    #[test]
    fn activatable_when_deployment_in_progress() {
        let epoch = derive_deployment_epoch("commit-1", "sha256:policy", "staging", 1).unwrap();
        let verdict = decide_node_activation(
            Some(&epoch),
            None,
            None,
            None,
            Some(DeploymentPhase::Staging),
        );
        assert!(matches!(&verdict,
            NodeActivationVerdict::Activatable {
                required_authority: ActivationAuthority::ReleaseCandidate,
                ..
            }
        ));
    }

    #[test]
    fn blocked_when_takeover_fails() {
        let epoch = derive_deployment_epoch("commit-1", "sha256:policy", "fleet", 1).unwrap();
        let request = RuntimeTakeoverActivationRequest {
            schema: "cynic.runtime_takeover.activation.v1".into(),
            node_id: "node-a".into(),
            fencing_token: 2,
            prior_fencing_token: 1,
            admission_proof_ref: "proof-1".into(),
            standby_unit: "cynic-node-standby".into(),
            active_unit: "cynic-node-active".into(),
            health_endpoint: "http://localhost:8080/health".into(),
            identity_endpoint: "http://localhost:8080/identity".into(),
        };
        let observation = RuntimeTakeoverActivationObservation {
            schema: "cynic.runtime_takeover.activation.v1".into(),
            node_id: "node-a".into(),
            active_unit: "cynic-node-active".into(),
            process_active: false, // FAIL: process not running
            observed_fencing_token: 2,
            identity_verified: true,
            proof_refs: vec!["proof-1".into()],
        };
        let verdict = decide_node_activation(
            Some(&epoch),
            None,
            Some(&request),
            Some(&observation),
            None,
        );
        assert!(matches!(&verdict,
            NodeActivationVerdict::Blocked { blockers, .. }
            if blockers.contains(&"takeover_activation_process_inactive".to_string())
        ));
    }

    #[test]
    fn blocked_when_bootstrap_incomplete() {
        let epoch = derive_deployment_epoch("commit-1", "sha256:policy", "fleet", 1).unwrap();
        let bootstrap = FederatedBootstrapObservation {
            schema: "cynic.federated_bootstrap.v1".into(),
            node_id: "node-a".into(),
            epoch_id: epoch.epoch_id.clone(),
            binding: FederatedBootstrapBinding {
                source_commit: "a".repeat(40),
                deployment_epoch_id: epoch.epoch_id.clone(),
                observed_at_unix_ms: 1,
                evidence_refs: vec!["evidence-1".into()],
            },
            phase: FederatedBootstrapPhase::NodeEnrolled, // incomplete
            seed_admitted: true,
            bootstrap_controller_ready: true,
            provider_ready: true,
            node_enrolled: true,
            storage_ready: false,
            node_ready: false,
            scheduler_ready: false,
            build_proven: false,
            release_read_back: false,
            blockers: vec!["storage_read_back_required".into()],
        };
        let verdict = decide_node_activation(
            Some(&epoch),
            Some(&bootstrap),
            None,
            None,
            None,
        );
        assert!(matches!(&verdict,
            NodeActivationVerdict::Blocked { blockers, .. }
            if blockers.contains(&"storage_read_back_required".to_string())
        ));
    }

    #[test]
    fn blocked_when_no_deployment_epoch() {
        let verdict = decide_node_activation(None, None, None, None, None);
        assert!(matches!(&verdict,
            NodeActivationVerdict::Blocked { blockers, .. }
            if blockers.contains(&"no_deployment_epoch_or_bootstrap_observation".to_string())
        ));
    }

    #[test]
    fn active_when_takeover_succeeds() {
        let epoch = derive_deployment_epoch("commit-1", "sha256:policy", "fleet", 1).unwrap();
        let request = RuntimeTakeoverActivationRequest {
            schema: "cynic.runtime_takeover.activation.v1".into(),
            node_id: "node-a".into(),
            fencing_token: 2,
            prior_fencing_token: 1,
            admission_proof_ref: "proof-1".into(),
            standby_unit: "cynic-node-standby".into(),
            active_unit: "cynic-node-active".into(),
            health_endpoint: "http://localhost:8080/health".into(),
            identity_endpoint: "http://localhost:8080/identity".into(),
        };
        let observation = RuntimeTakeoverActivationObservation {
            schema: "cynic.runtime_takeover.activation.v1".into(),
            node_id: "node-a".into(),
            active_unit: "cynic-node-active".into(),
            process_active: true,
            observed_fencing_token: 2,
            identity_verified: true,
            proof_refs: vec!["proof-1".into()],
        };
        let verdict = decide_node_activation(
            Some(&epoch),
            None,
            Some(&request),
            Some(&observation),
            None,
        );
        assert!(matches!(&verdict, NodeActivationVerdict::Active { .. }));
    }
}
