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
    DockerLocalRuntime, FsLocalModelInventory, InferenceSupervisorKind, ProcessLocalRuntime,
    SystemdUserLocalRuntime,
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

impl LocalRuntimeService<FsLocalModelInventory, DockerLocalRuntime> {
    /// Docker supervisor.
    pub fn from_docker() -> Self {
        let inventory = FsLocalModelInventory::from_env();
        let runtime = DockerLocalRuntime::with_inventory(FsLocalModelInventory::from_env());
        Self { inventory, runtime }
    }
}

/// Enum dispatch so CLI can pick supervisor without monomorphizing every call site.
#[derive(Debug)]
pub enum LocalRuntimeServiceAny {
    Systemd(LocalRuntimeService<FsLocalModelInventory, SystemdUserLocalRuntime>),
    Process(LocalRuntimeService<FsLocalModelInventory, ProcessLocalRuntime>),
    Docker(LocalRuntimeService<FsLocalModelInventory, DockerLocalRuntime>),
}

impl LocalRuntimeServiceAny {
    pub fn from_env() -> Self {
        match InferenceSupervisorKind::from_env() {
            InferenceSupervisorKind::Process => Self::Process(LocalRuntimeService::from_process()),
            InferenceSupervisorKind::SystemdUser => {
                Self::Systemd(LocalRuntimeService::from_systemd())
            }
            InferenceSupervisorKind::Docker => Self::Docker(LocalRuntimeService::from_docker()),
        }
    }

    pub fn supervisor_kind(&self) -> InferenceSupervisorKind {
        match self {
            Self::Systemd(_) => InferenceSupervisorKind::SystemdUser,
            Self::Process(_) => InferenceSupervisorKind::Process,
            Self::Docker(_) => InferenceSupervisorKind::Docker,
        }
    }

    pub fn list_inventory(&self) -> Result<LocalModelInventory, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.list_inventory(),
            Self::Process(s) => s.list_inventory(),
            Self::Docker(s) => s.list_inventory(),
        }
    }

    pub fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.status(),
            Self::Process(s) => s.status(),
            Self::Docker(s) => s.status(),
        }
    }

    pub fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.start_with_readiness(alias, true)
    }

    pub fn start_with_readiness(
        &self,
        alias: &str,
        wait_for_ready: bool,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        let mut model_size = None;
        let mut model_path = None;
        if let Ok(inventory) = self.list_inventory() {
            if let Some(p) = inventory.models.iter().find(|p| p.alias == alias) {
                model_size = p.size_bytes;
                model_path = Some(p.model_path.clone());
            }
        }

        if let Some(path) = model_path {
            let size = model_size
                .unwrap_or_else(|| std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
            if size > 0 {
                let hw = crate::domain::hardware_profile::HardwareProfile::probe();
                if hw.absolute_max_weight_bytes > 0 && size > hw.absolute_max_weight_bytes {
                    return Err(LocalRuntimeError::Rejected(format!(
                        "Model weight size ({:.2} GB) exceeds absolute memory budget of the host ({:.2} GB). Start rejected to prevent OOM crash.",
                        size as f64 / (1024.0 * 1024.0 * 1024.0),
                        hw.absolute_max_weight_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                    )));
                }
            }
        }

        match self {
            Self::Systemd(s) => s.runtime.start_with_readiness(alias, wait_for_ready),
            Self::Process(s) => s.runtime.start_with_readiness(alias, wait_for_ready),
            Self::Docker(s) => s.runtime.start_with_readiness(alias, wait_for_ready),
        }
    }

    pub fn stop(
        &self,
        alias: Option<&str>,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        match self {
            Self::Systemd(s) => s.runtime.stop(alias),
            Self::Process(s) => s.runtime.stop(alias),
            Self::Docker(s) => s.runtime.stop(alias),
        }
    }
}

impl LocalInferenceRuntimePort for LocalRuntimeServiceAny {
    fn status(&self) -> Result<LocalRuntimeStatus, LocalRuntimeError> {
        self.status()
    }

    fn start(&self, alias: &str) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.start(alias)
    }

    fn start_with_readiness(
        &self,
        alias: &str,
        wait_for_ready: bool,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.start_with_readiness(alias, wait_for_ready)
    }

    fn stop(&self, alias: Option<&str>) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.stop(alias)
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

    pub fn start_with_readiness(
        &self,
        alias: &str,
        wait_for_ready: bool,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.runtime.start_with_readiness(alias, wait_for_ready)
    }

    pub fn stop(
        &self,
        alias: Option<&str>,
    ) -> Result<LocalRuntimeMutationReport, LocalRuntimeError> {
        self.runtime.stop(alias)
    }
}
