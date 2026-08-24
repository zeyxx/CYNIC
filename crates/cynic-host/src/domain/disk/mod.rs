//! Disk space mapping and clean sub-domain.

pub mod build_runner;
pub mod port;
pub mod probe;
pub mod types;

pub use port::{DiskCleanupExecutionPort, DiskObservationPort};
pub use probe::{
    build_cleanup_plan, parse_df_filesystems, parse_lsblk_disks, ROTATED_SYSLOG_FILES,
};
pub use types::{
    DiskCacheEntry, DiskCleanupItem, DiskCleanupPlan, DiskCleanupResult, DiskCleanupStepExecution,
    DiskFileEntry, DiskFilesystem, DiskReport, PhysicalDisk, bytes_to_human,
};
