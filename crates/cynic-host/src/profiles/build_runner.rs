//! Build-runner host profile: principal + privilege only.
//!
//! No mesh DNS/WireGuard package or unit requirements — a CI/build host must
//! not inherit network-node substrate just because it shares a Debian family.

use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackagePolicy;
use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
use crate::ports::{
    HostContractPort, HostFilesystemPort, HostPackagePort, HostPrincipalPort, HostPrivilegePort,
    HostServicePort,
};
use crate::profiles::families::HostFamilyProfile;
use crate::runtime_layout::current_runtime_layout;
use cynic_types::paths::CynicPaths;

#[derive(Debug, Default, Clone, Copy)]
pub struct BuildRunnerHostProfile;

impl BuildRunnerHostProfile {
    pub fn family(&self) -> HostFamilyProfile {
        HostFamilyProfile::debian_systemd()
    }
}

impl HostPrincipalPort for BuildRunnerHostProfile {
    fn desired_principal(&self) -> crate::domain::principal::PrincipalPolicy {
        self.family().principal_policy()
    }
}

impl HostPrivilegePort for BuildRunnerHostProfile {
    fn desired_privilege(&self) -> crate::domain::privilege::PrivilegePolicy {
        self.family().privilege_policy()
    }
}

impl HostPackagePort for BuildRunnerHostProfile {
    fn desired_packages(&self) -> PackagePolicy {
        let family = self.family();
        let base = family.package_policy();
        PackagePolicy {
            manager: base.manager,
            policy_dir: base.policy_dir,
            source_projection_path: base.source_projection_path,
            approved_sources: base.approved_sources,
            provenance: base.provenance,
            promotion_policy: base.promotion_policy,
            // Build runners do not require mesh packages (coredns/wireguard).
            bootstrap_packages: vec![],
        }
    }
}

impl HostFilesystemPort for BuildRunnerHostProfile {
    fn desired_filesystem(&self) -> FilesystemPolicy {
        FilesystemPolicy {
            managed_paths: vec![],
        }
    }
}

impl HostServicePort for BuildRunnerHostProfile {
    fn desired_services(&self) -> ServicePolicy {
        ServicePolicy {
            manager: ServiceManagerKind::Systemd,
            unit_dir: current_runtime_layout().service_unit_dir,
            // The receiver unit is declared here so `service-reconcile`
            // owns its lifecycle (enable/start). Content is NOT declared
            // here: the unit file is rendered by
            // `cynic node service generate` and installed by the
            // operator/capability path; the reconciler never authors bytes.
            units: vec![ServiceUnitPolicy {
                service_id: "cynic-node-receiver".to_string(),
                unit_name: "cynic-node.service".to_string(),
                enable_on_boot: true,
                restart_mode: "always".to_string(),
                source: CynicPaths::state_dir()
                    .join("service-units/cynic-node.service")
                    .display()
                    .to_string(),
                health_port: Some(9090),
                process_name: Some("cynic-node".to_string()),
                supervisor_provider: None,
                health_kind: None,
            }],
            // The registry is the authority for what a node retires.
            retired_units: Vec::new(),
        }
    }
}

impl HostContractPort for BuildRunnerHostProfile {
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
    use super::BuildRunnerHostProfile;
    use crate::ports::{HostPackagePort, HostServicePort};

    #[test]
    fn build_runner_does_not_require_mesh_packages_or_units() {
        let profile = BuildRunnerHostProfile;
        assert!(profile.desired_packages().bootstrap_packages.is_empty());
    }

    #[test]
    fn build_runner_declares_the_node_receiver_unit() {
        let units = BuildRunnerHostProfile.desired_services().units;
        assert_eq!(units.len(), 1);
        let unit = &units[0];
        assert_eq!(unit.unit_name, "cynic-node.service");
        assert!(unit.enable_on_boot);
        assert_eq!(
            unit.source,
            "/var/lib/cynic/service-units/cynic-node.service"
        );
    }
}
