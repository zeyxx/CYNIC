//! Application service for **native** local inference control.
//!
//! Separated from ODS discovery (`sync_discovered_backends`):
//! - `list_inventory` / `status` / `start` / `stop` = native
//! - ODS discover/publish = integration adapter only

use crate::adapters::local::{FsLocalModelInventory, SystemdUserLocalRuntime};
use crate::domain::local_runtime::{
    LocalModelInventory, LocalRuntimeMutationReport, LocalRuntimeStatus,
};
use crate::ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};

#[derive(Debug)]
pub struct LocalRuntimeService<I, R> {
    inventory: I,
    runtime: R,
}

impl LocalRuntimeService<FsLocalModelInventory, SystemdUserLocalRuntime> {
    /// Default host wiring: FS profiles + systemd --user template units.
    pub fn from_env() -> Self {
        let inventory = FsLocalModelInventory::from_env();
        let runtime = SystemdUserLocalRuntime::with_inventory(FsLocalModelInventory::from_env());
        Self { inventory, runtime }
    }
}

impl<I, R> LocalRuntimeService<I, R>
where
    I: LocalModelInventoryPort,
    R: LocalInferenceRuntimePort,
{
    pub fn new(inventory: I, runtime: R) -> Self {
        Self { inventory, runtime }
    }

    pub fn list_inventory(&self) -> Result<LocalModelInventory, LocalRuntimeError> {
        self.inventory.list_profiles()
    }

    pub fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        self.runtime.status()
    }

    pub fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.runtime.start(alias)
    }

    pub fn stop(
        &self,
        alias: Option<&str>,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.runtime.stop(alias)
    }
}
