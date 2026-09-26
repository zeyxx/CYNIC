use cynic_eval::convergence::{CausalEpochIndex, CausalEpochIndexAppend, CausalEpochIndexEntry};

pub trait CausalEpochIndexStorePort: Send + Sync {
    fn append(&self, entry: &CausalEpochIndexEntry) -> Result<CausalEpochIndexAppend, String>;

    fn read_index(&self) -> Result<CausalEpochIndex, String>;

    fn latest_for(
        &self,
        subject_id: &str,
        vertical_id: &str,
    ) -> Result<Option<CausalEpochIndexEntry>, String> {
        Ok(self
            .read_index()?
            .latest_for(subject_id, vertical_id)
            .cloned())
    }
}
