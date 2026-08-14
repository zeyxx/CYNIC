use serde::{Deserialize, Serialize};

use crate::{
    MutationContract, PolicyAuthority, PolicyScope, RequiredProof, SecurityEvidenceState,
    digest_json,
};

pub const PRE_OS_EXECUTION_PLAN_SCHEMA: &str = "cynic.pre_os.execution_plan.v1";
pub const PRE_OS_ATTESTATION_SCHEMA: &str = "cynic.pre_os.attestation.v1";
pub const PRE_OS_VERIFICATION_SCHEMA: &str = "cynic.pre_os.verification.v1";
pub const PRE_OS_PLATFORM_OBSERVATION_SCHEMA: &str = "cynic.pre_os.platform_observation.v1";
pub const PRE_OS_ARTIFACT_MANIFEST_SCHEMA: &str = "cynic.pre_os.artifact_manifest.v1";
pub const PRE_OS_MAINTENANCE_LEASE_SCHEMA: &str = "cynic.pre_os.maintenance_lease.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsArtifactManifest {
    pub schema: String,
    pub artifacts: Vec<PreOsArtifactSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsArtifactSpec {
    pub harness_id: String,
    pub architecture: String,
    pub format: String,
    pub source_ref: String,
    pub digest: String,
}

impl PreOsArtifactManifest {
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != PRE_OS_ARTIFACT_MANIFEST_SCHEMA {
            blockers.push("artifact_manifest_schema_mismatch".to_string());
        }
        let mut ids = std::collections::BTreeSet::new();
        for artifact in &self.artifacts {
            if artifact.harness_id.trim().is_empty()
                || artifact.architecture.trim().is_empty()
                || artifact.format.trim().is_empty()
                || artifact.source_ref.trim().is_empty()
            {
                blockers.push("artifact_identity_incomplete".to_string());
            }
            if !artifact.digest.starts_with("sha256:") || artifact.digest.len() != 71 {
                blockers.push(format!("artifact_digest_invalid:{}", artifact.harness_id));
            }
            if !ids.insert(artifact.harness_id.as_str()) {
                blockers.push(format!(
                    "artifact_harness_duplicate:{}",
                    artifact.harness_id
                ));
            }
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreOsControlKind {
    UefiVariables,
    Redfish,
    Hypervisor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsPlatformObservation {
    pub schema: String,
    pub platform: String,
    pub controls: Vec<PreOsControlKind>,
    pub boot_once: SecurityEvidenceState,
    pub artifact_discovery: SecurityEvidenceState,
    pub attestation_sink: SecurityEvidenceState,
    pub evidence_refs: Vec<String>,
}

impl PreOsPlatformObservation {
    pub fn supports_controlled_cycle(&self) -> bool {
        !self.controls.is_empty()
            && self.boot_once == SecurityEvidenceState::Proven
            && self.artifact_discovery == SecurityEvidenceState::Proven
            && self.attestation_sink == SecurityEvidenceState::Proven
            && !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreOsPhase {
    Preflight,
    DrainWorkloads,
    AcquireMaintenanceLease,
    ArmBootOnce,
    Reboot,
    ExecuteHarness,
    PersistAttestation,
    RestoreBoot,
    ConfirmRuntimeReturn,
    ReleaseMaintenanceLease,
    Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreOsFailureDisposition {
    Quarantine,
    ReturnToService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsMaintenancePolicy {
    pub max_parallel_nodes: u32,
    pub minimum_healthy_nodes: u32,
    pub lease_timeout_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub runtime_return_timeout_ms: u64,
    pub failure_disposition: PreOsFailureDisposition,
}

impl Default for PreOsMaintenancePolicy {
    fn default() -> Self {
        Self {
            max_parallel_nodes: 1,
            minimum_healthy_nodes: 1,
            lease_timeout_ms: 8 * 60 * 60 * 1_000,
            heartbeat_timeout_ms: 5 * 60 * 1_000,
            runtime_return_timeout_ms: 15 * 60 * 1_000,
            failure_disposition: PreOsFailureDisposition::Quarantine,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsMaintenanceLease {
    pub schema: String,
    pub lease_id: String,
    pub subject: String,
    pub cohort: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub active_maintenance_nodes: u32,
    pub healthy_nodes_before_drain: u32,
    pub workloads_drained: bool,
    pub normal_boot_ref: String,
    pub control_plane_ack: String,
}

pub fn validate_pre_os_maintenance_lease(
    lease: &PreOsMaintenanceLease,
    policy: &PreOsMaintenancePolicy,
    now_unix_ms: u64,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if lease.schema != PRE_OS_MAINTENANCE_LEASE_SCHEMA {
        blockers.push("maintenance_lease_schema_mismatch".to_string());
    }
    if lease.lease_id.trim().is_empty() || lease.subject.trim().is_empty() {
        blockers.push("maintenance_lease_identity_incomplete".to_string());
    }
    if lease.issued_at_unix_ms == 0
        || lease.expires_at_unix_ms <= lease.issued_at_unix_ms
        || now_unix_ms >= lease.expires_at_unix_ms
        || lease
            .expires_at_unix_ms
            .saturating_sub(lease.issued_at_unix_ms)
            > policy.lease_timeout_ms
    {
        blockers.push("maintenance_lease_window_invalid".to_string());
    }
    if !lease.workloads_drained {
        blockers.push("workloads_not_drained".to_string());
    }
    if lease.active_maintenance_nodes >= policy.max_parallel_nodes {
        blockers.push("maintenance_parallelism_budget_exhausted".to_string());
    }
    if lease.healthy_nodes_before_drain <= policy.minimum_healthy_nodes {
        blockers.push("minimum_healthy_node_budget_would_be_violated".to_string());
    }
    if lease.normal_boot_ref.trim().is_empty() {
        blockers.push("normal_boot_fallback_unrecorded".to_string());
    }
    if lease.control_plane_ack.trim().is_empty() {
        blockers.push("control_plane_ack_missing".to_string());
    }
    blockers
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsExecutionPlan {
    pub schema: String,
    pub subject: String,
    pub harness_id: String,
    pub artifact_digest: String,
    pub nonce: String,
    pub phases: Vec<PreOsPhase>,
    pub mutation: MutationContract,
}

impl PreOsExecutionPlan {
    pub fn new(
        subject: impl Into<String>,
        harness_id: impl Into<String>,
        artifact_digest: impl Into<String>,
        nonce: impl Into<String>,
    ) -> Self {
        let subject = subject.into();
        Self {
            schema: PRE_OS_EXECUTION_PLAN_SCHEMA.to_string(),
            harness_id: harness_id.into(),
            artifact_digest: artifact_digest.into(),
            nonce: nonce.into(),
            phases: vec![
                PreOsPhase::Preflight,
                PreOsPhase::DrainWorkloads,
                PreOsPhase::AcquireMaintenanceLease,
                PreOsPhase::ArmBootOnce,
                PreOsPhase::Reboot,
                PreOsPhase::ExecuteHarness,
                PreOsPhase::PersistAttestation,
                PreOsPhase::RestoreBoot,
                PreOsPhase::ConfirmRuntimeReturn,
                PreOsPhase::ReleaseMaintenanceLease,
                PreOsPhase::Verify,
            ],
            mutation: MutationContract {
                kind: "pre_os_boot_once".to_string(),
                scope: PolicyScope {
                    domain: "firmware_boot".to_string(),
                    target: subject.clone(),
                },
                mutates: true,
                requires_privilege: true,
                requires_explicit_apply: true,
                required_authority: PolicyAuthority::HumanAdmin,
                preconditions: vec![
                    "artifact_digest_verified".to_string(),
                    "boot_once_supported".to_string(),
                    "attestation_sink_writable".to_string(),
                    "normal_boot_fallback_observed".to_string(),
                    "workloads_drained".to_string(),
                    "maintenance_lease_valid".to_string(),
                    "fleet_availability_budget_satisfied".to_string(),
                    "runtime_return_watchdog_armed".to_string(),
                ],
                rollback_intent: "clear the one-time boot target and restore the previously observed normal boot entry".to_string(),
                expected_proofs: vec![
                    RequiredProof {
                        gate: "pre_os_harness_completed".to_string(),
                        subject: subject.clone(),
                    },
                    RequiredProof {
                        gate: "normal_boot_restored".to_string(),
                        subject: subject.clone(),
                    },
                    RequiredProof {
                        gate: "runtime_return_confirmed".to_string(),
                        subject: subject.clone(),
                    },
                ],
            },
            subject,
        }
    }

    pub fn digest(&self) -> Result<String, serde_json::Error> {
        digest_json(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreOsOutcome {
    Passed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsAttestation {
    pub schema: String,
    pub plan_digest: String,
    pub subject: String,
    pub harness_id: String,
    pub artifact_digest: String,
    pub nonce: String,
    pub outcome: PreOsOutcome,
    pub pass_count: u32,
    pub error_count: u64,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsVerification {
    pub schema: String,
    pub status: SecurityEvidenceState,
    pub checks: Vec<PreOsVerificationCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreOsVerificationCheck {
    pub id: String,
    pub status: SecurityEvidenceState,
}

pub fn verify_pre_os_attestation(
    plan: &PreOsExecutionPlan,
    attestation: &PreOsAttestation,
) -> PreOsVerification {
    let expected_digest = plan.digest().ok();
    let checks = vec![
        check("schema", attestation.schema == PRE_OS_ATTESTATION_SCHEMA),
        check(
            "plan_digest",
            expected_digest.as_deref() == Some(attestation.plan_digest.as_str()),
        ),
        check("subject", attestation.subject == plan.subject),
        check("harness_id", attestation.harness_id == plan.harness_id),
        check(
            "artifact_digest",
            attestation.artifact_digest == plan.artifact_digest,
        ),
        check("nonce", attestation.nonce == plan.nonce),
        check(
            "time_order",
            attestation.started_at_unix_ms > 0
                && attestation.completed_at_unix_ms >= attestation.started_at_unix_ms,
        ),
        check(
            "successful_result",
            attestation.outcome == PreOsOutcome::Passed
                && attestation.pass_count > 0
                && attestation.error_count == 0,
        ),
    ];
    let status = if checks
        .iter()
        .all(|check| check.status == SecurityEvidenceState::Proven)
    {
        SecurityEvidenceState::Proven
    } else {
        SecurityEvidenceState::Refuted
    };
    PreOsVerification {
        schema: PRE_OS_VERIFICATION_SCHEMA.to_string(),
        status,
        checks,
    }
}

fn check(id: &str, satisfied: bool) -> PreOsVerificationCheck {
    PreOsVerificationCheck {
        id: id.to_string(),
        status: if satisfied {
            SecurityEvidenceState::Proven
        } else {
            SecurityEvidenceState::Refuted
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PreOsExecutionPlan, PreOsAttestation) {
        let plan = PreOsExecutionPlan::new("cortex", "memtest86plus", "sha256:artifact", "nonce-1");
        let attestation = PreOsAttestation {
            schema: PRE_OS_ATTESTATION_SCHEMA.to_string(),
            plan_digest: plan.digest().unwrap(),
            subject: "cortex".to_string(),
            harness_id: "memtest86plus".to_string(),
            artifact_digest: "sha256:artifact".to_string(),
            nonce: "nonce-1".to_string(),
            outcome: PreOsOutcome::Passed,
            pass_count: 4,
            error_count: 0,
            started_at_unix_ms: 1,
            completed_at_unix_ms: 2,
        };
        (plan, attestation)
    }

    #[test]
    fn matching_successful_attestation_is_proven() {
        let (plan, attestation) = fixture();
        assert_eq!(
            verify_pre_os_attestation(&plan, &attestation).status,
            SecurityEvidenceState::Proven
        );
    }

    #[test]
    fn nonce_mismatch_refutes_attestation() {
        let (plan, mut attestation) = fixture();
        attestation.nonce = "replayed".to_string();
        assert_eq!(
            verify_pre_os_attestation(&plan, &attestation).status,
            SecurityEvidenceState::Refuted
        );
    }

    #[test]
    fn platform_requires_complete_evidence_before_controlled_cycle() {
        let observation = PreOsPlatformObservation {
            schema: PRE_OS_PLATFORM_OBSERVATION_SCHEMA.to_string(),
            platform: "linux-uefi".to_string(),
            controls: vec![PreOsControlKind::UefiVariables],
            boot_once: SecurityEvidenceState::Proven,
            artifact_discovery: SecurityEvidenceState::Unknown,
            attestation_sink: SecurityEvidenceState::Proven,
            evidence_refs: vec!["sysfs:efi".to_string()],
        };
        assert!(!observation.supports_controlled_cycle());
    }

    #[test]
    fn artifact_manifest_rejects_duplicate_harnesses() {
        let artifact = PreOsArtifactSpec {
            harness_id: "memtest86plus".to_string(),
            architecture: "x86_64".to_string(),
            format: "efi".to_string(),
            source_ref: "file:/injected/memtest.efi".to_string(),
            digest: format!("sha256:{}", "a".repeat(64)),
        };
        let manifest = PreOsArtifactManifest {
            schema: PRE_OS_ARTIFACT_MANIFEST_SCHEMA.to_string(),
            artifacts: vec![artifact.clone(), artifact],
        };
        assert_eq!(manifest.validate().len(), 1);
    }

    #[test]
    fn maintenance_lease_preserves_fleet_availability() {
        let policy = PreOsMaintenancePolicy {
            max_parallel_nodes: 10,
            minimum_healthy_nodes: 900,
            ..Default::default()
        };
        let lease = PreOsMaintenanceLease {
            schema: PRE_OS_MAINTENANCE_LEASE_SCHEMA.to_string(),
            lease_id: "lease-1".to_string(),
            subject: "node-42".to_string(),
            cohort: "rack-2".to_string(),
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
            active_maintenance_nodes: 3,
            healthy_nodes_before_drain: 997,
            workloads_drained: true,
            normal_boot_ref: "uefi:Boot0002".to_string(),
            control_plane_ack: "scheduler:drained:node-42".to_string(),
        };
        assert!(validate_pre_os_maintenance_lease(&lease, &policy, 1_500).is_empty());
    }

    #[test]
    fn maintenance_lease_blocks_undrained_or_over_budget_node() {
        let policy = PreOsMaintenancePolicy {
            max_parallel_nodes: 4,
            minimum_healthy_nodes: 997,
            ..Default::default()
        };
        let lease = PreOsMaintenanceLease {
            schema: PRE_OS_MAINTENANCE_LEASE_SCHEMA.to_string(),
            lease_id: "lease-1".to_string(),
            subject: "node-42".to_string(),
            cohort: "rack-2".to_string(),
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
            active_maintenance_nodes: 4,
            healthy_nodes_before_drain: 997,
            workloads_drained: false,
            normal_boot_ref: "uefi:Boot0002".to_string(),
            control_plane_ack: String::new(),
        };
        let blockers = validate_pre_os_maintenance_lease(&lease, &policy, 1_500);
        assert!(blockers.contains(&"workloads_not_drained".to_string()));
        assert!(blockers.contains(&"maintenance_parallelism_budget_exhausted".to_string()));
        assert!(blockers.contains(&"minimum_healthy_node_budget_would_be_violated".to_string()));
        assert!(blockers.contains(&"control_plane_ack_missing".to_string()));
    }
}
