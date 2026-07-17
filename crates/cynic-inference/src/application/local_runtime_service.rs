//! Application service for **native** local inference control.
//!
//! Separated from ODS discovery (`sync_discovered_backends`):
//! - `list_inventory` / `status` / `start` / `stop` = native
//! - ODS discover/publish = integration adapter only
//!
//! Supervisor is **not** hardcoded to systemd. Select with
//! `CYNIC_INFERENCE_SUPERVISOR=process|systemd` (see
//! [`crate::adapters::local::process_supervisor::InferenceSupervisorKind`]).

use crate::adapters::local::{
    FsLocalModelInventory, InferenceSupervisorKind, ProcessLocalRuntime, SystemdUserLocalRuntime,
};
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
    /// systemd --user template units (`llama-server@`).
    pub fn from_systemd() -> Self {
        let inventory = FsLocalModelInventory::from_env();
        let runtime = SystemdUserLocalRuntime::with_inventory(FsLocalModelInventory::from_env());
        Self { inventory, runtime }
    }
}

impl LocalRuntimeService<FsLocalModelInventory, ProcessLocalRuntime> {
    /// Pidfile/process supervisor — no systemd required.
    pub fn from_process() -> Self {
        let inventory = FsLocalModelInventory::from_env();
        let runtime = ProcessLocalRuntime::with_inventory(FsLocalModelInventory::from_env());
        Self { inventory, runtime }
    }
}

/// Enum dispatch so CLI can pick supervisor without monomorphizing every call site.
#[derive(Debug)]
pub enum LocalRuntimeServiceAny {
    Systemd(LocalRuntimeService<FsLocalModelInventory, SystemdUserLocalRuntime>),
    Process(LocalRuntimeService<FsLocalModelInventory, ProcessLocalRuntime>),
}

impl LocalRuntimeServiceAny {
    pub fn from_env() -> Self {
        match InferenceSupervisorKind::from_env() {
            InferenceSupervisorKind::Process => Self::Process(LocalRuntimeService::from_process()),
            InferenceSupervisorKind::SystemdUser => {
                Self::Systemd(LocalRuntimeService::from_systemd())
            }
        }
    }

    pub fn supervisor_kind(&self) -> InferenceSupervisorKind {
        match self {
            Self::Systemd(_) => InferenceSupervisorKind::SystemdUser,
            Self::Process(_) => InferenceSupervisorKind::Process,
        }
    }

    pub fn list_inventory(&self) -> Result<LocalModelInventory, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.list_inventory(),
            Self::Process(s) => s.list_inventory(),
        }
    }

    pub fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.status(),
            Self::Process(s) => s.status(),
        }
    }

    pub fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.start(alias),
            Self::Process(s) => s.start(alias),
        }
    }

    pub fn stop(
        &self,
        alias: Option<&str>,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.stop(alias),
            Self::Process(s) => s.stop(alias),
        }
    }
}

impl LocalRuntimeService<FsLocalModelInventory, SystemdUserLocalRuntime> {
    /// Default host wiring: env-selected supervisor when callers still expect
    /// the old monomorphic type — prefer [`LocalRuntimeServiceAny::from_env`].
    pub fn from_env() -> Self {
        Self::from_systemd()
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
