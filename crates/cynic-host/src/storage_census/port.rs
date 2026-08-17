use super::types::StorageCensusReport;

pub trait StorageCensusObservationPort: Send + Sync {
    fn observe_storage_census(&self) -> Result<StorageCensusReport, String>;
}
