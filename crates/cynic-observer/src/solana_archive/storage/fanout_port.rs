use super::{StorageError, StoredPageReceipt};
use crate::solana_archive::{
    FanoutCheckpoint, FanoutSpec, RawTransactionObservation, SignaturePage,
};

pub trait FanoutStore: Send + Sync {
    fn persist_signature_page(
        &self,
        spec: &FanoutSpec,
        page_index: u32,
        page: &SignaturePage,
    ) -> Result<StoredPageReceipt, StorageError>;

    fn persist_transaction(
        &self,
        spec: &FanoutSpec,
        transaction: &RawTransactionObservation,
    ) -> Result<StoredPageReceipt, StorageError>;

    fn persist_checkpoint(
        &self,
        spec: &FanoutSpec,
        checkpoint: &FanoutCheckpoint,
    ) -> Result<(), StorageError>;

    fn load_checkpoint(&self, spec: &FanoutSpec) -> Result<Option<FanoutCheckpoint>, StorageError>;
}
