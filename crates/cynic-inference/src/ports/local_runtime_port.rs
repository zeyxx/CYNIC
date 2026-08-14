//! Ports for **native** CYNIC local inference control.
//!
//! | Port family | Meaning |
//! |---|---|
//! | **Native** (this module) | CYNIC-owned vault + lifecycle of declared engines |
//! | **ODS integration** (`adapters::ods`) | Consume [Osmantic ODS](https://github.com/Osmantic/ODS) (or compatible peers) as environment bootstrap — observe endpoints, publish into routing SSOT |
//!
//! CYNIC system ≠ ODS product. ODS bootstraps the AI environment; CYNIC owns
//! organism routing and optional native on-demand engines.

use crate::domain::local_runtime::{
    LocalModelInventory, LocalModelProfile, LocalRuntimeMutationReport, LocalRuntimeStatus,
};

#[derive(Debug, thiserror::Error)]
pub enum LocalRuntimeError {
    #[error("local model inventory unavailable: {0}")]
    Inventory(String),
    #[error("local runtime control failed: {0}")]
    Control(String),
    #[error("unknown local model profile: {0}")]
    UnknownProfile(String),
    #[error("local runtime rejected operation: {0}")]
    Rejected(String),
}

/// Read-only inventory of declared local models (vault + profiles).
pub trait LocalModelInventoryPort: Send + Sync {
    fn list_profiles(&self) -> Result<LocalModelInventory, LocalRuntimeError>;
    fn get_profile(&self, alias: &str) -> Result<LocalModelProfile, LocalRuntimeError>;
}

/// Lifecycle control for the native local inference engine instance.
pub trait LocalInferenceRuntimePort: Send + Sync {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError>;
    fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError>;
    fn start_with_readiness(
        &self,
        alias: &str,
        _wait_for_ready: bool,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.start(alias)
    }
    fn stop(&self, alias: Option<&str>) -> Result<LocalRuntimeMutationReport, LocalRuntimeError>;
}
