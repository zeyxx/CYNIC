use cynic_eval::convergence::{
    CAUSAL_CONVERGENCE_EPOCH_SCHEMA, CausalConvergenceEpoch, CausalEpochIndexEntry,
    ConvergenceStatus,
};

use super::types::{ProcessConvergenceHistoryItem, ProcessConvergenceProjection};
use crate::ports::causal_epoch_index::CausalEpochIndexStorePort;
use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;

pub const PROCESS_CONVERGENCE_PROJECTION_SCHEMA: &str =
    "cynic.host.process_convergence_projection.v1";

/// Projects stored index bindings and their read-back epochs. This function
/// only reads and validates; convergence policy remains in `decision`/`record`.
pub fn project_process_convergence(
    subject_id: &str,
    vertical_id: &str,
    history_limit: usize,
    index_store: &dyn CausalEpochIndexStorePort,
    evidence_store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<ProcessConvergenceProjection, String> {
    let index = index_store.read_index()?;
    let mut entries = index
        .entries
        .iter()
        .filter(|entry| entry.key.subject_id == subject_id && entry.key.vertical_id == vertical_id)
        .cloned()
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .observed_at_unix_ms
            .cmp(&left.observed_at_unix_ms)
            .then_with(|| right.key.epoch_id.cmp(&left.key.epoch_id))
    });
    entries.truncate(history_limit.clamp(1, 1_000));

    let mut residual_unknowns = Vec::new();
    let history = entries
        .into_iter()
        .map(|entry| {
            let mut item_unknowns = Vec::new();
            let epoch = match read_epoch(&entry, evidence_store)? {
                Some(epoch) => Some(epoch),
                None => {
                    let unknown = format!(
                        "process_convergence_epoch_read_back_missing:{}",
                        entry.epoch_digest
                    );
                    residual_unknowns.push(unknown.clone());
                    item_unknowns.push(unknown);
                    None
                }
            };
            Ok::<_, String>(ProcessConvergenceHistoryItem {
                index: entry,
                epoch,
                residual_unknowns: item_unknowns,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    if history.is_empty() {
        residual_unknowns.push("process_convergence_index_empty".to_string());
    }
    let latest = history.first().and_then(|item| item.epoch.clone());
    let status = if !residual_unknowns.is_empty() {
        ConvergenceStatus::Unknown
    } else {
        latest
            .as_ref()
            .map(|epoch| epoch.cell.status)
            .unwrap_or(ConvergenceStatus::Unknown)
    };
    residual_unknowns.sort();
    residual_unknowns.dedup();

    Ok(ProcessConvergenceProjection {
        schema: PROCESS_CONVERGENCE_PROJECTION_SCHEMA.to_string(),
        subject_id: subject_id.to_string(),
        vertical_id: vertical_id.to_string(),
        status,
        latest,
        history,
        residual_unknowns,
    })
}

fn read_epoch(
    entry: &CausalEpochIndexEntry,
    evidence_store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<Option<CausalConvergenceEpoch>, String> {
    let Some(envelope) = evidence_store.read(&entry.epoch_digest)? else {
        return Ok(None);
    };
    if envelope.digest != entry.epoch_digest {
        return Err("process_convergence_projection_digest_mismatch".to_string());
    }
    let epoch = serde_json::from_value::<CausalConvergenceEpoch>(envelope.payload)
        .map_err(|error| format!("decode process convergence epoch: {error}"))?;
    if epoch.schema != CAUSAL_CONVERGENCE_EPOCH_SCHEMA
        || epoch.epoch_id != entry.key.epoch_id
        || epoch.cell.subject_id != entry.key.subject_id
        || epoch.cell.vertical_id != entry.key.vertical_id
        || envelope.subject != entry.key.subject_id
    {
        return Err("process_convergence_projection_coordinate_mismatch".to_string());
    }
    Ok(Some(epoch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_causal_epoch_index::LocalCausalEpochIndexStore;
    use crate::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
    use crate::ports::causal_epoch_index::CausalEpochIndexStorePort;
    use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;
    use cynic_protocol::{
        CausalConvergenceEpoch, CausalEvidenceRefs, ConvergenceCell, EvidenceRef, TrustAssessment,
        TrustDimensionStatus, TrustRequirements, build_evidence_envelope,
    };

    /// A syntactically valid sha256 digest, distinguished by `marker`.
    ///
    /// The marker must be a hex digit: `valid_sha256_digest` rejects anything
    /// else, so a marker like 'z' produced an invalid digest and the index
    /// refused the entry (`causal_epoch_index_digest_invalid`) before the case
    /// under test was ever reached.
    fn digest(marker: char) -> String {
        assert!(
            marker.is_ascii_hexdigit(),
            "digest marker must be a hex digit, got {marker:?}"
        );
        format!("sha256:{}", marker.to_string().repeat(64))
    }

    fn epoch(epoch_id: &str, observed_at_unix_ms: u128, marker: char) -> CausalConvergenceEpoch {
        let evidence = |kind: &str, marker: char| EvidenceRef {
            kind: kind.to_string(),
            subject: "local-kernel/process".to_string(),
            digest: digest(marker),
            lineage: None,
        };
        CausalConvergenceEpoch {
            schema: CAUSAL_CONVERGENCE_EPOCH_SCHEMA.to_string(),
            epoch_id: epoch_id.to_string(),
            observed_at_unix_ms,
            cell: ConvergenceCell {
                subject_id: "local-kernel/process".to_string(),
                vertical_id: "process".to_string(),
                status: ConvergenceStatus::Complete,
                evidence_refs: vec![digest(marker)],
                blockers: Vec::new(),
            },
            causal_evidence: CausalEvidenceRefs {
                desired_state: evidence("desired", '1'),
                observations: vec![evidence("observation", '2')],
                decision: evidence("decision", '3'),
                authorization: None,
                action_receipt: None,
                read_back: Some(evidence("read_back", '4')),
            },
            trust_requirements: TrustRequirements {
                authority: true,
                authenticity: false,
                integrity: true,
                freshness: true,
                scope: true,
                replay_safety: false,
                read_back: true,
                independence: false,
            },
            trust: TrustAssessment {
                authority: TrustDimensionStatus::Satisfied,
                authenticity: TrustDimensionStatus::Unknown,
                integrity: TrustDimensionStatus::Satisfied,
                freshness: TrustDimensionStatus::Satisfied,
                scope: TrustDimensionStatus::Satisfied,
                replay_safety: TrustDimensionStatus::Unknown,
                read_back: TrustDimensionStatus::Satisfied,
                independence: cynic_protocol::EvidenceRelation::Unknown {
                    missing_evidence: vec!["independent_observer".to_string()],
                },
            },
            residual_unknowns: Vec::new(),
            blockers: Vec::new(),
        }
    }

    fn persist_epoch(
        epoch: &CausalConvergenceEpoch,
        index_store: &LocalCausalEpochIndexStore,
        evidence_store: &LocalContentAddressedEvidenceStore,
    ) {
        let envelope = build_evidence_envelope(
            "local-kernel/process",
            "test:process-convergence",
            epoch.observed_at_unix_ms,
            vec!["test".to_string()],
            epoch,
        )
        .unwrap();
        evidence_store.persist(&envelope).unwrap();
        index_store
            .append(&cynic_protocol::CausalEpochIndexEntry {
                key: cynic_protocol::CausalEpochKey {
                    subject_id: "local-kernel/process".to_string(),
                    vertical_id: "process".to_string(),
                    epoch_id: epoch.epoch_id.clone(),
                },
                epoch_digest: envelope.digest,
                observed_at_unix_ms: epoch.observed_at_unix_ms,
            })
            .unwrap();
    }

    #[test]
    fn projection_returns_latest_and_history_without_rewriting_epochs() {
        let index_root = tempfile::tempdir().unwrap();
        let evidence_root = tempfile::tempdir().unwrap();
        let index_store = LocalCausalEpochIndexStore::new(index_root.path());
        let evidence_store = LocalContentAddressedEvidenceStore::new(evidence_root.path());
        persist_epoch(&epoch("epoch-1", 1, 'a'), &index_store, &evidence_store);
        persist_epoch(&epoch("epoch-2", 2, 'b'), &index_store, &evidence_store);

        let projection = project_process_convergence(
            "local-kernel/process",
            "process",
            10,
            &index_store,
            &evidence_store,
        )
        .unwrap();

        assert_eq!(projection.status, ConvergenceStatus::Complete);
        assert_eq!(projection.latest.unwrap().epoch_id, "epoch-2");
        assert_eq!(projection.history.len(), 2);
        assert!(projection.residual_unknowns.is_empty());
    }

    #[test]
    fn missing_latest_epoch_is_unknown_not_older_success() {
        let index_root = tempfile::tempdir().unwrap();
        let evidence_root = tempfile::tempdir().unwrap();
        let index_store = LocalCausalEpochIndexStore::new(index_root.path());
        let evidence_store = LocalContentAddressedEvidenceStore::new(evidence_root.path());
        persist_epoch(&epoch("epoch-1", 1, 'a'), &index_store, &evidence_store);
        index_store
            .append(&cynic_protocol::CausalEpochIndexEntry {
                key: cynic_protocol::CausalEpochKey {
                    subject_id: "local-kernel/process".to_string(),
                    vertical_id: "process".to_string(),
                    epoch_id: "epoch-2".to_string(),
                },
                // A syntactically valid digest that was never persisted: the
                // point is a missing read-back, not a malformed digest.
                epoch_digest: digest('f'),
                observed_at_unix_ms: 2,
            })
            .unwrap();

        let projection = project_process_convergence(
            "local-kernel/process",
            "process",
            10,
            &index_store,
            &evidence_store,
        )
        .unwrap();

        assert_eq!(projection.status, ConvergenceStatus::Unknown);
        assert!(projection.latest.is_none());
        assert!(
            projection.residual_unknowns[0]
                .starts_with("process_convergence_epoch_read_back_missing:")
        );
    }
}
