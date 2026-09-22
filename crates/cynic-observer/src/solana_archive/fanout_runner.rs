use super::{
    FanoutAction, FanoutCheckpoint, FanoutFailure, FanoutHaltReason, FanoutRecordError,
    FanoutRetryDecision, FanoutRetryPolicy, FanoutSpec, FanoutStore, SignatureSource, SourceError,
    TransactionQuery, TransactionSource, next_fanout_action, record_fanout_failure,
    record_signature_page, record_transaction,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type BoxSleepFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub trait FanoutSleeper: Send + Sync {
    fn sleep(&self, duration: Duration) -> BoxSleepFuture;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokioFanoutSleeper;

impl FanoutSleeper for TokioFanoutSleeper {
    fn sleep(&self, duration: Duration) -> BoxSleepFuture {
        Box::pin(tokio::time::sleep(duration))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanoutRunError {
    Storage(super::StorageError),
    Record(FanoutRecordError),
}

impl From<FanoutRecordError> for FanoutRunError {
    fn from(error: FanoutRecordError) -> Self {
        Self::Record(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanoutRunReport {
    pub checkpoint: FanoutCheckpoint,
    pub stop_reason: FanoutHaltReason,
}

pub async fn run_fanout<Sig, Tx, Store, Sleep>(
    spec: &FanoutSpec,
    checkpoint: &mut FanoutCheckpoint,
    signatures: &Sig,
    transactions: &Tx,
    store: &Store,
    retry_policy: &FanoutRetryPolicy,
    sleeper: &Sleep,
) -> Result<FanoutRunReport, FanoutRunError>
where
    Sig: SignatureSource,
    Tx: TransactionSource,
    Store: FanoutStore,
    Sleep: FanoutSleeper,
{
    loop {
        match next_fanout_action(spec, checkpoint) {
            FanoutAction::FetchSignatures { query } => {
                let result = signatures.fetch_signatures(query).await;
                match result {
                    Ok(page) => {
                        record_signature_page(store, spec, checkpoint, &page)
                            .map_err(FanoutRunError::from)?;
                    }
                    Err(error) => {
                        if let Some(report) =
                            handle_failure(spec, checkpoint, store, retry_policy, sleeper, error)
                                .await?
                        {
                            return Ok(report);
                        }
                    }
                }
            }
            FanoutAction::FetchTransaction { signature } => {
                let query = TransactionQuery {
                    cluster: spec.cluster.clone(),
                    signature,
                    commitment: spec.commitment.clone(),
                };
                let result = transactions.fetch_transaction(query).await;
                match result {
                    Ok(transaction) => {
                        record_transaction(store, spec, checkpoint, &transaction)
                            .map_err(FanoutRunError::from)?;
                    }
                    Err(error) => {
                        if let Some(report) =
                            handle_failure(spec, checkpoint, store, retry_policy, sleeper, error)
                                .await?
                        {
                            return Ok(report);
                        }
                    }
                }
            }
            FanoutAction::Stop { reason } => {
                return Ok(FanoutRunReport {
                    checkpoint: checkpoint.clone(),
                    stop_reason: reason,
                });
            }
        }
    }
}

async fn handle_failure<Store, Sleep>(
    spec: &FanoutSpec,
    checkpoint: &mut FanoutCheckpoint,
    store: &Store,
    retry_policy: &FanoutRetryPolicy,
    sleeper: &Sleep,
    error: SourceError,
) -> Result<Option<FanoutRunReport>, FanoutRunError>
where
    Store: FanoutStore,
    Sleep: FanoutSleeper,
{
    let failure = FanoutFailure::from_source_error(&error);
    let decision = record_fanout_failure(store, spec, checkpoint, retry_policy, failure)
        .map_err(FanoutRunError::Record)?;
    match decision {
        FanoutRetryDecision::Retry { backoff_ms, .. } => {
            sleeper.sleep(Duration::from_millis(backoff_ms)).await;
            Ok(None)
        }
        FanoutRetryDecision::Stop { reason } => {
            checkpoint.completed = true;
            checkpoint.halt_reason = Some(reason.clone());
            store
                .persist_checkpoint(spec, checkpoint)
                .map_err(FanoutRunError::Storage)?;
            Ok(Some(FanoutRunReport {
                checkpoint: checkpoint.clone(),
                stop_reason: reason,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        BoxSourceFuture, Commitment, FanoutStore, ProviderKind, RawTransactionObservation,
        RequestMetrics, SignaturePage, SignatureQuery, SignatureRecord, SourceRole, StorageError,
        StoredPageReceipt, TransactionQuery,
    };
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone, Default)]
    struct FakeSignatures {
        responses: Arc<Mutex<VecDeque<Result<SignaturePage, SourceError>>>>,
    }

    impl SignatureSource for FakeSignatures {
        fn fetch_signatures(&self, _query: SignatureQuery) -> BoxSourceFuture<SignaturePage> {
            let responses = Arc::clone(&self.responses);
            Box::pin(async move {
                responses
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("fixture signature response")
            })
        }
    }

    #[derive(Clone, Default)]
    struct FakeTransactions {
        responses: Arc<Mutex<VecDeque<Result<RawTransactionObservation, SourceError>>>>,
    }

    impl TransactionSource for FakeTransactions {
        fn fetch_transaction(
            &self,
            _query: TransactionQuery,
        ) -> BoxSourceFuture<RawTransactionObservation> {
            let responses = Arc::clone(&self.responses);
            Box::pin(async move {
                responses
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("fixture transaction response")
            })
        }
    }

    #[derive(Clone, Default)]
    struct MemoryStore {
        checkpoint: Arc<Mutex<Option<FanoutCheckpoint>>>,
    }

    impl FanoutStore for MemoryStore {
        fn persist_signature_page(
            &self,
            _spec: &FanoutSpec,
            _page_index: u32,
            _page: &SignaturePage,
        ) -> Result<StoredPageReceipt, StorageError> {
            Ok(receipt("signature-page.json"))
        }

        fn persist_transaction(
            &self,
            _spec: &FanoutSpec,
            _transaction: &RawTransactionObservation,
        ) -> Result<StoredPageReceipt, StorageError> {
            Ok(receipt("transaction.json"))
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

    #[derive(Clone, Default)]
    struct NoopSleeper {
        durations: Arc<Mutex<Vec<Duration>>>,
    }

    impl FanoutSleeper for NoopSleeper {
        fn sleep(&self, duration: Duration) -> BoxSleepFuture {
            self.durations.lock().unwrap().push(duration);
            Box::pin(async {})
        }
    }

    fn receipt(path: &str) -> StoredPageReceipt {
        StoredPageReceipt {
            relative_path: path.to_string(),
            content_digest: "blake3:runner-fixture".to_string(),
            already_present: false,
        }
    }

    fn spec() -> FanoutSpec {
        FanoutSpec {
            cluster: "mainnet".to_string(),
            target: "runner-fixture".to_string(),
            signature_page_limit: 1,
            max_signature_pages: 1,
            max_transactions: 1,
            commitment: Commitment::Finalized,
        }
    }

    fn rate_limited() -> SourceError {
        SourceError::RateLimited {
            retry_after_ms: Some(25),
        }
    }

    fn page() -> SignaturePage {
        SignaturePage {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: SignatureQuery {
                cluster: "mainnet".to_string(),
                target: "runner-fixture".to_string(),
                before: None,
                until: None,
                limit: 1,
                commitment: Commitment::Finalized,
            },
            records: vec![SignatureRecord {
                signature: "runner-signature".to_string(),
                slot: Some(42),
                block_time: None,
                confirmation_status: Some("finalized".to_string()),
                err: None,
            }],
            next_before: None,
            response_digest: "blake3:runner-page".to_string(),
            metrics: RequestMetrics::unmetered(ProviderKind::Helius, "getSignaturesForAddress"),
        }
    }

    fn transaction() -> RawTransactionObservation {
        RawTransactionObservation {
            provider: ProviderKind::Helius,
            role: SourceRole::RawIndependent,
            query: TransactionQuery {
                cluster: "mainnet".to_string(),
                signature: "runner-signature".to_string(),
                commitment: Commitment::Finalized,
            },
            payload: json!({"signature": "runner-signature"}),
            response_digest: "blake3:runner-transaction".to_string(),
            retrieved_at_unix_ms: 42,
            metrics: RequestMetrics::unmetered(ProviderKind::Helius, "getTransaction"),
        }
    }

    #[tokio::test]
    async fn runner_retries_each_source_and_finishes_with_persisted_proof() {
        let signatures = FakeSignatures {
            responses: Arc::new(Mutex::new(VecDeque::from([
                Err(rate_limited()),
                Ok(page()),
            ]))),
        };
        let transactions = FakeTransactions {
            responses: Arc::new(Mutex::new(VecDeque::from([
                Err(rate_limited()),
                Ok(transaction()),
            ]))),
        };
        let store = MemoryStore::default();
        let sleeper = NoopSleeper::default();
        let mut checkpoint = FanoutCheckpoint::default();

        let report = run_fanout(
            &spec(),
            &mut checkpoint,
            &signatures,
            &transactions,
            &store,
            &FanoutRetryPolicy {
                max_attempts: 2,
                base_backoff_ms: 10,
                max_backoff_ms: 100,
            },
            &sleeper,
        )
        .await
        .unwrap();

        assert_eq!(report.stop_reason, FanoutHaltReason::Exhausted);
        assert!(report.checkpoint.completed);
        assert_eq!(report.checkpoint.transactions_fetched, 1);
        assert_eq!(checkpoint, report.checkpoint);
        assert_eq!(
            sleeper.durations.lock().unwrap().as_slice(),
            [Duration::from_millis(25), Duration::from_millis(25)]
        );
        assert_eq!(
            store.load_checkpoint(&spec()).unwrap(),
            Some(report.checkpoint)
        );
    }
}
