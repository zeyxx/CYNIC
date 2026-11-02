//! Storage mutation port — re-exported from canonical owner cynic-proxmox.
pub use cynic_proxmox::storage::ports::{
    StorageMutationPort, StorageMutationRequest, StorageMutationResult,
    StorageOperationReadBack, StorageOperationReadBackPort, StorageOperationStatus,
};
