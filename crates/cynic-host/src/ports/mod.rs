pub mod metabolism;
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
pub use metabolism::{MetabolicState, MetabolismProvider, OsMetabolism};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostOperation {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait HostCommandRunnerPort {
    fn run_operation(&self, operation: &HostOperation) -> HostCommandResult;
}

pub trait HostContractPort:
    HostPrincipalPort + HostPrivilegePort + HostPackagePort + HostFilesystemPort + HostServicePort
{
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract>;
}
