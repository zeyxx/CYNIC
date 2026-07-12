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

pub mod families;
pub mod network_node;
pub mod proxmox_hypervisor;

use network_node::NetworkNodeHostProfile;
use proxmox_hypervisor::ProxmoxHypervisorHostProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostProfile {
    NetworkNode,
    ProxmoxHypervisor,
}

impl HostProfile {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "network-node" | "network-baseline" => Ok(Self::NetworkNode),
            "proxmox-hypervisor" | "proxmox-host" => Ok(Self::ProxmoxHypervisor),
            other => Err(format!("unknown host profile: {other}")),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::NetworkNode => "network-node",
            Self::ProxmoxHypervisor => "proxmox-hypervisor",
        }
    }
}

impl HostPrincipalPort for HostProfile {
    fn desired_principal(&self) -> PrincipalPolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_principal(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_principal(),
        }
    }
}

impl HostPrivilegePort for HostProfile {
    fn desired_privilege(&self) -> PrivilegePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_privilege(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_privilege(),
        }
    }
}

impl HostPackagePort for HostProfile {
    fn desired_packages(&self) -> PackagePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_packages(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_packages(),
        }
    }
}

impl HostFilesystemPort for HostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_filesystem(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_filesystem(),
        }
    }
}

impl HostServicePort for HostProfile {
    fn desired_services(&self) -> ServicePolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_services(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_services(),
        }
    }
}

impl HostContractPort for HostProfile {
    fn declared_contracts(&self) -> Vec<SubstrateDomainContract> {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.declared_contracts(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.declared_contracts(),
        }
    }
}
