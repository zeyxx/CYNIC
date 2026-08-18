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
    /// Fleet member with local runtime authority but no implicit mesh/DNS role.
    ManagementNode,
    /// CI / compile host: principal + privilege, no mesh substrate.
    BuildRunner,
    WindowsNode,
}

impl HostProfile {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "network-node" | "network-baseline" => Ok(Self::NetworkNode),
            "alpine-openrc" | "network-node-openrc" => Ok(Self::AlpineOpenRcNetworkNode),
            "proxmox-hypervisor" | "proxmox-host" => Ok(Self::ProxmoxHypervisor),
            "management-node" | "management_node" => Ok(Self::ManagementNode),
            "build-runner" | "build_runner" => Ok(Self::BuildRunner),
            "windows-node" | "windows" | "windows-service" => Ok(Self::WindowsNode),
            other => Err(format!("unknown host profile: {other}")),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::NetworkNode => "network-node",
            Self::AlpineOpenRcNetworkNode => "alpine-openrc",
            Self::ProxmoxHypervisor => "proxmox-hypervisor",
            Self::ManagementNode => "management-node",
            Self::BuildRunner => "build-runner",
            Self::WindowsNode => "windows-node",
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
            Self::ManagementNode => BuildRunnerHostProfile.desired_principal(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_principal(),
            Self::WindowsNode => families::HostFamilyProfile::windows_service().principal_policy(),
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
            Self::ManagementNode => BuildRunnerHostProfile.desired_privilege(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_privilege(),
            Self::WindowsNode => families::HostFamilyProfile::windows_service().privilege_policy(),
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
            Self::ManagementNode => BuildRunnerHostProfile.desired_packages(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_packages(),
            Self::WindowsNode => families::HostFamilyProfile::windows_service().package_policy(),
        }
    }
}

impl HostFilesystemPort for HostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        match self {
            Self::NetworkNode => NetworkNodeHostProfile.desired_filesystem(),
            Self::AlpineOpenRcNetworkNode => NetworkNodeHostProfile.desired_filesystem(),
            Self::ProxmoxHypervisor => ProxmoxHypervisorHostProfile.desired_filesystem(),
            Self::ManagementNode => BuildRunnerHostProfile.desired_filesystem(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_filesystem(),
            Self::WindowsNode => NetworkNodeHostProfile.desired_filesystem(),
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
            Self::ManagementNode => BuildRunnerHostProfile.desired_services(),
            Self::BuildRunner => BuildRunnerHostProfile.desired_services(),
            Self::WindowsNode => families::HostFamilyProfile::windows_service().service_policy(),
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
            Self::ManagementNode => BuildRunnerHostProfile.declared_contracts(),
            Self::BuildRunner => BuildRunnerHostProfile.declared_contracts(),
            Self::WindowsNode => vec![
                self.desired_principal().contract(),
                self.desired_privilege().contract(),
                self.desired_packages().contract(),
                self.desired_filesystem().contract(),
                self.desired_services().contract(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HostProfile;
    use crate::ports::{HostPackagePort, HostServicePort};

    #[test]
    fn management_node_is_a_fleet_profile_without_mesh_substrate() {
        let profile = HostProfile::ManagementNode;
        assert!(profile.desired_packages().bootstrap_packages.is_empty());
        assert!(profile.desired_services().units.is_empty());
    }
}
