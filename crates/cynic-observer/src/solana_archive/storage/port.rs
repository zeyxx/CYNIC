use super::{StorageError, StoredPageReceipt};
use crate::solana_archive::{
    BackfillCheckpoint, BackfillSpec, FanoutSpec, IndexedHistoryPage, SignaturePage,
};

pub trait BackfillStore: Send + Sync {
    fn persist_page(
        &self,
        spec: &BackfillSpec,
        page_index: u32,
        page: &IndexedHistoryPage,
    ) -> Result<StoredPageReceipt, StorageError>;

    fn persist_checkpoint(
        &self,
        spec: &BackfillSpec,
        checkpoint: &BackfillCheckpoint,
    ) -> Result<(), StorageError>;

    fn load_checkpoint(
        &self,
        spec: &BackfillSpec,
    ) -> Result<Option<BackfillCheckpoint>, StorageError>;
}

pub trait FanoutReadStore: Send + Sync {
    fn load_signature_pages(&self, spec: &FanoutSpec) -> Result<Vec<SignaturePage>, StorageError>;

    fn load_fanout_checkpoint(
        &self,
        spec: &FanoutSpec,
    ) -> Result<Option<crate::solana_archive::FanoutCheckpoint>, StorageError>;
}
