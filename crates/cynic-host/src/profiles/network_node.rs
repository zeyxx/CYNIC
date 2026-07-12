use crate::domain::filesystem::FilesystemPolicy;
use crate::ports::{
    HostContractPort, HostFilesystemPort, HostPackagePort, HostPrincipalPort, HostPrivilegePort,
    HostServicePort,
};
use crate::profiles::families::HostFamilyProfile;

#[derive(Debug, Default, Clone, Copy)]
pub struct NetworkNodeHostProfile;

impl NetworkNodeHostProfile {
    pub fn family(&self) -> HostFamilyProfile {
        HostFamilyProfile::debian_systemd()
    }
}

impl HostPrincipalPort for NetworkNodeHostProfile {
    fn desired_principal(&self) -> crate::domain::principal::PrincipalPolicy {
        self.family().principal_policy()
    }
}

impl HostPrivilegePort for NetworkNodeHostProfile {
    fn desired_privilege(&self) -> crate::domain::privilege::PrivilegePolicy {
        self.family().privilege_policy()
    }
}

impl HostPackagePort for NetworkNodeHostProfile {
    fn desired_packages(&self) -> crate::domain::packages::PackagePolicy {
        self.family().package_policy()
    }
}

impl HostFilesystemPort for NetworkNodeHostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        FilesystemPolicy {
            managed_paths: vec![],
        }
    }
}

impl HostServicePort for NetworkNodeHostProfile {
    fn desired_services(&self) -> crate::domain::services::ServicePolicy {
        self.family().service_policy()
    }
}

impl HostContractPort for NetworkNodeHostProfile {
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
    use super::NetworkNodeHostProfile;
    use crate::ports::{HostPackagePort, HostPrivilegePort, HostServicePort};
    use crate::runtime_layout::current_runtime_layout;

    #[test]
    fn network_node_profile_derives_distribution_runtime_from_family() {
        let profile = NetworkNodeHostProfile;
        let privilege = profile.desired_privilege();
        let packages = profile.desired_packages();
        let services = profile.desired_services();
        let runtime_layout = current_runtime_layout();

        assert_eq!(privilege.helper_dir, runtime_layout.helper_dir);
        assert!(
            packages
                .bootstrap_packages
                .contains(&"wireguard-tools".to_string())
        );
        assert!(
            services
                .units
                .iter()
                .any(|unit| unit.unit_name == runtime_layout.wg_quick_unit())
        );
    }
}
