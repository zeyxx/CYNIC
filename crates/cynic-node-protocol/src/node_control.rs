use serde::{Deserialize, Serialize};

use cynic_protocol::{
    RuntimeSeedObservation, RuntimeSeedPlan, RuntimeSeedRequest, SecurityEvidenceState,
    reconcile_runtime_seed, validate_runtime_seed_request,
};

pub const NODE_DESIRED_STATE_SCHEMA: &str = "cynic.node_desired_state.v1";
pub const NODE_CONTROL_OBSERVATION_SCHEMA: &str = "cynic.node_control_observation.v1";
/// Scheduler runtime capabilities a node must attest at enrollment.
/// Wire-contract vocabulary shared by node enrollment, the build scheduler,
/// and host bootstrap; owned here so no build-scheduler dependency leaks
/// into node identity.
pub const BUILD_SCHEDULER_RUNTIME_CAPABILITIES: &[&str] = &[
    "kernel_to_node_workload_delivery_read_back",
    "scheduler_claim_renewal",
    "scheduler_cancellation_delivery",
    "scheduler_crash_recovery",
    "node_owned_build_grant_machine_signer",
    "build_runner_signature_and_live_workload_verifier",
    "scheduler_to_build_runner_execution_transport",
    "build_completion_and_artifact_read_back",
    "node_workspace_observer_wiring_and_proof",
];

pub const NODE_CONTROL_PLAN_SCHEMA: &str = "cynic.node_control_plan.v2";
pub const SIGNED_NODE_CONTROL_PLAN_SCHEMA: &str = "cynic.signed_node_control_plan.v1";
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

/// A cryptographically signed control plan from the Kernel authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedNodeControlPlan {
    pub schema: String,
    pub desired_state: NodeDesiredState,
    pub kernel_authority_id: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub signature_hex: String,
}

#[derive(Serialize)]
struct SignedNodeControlPlanSigningPayload<'a> {
    schema: &'a str,
    desired_state: &'a NodeDesiredState,
    kernel_authority_id: &'a str,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
}

impl SignedNodeControlPlan {
    pub fn new(
        desired_state: NodeDesiredState,
        kernel_authority_id: String,
        issued_at_unix_s: u64,
        expires_at_unix_s: u64,
        signature_hex: String,
    ) -> Self {
        Self {
            schema: SIGNED_NODE_CONTROL_PLAN_SCHEMA.to_string(),
            desired_state,
            kernel_authority_id,
            issued_at_unix_s,
            expires_at_unix_s,
            signature_hex,
        }
    }

    /// Canonical bytes signed by the Kernel authority.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = SignedNodeControlPlanSigningPayload {
            schema: &self.schema,
            desired_state: &self.desired_state,
            kernel_authority_id: &self.kernel_authority_id,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
        };
        cynic_protocol::canonical_json_bytes(&payload).expect("canonicalize signed control plan payload")
    }

    pub fn is_valid_at(&self, now_unix_s: u64) -> bool {
        self.issued_at_unix_s <= now_unix_s && now_unix_s <= self.expires_at_unix_s
    }

    /// Pure structural validation of the signed control plan.
    pub fn validate(&self, now_unix_s: u64) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != SIGNED_NODE_CONTROL_PLAN_SCHEMA {
            blockers.push("signed_node_control_plan_schema_mismatch".to_string());
        }
        if self.kernel_authority_id.trim().is_empty() {
            blockers.push("signed_node_control_plan_kernel_authority_id_empty".to_string());
        }
        if self.expires_at_unix_s <= self.issued_at_unix_s {
            blockers.push("signed_node_control_plan_expiry_invalid".to_string());
        }
        if !self.is_valid_at(now_unix_s) {
            blockers.push("signed_node_control_plan_expired_or_not_yet_valid".to_string());
        }
        if self.signature_hex.len() != 128
            || !self.signature_hex.bytes().all(|b| b.is_ascii_hexdigit())
        {
            blockers.push("signed_node_control_plan_signature_invalid".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// Verification status for a SignedNodeControlPlan before application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPlanVerificationStatus {
    Verified,
    StaleGeneration {
        observed: u64,
        plan: u64,
    },
    Expired {
        expires_at_unix_s: u64,
        now_unix_s: u64,
    },
    UntrustedAuthority(String),
    InvalidSignature,
    StructuralErrors(Vec<String>),
}

/// Pure rule: check preconditions before applying a control plan.
pub fn verify_control_plan_preconditions(
    plan: &SignedNodeControlPlan,
    observed_generation: Option<u64>,
    authorized_kernel_ids: &[String],
    now_unix_s: u64,
) -> Result<(), ControlPlanVerificationStatus> {
    let blockers = plan.validate(now_unix_s);
    if !blockers.is_empty() {
        return Err(ControlPlanVerificationStatus::StructuralErrors(blockers));
    }

    if !authorized_kernel_ids.contains(&plan.kernel_authority_id) {
        return Err(ControlPlanVerificationStatus::UntrustedAuthority(
            plan.kernel_authority_id.clone(),
        ));
    }

    if let Some(obs_gen) = observed_generation {
        if plan.desired_state.generation <= obs_gen {
            return Err(ControlPlanVerificationStatus::StaleGeneration {
                observed: obs_gen,
                plan: plan.desired_state.generation,
            });
        }
    }

    Ok(())
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
        desired.release.schema == cynic_protocol::RUNTIME_SEED_REQUEST_SCHEMA,
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
        decision,
        execution: None,
        evidence_refs: observed.evidence_refs.clone(),
        unknowns,
    }
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
            schema: cynic_protocol::RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
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
            schema: cynic_protocol::RUNTIME_SEED_OBSERVATION_SCHEMA.to_string(),
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
    fn signed_control_plan_preconditions_verified() {
        let desired = desired();
        let signed = SignedNodeControlPlan::new(
            desired,
            "kernel-alpha".to_string(),
            1000,
            2000,
            "a".repeat(128),
        );

        let auth_kernels = vec!["kernel-alpha".to_string()];
        assert!(verify_control_plan_preconditions(&signed, Some(0), &auth_kernels, 1500).is_ok());

        // Stale generation rejected: applying generation N requires the node's
        // observed generation to be strictly below N (idempotence guard).
        let stale = verify_control_plan_preconditions(&signed, Some(2), &auth_kernels, 1500);
        assert!(matches!(
            stale,
            Err(ControlPlanVerificationStatus::StaleGeneration {
                observed: 2,
                plan: 2
            })
        ));

        // Untrusted authority rejected
        let untrusted =
            verify_control_plan_preconditions(&signed, Some(0), &["kernel-beta".to_string()], 1500);
        assert!(matches!(
            untrusted,
            Err(ControlPlanVerificationStatus::UntrustedAuthority(_))
        ));

        // Expired plan rejected
        let expired = verify_control_plan_preconditions(&signed, Some(0), &auth_kernels, 2500);
        assert!(matches!(
            expired,
            Err(ControlPlanVerificationStatus::StructuralErrors(_))
        ));
    }

    #[test]
    fn signed_control_plan_signing_bytes_are_deterministic_and_canonical() {
        let desired = desired();
        let signed = SignedNodeControlPlan::new(
            desired,
            "kernel-alpha".to_string(),
            1000,
            2000,
            "a".repeat(128),
        );
        let bytes = signed.signing_bytes();
        let text = String::from_utf8(bytes.clone()).expect("valid utf-8 json");
        // Ensure top-level structure has canonical key ordering and matches expected suffix
        assert!(text.starts_with("{\"desired_state\":{"));
        assert!(text.ends_with(",\"expires_at_unix_s\":2000,\"issued_at_unix_s\":1000,\"kernel_authority_id\":\"kernel-alpha\",\"schema\":\"cynic.signed_node_control_plan.v1\"}"));
    }
}
