use cynic_protocol::RuntimeProofBundle;
use serde::Serialize;

use super::admission::FrameworkEnrollmentAdmissionReport;

const MAX_PROOF_AGE_MS: u128 = 300_000;
const FUTURE_PROOF_TOLERANCE_MS: u128 = 5_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum FrameworkActivationOperation {
    WriteRoleProjection { role: String },
    ReconcileRuntimeServices,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkRoleActivationPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub machine_id: String,
    pub role: String,
    pub admission_request_id: String,
    pub required_proofs: Vec<&'static str>,
    pub missing_proofs: Vec<String>,
    pub proof_bundle_valid: bool,
    pub operations: Vec<FrameworkActivationOperation>,
    pub rollback_intent: &'static str,
}

pub fn build_framework_role_activation_plan(
    admission: &FrameworkEnrollmentAdmissionReport,
    machine_id: &str,
    proofs: &RuntimeProofBundle,
    now_unix_ms: u128,
) -> FrameworkRoleActivationPlan {
    let required_proofs = required_proofs_for_role(&admission.requested_role);
    let proof_bundle_valid = proofs
        .validate_for_subject(
            machine_id,
            now_unix_ms,
            MAX_PROOF_AGE_MS,
            FUTURE_PROOF_TOLERANCE_MS,
        )
        .is_ok();
    let observed = if proof_bundle_valid {
        proofs.observed_gates()
    } else {
        Vec::new()
    };
    let mut missing_proofs = required_proofs
        .iter()
        .filter(|proof| !observed.iter().any(|gate| gate == **proof))
        .map(|proof| (*proof).to_string())
        .collect::<Vec<_>>();
    if !proof_bundle_valid {
        missing_proofs.push("runtime_proof_bundle_valid".to_string());
    }
    if !admission.activation_permitted {
        missing_proofs.push("verified_enrollment_admission".to_string());
    }
    missing_proofs.sort();
    missing_proofs.dedup();
    let status = if missing_proofs.is_empty() {
        "planned"
    } else {
        "blocked"
    };

    FrameworkRoleActivationPlan {
        schema: "cynic.framework_role_activation.plan.v1",
        status,
        mutates: false,
        machine_id: machine_id.to_string(),
        role: admission.requested_role.clone(),
        admission_request_id: admission.request_id.clone(),
        required_proofs,
        missing_proofs,
        proof_bundle_valid,
        operations: activation_operations(&admission.requested_role),
        rollback_intent: "remove the local role projection and reconcile runtime services to the pre-activation state",
    }
}

fn required_proofs_for_role(role: &str) -> Vec<&'static str> {
    let mut proofs = vec![
        "seed_artifacts_integrity_verified",
        "host_identity_observed",
        "bridge_status_observed",
    ];
    if role == "kernel" {
        proofs.push("kernel_runtime_authority_ready");
    }
    proofs
}

fn activation_operations(role: &str) -> Vec<FrameworkActivationOperation> {
    vec![
        FrameworkActivationOperation::WriteRoleProjection {
            role: role.to_string(),
        },
        FrameworkActivationOperation::ReconcileRuntimeServices,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofEvidence,
    };

    fn admitted_kernel() -> FrameworkEnrollmentAdmissionReport {
        FrameworkEnrollmentAdmissionReport {
            schema: "cynic.framework_enrollment.admission.v1",
            status: "admitted",
            mutates: false,
            request_id: "request-42".to_string(),
            grant_id: "grant-42".to_string(),
            authority_id: "runtime-authority".to_string(),
            requested_role: "kernel".to_string(),
            proof_id: Some("proof-42".to_string()),
            activation_permitted: true,
            blockers: Vec::new(),
            rollback_intent: "invalidate",
        }
    }

    fn proofs(gates: &[&str]) -> RuntimeProofBundle {
        RuntimeProofBundle {
            schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
            status: "satisfied".to_string(),
            proofs: gates
                .iter()
                .map(|gate| RuntimeProof {
                    schema: RUNTIME_PROOF_SCHEMA.to_string(),
                    gate: (*gate).to_string(),
                    status: "satisfied".to_string(),
                    producer: "cynic-host".to_string(),
                    subject: "pve-vm-103".to_string(),
                    observed_at_unix_ms: 100_000,
                    evidence: vec![RuntimeProofEvidence {
                        id: "evidence".to_string(),
                        status: "satisfied".to_string(),
                        detail: "observed".to_string(),
                    }],
                })
                .collect(),
        }
    }

    #[test]
    fn kernel_activation_is_planned_only_with_admission_and_all_required_proofs() {
        let plan = build_framework_role_activation_plan(
            &admitted_kernel(),
            "pve-vm-103",
            &proofs(&[
                "seed_artifacts_integrity_verified",
                "host_identity_observed",
                "bridge_status_observed",
                "kernel_runtime_authority_ready",
            ]),
            100_001,
        );

        assert_eq!(plan.status, "planned");
        assert!(!plan.mutates);
        assert_eq!(plan.operations.len(), 2);
    }

    #[test]
    fn activation_is_blocked_when_the_authority_proof_is_missing() {
        let plan = build_framework_role_activation_plan(
            &admitted_kernel(),
            "pve-vm-103",
            &proofs(&[
                "seed_artifacts_integrity_verified",
                "host_identity_observed",
                "bridge_status_observed",
            ]),
            100_001,
        );

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.missing_proofs
                .contains(&"kernel_runtime_authority_ready".to_string())
        );
    }

    #[test]
    fn activation_rejects_proofs_for_another_machine() {
        let plan = build_framework_role_activation_plan(
            &admitted_kernel(),
            "pve-vm-104",
            &proofs(&[
                "seed_artifacts_integrity_verified",
                "host_identity_observed",
                "bridge_status_observed",
                "kernel_runtime_authority_ready",
            ]),
            100_001,
        );

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.missing_proofs
                .contains(&"runtime_proof_bundle_valid".to_string())
        );
    }
}
