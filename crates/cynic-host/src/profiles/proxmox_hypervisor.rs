use crate::domain::filesystem::FilesystemPolicy;
use crate::ports::{
    HostContractPort, HostFilesystemPort, HostPackagePort, HostPrincipalPort, HostPrivilegePort,
    HostServicePort,
};
use crate::profiles::families::HostFamilyProfile;

#[derive(Debug, Default, Clone, Copy)]
pub struct ProxmoxHypervisorHostProfile;

impl ProxmoxHypervisorHostProfile {
    pub fn family(&self) -> HostFamilyProfile {
        HostFamilyProfile::proxmox_hypervisor()
    }
}

impl HostPrincipalPort for ProxmoxHypervisorHostProfile {
    fn desired_principal(&self) -> crate::domain::principal::PrincipalPolicy {
        self.family().principal_policy()
    }
}

impl HostPrivilegePort for ProxmoxHypervisorHostProfile {
    fn desired_privilege(&self) -> crate::domain::privilege::PrivilegePolicy {
        self.family().privilege_policy()
    }
}

impl HostPackagePort for ProxmoxHypervisorHostProfile {
    fn desired_packages(&self) -> crate::domain::packages::PackagePolicy {
        self.family().package_policy()
    }
}

impl HostFilesystemPort for ProxmoxHypervisorHostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        FilesystemPolicy {
            managed_paths: vec![],
        }
    }
}

impl HostServicePort for ProxmoxHypervisorHostProfile {
    fn desired_services(&self) -> crate::domain::services::ServicePolicy {
        self.family().service_policy()
    }
}

impl HostContractPort for ProxmoxHypervisorHostProfile {
    fn declared_contracts(&self) -> Vec<crate::domain::shared::SubstrateDomainContract> {
        vec![
            self.desired_principal().contract(),
            self.desired_privilege().contract(),
            self.desired_packages().contract(),
            self.desired_filesystem().contract(),
            self.desired_services().contract(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::ProxmoxHypervisorHostProfile;
    use crate::ports::{HostPackagePort, HostPrivilegePort, HostServicePort};
    use crate::runtime_layout::current_runtime_layout;

    #[test]
    fn proxmox_hypervisor_profile_derives_hypervisor_runtime_from_family() {
        let profile = ProxmoxHypervisorHostProfile;
        let privilege = profile.desired_privilege();
        let packages = profile.desired_packages();
        let services = profile.desired_services();

        assert_eq!(privilege.helper_dir, current_runtime_layout().helper_dir);
        assert!(
            packages
                .bootstrap_packages
                .contains(&"proxmox-ve".to_string())
        );
        assert!(
            services
                .units
                .iter()
                .any(|unit| unit.unit_name == "pveproxy.service")
        );
    }
}
