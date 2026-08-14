use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;
use cynic_protocol::{EvidenceEnvelope, TargetCandidate, build_evidence_envelope};
use serde::Serialize;

pub const DISCOVERY_CANDIDATE_STORE_REPORT_SCHEMA: &str =
    "cynic.discovery_candidate.store_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveryCandidateStoreReport {
    pub schema: String,
    pub status: String,
    pub mutates: bool,
    pub digest: String,
    pub evidence_ref: Option<String>,
    pub blockers: Vec<String>,
}

pub fn build_candidate_record(
    candidate: TargetCandidate,
    observed_at_unix_ms: u128,
    source: &str,
) -> Result<EvidenceEnvelope, String> {
    if source.trim().is_empty() {
        return Err("candidate_source_missing".to_string());
    }
    if candidate.evidence_refs.is_empty() {
        return Err("candidate_provenance_missing".to_string());
    }
    build_evidence_envelope(
        &candidate.id,
        source,
        observed_at_unix_ms,
        candidate.evidence_refs.clone(),
        &candidate,
    )
}

pub fn store_candidate(
    record: &EvidenceEnvelope,
    apply: bool,
    store: &dyn ContentAddressedEvidenceStorePort,
) -> DiscoveryCandidateStoreReport {
    if !apply {
        return DiscoveryCandidateStoreReport {
            schema: DISCOVERY_CANDIDATE_STORE_REPORT_SCHEMA.to_string(),
            status: "planned".to_string(),
            mutates: false,
            digest: record.digest.clone(),
            evidence_ref: None,
            blockers: vec!["explicit_apply_required".to_string()],
        };
    }
    match store.persist(record) {
        Ok(evidence_ref) => DiscoveryCandidateStoreReport {
            schema: DISCOVERY_CANDIDATE_STORE_REPORT_SCHEMA.to_string(),
            status: "persisted".to_string(),
            mutates: true,
            digest: record.digest.clone(),
            evidence_ref: Some(evidence_ref),
            blockers: Vec::new(),
        },
        Err(error) => DiscoveryCandidateStoreReport {
            schema: DISCOVERY_CANDIDATE_STORE_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            mutates: false,
            digest: record.digest.clone(),
            evidence_ref: None,
            blockers: vec![error],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::DiscoveryState;

    struct RejectingStore;

    impl ContentAddressedEvidenceStorePort for RejectingStore {
        fn persist(&self, _record: &EvidenceEnvelope) -> Result<String, String> {
            Err("must_not_be_called_without_apply".to_string())
        }
    }

    fn candidate() -> TargetCandidate {
        TargetCandidate {
            id: "forgejo".into(),
            provider: "proxmox".into(),
            resource_id: Some("lxc/101".into()),
            kind: Some("lxc".into()),
            declared: DiscoveryState::Unknown,
            discovered: DiscoveryState::Confirmed,
            reachable: DiscoveryState::Confirmed,
            attestable: DiscoveryState::Unknown,
            addresses: vec!["10.10.10.101".into()],
            transports: vec!["proxmox-api".into()],
            evidence_refs: vec!["proxmox-inventory.json".into()],
        }
    }

    #[test]
    fn candidate_identity_is_stable_while_observation_records_are_distinct() {
        let first = build_candidate_record(candidate(), 1, "proxmox").unwrap();
        let second = build_candidate_record(candidate(), 2, "proxmox").unwrap();
        assert_eq!(first.payload_digest, second.payload_digest);
        assert_ne!(first.digest, second.digest);
    }

    #[test]
    fn persistence_requires_explicit_apply() {
        let record = build_candidate_record(candidate(), 1, "proxmox").unwrap();
        let report = store_candidate(&record, false, &RejectingStore);
        assert_eq!(report.status, "planned");
        assert!(!report.mutates);
    }
}
