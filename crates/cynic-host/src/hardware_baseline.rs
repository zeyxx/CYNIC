use std::path::Path;

use cynic_protocol::{
    MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, PolicyViolation, RequiredProof,
    SecurityEvidenceState, digest_json,
};
use serde::Serialize;

use crate::hardware_conformance::{
    HARDWARE_BASELINE_SCHEMA, HardwareBaseline, HardwareConformanceReport,
};

pub const HARDWARE_BASELINE_PLAN_SCHEMA: &str = "cynic.host.hardware_baseline_plan.v1";
const HARDWARE_BASELINE_POLICY_ID: &str = "cynic.host.hardware_baseline.enrollment";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareBaselinePlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub target_path: String,
    pub candidate: HardwareBaseline,
    pub identity_state: SecurityEvidenceState,
    pub mutation: MutationContract,
    pub admission: PolicyAdmissionReport,
    pub blockers: Vec<String>,
}

pub fn plan_hardware_baseline(
    subject: &str,
    actor: &str,
    target_path: &Path,
    report: &HardwareConformanceReport,
) -> HardwareBaselinePlan {
    let candidate = HardwareBaseline {
        schema: HARDWARE_BASELINE_SCHEMA.to_string(),
        subject: subject.to_string(),
        inventory_digest: report
            .inventory_digest
            .clone()
            .unwrap_or_else(|| "unavailable".to_string()),
    };
    let target_path = target_path.display().to_string();
    let enrolling = report.identity_state == SecurityEvidenceState::Unknown;
    let mutation = MutationContract {
        kind: "hardware_baseline_enrollment".to_string(),
        scope: PolicyScope {
            domain: "hardware_identity".to_string(),
            target: subject.to_string(),
        },
        mutates: enrolling,
        requires_privilege: enrolling,
        requires_explicit_apply: enrolling,
        required_authority: if enrolling {
            PolicyAuthority::HumanAdmin
        } else {
            PolicyAuthority::ReadOnly
        },
        preconditions: if enrolling {
            vec![
                "hardware_inventory_complete".to_string(),
                "hardware_baseline_absent".to_string(),
                "operator_confirms_physical_machine_identity".to_string(),
            ]
        } else {
            vec!["hardware_baseline_matches_inventory".to_string()]
        },
        rollback_intent: if enrolling {
            "remove only the newly enrolled baseline and return hardware identity to unknown"
        } else {
            "no mutation to roll back"
        }
        .to_string(),
        expected_proofs: vec![RequiredProof {
            gate: "hardware_identity_stable".to_string(),
            subject: subject.to_string(),
        }],
    };
    let mut blockers = Vec::new();
    if report.inventory_digest.is_none() {
        blockers.push("hardware_inventory_digest_unavailable".to_string());
    }
    if report.identity_state == SecurityEvidenceState::Refuted {
        blockers.push("hardware_baseline_drift_requires_separate_reenrollment".to_string());
    }

    let decision = if blockers.is_empty() {
        PolicyDecision::Allow {
            required_authority: mutation.required_authority,
            required_proofs: mutation.expected_proofs.clone(),
        }
    } else {
        PolicyDecision::Deny {
            violations: blockers
                .iter()
                .map(|blocker| PolicyViolation {
                    policy_id: HARDWARE_BASELINE_POLICY_ID.to_string(),
                    code: blocker.clone(),
                    message: "hardware baseline enrollment is fail-closed".to_string(),
                })
                .collect(),
        }
    };
    let candidate_digest = digest_json(&candidate).unwrap_or_else(|_| "unavailable".to_string());
    let admission = PolicyAdmissionReport {
        schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
        policy_set: PolicySetRef {
            id: HARDWARE_BASELINE_POLICY_ID.to_string(),
            version: "v1".to_string(),
            digest: digest_json(&(HARDWARE_BASELINE_POLICY_ID, "v1"))
                .unwrap_or_else(|_| "unavailable".to_string()),
        },
        intent: PolicyIntentRef {
            id: format!("hardware-baseline-enroll:{subject}"),
            kind: mutation.kind.clone(),
            actor: actor.to_string(),
            subject: subject.to_string(),
            digest: candidate_digest.clone(),
            plan_digest: candidate_digest,
        },
        mutation: mutation.clone(),
        decision,
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if report.identity_state == SecurityEvidenceState::Proven {
        "converged"
    } else {
        "ready"
    };

    HardwareBaselinePlan {
        schema: HARDWARE_BASELINE_PLAN_SCHEMA,
        status,
        mutates: mutation.mutates,
        target_path,
        candidate,
        identity_state: report.identity_state,
        mutation,
        admission,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_conformance::{
        EvidenceState, HardwareDevice, HardwareInventory, HardwareObservation, assess_hardware,
    };

    fn report(baseline_digest: Option<String>) -> HardwareConformanceReport {
        assess_hardware(HardwareObservation {
            inventory: HardwareInventory {
                board_name: Some("fixture".into()),
                cpu_model: Some("fixture".into()),
                devices: vec![HardwareDevice {
                    bus: "fixture".into(),
                    address: "0".into(),
                    class: None,
                    vendor_id: None,
                    device_id: None,
                    model: None,
                    firmware: None,
                    driver: None,
                }],
                ..Default::default()
            },
            baseline_digest,
            firmware_version: Some("1".into()),
            max_temperature_millicelsius: Some(40_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Satisfied,
            memory_tuning: EvidenceState::Satisfied,
            runtime_tuning_overrides: Vec::new(),
        })
    }

    #[test]
    fn absent_baseline_produces_explicit_human_admin_enrollment() {
        let plan = plan_hardware_baseline(
            "cortex",
            "operator",
            Path::new("/injected/hardware-baseline.json"),
            &report(None),
        );
        assert_eq!(plan.status, "ready");
        assert!(plan.mutates);
        assert!(plan.admission.admitted());
        assert_eq!(
            plan.mutation.required_authority,
            PolicyAuthority::HumanAdmin
        );
    }

    #[test]
    fn drift_cannot_overwrite_baseline_through_initial_enrollment() {
        let plan = plan_hardware_baseline(
            "cortex",
            "operator",
            Path::new("/injected/hardware-baseline.json"),
            &report(Some(format!("sha256:{}", "0".repeat(64)))),
        );
        assert_eq!(plan.status, "blocked");
        assert!(!plan.admission.admitted());
    }
}
