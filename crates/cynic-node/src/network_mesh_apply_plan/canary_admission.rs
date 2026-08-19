//! Non-mutating admission for the first real-fleet mesh rollout.
//!
//! The canary is deliberately an evidence gate, not an executor.  Execution
//! remains owned by the existing target transaction and host actuation
//! boundaries after this report is admitted.

use cynic_host::runtime_harness::{
    RuntimeHarnessTargetAttestation, canary_context, validate_runtime_harness_target_attestation,
};
use serde::{Deserialize, Serialize};

const CANARY_TARGET_COUNT: usize = 2;
const NORMAL_HUMAN_APPROVALS: u8 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct MeshCanaryEvidence {
    pub(crate) target_ids: Vec<String>,
    pub(crate) target_attestations: Vec<RuntimeHarnessTargetAttestation>,
    pub(crate) provider: String,
    pub(crate) package_provenance_selected: bool,
    pub(crate) package_promotion_eligible: bool,
    pub(crate) package_proof_refs: Vec<String>,
    pub(crate) human_approval_count: u8,
    pub(crate) dns_zone_scoped: bool,
    pub(crate) dns_loopback_only: bool,
}

impl Default for MeshCanaryEvidence {
    fn default() -> Self {
        Self {
            target_ids: Vec::new(),
            target_attestations: Vec::new(),
            provider: "wireguard".to_string(),
            package_provenance_selected: false,
            package_promotion_eligible: false,
            package_proof_refs: Vec::new(),
            human_approval_count: 0,
            dns_zone_scoped: false,
            dns_loopback_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeshCanaryAdmissionReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) required_target_count: usize,
    pub(crate) provider: String,
    pub(crate) target_ids: Vec<String>,
    pub(crate) required_evidence: Vec<&'static str>,
    pub(crate) blockers: Vec<String>,
}

pub(super) fn assess_mesh_canary_admission(
    evidence: &MeshCanaryEvidence,
) -> MeshCanaryAdmissionReport {
    let mut blockers = Vec::new();
    let unique_targets = evidence
        .target_ids
        .iter()
        .filter(|target| !target.trim().is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    if evidence.target_ids.len() != CANARY_TARGET_COUNT
        || unique_targets.len() != CANARY_TARGET_COUNT
    {
        blockers.push("canary_requires_exactly_two_distinct_targets".to_string());
    }
    if evidence.provider != "wireguard" {
        blockers.push("canary_requires_direct_wireguard_provider".to_string());
    }
    if !evidence.package_provenance_selected {
        blockers.push("canary_package_provenance_unselected".to_string());
    }
    if !evidence.package_promotion_eligible {
        blockers.push("canary_package_promotion_ineligible".to_string());
    }
    if evidence.package_proof_refs.is_empty()
        || evidence
            .package_proof_refs
            .iter()
            .any(|proof| proof.trim().is_empty())
    {
        blockers.push("canary_package_proof_missing".to_string());
    }
    if evidence.human_approval_count < NORMAL_HUMAN_APPROVALS {
        blockers.push("canary_human_approval_threshold_unmet".to_string());
    }
    if !evidence.dns_zone_scoped {
        blockers.push("canary_dns_zone_scope_unverified".to_string());
    }
    if !evidence.dns_loopback_only {
        blockers.push("canary_dns_loopback_scope_unverified".to_string());
    }

    let context = canary_context();
    if evidence.target_attestations.len() != CANARY_TARGET_COUNT {
        blockers.push("canary_target_attestation_count_invalid".to_string());
    }
    for target in &evidence.target_ids {
        let matching = evidence
            .target_attestations
            .iter()
            .filter(|attestation| attestation.target_id.as_str() == target.as_str())
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            blockers.push(format!("canary_target_attestation_missing:{target}"));
            continue;
        }
        blockers.extend(
            validate_runtime_harness_target_attestation(&context, matching[0])
                .into_iter()
                .map(|violation| format!("canary_{}:{target}", violation.code)),
        );
    }
    blockers.sort();
    blockers.dedup();

    MeshCanaryAdmissionReport {
        schema: "cynic.network_mesh_canary_admission.v1",
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        },
        mutates: false,
        required_target_count: CANARY_TARGET_COUNT,
        provider: evidence.provider.clone(),
        target_ids: evidence.target_ids.clone(),
        required_evidence: vec![
            "two_distinct_canary_targets",
            "fresh_canary_target_attestation_per_target",
            "selected_and_promoted_package_provenance",
            "explicit_human_approval",
            "zone_scoped_loopback_dns_proof",
            "post_apply_readback_and_rollback_intent",
        ],
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
    };

    use super::{MeshCanaryEvidence, assess_mesh_canary_admission};

    fn attestation(
        target_id: &str,
    ) -> cynic_host::runtime_harness::RuntimeHarnessTargetAttestation {
        let context = cynic_host::runtime_harness::canary_context();
        cynic_host::runtime_harness::RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::Canary,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: target_id.to_string(),
            provider: "proxmox".to_string(),
            transport: "mcp-mtls".to_string(),
            transport_target: format!("https://{target_id}.synapse.cynic"),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "declared-canary-subset".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec![format!("runtime-proof://canary/{target_id}")],
        }
    }

    #[test]
    fn missing_evidence_is_blocked_without_creating_a_single_node_canary() {
        let report = assess_mesh_canary_admission(&MeshCanaryEvidence::default());

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker == "canary_requires_exactly_two_distinct_targets")
        );
        assert!(!report.mutates);
    }

    #[test]
    fn two_attested_targets_with_promoted_provenance_are_admitted() {
        let evidence = MeshCanaryEvidence {
            target_ids: vec!["node-a".to_string(), "node-b".to_string()],
            target_attestations: vec![attestation("node-a"), attestation("node-b")],
            provider: "wireguard".to_string(),
            package_provenance_selected: true,
            package_promotion_eligible: true,
            package_proof_refs: vec!["proof://packages/coredns".to_string()],
            human_approval_count: 1,
            dns_zone_scoped: true,
            dns_loopback_only: true,
        };

        assert_eq!(assess_mesh_canary_admission(&evidence).status, "admitted");
    }
}
