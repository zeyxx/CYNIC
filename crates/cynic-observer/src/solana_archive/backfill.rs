use super::{
    Commitment, HistoryFilters, IndexedHistoryPage, IndexedHistoryQuery, MetricsAggregate,
    SortOrder,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackfillSpec {
    pub cluster: String,
    pub target: String,
    pub page_limit: u16,
    pub max_pages: u32,
    pub max_records: u64,
    pub sort_order: SortOrder,
    pub commitment: Commitment,
    pub filters: HistoryFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackfillHaltReason {
    Exhausted,
    PageBudgetReached,
    RecordBudgetReached,
    BlockedInvalidSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackfillCheckpoint {
    pub pagination_token: Option<String>,
    pub pages_fetched: u32,
    pub records_seen: u64,
    pub unique_signatures: u64,
    pub duplicate_records: u64,
    pub last_response_digest: Option<String>,
    pub completed: bool,
    pub halt_reason: Option<BackfillHaltReason>,
    #[serde(default)]
    pub metrics: MetricsAggregate,
    #[serde(default)]
    pub seen_signatures: BTreeSet<String>,
}

impl Default for BackfillCheckpoint {
    fn default() -> Self {
        Self {
            pagination_token: None,
            pages_fetched: 0,
            records_seen: 0,
            unique_signatures: 0,
            duplicate_records: 0,
            last_response_digest: None,
            completed: false,
            halt_reason: None,
            metrics: MetricsAggregate::default(),
            seen_signatures: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BackfillAction {
    FetchIndexedHistory { query: IndexedHistoryQuery },
    Stop { reason: BackfillHaltReason },
}

pub fn next_backfill_action(
    spec: &BackfillSpec,
    checkpoint: &BackfillCheckpoint,
) -> BackfillAction {
    if checkpoint.completed {
        return BackfillAction::Stop {
            reason: checkpoint
                .halt_reason
                .clone()
                .unwrap_or(BackfillHaltReason::Exhausted),
        };
    }
    if spec.target.trim().is_empty()
        || spec.cluster.trim().is_empty()
        || spec.page_limit == 0
        || spec.page_limit > 100
        || spec.max_pages == 0
        || spec.max_records == 0
        || matches!(spec.commitment, Commitment::Processed | Commitment::Unknown)
    {
        return BackfillAction::Stop {
            reason: BackfillHaltReason::BlockedInvalidSpec,
        };
    }
    if checkpoint.pages_fetched >= spec.max_pages {
        return BackfillAction::Stop {
            reason: BackfillHaltReason::PageBudgetReached,
        };
    }
    if checkpoint.records_seen >= spec.max_records {
        return BackfillAction::Stop {
            reason: BackfillHaltReason::RecordBudgetReached,
        };
    }

    let remaining = spec.max_records.saturating_sub(checkpoint.records_seen);
    let limit = u64::from(spec.page_limit).min(remaining) as u16;
    BackfillAction::FetchIndexedHistory {
        query: IndexedHistoryQuery {
            cluster: spec.cluster.clone(),
            target: spec.target.clone(),
            pagination_token: checkpoint.pagination_token.clone(),
            limit,
            sort_order: spec.sort_order.clone(),
            commitment: spec.commitment.clone(),
            filters: spec.filters.clone(),
        },
    }
}

pub fn apply_history_page(checkpoint: &mut BackfillCheckpoint, page: &IndexedHistoryPage) {
    checkpoint.pages_fetched += 1;
    checkpoint.last_response_digest = Some(page.response_digest.clone());
    checkpoint.metrics.record(&page.metrics);
    for record in &page.records {
        checkpoint.records_seen += 1;
        if !checkpoint.seen_signatures.insert(record.signature.clone()) {
            checkpoint.duplicate_records += 1;
        }
    }
    checkpoint.unique_signatures = checkpoint.seen_signatures.len() as u64;
    checkpoint.pagination_token = page.next_pagination_token.clone();
    if page.records.is_empty() || page.next_pagination_token.is_none() {
        checkpoint.completed = true;
        checkpoint.halt_reason = Some(BackfillHaltReason::Exhausted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        IndexedTransactionRecord, ProviderKind, RequestMetrics, SourceRole,
    };
    use serde_json::json;

    fn spec() -> BackfillSpec {
        BackfillSpec {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            page_limit: 100,
            max_pages: 2,
            max_records: 150,
            sort_order: SortOrder::Ascending,
            commitment: Commitment::Finalized,
            filters: HistoryFilters::default(),
        }
    }

    #[test]
    fn first_action_requests_helius_indexed_history() {
        let action = next_backfill_action(&spec(), &BackfillCheckpoint::default());
        let BackfillAction::FetchIndexedHistory { query } = action else {
            panic!("expected fetch action");
        };
        assert_eq!(query.limit, 100);
        assert_eq!(query.sort_order, SortOrder::Ascending);
    }

    #[test]
    fn next_page_is_bounded_by_remaining_record_budget() {
        let mut checkpoint = BackfillCheckpoint::default();
        checkpoint.records_seen = 120;
        let action = next_backfill_action(&spec(), &checkpoint);
        let BackfillAction::FetchIndexedHistory { query } = action else {
            panic!("expected fetch action");
        };
        assert_eq!(query.limit, 30);
    }

    #[test]
    fn page_application_tracks_duplicates_and_cursor() {
        let query = match next_backfill_action(&spec(), &BackfillCheckpoint::default()) {
            BackfillAction::FetchIndexedHistory { query } => query,
            _ => unreachable!(),
        };
        let page = IndexedHistoryPage {
            provider: ProviderKind::Helius,
            role: SourceRole::IndexedPrimary,
            query,
            records: vec![
                IndexedTransactionRecord {
                    signature: "a".to_string(),
                    slot: Some(1),
                    block_time: None,
                    transaction_index: Some(0),
                    payload: json!({}),
                },
                IndexedTransactionRecord {
                    signature: "a".to_string(),
                    slot: Some(1),
                    block_time: None,
                    transaction_index: Some(0),
                    payload: json!({}),
                },
            ],
            next_pagination_token: Some("2:0".to_string()),
            response_digest: "blake3:digest".to_string(),
            metrics: RequestMetrics::unmetered(ProviderKind::Helius, "getTransactionsForAddress"),
        };
        let mut checkpoint = BackfillCheckpoint::default();
        apply_history_page(&mut checkpoint, &page);
        assert_eq!(checkpoint.records_seen, 2);
        assert_eq!(checkpoint.unique_signatures, 1);
        assert_eq!(checkpoint.duplicate_records, 1);
        assert_eq!(checkpoint.pagination_token.as_deref(), Some("2:0"));
    }

    #[test]
    fn processed_commitment_is_rejected_for_indexed_history() {
        let mut invalid = spec();
        invalid.commitment = Commitment::Processed;
        assert_eq!(
            next_backfill_action(&invalid, &BackfillCheckpoint::default()),
            BackfillAction::Stop {
                reason: BackfillHaltReason::BlockedInvalidSpec
            }
        );
    }
}
