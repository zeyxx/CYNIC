use serde::{Deserialize, Serialize};

use crate::{
    EvidenceRelation, RuntimeSeedObservation, RuntimeSeedPlan, RuntimeSeedRequest,
    SecurityEvidenceState,
    convergence::{
        COHERENCE_VECTOR_SCHEMA, CausalConvergenceEpoch, Compass, ConvergenceStatus,
        TrustAssessment, TrustDimensionStatus, TrustRequirements, Vector, assess_coherence_vector,
    },
    digest_json, reconcile_runtime_seed, validate_runtime_seed_request,
};

pub const NODE_DESIRED_STATE_SCHEMA: &str = "cynic.node_desired_state.v1";
pub const NODE_CONTROL_OBSERVATION_SCHEMA: &str = "cynic.node_control_observation.v1";
pub const NODE_CONTROL_PLAN_SCHEMA: &str = "cynic.node_control_plan.v2";

fn unknown_fleet_membership() -> SecurityEvidenceState {
    SecurityEvidenceState::Unknown
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDesiredState {
    pub schema: String,
    pub node_id: String,
    pub role: String,
    pub generation: u64,
    pub policy_digest: String,
    pub release: RuntimeSeedRequest,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub required_services: Vec<String>,
    #[serde(default)]
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeControlConnectionState {
    Connected,
    Disconnected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeControlObservation {
    pub schema: String,
    pub node_id: String,
    pub observed_at_unix_ms: u64,
    pub observed_generation: Option<u64>,
    pub enrollment: SecurityEvidenceState,
    /// Fleet membership is separate from transport reachability. A node may
    /// temporarily reach control through its provider fallback while its
    /// canonical mesh remains refuted.
    #[serde(default = "unknown_fleet_membership")]
    pub fleet_membership: SecurityEvidenceState,
    pub connection: NodeControlConnectionState,
    pub release: RuntimeSeedObservation,
    #[serde(default)]
    pub observed_capabilities: Vec<String>,
    #[serde(default)]
    pub authorized_capabilities: Vec<String>,
    #[serde(default)]
    pub observed_services: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum NodeControlDecision {
    Paused,
    EnrollmentRequired,
    ReconcileRelease {
        plan: RuntimeSeedPlan,
    },
    ReconcileCapabilities {
        missing_capabilities: Vec<String>,
        missing_services: Vec<String>,
    },
    Converged,
    Unknown {
        missing_evidence: Vec<String>,
    },
    Blocked {
        violations: Vec<String>,
    },
}

impl NodeControlDecision {
    /// Project onto the canonical compass/vector decision vocabulary.
    pub fn label(&self) -> String {
        match self {
            NodeControlDecision::Paused => "paused",
            NodeControlDecision::EnrollmentRequired => "enrollment_required",
            NodeControlDecision::ReconcileRelease { .. } => "reconcile_release",
            NodeControlDecision::ReconcileCapabilities { .. } => "reconcile_capabilities",
            NodeControlDecision::Converged => "converged",
            NodeControlDecision::Unknown { .. } => "unknown",
            NodeControlDecision::Blocked { .. } => "blocked",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeControlExecutionEvidence {
    pub schema: String,
    pub operation: String,
    pub status: String,
    pub mutates: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeControlPlan {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub role: String,
    pub desired_generation: u64,
    pub observed_generation: Option<u64>,
    pub connection: NodeControlConnectionState,
    pub decision: NodeControlDecision,
    #[serde(default)]
    pub execution: Option<NodeControlExecutionEvidence>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
    /// Canonical Compass/Vector coherence proof for this reconcile step. The
    /// direction is the signed desired state; the vector is the pure decision
    /// plus the observed read-back evidence. `None` only when the observation
    /// or desired state itself failed schema/identity checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<CausalConvergenceEpoch>,
}

/// Map a `NodeControlDecision` onto the canonical compass/vector vocabulary so
/// vertical-specific enums still project from the shared decision source
/// (structural-invariants rule #6).
fn coherence_status_for(decision: &NodeControlDecision) -> ConvergenceStatus {
    match decision {
        NodeControlDecision::Converged => ConvergenceStatus::Complete,
        NodeControlDecision::Paused
        | NodeControlDecision::ReconcileRelease { .. }
        | NodeControlDecision::ReconcileCapabilities { .. } => ConvergenceStatus::Partial,
        NodeControlDecision::EnrollmentRequired | NodeControlDecision::Unknown { .. } => {
            ConvergenceStatus::Unknown
        }
        NodeControlDecision::Blocked { .. } => ConvergenceStatus::Blocked,
    }
}

pub fn validate_node_desired_state(desired: &NodeDesiredState) -> Vec<String> {
    let mut violations = Vec::new();
    check(
        desired.schema == NODE_DESIRED_STATE_SCHEMA,
        "node_desired_state_schema_mismatch",
        &mut violations,
    );
    check(
        !desired.node_id.trim().is_empty() && desired.release.node_id == desired.node_id,
        "node_desired_state_identity_invalid",
        &mut violations,
    );
    check(
        !desired.role.trim().is_empty(),
        "node_desired_state_role_missing",
        &mut violations,
    );
    check(
        desired.generation > 0,
        "node_desired_state_generation_invalid",
        &mut violations,
    );
    check(
        valid_sha256(&desired.policy_digest),
        "node_desired_state_policy_digest_invalid",
        &mut violations,
    );
    check(
        desired.release.schema == crate::RUNTIME_SEED_REQUEST_SCHEMA,
        "node_desired_state_release_schema_mismatch",
        &mut violations,
    );
    violations.extend(validate_runtime_seed_request(&desired.release));
    violations.sort();
    violations.dedup();
    violations
}

pub fn validate_node_control_plan(plan: &NodeControlPlan) -> Vec<String> {
    let mut violations = Vec::new();
    check(
        plan.schema == NODE_CONTROL_PLAN_SCHEMA,
        "node_control_plan_schema_mismatch",
        &mut violations,
    );
    check(
        !plan.node_id.trim().is_empty(),
        "node_control_plan_node_id_missing",
        &mut violations,
    );
    check(
        !plan.role.trim().is_empty(),
        "node_control_plan_role_missing",
        &mut violations,
    );
    check(
        plan.desired_generation > 0,
        "node_control_plan_generation_invalid",
        &mut violations,
    );
    check(
        matches!(
            plan.status.as_str(),
            "converged"
                | "degraded_disconnected"
                | "paused"
                | "unknown"
                | "blocked"
                | "enrollment_required"
                | "planned"
        ),
        "node_control_plan_status_invalid",
        &mut violations,
    );
    if let Some(execution) = &plan.execution {
        check(
            execution.schema == "cynic.node_control.execution_evidence.v1",
            "node_control_execution_schema_mismatch",
            &mut violations,
        );
        check(
            matches!(execution.status.as_str(), "proved" | "blocked" | "unknown"),
            "node_control_execution_status_invalid",
            &mut violations,
        );
    }
    violations
}

pub fn reconcile_node_control(
    desired: &NodeDesiredState,
    observed: &NodeControlObservation,
) -> NodeControlPlan {
    let mut violations = validate_node_desired_state(desired);
    let mut unknowns = observed.unknowns.clone();
    check(
        observed.schema == NODE_CONTROL_OBSERVATION_SCHEMA,
        "node_control_observation_schema_mismatch",
        &mut violations,
    );
    check(
        desired.node_id == observed.node_id,
        "node_control_identity_mismatch",
        &mut violations,
    );
    if observed.observed_at_unix_ms == 0 {
        unknowns.push("node_control_observation_time_missing".to_string());
    }
    if observed.connection == NodeControlConnectionState::Unknown {
        unknowns.push("node_control_connection_unobserved".to_string());
    }
    unknowns.sort();
    unknowns.dedup();
    let missing_authority = missing(
        &desired.required_capabilities,
        &observed.authorized_capabilities,
    );

    let decision = if !violations.is_empty() {
        NodeControlDecision::Blocked { violations }
    } else if desired.paused {
        NodeControlDecision::Paused
    } else if observed.enrollment != SecurityEvidenceState::Proven {
        match observed.enrollment {
            SecurityEvidenceState::Absent => NodeControlDecision::EnrollmentRequired,
            SecurityEvidenceState::Unknown
            | SecurityEvidenceState::Declared
            | SecurityEvidenceState::Proven => NodeControlDecision::Unknown {
                missing_evidence: vec!["node_enrollment_unproved".to_string()],
            },
            SecurityEvidenceState::Refuted | SecurityEvidenceState::NotApplicable => {
                NodeControlDecision::Blocked {
                    violations: vec!["node_enrollment_invalid".to_string()],
                }
            }
        }
    } else if observed.fleet_membership != SecurityEvidenceState::Proven {
        match observed.fleet_membership {
            SecurityEvidenceState::Refuted => NodeControlDecision::Blocked {
                violations: vec!["fleet_membership_mesh_unproved".to_string()],
            },
            SecurityEvidenceState::Unknown
            | SecurityEvidenceState::Declared
            | SecurityEvidenceState::Absent => NodeControlDecision::Unknown {
                missing_evidence: vec!["fleet_membership_unobserved".to_string()],
            },
            SecurityEvidenceState::NotApplicable | SecurityEvidenceState::Proven => {
                NodeControlDecision::Unknown {
                    missing_evidence: vec!["fleet_membership_invalid_state".to_string()],
                }
            }
        }
    } else if !missing_authority.is_empty() {
        NodeControlDecision::Blocked {
            violations: missing_authority
                .into_iter()
                .map(|capability| format!("node_capability_not_granted:{capability}"))
                .collect(),
        }
    } else {
        let release_plan = reconcile_runtime_seed(&desired.release, &observed.release);
        match release_plan.status.as_str() {
            "blocked" => NodeControlDecision::Blocked {
                violations: release_plan.blockers.clone(),
            },
            "unknown" => NodeControlDecision::Unknown {
                missing_evidence: release_plan.unknowns.clone(),
            },
            "planned" => NodeControlDecision::ReconcileRelease { plan: release_plan },
            "converged" => {
                let missing_capabilities = missing(
                    &desired.required_capabilities,
                    &observed.observed_capabilities,
                );
                let missing_services =
                    missing(&desired.required_services, &observed.observed_services);
                if missing_capabilities.is_empty() && missing_services.is_empty() {
                    NodeControlDecision::Converged
                } else {
                    NodeControlDecision::ReconcileCapabilities {
                        missing_capabilities,
                        missing_services,
                    }
                }
            }
            _ => NodeControlDecision::Blocked {
                violations: vec!["runtime_seed_plan_status_invalid".to_string()],
            },
        }
    };

    let status = match &decision {
        NodeControlDecision::Converged
            if observed.connection == NodeControlConnectionState::Connected =>
        {
            "converged"
        }
        NodeControlDecision::Converged => "degraded_disconnected",
        NodeControlDecision::Paused => "paused",
        NodeControlDecision::Unknown { .. } => "unknown",
        NodeControlDecision::Blocked { .. } => "blocked",
        NodeControlDecision::EnrollmentRequired => "enrollment_required",
        NodeControlDecision::ReconcileRelease { .. }
        | NodeControlDecision::ReconcileCapabilities { .. } => "planned",
    };

    NodeControlPlan {
        schema: NODE_CONTROL_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: desired.node_id.clone(),
        role: desired.role.clone(),
        desired_generation: desired.generation,
        observed_generation: observed.observed_generation,
        connection: observed.connection,
        decision: decision.clone(),
        execution: None,
        evidence_refs: observed.evidence_refs.clone(),
        unknowns: unknowns.clone(),
        epoch: Some(build_node_control_epoch(
            desired, observed, &decision, status, &unknowns,
        )),
    }
}

/// Build the canonical Compass/Vector coherence proof for this reconcile step.
///
/// COMPASS  = the signed desired state (policy_digest is the comparable,
///   non-mutating direction).
/// VECTOR   = the pure `NodeControlDecision` plus the observed read-back
///   evidence (the magnitude applied and independently observed).
///
/// A `Converged` claim without observed read-back evidence is honestly
/// downgraded to `Unknown` by `assess_coherence_vector` — completeness is not
/// asserted without proof (structural-invariants rule #5).
fn build_node_control_epoch(
    desired: &NodeDesiredState,
    observed: &NodeControlObservation,
    decision: &NodeControlDecision,
    status: &str,
    unknowns: &[String],
) -> CausalConvergenceEpoch {
    let compass = Compass {
        schema: COHERENCE_VECTOR_SCHEMA.to_string(),
        desired_state_digest: desired.policy_digest.clone(),
        subject_id: desired.node_id.clone(),
        vertical_id: "node_control".to_string(),
    };
    let vector = Vector {
        decision: decision.label(),
        action: None,
        read_back_digest: None,
    };
    let observation_digest = digest_json(observed).ok();
    let has_read_back = vector.read_back_digest.is_some();
    let blockers = match decision {
        NodeControlDecision::Blocked { violations } => violations.clone(),
        _ => Vec::new(),
    };
    assess_coherence_vector(
        &format!("node-control:{}", desired.node_id),
        1,
        &compass,
        &vector,
        observation_digest,
        coherence_status_for(decision),
        node_control_trust_requirements(),
        node_control_trust(decision, status, has_read_back),
        unknowns.to_vec(),
        blockers,
    )
}

fn node_control_trust_requirements() -> TrustRequirements {
    TrustRequirements {
        authority: true,
        authenticity: false,
        integrity: true,
        freshness: false,
        scope: true,
        replay_safety: false,
        read_back: true,
        independence: false,
    }
}

fn node_control_trust(
    decision: &NodeControlDecision,
    _status: &str,
    has_read_back: bool,
) -> TrustAssessment {
    let satisfied = decision == &NodeControlDecision::Converged;
    let authority = if desired_policy_digest_present(decision) {
        TrustDimensionStatus::Satisfied
    } else {
        TrustDimensionStatus::Unknown
    };
    let read_back = if has_read_back {
        TrustDimensionStatus::Satisfied
    } else {
        TrustDimensionStatus::Unknown
    };
    TrustAssessment {
        authority,
        authenticity: TrustDimensionStatus::Unknown,
        integrity: TrustDimensionStatus::Satisfied,
        freshness: TrustDimensionStatus::Unknown,
        scope: if satisfied {
            TrustDimensionStatus::Satisfied
        } else {
            TrustDimensionStatus::Unknown
        },
        replay_safety: TrustDimensionStatus::Unknown,
        read_back,
        independence: EvidenceRelation::Unknown {
            missing_evidence: vec!["independent_node_observer".to_string()],
        },
    }
}

fn desired_policy_digest_present(decision: &NodeControlDecision) -> bool {
    !matches!(decision, NodeControlDecision::Blocked { .. })
}

fn missing(desired: &[String], observed: &[String]) -> Vec<String> {
    let mut values = desired
        .iter()
        .filter(|value| !observed.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn check(condition: bool, violation: &str, violations: &mut Vec<String>) {
    if !condition {
        violations.push(violation.to_string());
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RuntimeArtifactSource, RuntimeArtifactTransportKind};

    fn release_request() -> RuntimeSeedRequest {
        RuntimeSeedRequest {
            schema: crate::RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            release_id: "release-2".to_string(),
            release_epoch: 2,
            source_commit: "c".repeat(40),
            build_proof_digest: format!("sha256:{}", "d".repeat(64)),
            basket_digest: format!("sha256:{}", "a".repeat(64)),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.synapse.cynic/release-2.tar".to_string(),
            },
            basket_path: "/var/lib/cynic/inbox/release-2.tar".to_string(),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.synapse.cynic/release-2.tar.sig".to_string(),
            },
            detached_signature_path: "/var/lib/cynic/inbox/release-2.tar.sig".to_string(),
            allowed_signers_path: "/etc/cynic/runtime-allowed-signers".to_string(),
            registry_path: "/etc/cynic/registry.json".to_string(),
            signer_identity: "cynic-release-authority".to_string(),
            signature_namespace: "cynic-runtime-seed".to_string(),
            required_seed_contract_revision: 4,
            install_root: "/var/lib/cynic/runtime".to_string(),
            staged_slot: "releases/release-2".to_string(),
            current_link: "current".to_string(),
            previous_link: "previous".to_string(),
            postflight_argv: vec![
                "/var/lib/cynic/runtime/current/bin/cynic".to_string(),
                "runtime".to_string(),
                "paths".to_string(),
                "--json".to_string(),
            ],
        }
    }

    fn release_observation(converged: bool) -> RuntimeSeedObservation {
        let request = release_request();
        RuntimeSeedObservation {
            schema: crate::RUNTIME_SEED_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 10,
            node_id: "node-a".to_string(),
            seed_contract_revision: Some(4),
            trust_root: SecurityEvidenceState::Proven,
            signature: SecurityEvidenceState::Proven,
            basket_present: true,
            detached_signature_present: true,
            basket_digest_read_back: Some(request.basket_digest.clone()),
            current_release_id: converged.then_some(request.release_id),
            current_release_epoch: converged.then_some(request.release_epoch),
            current_source_commit: converged.then_some(request.source_commit),
            current_build_proof_digest: converged.then_some(request.build_proof_digest),
            current_basket_digest: converged.then_some(request.basket_digest),
            current_registry_digest: converged.then_some(request.registry_digest.clone()),
            installed_registry_digest: converged.then_some(request.registry_digest),
            previous_release_id: Some("release-1".to_string()),
            slot_layout: SecurityEvidenceState::Proven,
            postflight: if converged {
                SecurityEvidenceState::Proven
            } else {
                SecurityEvidenceState::Unknown
            },
            evidence_refs: vec!["sha256:release-observation".to_string()],
            unknowns: Vec::new(),
        }
    }

    fn desired() -> NodeDesiredState {
        NodeDesiredState {
            schema: NODE_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            generation: 2,
            policy_digest: format!("sha256:{}", "b".repeat(64)),
            release: release_request(),
            required_capabilities: vec!["build.execute".to_string()],
            required_services: vec!["cynic-node".to_string()],
            paused: false,
        }
    }

    fn observation(enrollment: SecurityEvidenceState, converged: bool) -> NodeControlObservation {
        NodeControlObservation {
            schema: NODE_CONTROL_OBSERVATION_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            observed_at_unix_ms: 10,
            observed_generation: Some(1),
            enrollment,
            fleet_membership: SecurityEvidenceState::Proven,
            connection: NodeControlConnectionState::Connected,
            release: release_observation(converged),
            observed_capabilities: vec!["build.execute".to_string()],
            authorized_capabilities: vec!["build.execute".to_string()],
            observed_services: vec!["cynic-node".to_string()],
            evidence_refs: vec!["sha256:node-observation".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn unenrolled_node_cannot_reconcile_a_release() {
        let plan = reconcile_node_control(
            &desired(),
            &observation(SecurityEvidenceState::Absent, false),
        );
        assert_eq!(plan.status, "enrollment_required");
        assert_eq!(plan.decision, NodeControlDecision::EnrollmentRequired);
    }

    #[test]
    fn refuted_enrollment_blocks_node_control() {
        let plan = reconcile_node_control(
            &desired(),
            &observation(SecurityEvidenceState::Refuted, false),
        );
        assert_eq!(plan.status, "blocked");
        assert_eq!(
            plan.decision,
            NodeControlDecision::Blocked {
                violations: vec!["node_enrollment_invalid".to_string()]
            }
        );
    }

    #[test]
    fn release_reconciliation_reuses_the_runtime_seed_plan() {
        let mut observed = observation(SecurityEvidenceState::Proven, false);
        observed.release.postflight = SecurityEvidenceState::Proven;
        let plan = reconcile_node_control(&desired(), &observed);
        assert_eq!(plan.status, "planned");
        assert!(matches!(
            plan.decision,
            NodeControlDecision::ReconcileRelease { .. }
        ));
    }

    #[test]
    fn convergence_requires_release_capabilities_services_and_connection() {
        let mut observed = observation(SecurityEvidenceState::Proven, true);
        let plan = reconcile_node_control(&desired(), &observed);
        assert_eq!(plan.status, "converged");

        observed.connection = NodeControlConnectionState::Disconnected;
        let plan = reconcile_node_control(&desired(), &observed);
        assert_eq!(plan.status, "degraded_disconnected");

        observed.observed_services.clear();
        let plan = reconcile_node_control(&desired(), &observed);
        assert!(matches!(
            plan.decision,
            NodeControlDecision::ReconcileCapabilities { .. }
        ));
    }

    #[test]
    fn provider_fallback_does_not_admit_fleet_membership() {
        let mut observed = observation(SecurityEvidenceState::Proven, true);
        observed.fleet_membership = SecurityEvidenceState::Refuted;
        let plan = reconcile_node_control(&desired(), &observed);
        assert_eq!(plan.status, "blocked");
        assert!(matches!(plan.decision, NodeControlDecision::Blocked { .. }));
    }

    #[test]
    fn reconcile_emits_canonical_compass_vector_epoch() {
        // A converged node emits a CausalConvergenceEpoch whose compass carries
        // the signed desired state digest. Because node_control has no
        // independent post-action read-back yet, the epoch is honestly
        // downgraded to Unknown rather than falsely Complete.
        let observed = observation(SecurityEvidenceState::Proven, true);
        let plan = reconcile_node_control(&desired(), &observed);
        let epoch = plan
            .epoch
            .expect("node_control reconcile must emit a coherence epoch");
        assert_eq!(epoch.cell.subject_id, "node-a");
        assert_eq!(epoch.cell.vertical_id, "node_control");
        assert_eq!(
            epoch.causal_evidence.desired_state.digest,
            desired().policy_digest
        );
        assert_eq!(epoch.cell.status, ConvergenceStatus::Unknown);
        assert!(
            epoch
                .residual_unknowns
                .contains(&"coherence_independent_read_back_missing".to_string())
        );
    }

    #[test]
    fn blocked_decision_produces_blocked_epoch() {
        let plan = reconcile_node_control(
            &desired(),
            &observation(SecurityEvidenceState::Refuted, false),
        );
        let epoch = plan
            .epoch
            .expect("node_control reconcile must emit a coherence epoch");
        assert_eq!(epoch.cell.status, ConvergenceStatus::Blocked);
        assert!(
            epoch
                .blockers
                .contains(&"node_enrollment_invalid".to_string())
        );
    }
}
