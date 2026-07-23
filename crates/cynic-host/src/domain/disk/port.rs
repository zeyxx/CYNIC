//! Universal port interfaces for disk space observation and cleanup.

use super::types::{DiskCleanupPlan, DiskCleanupResult, DiskReport};

pub trait DiskObservationPort: Send + Sync {
    /// Observe host storage space (filesystems, caches, large files).
    fn observe_disk(&self) -> Result<DiskReport, String>;

    /// Build a cleanup plan based on an observed disk report.
    fn plan_cleanup(&self, report: &DiskReport) -> DiskCleanupPlan;
}

pub trait DiskCleanupExecutionPort: Send + Sync {
    /// Execute the proposed cleanup actions in a plan.
    fn execute_cleanup(&self, plan: &DiskCleanupPlan) -> DiskCleanupResult;
}
