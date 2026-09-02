use super::{
    apply_fanout_failure, apply_signature_page, apply_transaction, FanoutCheckpoint, FanoutFailure,
    FanoutFailureKind, FanoutRetryDecision, FanoutRetryPolicy, FanoutSpec, FanoutStore,
    RawTransactionObservation, SignaturePage, StorageError, StoredPageReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanoutRecordError {
    CursorMismatch,
    TransactionNotQueued,
    Storage(StorageError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePageRecordReceipt {
    pub page: StoredPageReceipt,
    pub checkpoint: FanoutCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionRecordReceipt {
    pub transaction: StoredPageReceipt,
    pub checkpoint: FanoutCheckpoint,
}

pub fn record_fanout_failure<S: FanoutStore>(
    store: &S,
    spec: &FanoutSpec,
    checkpoint: &mut FanoutCheckpoint,
    policy: &FanoutRetryPolicy,
    failure: FanoutFailure,
) -> Result<FanoutRetryDecision, FanoutRecordError> {
    let mut next = checkpoint.clone();
    let decision = apply_fanout_failure(&mut next, policy, failure);
    store
        .persist_checkpoint(spec, &next)
        .map_err(FanoutRecordError::Storage)?;
    *checkpoint = next;
    Ok(decision)
}

pub fn record_signature_page<S: FanoutStore>(
    store: &S,
    spec: &FanoutSpec,
    checkpoint: &mut FanoutCheckpoint,
    page: &SignaturePage,
) -> Result<SignaturePageRecordReceipt, FanoutRecordError> {
    if page.query.before != checkpoint.before_signature {
        return Err(FanoutRecordError::CursorMismatch);
    }
    let receipt = store
        .persist_signature_page(spec, checkpoint.signature_pages_fetched, page)
        .map_err(FanoutRecordError::Storage)?;
    let mut next = checkpoint.clone();
    apply_signature_page(&mut next, page);
    store
        .persist_checkpoint(spec, &next)
        .map_err(FanoutRecordError::Storage)?;
    *checkpoint = next.clone();
    Ok(SignaturePageRecordReceipt {
        page: receipt,
        checkpoint: next,
    })
}

pub fn record_transaction<S: FanoutStore>(
    store: &S,
    spec: &FanoutSpec,
    checkpoint: &mut FanoutCheckpoint,
    transaction: &RawTransactionObservation,
) -> Result<TransactionRecordReceipt, FanoutRecordError> {
    if !checkpoint
        .pending_signatures
        .iter()
        .any(|signature| signature == &transaction.query.signature)
    {
        return Err(FanoutRecordError::TransactionNotQueued);
    }
    let receipt = store
        .persist_transaction(spec, transaction)
        .map_err(FanoutRecordError::Storage)?;
    let mut next = checkpoint.clone();
    apply_transaction(&mut next, transaction)
        .map_err(|_| FanoutRecordError::TransactionNotQueued)?;
    store
        .persist_checkpoint(spec, &next)
        .map_err(FanoutRecordError::Storage)?;
    *checkpoint = next.clone();
    Ok(TransactionRecordReceipt {
        transaction: receipt,
        checkpoint: next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        Commitment, CreditConfidence, CreditMeasurement, FanoutStore, ProviderKind, RequestMetrics,
        SignatureQuery, SignatureRecord, SourceRole, TransactionQuery,
    };
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore {
        signature_pages: Mutex<Vec<SignaturePage>>,
        transactions: Mutex<Vec<RawTransactionObservation>>,
        checkpoint: Mutex<Option<FanoutCheckpoint>>,
    }

    impl FanoutStore for MemoryStore {
        fn persist_signature_page(
            &self,
            _spec: &FanoutSpec,
            page_index: u32,
            page: &SignaturePage,
        ) -> Result<StoredPageReceipt, StorageError> {
            self.signature_pages.lock().unwrap().push(page.clone());
            Ok(receipt(format!("signature-pages/{page_index}.json")))
        }

        fn persist_transaction(
            &self,
            _spec: &FanoutSpec,
            transaction: &RawTransactionObservation,
        ) -> Result<StoredPageReceipt, StorageError> {
            self.transactions.lock().unwrap().push(transaction.clone());
            Ok(receipt(format!(
                "transactions/{}.json",
                transaction.query.signature
            )))
        }

        fn persist_checkpoint(
            &self,
            _spec: &FanoutSpec,
            checkpoint: &FanoutCheckpoint,
        ) -> Result<(), StorageError> {
            *self.checkpoint.lock().unwrap() = Some(checkpoint.clone());
            Ok(())
        }

        fn load_checkpoint(
            &self,
            _spec: &FanoutSpec,
        ) -> Result<Option<FanoutCheckpoint>, StorageError> {
            Ok(self.checkpoint.lock().unwrap().clone())
        }
    }

    fn receipt(path: String) -> StoredPageReceipt {
        StoredPageReceipt {
            relative_path: path,
            content_digest: "blake3:fixture".to_string(),
            already_present: false,
        }
    }

    fn spec() -> FanoutSpec {
        FanoutSpec {
            cluster: "mainnet".to_string(),
            target: "fixture-mint".to_string(),
            signature_page_limit: 2,
            max_signature_pages: 1,
            max_transactions: 2,
            commitment: Commitment::Finalized,
        }
    }

    fn metrics(method: &str) -> RequestMetrics {
        let mut metrics = RequestMetrics::unmetered(ProviderKind::Helius, method);
        metrics.records_returned = if method == "getSignaturesForAddress" {
            2
        } else {
            1
        };
        metrics.credits = CreditMeasurement {
            credits: Some(1),
            confidence: CreditConfidence::CatalogEstimate,
            catalog_version: Some("fixture-catalog".to_string()),
        };
        metrics
    }

    fn page() -> SignaturePage {
        SignaturePage {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: SignatureQuery {
                cluster: "mainnet".to_string(),
                target: "fixture-mint".to_string(),
                before: None,
                until: None,
                limit: 2,
                commitment: Commitment::Finalized,
            },
            records: vec![
                SignatureRecord {
                    signature: "fixture-a".to_string(),
                    slot: Some(10),
                    block_time: None,
                    confirmation_status: Some("finalized".to_string()),
                    err: None,
                },
                SignatureRecord {
                    signature: "fixture-b".to_string(),
                    slot: Some(11),
                    block_time: None,
                    confirmation_status: Some("finalized".to_string()),
                    err: None,
                },
            ],
            next_before: None,
            response_digest: "blake3:signature-fixture".to_string(),
            metrics: metrics("getSignaturesForAddress"),
        }
    }

    fn transaction(signature: &str) -> RawTransactionObservation {
        RawTransactionObservation {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: TransactionQuery {
                cluster: "mainnet".to_string(),
                signature: signature.to_string(),
                commitment: Commitment::Finalized,
            },
            payload: json!({"signature": signature, "meta": {"err": null}}),
            response_digest: format!("blake3:{signature}-fixture"),
            retrieved_at_unix_ms: 1,
            metrics: metrics("getTransaction"),
        }
    }

    #[test]
    fn fixture_replay_is_resumable_between_signature_and_transaction_phases() {
        let store = MemoryStore::default();
        let spec = spec();
        let mut checkpoint = FanoutCheckpoint::default();

        record_signature_page(&store, &spec, &mut checkpoint, &page()).unwrap();
        assert!(checkpoint.signature_source_exhausted);
        assert_eq!(checkpoint.pending_signatures.len(), 2);
        assert_eq!(store.signature_pages.lock().unwrap().len(), 1);

        // Simulate a process restart after the page was committed.
        let mut resumed = store.load_checkpoint(&spec).unwrap().unwrap();
        record_transaction(&store, &spec, &mut resumed, &transaction("fixture-a")).unwrap();
        assert!(!resumed.completed);
        assert_eq!(
            resumed.pending_signatures.front().map(String::as_str),
            Some("fixture-b")
        );

        record_transaction(&store, &spec, &mut resumed, &transaction("fixture-b")).unwrap();
        assert!(resumed.completed);
        assert_eq!(resumed.transactions_fetched, 2);
        assert_eq!(resumed.metrics.requests, 3);
        assert_eq!(resumed.metrics.records, 4);
        assert_eq!(resumed.metrics.estimated_credits, 3);
        assert_eq!(store.transactions.lock().unwrap().len(), 2);
        assert_eq!(store.load_checkpoint(&spec).unwrap().unwrap(), resumed);
    }

    #[test]
    fn fixture_replay_rejects_transaction_not_in_checkpoint_queue() {
        let store = MemoryStore::default();
        let spec = spec();
        let mut checkpoint = FanoutCheckpoint::default();
        assert_eq!(
            record_transaction(&store, &spec, &mut checkpoint, &transaction("unknown")),
            Err(FanoutRecordError::TransactionNotQueued)
        );
        assert!(store.transactions.lock().unwrap().is_empty());
    }

    #[test]
    fn retry_decision_is_checkpointed_before_worker_retries() {
        let store = MemoryStore::default();
        let spec = spec();
        let mut checkpoint = FanoutCheckpoint::default();
        let decision = record_fanout_failure(
            &store,
            &spec,
            &mut checkpoint,
            &FanoutRetryPolicy::default(),
            FanoutFailure {
                kind: FanoutFailureKind::RateLimited,
                detail: "fixture rate limit".to_string(),
                http_status: Some(429),
                retry_after_ms: Some(1_000),
            },
        )
        .unwrap();
        assert_eq!(
            decision,
            FanoutRetryDecision::Retry {
                attempt: 1,
                backoff_ms: 1_000
            }
        );
        assert_eq!(store.load_checkpoint(&spec).unwrap().unwrap(), checkpoint);
        assert_eq!(checkpoint.retry_attempts, 1);
    }
}
