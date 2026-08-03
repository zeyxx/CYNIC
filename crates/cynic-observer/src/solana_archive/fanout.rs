use super::{
    Commitment, MetricsAggregate, RawTransactionObservation, SignaturePage, SignatureQuery,
    SourceError,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutSpec {
    pub cluster: String,
    pub target: String,
    pub signature_page_limit: u16,
    pub max_signature_pages: u32,
    pub max_transactions: u64,
    pub commitment: Commitment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FanoutHaltReason {
    Exhausted,
    SignaturePageBudgetReached,
    TransactionBudgetReached,
    RetryBudgetReached,
    UnretryableFailure,
    BlockedInvalidSpec,
    TransactionNotQueued,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FanoutFailureKind {
    RateLimited,
    Transport,
    Http,
    Rpc,
    InvalidResponse,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutFailure {
    pub kind: FanoutFailureKind,
    pub detail: String,
    pub http_status: Option<u16>,
    pub retry_after_ms: Option<u64>,
}

impl FanoutFailure {
    pub fn from_source_error(error: &SourceError) -> Self {
        match error {
            SourceError::Transport(detail) => Self {
                kind: FanoutFailureKind::Transport,
                detail: detail.clone(),
                http_status: None,
                retry_after_ms: None,
            },
            SourceError::Http { status, detail } => Self {
                kind: FanoutFailureKind::Http,
                detail: detail.clone(),
                http_status: Some(*status),
                retry_after_ms: None,
            },
            SourceError::Rpc { code, detail } => Self {
                kind: FanoutFailureKind::Rpc,
                detail: format!("code={code:?}: {detail}"),
                http_status: None,
                retry_after_ms: None,
            },
            SourceError::RateLimited { retry_after_ms } => Self {
                kind: FanoutFailureKind::RateLimited,
                detail: "provider rate limit".to_string(),
                http_status: Some(429),
                retry_after_ms: *retry_after_ms,
            },
            SourceError::InvalidResponse(detail) => Self {
                kind: FanoutFailureKind::InvalidResponse,
                detail: detail.clone(),
                http_status: None,
                retry_after_ms: None,
            },
            SourceError::Blocked(detail) => Self {
                kind: FanoutFailureKind::Blocked,
                detail: detail.clone(),
                http_status: None,
                retry_after_ms: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutRetryPolicy {
    pub max_attempts: u16,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for FanoutRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_ms: 500,
            max_backoff_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum FanoutRetryDecision {
    Retry { attempt: u16, backoff_ms: u64 },
    Stop { reason: FanoutHaltReason },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanoutCheckpoint {
    pub before_signature: Option<String>,
    pub signature_pages_fetched: u32,
    pub signatures_seen: u64,
    pub unique_signatures: u64,
    pub transactions_fetched: u64,
    pub signature_source_exhausted: bool,
    pub completed: bool,
    pub halt_reason: Option<FanoutHaltReason>,
    pub pending_signatures: VecDeque<String>,
    #[serde(default)]
    pub seen_signatures: BTreeSet<String>,
    #[serde(default)]
    pub retry_attempts: u16,
    #[serde(default)]
    pub last_failure: Option<FanoutFailure>,
    #[serde(default)]
    pub metrics: MetricsAggregate,
}

impl Default for FanoutCheckpoint {
    fn default() -> Self {
        Self {
            before_signature: None,
            signature_pages_fetched: 0,
            signatures_seen: 0,
            unique_signatures: 0,
            transactions_fetched: 0,
            signature_source_exhausted: false,
            completed: false,
            halt_reason: None,
            pending_signatures: VecDeque::new(),
            seen_signatures: BTreeSet::new(),
            retry_attempts: 0,
            last_failure: None,
            metrics: MetricsAggregate::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FanoutAction {
    FetchSignatures { query: SignatureQuery },
    FetchTransaction { signature: String },
    Stop { reason: FanoutHaltReason },
}

pub fn next_fanout_action(spec: &FanoutSpec, checkpoint: &FanoutCheckpoint) -> FanoutAction {
    if checkpoint.completed {
        return FanoutAction::Stop {
            reason: checkpoint
                .halt_reason
                .clone()
                .unwrap_or(FanoutHaltReason::Exhausted),
        };
    }
    if spec.cluster.trim().is_empty()
        || spec.target.trim().is_empty()
        || spec.signature_page_limit == 0
        || spec.signature_page_limit > 1_000
        || spec.max_signature_pages == 0
        || spec.max_transactions == 0
        || matches!(spec.commitment, Commitment::Processed | Commitment::Unknown)
    {
        return FanoutAction::Stop {
            reason: FanoutHaltReason::BlockedInvalidSpec,
        };
    }
    if checkpoint.transactions_fetched >= spec.max_transactions {
        return FanoutAction::Stop {
            reason: FanoutHaltReason::TransactionBudgetReached,
        };
    }
    if let Some(signature) = checkpoint.pending_signatures.front() {
        return FanoutAction::FetchTransaction {
            signature: signature.clone(),
        };
    }
    if checkpoint.signature_source_exhausted {
        return FanoutAction::Stop {
            reason: FanoutHaltReason::Exhausted,
        };
    }
    if checkpoint.signature_pages_fetched >= spec.max_signature_pages {
        return FanoutAction::Stop {
            reason: FanoutHaltReason::SignaturePageBudgetReached,
        };
    }
    FanoutAction::FetchSignatures {
        query: SignatureQuery {
            cluster: spec.cluster.clone(),
            target: spec.target.clone(),
            before: checkpoint.before_signature.clone(),
            until: None,
            limit: spec.signature_page_limit,
            commitment: spec.commitment.clone(),
        },
    }
}

pub fn apply_signature_page(checkpoint: &mut FanoutCheckpoint, page: &SignaturePage) {
    checkpoint.retry_attempts = 0;
    checkpoint.last_failure = None;
    checkpoint.signature_pages_fetched += 1;
    checkpoint.metrics.record(&page.metrics);
    checkpoint.signatures_seen += page.records.len() as u64;
    for record in &page.records {
        if checkpoint.seen_signatures.insert(record.signature.clone()) {
            checkpoint
                .pending_signatures
                .push_back(record.signature.clone());
        }
    }
    checkpoint.unique_signatures = checkpoint.seen_signatures.len() as u64;
    checkpoint.before_signature = page.next_before.clone();
    if page.records.is_empty() || page.next_before.is_none() {
        checkpoint.signature_source_exhausted = true;
    }
}

pub fn apply_transaction(
    checkpoint: &mut FanoutCheckpoint,
    transaction: &RawTransactionObservation,
) -> Result<(), FanoutHaltReason> {
    let Some(position) = checkpoint
        .pending_signatures
        .iter()
        .position(|signature| signature == &transaction.query.signature)
    else {
        return Err(FanoutHaltReason::TransactionNotQueued);
    };
    checkpoint.pending_signatures.remove(position);
    checkpoint.retry_attempts = 0;
    checkpoint.last_failure = None;
    checkpoint.transactions_fetched += 1;
    checkpoint.metrics.record(&transaction.metrics);
    if checkpoint.signature_source_exhausted && checkpoint.pending_signatures.is_empty() {
        checkpoint.completed = true;
        checkpoint.halt_reason = Some(FanoutHaltReason::Exhausted);
    }
    Ok(())
}

pub fn apply_fanout_failure(
    checkpoint: &mut FanoutCheckpoint,
    policy: &FanoutRetryPolicy,
    failure: FanoutFailure,
) -> FanoutRetryDecision {
    checkpoint.last_failure = Some(failure.clone());
    if !is_retryable(&failure) || policy.max_attempts == 0 {
        return FanoutRetryDecision::Stop {
            reason: FanoutHaltReason::UnretryableFailure,
        };
    }
    if checkpoint.retry_attempts >= policy.max_attempts {
        return FanoutRetryDecision::Stop {
            reason: FanoutHaltReason::RetryBudgetReached,
        };
    }
    checkpoint.retry_attempts += 1;
    let exponent = u32::from(checkpoint.retry_attempts.saturating_sub(1)).min(16);
    let calculated = policy
        .base_backoff_ms
        .saturating_mul(2u64.saturating_pow(exponent));
    let mut backoff_ms = calculated.min(policy.max_backoff_ms);
    if let Some(retry_after_ms) = failure.retry_after_ms {
        backoff_ms = backoff_ms.max(retry_after_ms).min(policy.max_backoff_ms);
    }
    FanoutRetryDecision::Retry {
        attempt: checkpoint.retry_attempts,
        backoff_ms,
    }
}

fn is_retryable(failure: &FanoutFailure) -> bool {
    match failure.kind {
        FanoutFailureKind::RateLimited | FanoutFailureKind::Transport => true,
        FanoutFailureKind::Http => failure
            .http_status
            .map(|status| status == 408 || status == 425 || status == 429 || status >= 500)
            .unwrap_or(false),
        FanoutFailureKind::Rpc => true,
        FanoutFailureKind::InvalidResponse | FanoutFailureKind::Blocked => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        ProviderKind, RequestMetrics, SignatureRecord, SourceRole, TransactionQuery,
    };
    use serde_json::json;

    fn spec() -> FanoutSpec {
        FanoutSpec {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            signature_page_limit: 2,
            max_signature_pages: 2,
            max_transactions: 3,
            commitment: Commitment::Finalized,
        }
    }

    fn metrics(provider: ProviderKind, method: &str) -> RequestMetrics {
        RequestMetrics::unmetered(provider, method)
    }

    #[test]
    fn fanout_fetches_signatures_then_queued_transactions() {
        let mut checkpoint = FanoutCheckpoint::default();
        let FanoutAction::FetchSignatures { query } = next_fanout_action(&spec(), &checkpoint)
        else {
            panic!("expected signature fetch");
        };
        let page = SignaturePage {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query,
            records: vec![
                SignatureRecord {
                    signature: "a".to_string(),
                    slot: Some(1),
                    block_time: None,
                    confirmation_status: Some("finalized".to_string()),
                    err: None,
                },
                SignatureRecord {
                    signature: "b".to_string(),
                    slot: Some(2),
                    block_time: None,
                    confirmation_status: Some("finalized".to_string()),
                    err: None,
                },
            ],
            next_before: None,
            response_digest: "blake3:page".to_string(),
            metrics: metrics(ProviderKind::Helius, "getSignaturesForAddress"),
        };
        apply_signature_page(&mut checkpoint, &page);
        assert_eq!(
            next_fanout_action(&spec(), &checkpoint),
            FanoutAction::FetchTransaction {
                signature: "a".to_string()
            }
        );
    }

    #[test]
    fn transaction_completion_waits_for_all_queued_signatures() {
        let mut checkpoint = FanoutCheckpoint {
            signature_source_exhausted: true,
            pending_signatures: VecDeque::from(["a".to_string(), "b".to_string()]),
            seen_signatures: BTreeSet::from(["a".to_string(), "b".to_string()]),
            unique_signatures: 2,
            ..FanoutCheckpoint::default()
        };
        let transaction = RawTransactionObservation {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: TransactionQuery {
                cluster: "mainnet".to_string(),
                signature: "a".to_string(),
                commitment: Commitment::Finalized,
            },
            payload: json!({}),
            response_digest: "blake3:tx".to_string(),
            retrieved_at_unix_ms: 1,
            metrics: metrics(ProviderKind::Helius, "getTransaction"),
        };
        apply_transaction(&mut checkpoint, &transaction).unwrap();
        assert!(!checkpoint.completed);
        assert_eq!(checkpoint.pending_signatures.front().unwrap(), "b");
    }

    #[test]
    fn multi_page_fixture_deduplicates_signatures_before_transaction_fetch() {
        let mut checkpoint = FanoutCheckpoint::default();
        let first_query = match next_fanout_action(&spec(), &checkpoint) {
            FanoutAction::FetchSignatures { query } => query,
            _ => unreachable!(),
        };
        apply_signature_page(
            &mut checkpoint,
            &SignaturePage {
                provider: ProviderKind::Helius,
                role: SourceRole::RawIndependent,
                query: first_query,
                records: vec![signature("a", 1), signature("b", 2)],
                next_before: Some("b".to_string()),
                response_digest: "blake3:first-page".to_string(),
                metrics: metrics(ProviderKind::Helius, "getSignaturesForAddress"),
            },
        );

        apply_transaction(&mut checkpoint, &raw_transaction("a")).unwrap();
        apply_transaction(&mut checkpoint, &raw_transaction("b")).unwrap();
        let FanoutAction::FetchSignatures {
            query: second_query,
        } = next_fanout_action(&spec(), &checkpoint)
        else {
            unreachable!()
        };
        apply_signature_page(
            &mut checkpoint,
            &SignaturePage {
                provider: ProviderKind::Helius,
                role: SourceRole::RawIndependent,
                query: second_query,
                records: vec![signature("b", 2), signature("c", 3)],
                next_before: None,
                response_digest: "blake3:second-page".to_string(),
                metrics: metrics(ProviderKind::Helius, "getSignaturesForAddress"),
            },
        );

        assert_eq!(checkpoint.signatures_seen, 4);
        assert_eq!(checkpoint.unique_signatures, 3);
        assert_eq!(checkpoint.pending_signatures.len(), 2);
        assert!(checkpoint.signature_source_exhausted);
    }

    fn signature(value: &str, slot: u64) -> SignatureRecord {
        SignatureRecord {
            signature: value.to_string(),
            slot: Some(slot),
            block_time: None,
            confirmation_status: Some("finalized".to_string()),
            err: None,
        }
    }

    fn raw_transaction(signature: &str) -> RawTransactionObservation {
        RawTransactionObservation {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: TransactionQuery {
                cluster: "mainnet".to_string(),
                signature: signature.to_string(),
                commitment: Commitment::Finalized,
            },
            payload: json!({"signature": signature}),
            response_digest: format!("blake3:{signature}"),
            retrieved_at_unix_ms: 1,
            metrics: metrics(ProviderKind::Helius, "getTransaction"),
        }
    }

    #[test]
    fn retry_policy_is_bounded_and_honors_rate_limit_delay() {
        let mut checkpoint = FanoutCheckpoint::default();
        let policy = FanoutRetryPolicy {
            max_attempts: 2,
            base_backoff_ms: 100,
            max_backoff_ms: 500,
        };
        let failure = FanoutFailure {
            kind: FanoutFailureKind::RateLimited,
            detail: "fixture 429".to_string(),
            http_status: Some(429),
            retry_after_ms: Some(350),
        };
        assert_eq!(
            apply_fanout_failure(&mut checkpoint, &policy, failure.clone()),
            FanoutRetryDecision::Retry {
                attempt: 1,
                backoff_ms: 350
            }
        );
        assert_eq!(
            apply_fanout_failure(&mut checkpoint, &policy, failure.clone()),
            FanoutRetryDecision::Retry {
                attempt: 2,
                backoff_ms: 500
            }
        );
        assert_eq!(
            apply_fanout_failure(&mut checkpoint, &policy, failure),
            FanoutRetryDecision::Stop {
                reason: FanoutHaltReason::UnretryableFailure
            }
        );
        assert_eq!(checkpoint.retry_attempts, 2);
    }

    #[test]
    fn structural_failure_is_not_retried() {
        let mut checkpoint = FanoutCheckpoint::default();
        let decision = apply_fanout_failure(
            &mut checkpoint,
            &FanoutRetryPolicy::default(),
            FanoutFailure {
                kind: FanoutFailureKind::InvalidResponse,
                detail: "fixture malformed response".to_string(),
                http_status: Some(200),
                retry_after_ms: None,
            },
        );
        assert_eq!(
            decision,
            FanoutRetryDecision::Stop {
                reason: FanoutHaltReason::UnretryableFailure
            }
        );
        assert_eq!(checkpoint.retry_attempts, 0);
    }

    #[test]
    fn source_errors_map_without_leaking_adapter_types() {
        let failure = FanoutFailure::from_source_error(&SourceError::RateLimited {
            retry_after_ms: Some(750),
        });
        assert_eq!(failure.kind, FanoutFailureKind::RateLimited);
        assert_eq!(failure.http_status, Some(429));
        assert_eq!(failure.retry_after_ms, Some(750));

        let failure = FanoutFailure::from_source_error(&SourceError::Http {
            status: 503,
            detail: "fixture unavailable".to_string(),
        });
        assert_eq!(failure.kind, FanoutFailureKind::Http);
        assert_eq!(failure.http_status, Some(503));
    }
}
