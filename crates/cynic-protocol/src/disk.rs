//! Universal contract types for CYNIC disk observation and cleanup.
//!
//! No system/process execution here — only the shared vocabulary for
//! observe/plan/apply/proof. Lives in `cynic-protocol` (not `cynic-host`) so the
//! kernel, node, host, and CLI surfaces all reference one canonical definition
//! with never a circular dependency.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskFilesystem {
    pub source: String,
    pub fstype: String,
    pub size_human: String,
    pub used_human: String,
    pub avail_human: String,
    pub use_pct: String,
    pub mount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskFileEntry {
    pub path: String,
    pub size_human: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskCacheEntry {
    pub label: String,
    pub path: String,
    pub size_human: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskReport {
    pub schema: String,
    pub timestamp: String,
    pub filesystems: Vec<DiskFilesystem>,
    pub known_caches: Vec<DiskCacheEntry>,
    pub large_files: Vec<DiskFileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskCleanupItem {
    pub id: String,
    pub label: String,
    pub size_human: String,
    pub size_bytes: u64,
    /// The executable command to run (argv).
    pub argv: Vec<String>,
    pub requires_privilege: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskCleanupPlan {
    pub schema: String,
    pub items: Vec<DiskCleanupItem>,
    pub total_reclaimable_bytes: u64,
    pub total_reclaimable_human: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskCleanupStepExecution {
    pub item_id: String,
    pub label: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskCleanupResult {
    pub schema: String,
    pub timestamp: String,
    pub success: bool,
    pub items_executed: Vec<DiskCleanupStepExecution>,
    pub total_reclaimed_bytes: u64,
    pub total_reclaimed_human: String,
}

/// Helper to convert raw bytes to a human-readable string (G, M, K, etc.).
pub fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}
