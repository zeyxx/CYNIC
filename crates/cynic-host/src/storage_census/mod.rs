mod growth;
mod local;
mod policy;
mod port;
mod process;
mod types;

pub use growth::{
    compare_snapshots, load_previous_snapshot_from_env, observe_snapshot, read_previous_snapshot,
};
pub use local::LocalStorageCensusAdapter;
pub use policy::decide_storage_census;
pub use port::StorageCensusObservationPort;
pub use process::observe_deleted_open_files;
pub use types::{
    CensusKnowledge, StorageCensusAction, StorageCensusCoverage, StorageCensusDecision,
    StorageCensusInputs, StorageCensusReport, StorageCensusSnapshot, StorageCensusState,
    StorageDeviceObservation, StorageFilesystemSnapshot, StorageGrowthObservation,
    assemble_storage_census,
};
