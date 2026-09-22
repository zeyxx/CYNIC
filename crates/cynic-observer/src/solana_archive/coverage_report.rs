use super::{
    CoverageGap, FanoutCheckpoint, FanoutHaltReason, FanoutReadStore, FanoutSpec,
    ReconciliationStatus, SignaturePage, SignatureReconciliation, StorageError,
    reconcile_signature_pages,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationWindow {
    pub cluster: String,
    pub target: String,
    pub start_slot: Option<u64>,
    pub end_slot: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageReportStatus {
    Complete,
    Partial,
    Conflicting,
    InProgress,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageReport {
    pub schema: String,
    pub window: ObservationWindow,
    pub status: CoverageReportStatus,
    pub pages: u64,
    pub signatures_seen: u64,
    pub unique_signatures: u64,
    pub next_cursor: Option<String>,
    pub providers: Vec<String>,
    pub corroborated_signatures: u64,
    pub single_source_signatures: u64,
    pub conflicting_signatures: u64,
    pub gaps: Vec<CoverageGap>,
    pub residual_unknowns: Vec<String>,
}

pub fn build_coverage_report(
    window: ObservationWindow,
    checkpoint: &FanoutCheckpoint,
    reconciliations: &[SignatureReconciliation],
) -> CoverageReport {
    let mut gaps = Vec::new();
    let mut residual_unknowns = Vec::new();
    let mut providers = BTreeSet::new();
    let mut corroborated_signatures = 0;
    let mut single_source_signatures = 0;
    let mut conflicting_signatures = 0;

    for reconciliation in reconciliations {
        for provider in &reconciliation.providers {
            providers.insert(format!("{provider:?}"));
        }
        match &reconciliation.status {
            ReconciliationStatus::Corroborated => corroborated_signatures += 1,
            ReconciliationStatus::SingleSource => single_source_signatures += 1,
            ReconciliationStatus::Conflicting => conflicting_signatures += 1,
            ReconciliationStatus::Unknown => residual_unknowns.push(format!(
                "signature {} has no attributable provider",
                reconciliation.signature
            )),
        }
    }

    if checkpoint.signature_pages_fetched == 0 {
        residual_unknowns.push("no signature page has been observed".to_string());
    }
    if let Some(reason) = &checkpoint.halt_reason {
        match reason {
            FanoutHaltReason::Exhausted => {}
            FanoutHaltReason::SignaturePageBudgetReached => gaps.push(CoverageGap {
                code: "signature_page_budget".to_string(),
                detail: "signature pagination stopped at the declared budget".to_string(),
            }),
            FanoutHaltReason::TransactionBudgetReached => gaps.push(CoverageGap {
                code: "transaction_budget".to_string(),
                detail: "transaction retrieval stopped at the declared budget".to_string(),
            }),
            other => gaps.push(CoverageGap {
                code: "fanout_halted".to_string(),
                detail: format!("fan-out halted with {other:?}"),
            }),
        }
    }
    if checkpoint.last_failure.is_some() {
        residual_unknowns.push("last provider failure requires coverage review".to_string());
    }
    if checkpoint.unique_signatures > reconciliations.len() as u64 {
        residual_unknowns.push(
            "some observed signatures have no cross-source reconciliation record".to_string(),
        );
    }
    if conflicting_signatures > 0 {
        residual_unknowns.push("at least one signature conflicts across observations".to_string());
    }

    let status = if conflicting_signatures > 0 {
        CoverageReportStatus::Conflicting
    } else if !checkpoint.completed {
        if checkpoint.signature_pages_fetched == 0 {
            CoverageReportStatus::Unknown
        } else {
            CoverageReportStatus::InProgress
        }
    } else if !gaps.is_empty()
        || !residual_unknowns.is_empty()
        || checkpoint.halt_reason != Some(FanoutHaltReason::Exhausted)
    {
        CoverageReportStatus::Partial
    } else {
        CoverageReportStatus::Complete
    };

    CoverageReport {
        schema: "cynic.solana.coverage_report.v1".to_string(),
        window,
        status,
        pages: u64::from(checkpoint.signature_pages_fetched),
        signatures_seen: checkpoint.signatures_seen,
        unique_signatures: checkpoint.unique_signatures,
        next_cursor: checkpoint.before_signature.clone(),
        providers: providers.into_iter().collect(),
        corroborated_signatures,
        single_source_signatures,
        conflicting_signatures,
        gaps,
        residual_unknowns,
    }
}

pub fn build_coverage_report_from_pages(
    window: ObservationWindow,
    checkpoint: &FanoutCheckpoint,
    pages: &[SignaturePage],
) -> CoverageReport {
    let reconciliations = reconcile_signature_pages(pages);
    build_coverage_report(window, checkpoint, &reconciliations)
}

pub fn read_coverage_report<S: FanoutReadStore>(
    store: &S,
    spec: &FanoutSpec,
    window: ObservationWindow,
) -> Result<CoverageReport, StorageError> {
    let pages = store.load_signature_pages(spec)?;
    let checkpoint = store.load_fanout_checkpoint(spec)?.unwrap_or_default();
    Ok(build_coverage_report_from_pages(
        window,
        &checkpoint,
        &pages,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{FanoutCheckpoint, ProviderKind, StorageError};

    fn window() -> ObservationWindow {
        ObservationWindow {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            start_slot: Some(1),
            end_slot: Some(10),
        }
    }

    #[test]
    fn exhausted_without_conflict_is_complete() {
        let checkpoint = FanoutCheckpoint {
            signature_pages_fetched: 1,
            signatures_seen: 1,
            unique_signatures: 1,
            completed: true,
            halt_reason: Some(FanoutHaltReason::Exhausted),
            ..FanoutCheckpoint::default()
        };
        let reconciliations = vec![SignatureReconciliation {
            signature: "a".to_string(),
            providers: vec![ProviderKind::Helius],
            slots: vec![1],
            statuses: vec!["finalized".to_string()],
            status: ReconciliationStatus::SingleSource,
        }];
        let report = build_coverage_report(window(), &checkpoint, &reconciliations);
        assert_eq!(report.status, CoverageReportStatus::Complete);
        assert_eq!(report.single_source_signatures, 1);
    }

    #[test]
    fn budget_stop_is_partial_and_conflict_wins() {
        let checkpoint = FanoutCheckpoint {
            signature_pages_fetched: 1,
            completed: true,
            halt_reason: Some(FanoutHaltReason::SignaturePageBudgetReached),
            ..FanoutCheckpoint::default()
        };
        let reconciliations = vec![SignatureReconciliation {
            signature: "a".to_string(),
            providers: vec![ProviderKind::Helius, ProviderKind::PublicRpc],
            slots: vec![1, 2],
            statuses: vec!["confirmed".to_string(), "finalized".to_string()],
            status: ReconciliationStatus::Conflicting,
        }];
        let report = build_coverage_report(window(), &checkpoint, &reconciliations);
        assert_eq!(report.status, CoverageReportStatus::Conflicting);
        assert_eq!(report.gaps[0].code, "signature_page_budget");
    }

    #[test]
    fn pages_feed_provenance_and_conflict_into_the_report() {
        let page = |provider: ProviderKind, slot: u64| SignaturePage {
            provider: provider.clone(),
            role: crate::solana_archive::SourceRole::RawIndependent,
            query: crate::solana_archive::SignatureQuery {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                before: None,
                until: None,
                limit: 1,
                commitment: crate::solana_archive::Commitment::Finalized,
            },
            records: vec![crate::solana_archive::SignatureRecord {
                signature: "same-signature".to_string(),
                slot: Some(slot),
                block_time: None,
                confirmation_status: Some("finalized".to_string()),
                err: None,
            }],
            next_before: None,
            response_digest: format!("blake3:{slot}"),
            metrics: crate::solana_archive::RequestMetrics::unmetered(
                provider,
                "getSignaturesForAddress",
            ),
        };
        let checkpoint = FanoutCheckpoint {
            signature_pages_fetched: 2,
            signatures_seen: 2,
            unique_signatures: 1,
            completed: true,
            halt_reason: Some(FanoutHaltReason::Exhausted),
            ..FanoutCheckpoint::default()
        };
        let report = build_coverage_report_from_pages(
            window(),
            &checkpoint,
            &[
                page(ProviderKind::Helius, 10),
                page(ProviderKind::PublicRpc, 11),
            ],
        );
        assert_eq!(report.status, CoverageReportStatus::Conflicting);
        assert_eq!(report.conflicting_signatures, 1);
        assert_eq!(report.providers, vec!["Helius", "PublicRpc"]);
    }

    #[test]
    fn readback_without_checkpoint_is_explicitly_unknown() {
        struct EmptyStore;

        impl FanoutReadStore for EmptyStore {
            fn load_signature_pages(
                &self,
                _spec: &FanoutSpec,
            ) -> Result<Vec<SignaturePage>, StorageError> {
                Ok(Vec::new())
            }

            fn load_fanout_checkpoint(
                &self,
                _spec: &FanoutSpec,
            ) -> Result<Option<FanoutCheckpoint>, StorageError> {
                Ok(None)
            }
        }

        let spec = FanoutSpec {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            signature_page_limit: 1,
            max_signature_pages: 1,
            max_transactions: 1,
            commitment: crate::solana_archive::Commitment::Finalized,
        };
        let report = read_coverage_report(&EmptyStore, &spec, window()).unwrap();
        assert_eq!(report.status, CoverageReportStatus::Unknown);
        assert_eq!(report.pages, 0);
    }
}
