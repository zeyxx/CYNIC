use cynic_protocol::{
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, PolicyAuthority, digest_json,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::hardware_baseline::HardwareBaselinePlan;
use crate::hardware_conformance::HardwareBaseline;

pub const HARDWARE_BASELINE_BUNDLE_SCHEMA: &str = "cynic.host.hardware_baseline_bundle.v1";
pub const HARDWARE_BASELINE_APPLY_SCHEMA: &str = "cynic.host.hardware_baseline_apply.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareBaselineBundle {
    pub schema: String,
    pub target_path: String,
    pub candidate: HardwareBaseline,
    pub candidate_digest: String,
    pub payload: Vec<u8>,
    pub admission: cynic_protocol::PolicyAdmissionReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareBaselineInstallSpec {
    pub target_path: String,
    pub payload: Vec<u8>,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareBaselineInstallProof {
    pub target_path: String,
    pub payload_digest: String,
    pub created: bool,
}

pub trait HardwareBaselineInstallPort {
    fn install_absent_only(
        &self,
        spec: &HardwareBaselineInstallSpec,
    ) -> Result<HardwareBaselineInstallProof, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareBaselineBundleProof {
    pub path: String,
    pub bundle_digest: String,
    pub created: bool,
}

pub trait HardwareBaselineBundleStorePort {
    fn persist_absent_or_matching(
        &self,
        path: &Path,
        bundle: &HardwareBaselineBundle,
    ) -> Result<HardwareBaselineBundleProof, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareBaselineApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub candidate_digest: String,
    pub proof: Option<HardwareBaselineInstallProof>,
    pub blockers: Vec<String>,
}

pub fn build_hardware_baseline_bundle(
    plan: &HardwareBaselinePlan,
) -> Result<HardwareBaselineBundle, Vec<String>> {
    let mut blockers = Vec::new();
    if plan.status != "ready" || !plan.mutates {
        blockers.push("hardware_baseline_plan_not_ready_for_enrollment".to_string());
    }
    if !plan.admission.admitted() {
        blockers.push("hardware_baseline_admission_not_allow".to_string());
    }
    let candidate_digest = digest_json(&plan.candidate).map_err(|error| vec![error.to_string()])?;
    if plan.admission.intent.digest != candidate_digest
        || plan.admission.intent.plan_digest != candidate_digest
    {
        blockers.push("hardware_baseline_candidate_admission_mismatch".to_string());
    }
    if !blockers.is_empty() {
        return Err(blockers);
    }
    let mut payload =
        serde_json::to_vec_pretty(&plan.candidate).map_err(|error| vec![error.to_string()])?;
    payload.push(b'\n');
    Ok(HardwareBaselineBundle {
        schema: HARDWARE_BASELINE_BUNDLE_SCHEMA.to_string(),
        target_path: plan.target_path.clone(),
        candidate: plan.candidate.clone(),
        candidate_digest,
        payload,
        admission: plan.admission.clone(),
    })
}

pub fn apply_hardware_baseline(
    bundle: &HardwareBaselineBundle,
    authorization: &ExecutionAuthorization,
    installer: &impl HardwareBaselineInstallPort,
) -> HardwareBaselineApplyReport {
    let mut blockers = Vec::new();
    if authorization.schema != EXECUTION_AUTHORIZATION_SCHEMA {
        blockers.push("authorization_schema_invalid".to_string());
    }
    if authorization.actor != bundle.admission.intent.actor {
        blockers.push("authorization_actor_mismatch".to_string());
    }
    if authorization.authority != PolicyAuthority::HumanAdmin {
        blockers.push("human_admin_authority_required".to_string());
    }
    if !authorization.explicit_apply {
        blockers.push("explicit_apply_required".to_string());
    }
    if digest_json(&bundle.candidate).ok().as_deref() != Some(bundle.candidate_digest.as_str()) {
        blockers.push("bundle_candidate_digest_mismatch".to_string());
    }
    if !blockers.is_empty() {
        return blocked(bundle, blockers);
    }

    let spec = HardwareBaselineInstallSpec {
        target_path: bundle.target_path.clone(),
        payload: bundle.payload.clone(),
        mode: 0o644,
    };
    match installer.install_absent_only(&spec) {
        Ok(proof) if proof.created && proof.payload_digest == bundle.candidate_digest => {
            HardwareBaselineApplyReport {
                schema: HARDWARE_BASELINE_APPLY_SCHEMA,
                status: "applied_pending_reobservation",
                mutates: true,
                candidate_digest: bundle.candidate_digest.clone(),
                proof: Some(proof),
                blockers: Vec::new(),
            }
        }
        Ok(_) => blocked(bundle, vec!["install_proof_mismatch".to_string()]),
        Err(error) => blocked(bundle, vec![error]),
    }
}

fn blocked(bundle: &HardwareBaselineBundle, blockers: Vec<String>) -> HardwareBaselineApplyReport {
    HardwareBaselineApplyReport {
        schema: HARDWARE_BASELINE_APPLY_SCHEMA,
        status: "blocked",
        mutates: false,
        candidate_digest: bundle.candidate_digest.clone(),
        proof: None,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_baseline::plan_hardware_baseline;
    use crate::hardware_conformance::{
        EvidenceState, HardwareDevice, HardwareInventory, HardwareObservation, assess_hardware,
    };

    struct Installer;

    impl HardwareBaselineInstallPort for Installer {
        fn install_absent_only(
            &self,
            spec: &HardwareBaselineInstallSpec,
        ) -> Result<HardwareBaselineInstallProof, String> {
            let baseline: HardwareBaseline =
                serde_json::from_slice(&spec.payload).map_err(|error| error.to_string())?;
            Ok(HardwareBaselineInstallProof {
                target_path: spec.target_path.clone(),
                payload_digest: digest_json(&baseline).map_err(|error| error.to_string())?,
                created: true,
            })
        }
    }

    fn bundle() -> HardwareBaselineBundle {
        let report = assess_hardware(HardwareObservation {
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
            baseline_digest: None,
            firmware_version: Some("1".into()),
            max_temperature_millicelsius: Some(40_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Unknown,
            memory_tuning: EvidenceState::Unknown,
            runtime_tuning_overrides: Vec::new(),
        });
        let plan = plan_hardware_baseline(
            "cortex",
            "operator",
            std::path::Path::new("/injected/hardware-baseline.json"),
            &report,
        );
        build_hardware_baseline_bundle(&plan).expect("bundle")
    }

    #[test]
    fn apply_requires_explicit_human_admin_authorization() {
        let report = apply_hardware_baseline(
            &bundle(),
            &ExecutionAuthorization::new(
                "operator",
                PolicyAuthority::HumanAdmin,
                false,
                Vec::new(),
            ),
            &Installer,
        );
        assert_eq!(report.status, "blocked");
    }

    #[test]
    fn admitted_bundle_returns_pending_reobservation() {
        let report = apply_hardware_baseline(
            &bundle(),
            &ExecutionAuthorization::new("operator", PolicyAuthority::HumanAdmin, true, Vec::new()),
            &Installer,
        );
        assert_eq!(report.status, "applied_pending_reobservation");
    }
}
