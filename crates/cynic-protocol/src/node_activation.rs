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

/// The bounded action plan — what to execute, with rollback intent and
/// stop condition.
///
/// Per AGENTS.md Coherence Pattern:
///   bounded action = an explicit, reviewable mutation
///   Every mutation is a small transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationActionPlan {
    /// Human-readable description of the action.
    pub description: String,
    /// The argv to execute (systemctl start, etc.).
    pub apply_argv: Vec<String>,
    /// Read-back argv — verify the action succeeded.
    pub read_back_argv: Vec<String>,
    /// Rollback argv — what to execute if read-back fails.
    pub rollback_argv: Vec<String>,
    /// Whether this action requires privilege (sudo/root).
    pub requires_privilege: bool,
    /// Whether explicit `--yes` consent is required.
    pub requires_consent: bool,
    /// Evidence refs that should be collected after the action.
    pub expected_evidence_refs: Vec<String>,
}

/// Post-action read-back proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationProof {
    /// Activation succeeded — all read-backs passed.
    Activated {
        node_id: String,
        evidence_refs: Vec<String>,
    },
    /// Activation failed — read-back detected the failure.
    Failed {
        node_id: String,
        blockers: Vec<String>,
    },
    /// Activation was rolled back — read-back failed but rollback succeeded.
    RolledBack {
        node_id: String,
        rollback_evidence_refs: Vec<String>,
    },
}

/// Pure decision: plan the bounded action for an activatable node.
///
/// Returns the argv plan with rollback intent and stop condition.
/// Never executes — that's the runtime's job.
pub fn plan_activation_action(
    verdict: &NodeActivationVerdict,
) -> Result<ActivationActionPlan, String> {
    match verdict {
        NodeActivationVerdict::Activatable { node_id, next_action, .. } => {
            match next_action {
                ActivationAction::Bootstrap => Ok(ActivationActionPlan {
                    description: format!("Bootstrap node {}", node_id),
                    apply_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "bootstrap".into(),
                        node_id.clone(),
                    ],
                    read_back_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "activation".into(),
                        "status".into(),
                        node_id.clone(),
                    ],
                    rollback_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "deactivate".into(),
                        node_id.clone(),
                    ],
                    requires_privilege: true,
                    requires_consent: true,
                    expected_evidence_refs: vec![
                        format!("node://{}/bootstrap", node_id),
                    ],
                }),
                ActivationAction::AdvanceDeployment { from, to } => Ok(ActivationActionPlan {
                    description: format!("Advance deployment {:?} -> {:?}", from, to),
                    apply_argv: vec![
                        "cynic".into(),
                        "deployment".into(),
                        "advance".into(),
                        format!("{:?}", from),
                        format!("{:?}", to),
                    ],
                    read_back_argv: vec![
                        "cynic".into(),
                        "deployment".into(),
                        "status".into(),
                    ],
                    rollback_argv: vec![
                        "cynic".into(),
                        "deployment".into(),
                        "rollback".into(),
                    ],
                    requires_privilege: false,
                    requires_consent: true,
                    expected_evidence_refs: vec![
                        format!("deployment://{:?}->{:?}", from, to),
                    ],
                }),
                ActivationAction::ExecuteTakeover { fencing_token } => Ok(ActivationActionPlan {
                    description: format!("Execute takeover (fencing_token={})", fencing_token),
                    apply_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "takeover".into(),
                        "--fencing-token".into(),
                        format!("{}", fencing_token),
                    ],
                    read_back_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "activation".into(),
                        "status".into(),
                    ],
                    rollback_argv: vec![
                        "cynic".into(),
                        "node".into(),
                        "takeover".into(),
                        "--rollback".into(),
                    ],
                    requires_privilege: true,
                    requires_consent: true,
                    expected_evidence_refs: vec![
                        format!("takeover://fencing={}", fencing_token),
                    ],
                }),
                ActivationAction::None => Err("no action needed".into()),
            }
        }
        NodeActivationVerdict::Active { .. } => Err("node is already active".into()),
        NodeActivationVerdict::Blocked { blockers, .. } => {
            Err(format!("node is blocked: {:?}", blockers))
        }
    }
}

/// Pure decision: evaluate read-back proof after an activation action.
pub fn evaluate_activation_proof(
    plan: &ActivationActionPlan,
    apply_success: bool,
    read_back_output: &str,
    rollback_performed: bool,
    rollback_output: &str,
) -> ActivationProof {
    let node_id = plan
        .apply_argv
        .get(3)
        .cloned()
        .unwrap_or_else(|| "unknown".into());

    if !apply_success {
        if rollback_performed {
            return ActivationProof::RolledBack {
                node_id,
                rollback_evidence_refs: vec![format!("rollback://{}", plan.description)],
            };
        }
        return ActivationProof::Failed {
            node_id,
            blockers: vec!["apply_failed".into()],
        };
    }

    // Parse read-back output — if it contains "active", success.
    let read_back_ok = read_back_output.contains("active")
        || read_back_output.contains("\"status\":\"ok\"");

    if read_back_ok {
        ActivationProof::Activated {
            node_id,
            evidence_refs: plan.expected_evidence_refs.clone(),
        }
    } else if rollback_performed {
        ActivationProof::RolledBack {
            node_id,
            rollback_evidence_refs: vec![format!("rollback://{}: {}", plan.description, rollback_output)],
        }
    } else {
        ActivationProof::Failed {
            node_id,
            blockers: vec![format!("read_back_failed: {}", read_back_output)],
        }
    }
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

    #[test]
    fn plan_for_bootstrap_verdict() {
        let verdict = NodeActivationVerdict::Activatable {
            node_id: "node-a".into(),
            deployment_epoch_id: "epoch-1".into(),
            required_authority: ActivationAuthority::ConformanceGate,
            next_action: ActivationAction::Bootstrap,
        };
        let plan = plan_activation_action(&verdict).unwrap();
        assert!(plan.requires_consent);
        assert!(plan.requires_privilege);
        assert_eq!(plan.apply_argv, vec!["cynic", "node", "bootstrap", "node-a"]);
    }

    #[test]
    fn plan_for_blocked_verdict_fails() {
        let verdict = NodeActivationVerdict::Blocked {
            node_id: "node-a".into(),
            deployment_epoch_id: None,
            blockers: vec!["some_blocker".into()],
        };
        assert!(plan_activation_action(&verdict).is_err());
    }

    #[test]
    fn proof_evaluation_activated() {
        let plan = ActivationActionPlan {
            description: "test".into(),
            apply_argv: vec!["echo".into(), "hello".into()],
            read_back_argv: vec!["true".into()],
            rollback_argv: vec!["false".into()],
            requires_privilege: false,
            requires_consent: false,
            expected_evidence_refs: vec!["test://evidence".into()],
        };
        let proof = evaluate_activation_proof(&plan, true, "{\"status\":\"ok\"}", false, "");
        assert!(matches!(&proof, ActivationProof::Activated { .. }));
    }

    #[test]
    fn proof_evaluation_failed_with_rollback() {
        let plan = ActivationActionPlan {
            description: "test".into(),
            apply_argv: vec!["echo".into(), "hello".into()],
            read_back_argv: vec!["true".into()],
            rollback_argv: vec!["false".into()],
            requires_privilege: false,
            requires_consent: false,
            expected_evidence_refs: vec!["test://evidence".into()],
        };
        let proof = evaluate_activation_proof(&plan, true, "FAIL", true, "rolled back");
        assert!(matches!(&proof, ActivationProof::RolledBack { .. }));
    }
}
