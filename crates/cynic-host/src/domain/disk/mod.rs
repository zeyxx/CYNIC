//! Disk space mapping and clean sub-domain.

pub use cynic_protocol::host_substrate::disk::{
    DiskCacheEntry, DiskCleanupItem, DiskCleanupPlan, DiskCleanupResult, DiskCleanupStepExecution,
    DiskFileEntry, DiskFilesystem, DiskReport, PhysicalDisk, bytes_to_human,
};
