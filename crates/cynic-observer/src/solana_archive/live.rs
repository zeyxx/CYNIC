use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use super::{Commitment, ProviderKind, RequestMetrics};

pub type BoxSourceFuture<T> = Pin<Box<dyn Future<Output = Result<T, SourceError>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    IndexedPrimary,
    RawIndependent,
    LiveNotification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HistoryFilters {
    pub start_slot: Option<u64>,
    pub end_slot: Option<u64>,
    pub status: Option<String>,
    pub token_accounts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedHistoryQuery {
    pub cluster: String,
    pub target: String,
    pub pagination_token: Option<String>,
    pub limit: u16,
    pub sort_order: SortOrder,
    pub commitment: Commitment,
    pub filters: HistoryFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedTransactionRecord {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub transaction_index: Option<u64>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexedHistoryPage {
    pub provider: ProviderKind,
    pub role: SourceRole,
    pub query: IndexedHistoryQuery,
    pub records: Vec<IndexedTransactionRecord>,
    pub next_pagination_token: Option<String>,
    pub response_digest: String,
    pub metrics: RequestMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureQuery {
    pub cluster: String,
    pub target: String,
    pub before: Option<String>,
    pub until: Option<String>,
    pub limit: u16,
    pub commitment: Commitment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignatureRecord {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub confirmation_status: Option<String>,
    pub err: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignaturePage {
    pub provider: ProviderKind,
    pub role: SourceRole,
    pub query: SignatureQuery,
    pub records: Vec<SignatureRecord>,
    pub next_before: Option<String>,
    pub response_digest: String,
    pub metrics: RequestMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionQuery {
    pub cluster: String,
    pub signature: String,
    pub commitment: Commitment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawTransactionObservation {
    pub provider: ProviderKind,
    pub role: SourceRole,
    pub query: TransactionQuery,
    pub payload: Value,
    pub response_digest: String,
    pub retrieved_at_unix_ms: u128,
    pub metrics: RequestMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceError {
    Transport(String),
    Http { status: u16, detail: String },
    Rpc { code: Option<i64>, detail: String },
    RateLimited { retry_after_ms: Option<u64> },
    InvalidResponse(String),
    Blocked(String),
}

pub trait SignatureSource: Send + Sync {
    fn fetch_signatures(&self, query: SignatureQuery) -> BoxSourceFuture<SignaturePage>;
}

pub trait TransactionSource: Send + Sync {
    fn fetch_transaction(
        &self,
        query: TransactionQuery,
    ) -> BoxSourceFuture<RawTransactionObservation>;
}

pub trait IndexedHistorySource: Send + Sync {
    fn fetch_history(&self, query: IndexedHistoryQuery) -> BoxSourceFuture<IndexedHistoryPage>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_query_keeps_provider_neutral_shape() {
        let query = SignatureQuery {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            before: None,
            until: None,
            limit: 1000,
            commitment: Commitment::Finalized,
        };
        assert_eq!(query.commitment, Commitment::Finalized);
        assert_eq!(query.limit, 1000);
    }
}
