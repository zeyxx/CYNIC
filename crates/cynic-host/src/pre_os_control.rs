use cynic_protocol::{PreOsExecutionPlan, PreOsPlatformObservation, SecurityEvidenceState};
use serde::Serialize;

use crate::ports::{
    PreOsArtifactObservationPort, PreOsAttestationObservationPort, PreOsPlatformObservationPort,
};

pub const PRE_OS_CONTROL_PLAN_SCHEMA: &str = "cynic.host.pre_os_control_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreOsArtifactObservation {
    pub harness_id: String,
    pub digest: Option<String>,
    pub integrity: SecurityEvidenceState,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreOsAttestationSinkObservation {
    pub kind: String,
    pub readiness: SecurityEvidenceState,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreOsControlPlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub platform: PreOsPlatformObservation,
    pub artifacts: Vec<PreOsArtifactObservation>,
    pub attestation_sink: PreOsAttestationSinkObservation,
    pub plan: Option<PreOsExecutionPlan>,
    pub blockers: Vec<String>,
}

pub fn plan_pre_os_control(
    subject: &str,
    harness_id: &str,
    nonce: &str,
    observer: &(
         impl PreOsPlatformObservationPort
         + PreOsArtifactObservationPort
         + PreOsAttestationObservationPort
     ),
) -> PreOsControlPlanReport {
    let mut platform = observer.observe_pre_os_platform();
    let artifacts = observer.observe_pre_os_artifacts();
    let attestation_sink = observer.observe_pre_os_attestation_sink();
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact.harness_id == harness_id);

    platform.artifact_discovery = artifact
        .map(|artifact| artifact.integrity)
        .unwrap_or(SecurityEvidenceState::Unknown);
    platform.attestation_sink = attestation_sink.readiness;
    platform
        .evidence_refs
        .extend(attestation_sink.evidence_refs.iter().cloned());
    platform.evidence_refs.sort();
    platform.evidence_refs.dedup();

    let mut blockers = Vec::new();
    if platform.boot_once != SecurityEvidenceState::Proven {
        blockers.push("boot_once_control_unproven".to_string());
    }
    if artifact.is_none() {
        blockers.push(format!("harness_artifact_missing:{harness_id}"));
    }
    if artifact.is_some_and(|artifact| artifact.integrity != SecurityEvidenceState::Proven) {
        blockers.push(format!("harness_artifact_integrity_unproven:{harness_id}"));
    }
    if attestation_sink.readiness != SecurityEvidenceState::Proven {
        blockers.push("attestation_sink_unproven".to_string());
    }
    if nonce.trim().is_empty() {
        blockers.push("nonce_missing".to_string());
    }

    let plan = if blockers.is_empty() {
        artifact.and_then(|artifact| {
            artifact
                .digest
                .as_ref()
                .map(|digest| PreOsExecutionPlan::new(subject, harness_id, digest.clone(), nonce))
        })
    } else {
        None
    };
    if blockers.is_empty() && plan.is_none() {
        blockers.push(format!("harness_artifact_digest_missing:{harness_id}"));
    }

    PreOsControlPlanReport {
        schema: PRE_OS_CONTROL_PLAN_SCHEMA,
        status: if plan.is_some() { "ready" } else { "blocked" },
        mutates: false,
        platform,
        artifacts,
        attestation_sink,
        plan,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        PRE_OS_PLATFORM_OBSERVATION_SCHEMA, PreOsControlKind, SecurityEvidenceState,
    };

    struct Fixture;

    impl PreOsPlatformObservationPort for Fixture {
        fn observe_pre_os_platform(&self) -> PreOsPlatformObservation {
            PreOsPlatformObservation {
                schema: PRE_OS_PLATFORM_OBSERVATION_SCHEMA.to_string(),
                platform: "fixture".to_string(),
                controls: vec![PreOsControlKind::Hypervisor],
                boot_once: SecurityEvidenceState::Proven,
                artifact_discovery: SecurityEvidenceState::Unknown,
                attestation_sink: SecurityEvidenceState::Unknown,
                evidence_refs: vec!["fixture:control".to_string()],
            }
        }
    }

    impl PreOsArtifactObservationPort for Fixture {
        fn observe_pre_os_artifacts(&self) -> Vec<PreOsArtifactObservation> {
            vec![PreOsArtifactObservation {
                harness_id: "memtest86plus".to_string(),
                digest: Some("sha256:fixture".to_string()),
                integrity: SecurityEvidenceState::Proven,
                source: "fixture".to_string(),
            }]
        }
    }

    impl PreOsAttestationObservationPort for Fixture {
        fn observe_pre_os_attestation_sink(&self) -> PreOsAttestationSinkObservation {
            PreOsAttestationSinkObservation {
                kind: "fixture".to_string(),
                readiness: SecurityEvidenceState::Proven,
                evidence_refs: vec!["fixture:sink".to_string()],
            }
        }
    }

    #[test]
    fn complete_capabilities_produce_non_mutating_plan() {
        let report = plan_pre_os_control("cortex", "memtest86plus", "nonce", &Fixture);
        assert_eq!(report.status, "ready");
        assert!(!report.mutates);
        assert!(report.plan.is_some());
    }

    #[test]
    fn missing_harness_is_explicitly_blocked() {
        let report = plan_pre_os_control("cortex", "other", "nonce", &Fixture);
        assert_eq!(report.status, "blocked");
        assert!(report.plan.is_none());
    }
}
