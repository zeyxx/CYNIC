use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackagePolicy;
use crate::domain::principal::PrincipalPolicy;
use crate::domain::privilege::PrivilegePolicy;
use crate::domain::services::ServicePolicy;
use crate::domain::shared::SubstrateDomainContract;
use crate::ports::{
    HostContractPort, HostFilesystemPort, HostPackagePort, HostPrincipalPort, HostPrivilegePort,
    HostServicePort,
};

pub mod build_runner;
pub mod families;
pub mod network_node;
pub mod proxmox_hypervisor;

use build_runner::BuildRunnerHostProfile;
use network_node::NetworkNodeHostProfile;
use proxmox_hypervisor::ProxmoxHypervisorHostProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostProfile {
    NetworkNode,
    AlpineOpenRcNetworkNode,
    ProxmoxHypervisor,
    /// CI / compile host: principal + privilege, no mesh substrate.
    BuildRunner,
}

impl HostProfile {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "network-node" | "network-baseline" => Ok(Self::NetworkNode),
            "alpine-openrc" | "network-node-openrc" => Ok(Self::AlpineOpenRcNetworkNode),
            "proxmox-hypervisor" | "proxmox-host" => Ok(Self::ProxmoxHypervisor),
            "build-runner" | "build_runner" => Ok(Self::BuildRunner),
            other => Err(format!("unknown host profile: {other}")),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::NetworkNode => "network-node",
            Self::AlpineOpenRcNetworkNode => "alpine-openrc",
            Self::ProxmoxHypervisor => "proxmox-hypervisor",
            Self::BuildRunner => "build-runner",
        }
    }
}

impl HostPrincipalPort for HostProfile {
    fn desired_principal(&self) -> PrincipalPolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_principal(),
            Self::AlpineOpenRcNetworkNode => {
                families::HostFamilyProfile::alpine_openrc().principal_policy()
            }
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_principal(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_principal(),
        }
    }
}

impl HostPrivilegePort for HostProfile {
    fn desired_privilege(&self) -> PrivilegePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_privilege(),
            Self::AlpineOpenRcNetworkNode => {
                families::HostFamilyProfile::alpine_openrc().privilege_policy()
            }
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_privilege(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_privilege(),
        }
    }
}

impl HostPackagePort for HostProfile {
    fn desired_packages(&self) -> PackagePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_packages(),
            Self::AlpineOpenRcNetworkNode => {
                families::HostFamilyProfile::alpine_openrc().package_policy()
            }
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_packages(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_packages(),
        }
    }
}

impl HostFilesystemPort for HostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_filesystem(),
            Self::AlpineOpenRcNetworkNode => NetworkNodeHostProfile.desired_filesystem(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_filesystem(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_filesystem(),
        }
    }
}

impl HostServicePort for HostProfile {
    fn desired_services(&self) -> ServicePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_services(),
            Self::AlpineOpenRcNetworkNode => {
                families::HostFamilyProfile::alpine_openrc().service_policy()
            }
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_services(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_services(),
        }
    }
}

impl HostContractPort for HostProfile {
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract> {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.declared_contracts(),
            Self::AlpineOpenRcNetworkNode => vec![
                self.desired_principal().contract(),
                self.desired_privilege().contract(),
                self.desired_packages().contract(),
                self.desired_filesystem().contract(),
                self.desired_services().contract(),
            ],
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.declared_contracts(),
            Self::BuildRunner => BuildRunnerHostProfile.declared_contracts(),
        }
    }
}
