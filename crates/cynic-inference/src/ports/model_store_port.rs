//! Port for the centralized model store (layout + artifacts + profiles).

use crate::domain::model_store::ModelStoreReport;
use crate::ports::local_runtime_port::LocalRuntimeError;

pub trait ModelStorePort: Send + Sync {
    /// Scan vault, profiles, and known caches into one report.
    fn scan(&self) -> Result<ModelStoreReport, LocalRuntimeError>;

    /// Persist the scan to `ModelLayout.registry_path` (central registry file).
    fn persist_registry(&self, report: &ModelStoreReport) -> Result<(), LocalRuntimeError>;
}
