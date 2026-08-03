use super::{ProviderKind, SignaturePage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Corroborated,
    SingleSource,
    Conflicting,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureReconciliation {
    pub signature: String,
    pub providers: Vec<ProviderKind>,
    pub slots: Vec<u64>,
    pub statuses: Vec<String>,
    pub status: ReconciliationStatus,
}

pub fn reconcile_signature_pages(pages: &[SignaturePage]) -> Vec<SignatureReconciliation> {
    let mut by_signature: BTreeMap<String, (Vec<ProviderKind>, Vec<u64>, Vec<String>)> =
        BTreeMap::new();

    for page in pages {
        for record in &page.records {
            let entry = by_signature.entry(record.signature.clone()).or_default();
            if !entry.0.contains(&page.provider) {
                entry.0.push(page.provider.clone());
            }
            if let Some(slot) = record.slot {
                if !entry.1.contains(&slot) {
                    entry.1.push(slot);
                }
            }
            if let Some(status) = &record.confirmation_status {
                if !entry.2.contains(status) {
                    entry.2.push(status.clone());
                }
            }
        }
    }

    by_signature
        .into_iter()
        .map(|(signature, (providers, slots, statuses))| {
            let status = if providers.is_empty() {
                ReconciliationStatus::Unknown
            } else if slots.len() > 1 || statuses.len() > 1 {
                ReconciliationStatus::Conflicting
            } else if providers.len() > 1 {
                ReconciliationStatus::Corroborated
            } else {
                ReconciliationStatus::SingleSource
            };
            SignatureReconciliation {
                signature,
                providers,
                slots,
                statuses,
                status,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        Commitment, RequestMetrics, SignatureQuery, SignatureRecord, SourceRole,
    };

    fn page(provider: ProviderKind, signature: &str, slot: u64, status: &str) -> SignaturePage {
        let response_digest = format!("digest-{provider:?}");
        SignaturePage {
            provider: provider.clone(),
            role: SourceRole::RawIndependent,
            query: SignatureQuery {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                before: None,
                until: None,
                limit: 1000,
                commitment: Commitment::Finalized,
            },
            records: vec![SignatureRecord {
                signature: signature.to_string(),
                slot: Some(slot),
                block_time: None,
                confirmation_status: Some(status.to_string()),
                err: None,
            }],
            next_before: None,
            response_digest,
            metrics: RequestMetrics::unmetered(provider, "getSignaturesForAddress"),
        }
    }

    #[test]
    fn same_signature_from_two_providers_is_corroborated() {
        let results = reconcile_signature_pages(&[
            page(ProviderKind::Helius, "sig", 10, "finalized"),
            page(ProviderKind::PublicRpc, "sig", 10, "finalized"),
        ]);
        assert_eq!(results[0].status, ReconciliationStatus::Corroborated);
    }

    #[test]
    fn disagreement_is_retained_as_conflict() {
        let results = reconcile_signature_pages(&[
            page(ProviderKind::Helius, "sig", 10, "confirmed"),
            page(ProviderKind::PublicRpc, "sig", 11, "finalized"),
        ]);
        assert_eq!(results[0].status, ReconciliationStatus::Conflicting);
        assert_eq!(results[0].slots, vec![10, 11]);
    }
}
