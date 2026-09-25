pub mod argv_execution;
pub mod causal_epoch_index;
pub mod evidence_store;
pub mod metabolism;
pub mod provider_substrate;
pub mod remote_execution;
use crate::bridge::{PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState};
use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackageObservedState;
use crate::domain::packages::PackagePolicy;
use crate::domain::principal::PrincipalObservedState;
use crate::domain::principal::PrincipalPolicy;
use crate::domain::privilege::PrivilegeObservedState;
use crate::domain::privilege::PrivilegePolicy;
use crate::domain::services::ServiceObservedState;
use crate::domain::services::ServicePolicy;
use crate::domain::shared::SubstrateDomainContract;
use crate::execution::AdmittedHostOperation;
use crate::hardware_conformance::HardwareObservation;
use cynic_protocol::PreOsPlatformObservation;
pub use metabolism::{MetabolicState, MetabolismProvider, OsMetabolism};
pub use provider_substrate::{NodeSubstrateControlPort, NodeSubstrateObservationPort};
pub use remote_execution::RemoteExecutionPort;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostOperation {
    EnsureSystemGroup {
        group: String,
    },
    EnsureSystemUser {
        user: String,
        group: String,
        home: String,
        shell: PrincipalLoginShell,
    },
    EnsureSupplementaryGroupMember {
        user: String,
        group: String,
    },
    EnsureOwnedDir {
        path: String,
        user: String,
        group: String,
        mode: u32,
    },
    InstallOwnedFile {
        source: String,
        target: String,
        user: String,
        group: String,
        mode: u32,
    },
    ValidatePrivilegeProjection {
        provider: String,
        path: String,
    },
    EnsureDir {
        path: String,
        mode: u32,
    },
    WritePackageSourceProjection {
        path: String,
        approved_sources: Vec<String>,
    },
    InstallBootstrapPackages {
        approved_sources: Vec<String>,
        packages: Vec<String>,
    },
    ServiceManagerDaemonReload,
    ServiceEnableNow {
        unit: String,
    },
    ServiceStart {
        unit: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalLoginShell {
    NoLogin,
}

pub trait HostPrincipalPort {
    fn desired_principal(&self) -> PrincipalPolicy;
}

pub trait HostPrincipalObservationPort {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState;
}

pub trait HostPrivilegePort {
    fn desired_privilege(&self) -> PrivilegePolicy;
}

pub trait HostPrivilegeObservationPort {
    fn observe_privilege(&self, policy: &PrivilegePolicy) -> PrivilegeObservedState;
}

pub trait HostPrivilegeBridgeObservationPort {
    fn observe_privilege_bridge(
        &self,
        spec: &PrivilegeBridgeInstallSpec,
    ) -> PrivilegeBridgeObservedState;
}

pub trait HostPackagePort {
    fn desired_packages(&self) -> PackagePolicy;
}

pub trait HostPackageObservationPort {
    fn observe_packages(&self, policy: &PackagePolicy) -> PackageObservedState;
}

pub trait HostFilesystemPort {
    fn desired_filesystem(&self) -> FilesystemPolicy;
}

pub trait HostServicePort {
    fn desired_services(&self) -> ServicePolicy;
}

pub trait HostServiceObservationPort {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState;
}

pub trait HostHardwareObservationPort {
    fn observe_hardware(&self) -> HardwareObservation;
}

pub trait PreOsPlatformObservationPort {
    fn observe_pre_os_platform(&self) -> PreOsPlatformObservation;
}

pub trait PreOsArtifactObservationPort {
    fn observe_pre_os_artifacts(&self) -> Vec<crate::pre_os_control::PreOsArtifactObservation>;
}

pub trait PreOsAttestationObservationPort {
    fn observe_pre_os_attestation_sink(
        &self,
    ) -> crate::pre_os_control::PreOsAttestationSinkObservation;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait HostCommandRunnerPort {
    fn run_operation(&self, operation: &AdmittedHostOperation) -> HostCommandResult;
}

pub trait HostContractPort:
    HostPrincipalPort + HostPrivilegePort + HostPackagePort + HostFilesystemPort + HostServicePort
{
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract>;
}
