use super::{
    apply_history_page, BackfillCheckpoint, BackfillSpec, BackfillStore, IndexedHistoryPage,
    StorageError, StoredPageReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillRecordError {
    CursorMismatch,
    Storage(StorageError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillRecordReceipt {
    pub page: StoredPageReceipt,
    pub checkpoint: BackfillCheckpoint,
}

/// Records a provider page before advancing the mutable checkpoint. If the
/// checkpoint write fails, the append-only page remains replayable and the
/// same operation can be retried idempotently.
pub fn record_history_page<S: BackfillStore>(
    store: &S,
    spec: &BackfillSpec,
    checkpoint: &mut BackfillCheckpoint,
    page: &IndexedHistoryPage,
) -> Result<BackfillRecordReceipt, BackfillRecordError> {
    if page.query.pagination_token != checkpoint.pagination_token {
        return Err(BackfillRecordError::CursorMismatch);
    }

    let page_receipt = store
        .persist_page(spec, checkpoint.pages_fetched, page)
        .map_err(BackfillRecordError::Storage)?;
    let mut next_checkpoint = checkpoint.clone();
    apply_history_page(&mut next_checkpoint, page);
    store
        .persist_checkpoint(spec, &next_checkpoint)
        .map_err(BackfillRecordError::Storage)?;
    *checkpoint = next_checkpoint.clone();

    Ok(BackfillRecordReceipt {
        page: page_receipt,
        checkpoint: next_checkpoint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana_archive::{
        Commitment, HistoryFilters, ProviderKind, RequestMetrics, SortOrder, SourceRole,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore {
        checkpoint: Mutex<Option<BackfillCheckpoint>>,
    }

    impl BackfillStore for MemoryStore {
        fn persist_page(
            &self,
            _spec: &BackfillSpec,
            page_index: u32,
            _page: &IndexedHistoryPage,
        ) -> Result<StoredPageReceipt, StorageError> {
            Ok(StoredPageReceipt {
                relative_path: format!("pages/{page_index}.json"),
                content_digest: "blake3:digest".to_string(),
                already_present: false,
            })
        }

        fn persist_checkpoint(
            &self,
            _spec: &BackfillSpec,
            checkpoint: &BackfillCheckpoint,
        ) -> Result<(), StorageError> {
            *self.checkpoint.lock().unwrap() = Some(checkpoint.clone());
            Ok(())
        }

        fn load_checkpoint(
            &self,
            _spec: &BackfillSpec,
        ) -> Result<Option<BackfillCheckpoint>, StorageError> {
            Ok(self.checkpoint.lock().unwrap().clone())
        }
    }

    fn spec() -> BackfillSpec {
        BackfillSpec {
            cluster: "mainnet".to_string(),
            target: "mint".to_string(),
            page_limit: 100,
            max_pages: 1,
            max_records: 100,
            sort_order: SortOrder::Ascending,
            commitment: Commitment::Finalized,
            filters: HistoryFilters::default(),
        }
    }

    #[test]
    fn checkpoint_advances_only_after_page_is_recorded() {
        let query =
            match super::super::next_backfill_action(&spec(), &BackfillCheckpoint::default()) {
                super::super::BackfillAction::FetchIndexedHistory { query } => query,
                _ => unreachable!(),
            };
        let page = IndexedHistoryPage {
            provider: ProviderKind::Helius,
            role: SourceRole::IndexedPrimary,
            query,
            records: Vec::new(),
            next_pagination_token: None,
            response_digest: "blake3:provider".to_string(),
            metrics: RequestMetrics::unmetered(ProviderKind::Helius, "getTransactionsForAddress"),
        };
        let store = MemoryStore::default();
        let mut checkpoint = BackfillCheckpoint::default();
        record_history_page(&store, &spec(), &mut checkpoint, &page).unwrap();
        assert!(checkpoint.completed);
        assert_eq!(checkpoint.pages_fetched, 1);
        assert!(store.load_checkpoint(&spec()).unwrap().is_some());
    }

    #[test]
    fn wrong_page_cursor_is_rejected_before_persistence() {
        let mut checkpoint = BackfillCheckpoint {
            pagination_token: Some("expected".to_string()),
            ..BackfillCheckpoint::default()
        };
        let page = IndexedHistoryPage {
            provider: ProviderKind::Helius,
            role: SourceRole::IndexedPrimary,
            query: crate::solana_archive::IndexedHistoryQuery {
                cluster: "mainnet".to_string(),
                target: "mint".to_string(),
                pagination_token: Some("wrong".to_string()),
                limit: 100,
                sort_order: SortOrder::Ascending,
                commitment: Commitment::Finalized,
                filters: HistoryFilters::default(),
            },
            records: Vec::new(),
            next_pagination_token: None,
            response_digest: "blake3:provider".to_string(),
            metrics: RequestMetrics::unmetered(ProviderKind::Helius, "getTransactionsForAddress"),
        };
        assert_eq!(
            record_history_page(&MemoryStore::default(), &spec(), &mut checkpoint, &page),
            Err(BackfillRecordError::CursorMismatch)
        );
    }
}
