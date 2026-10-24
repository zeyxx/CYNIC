//! Disk space mapping and clean sub-domain.

pub mod attribution;
pub mod build_runner;
pub mod port;
pub mod probe;
pub mod types;

pub use attribution::{
    DiskClass, DiskClassAttribution, aggregate_disk_classes, classify_disk_path,
};
pub use port::{DiskCleanupExecutionPort, DiskObservationPort};
pub use probe::{
    ROTATED_SYSLOG_FILES, build_cleanup_plan, parse_df_filesystems, parse_lsblk_disks,
};
pub use types::{
    DiskCacheEntry, DiskCleanupItem, DiskCleanupPlan, DiskCleanupResult, DiskCleanupStepExecution,
    DiskFileEntry, DiskFilesystem, DiskReport, PhysicalDisk, bytes_to_human,
};
