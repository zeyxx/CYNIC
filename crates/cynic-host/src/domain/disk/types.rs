//! Disk contract types now live canonically in `cynic-protocol::disk` so the
//! kernel, node, host, and CLI all share one definition (structural invariant
//! #6: one canonical decision source per concern).
//!
//! This module re-exports those types for backwards-compatible access paths
//! (`cynic_host::domain::disk::DiskReport`, ...).

pub use cynic_protocol::disk::{
    bytes_to_human, DiskCacheEntry, DiskCleanupItem, DiskCleanupPlan, DiskCleanupResult,
    DiskCleanupStepExecution, DiskFileEntry, DiskFilesystem, DiskReport,
};
