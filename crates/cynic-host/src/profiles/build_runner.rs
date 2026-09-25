//! Build-runner host profile: principal + privilege only.
//!
//! No mesh DNS/WireGuard package or unit requirements — a CI/build host must
//! not inherit network-node substrate just because it shares a Debian family.

use crate::domain::filesystem::FilesystemPolicy;
use crate::domain::packages::PackagePolicy;
use crate::domain::services::{ServiceManagerKind, ServicePolicy};
use crate::ports::{
    HostContractPort, HostFilesystemPort, HostPackagePort, HostPrincipalPort, HostPrivilegePort,
    HostServicePort,
};
use crate::profiles::families::HostFamilyProfile;
use crate::runtime_layout::current_runtime_layout;

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
            // Empty until the resident node controller owns a supervisor-agnostic lifecycle.
            units: vec![],
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
        assert!(profile.desired_services().units.is_empty());
    }
}
