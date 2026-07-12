use crate::domain::packages::PackagePolicy;
use crate::domain::principal::PrincipalPolicy;
use crate::domain::privilege::PrivilegePolicy;
use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFamilyKind {
    DebianSystemd,
    ProxmoxHypervisor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFamilyProfile {
    pub kind: HostFamilyKind,
    pub config_dir: String,
    pub helper_dir: String,
    pub sudoers_dir: String,
    pub service_unit_dir: String,
    pub package_approved_sources: Vec<String>,
    pub bootstrap_packages: Vec<String>,
    pub service_units: Vec<ServiceUnitPolicy>,
}

impl HostFamilyProfile {
    pub fn debian_systemd() -> Self {
        let layout = current_runtime_layout();
        let wg_quick_unit = layout.wg_quick_unit();
        let dns_service_unit = layout.dns_service_unit();
        let resolved_service_unit = layout.resolved_service_unit();
        Self {
            kind: HostFamilyKind::DebianSystemd,
            config_dir: layout.config_dir.clone(),
            helper_dir: layout.helper_dir.clone(),
            sudoers_dir: layout.sudoers_dir.clone(),
            service_unit_dir: layout.service_unit_dir.clone(),
            package_approved_sources: vec![
                "distribution_package".to_string(),
                "approved_internal_package_mirror".to_string(),
            ],
            bootstrap_packages: vec![
                "wireguard-tools".to_string(),
                "iproute2".to_string(),
                "coredns".to_string(),
            ],
            service_units: vec![
                ServiceUnitPolicy {
                    service_id: "wireguard_mesh".to_string(),
                    unit_name: wg_quick_unit,
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:debian_systemd".to_string(),
                    health_port: None,
                    process_name: None,
                    supervisor_provider: None,
                    health_kind: None,
                },
                ServiceUnitPolicy {
                    service_id: "dns_service".to_string(),
                    unit_name: dns_service_unit,
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:debian_systemd".to_string(),
                    health_port: None,
                    process_name: Some("coredns".to_string()),
                    supervisor_provider: None,
                    health_kind: None,
                },
                ServiceUnitPolicy {
                    service_id: "system_resolver".to_string(),
                    unit_name: resolved_service_unit,
                    enable_on_boot: true,
                    restart_mode: "always".to_string(),
                    source: "profile:debian_systemd".to_string(),
                    health_port: None,
                    process_name: None,
                    supervisor_provider: None,
                    health_kind: None,
                },
            ],
        }
    }

    pub fn proxmox_hypervisor() -> Self {
        let layout = current_runtime_layout();
        Self {
            kind: HostFamilyKind::ProxmoxHypervisor,
            config_dir: layout.config_dir.clone(),
            helper_dir: layout.helper_dir.clone(),
            sudoers_dir: layout.sudoers_dir.clone(),
            service_unit_dir: layout.service_unit_dir.clone(),
            package_approved_sources: vec![
                "distribution_package".to_string(),
                "approved_internal_package_mirror".to_string(),
            ],
            bootstrap_packages: vec![
                "proxmox-ve".to_string(),
                "pve-manager".to_string(),
                "sudo".to_string(),
            ],
            service_units: vec![
                ServiceUnitPolicy {
                    service_id: "pvedaemon".to_string(),
                    unit_name: "pvedaemon.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:proxmox_hypervisor".to_string(),
                    health_port: None,
                    process_name: None,
                    supervisor_provider: None,
                    health_kind: None,
                },
                ServiceUnitPolicy {
                    service_id: "pveproxy".to_string(),
                    unit_name: "pveproxy.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:proxmox_hypervisor".to_string(),
                    health_port: None,
                    process_name: None,
                    supervisor_provider: None,
                    health_kind: None,
                },
                ServiceUnitPolicy {
                    service_id: "pvestatd".to_string(),
                    unit_name: "pvestatd.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:proxmox_hypervisor".to_string(),
                    health_port: None,
                    process_name: None,
                    supervisor_provider: None,
                    health_kind: None,
                },
                ServiceUnitPolicy {
                    service_id: "cynic-node".to_string(),
                    unit_name: "cynic-node.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "profile:proxmox_hypervisor".to_string(),
                    health_port: None,
                    process_name: Some("cynic-node".to_string()),
                    supervisor_provider: None,
                    health_kind: None,
                },
            ],
        }
    }

    pub fn package_policy(&self) -> PackagePolicy {
        PackagePolicy {
            policy_dir: self.config_dir.clone(),
            source_projection_path: format!("{}/package-sources.json", self.config_dir),
            approved_sources: self.package_approved_sources.clone(),
            bootstrap_packages: self.bootstrap_packages.clone(),
        }
    }

    pub fn service_policy(&self) -> ServicePolicy {
        ServicePolicy {
            manager: ServiceManagerKind::Systemd,
            unit_dir: self.service_unit_dir.clone(),
            units: self.service_units.clone(),
        }
    }

    pub fn principal_policy(&self) -> PrincipalPolicy {
        let layout = current_runtime_layout();
        let state_dirs = layout.state_dirs();
        PrincipalPolicy {
            runtime_user: "cynic".to_string(),
            runtime_group: "cynic".to_string(),
            home_dir: layout.runtime_home_dir,
            config_dir: self.config_dir.clone(),
            state_dirs,
        }
    }

    pub fn privilege_policy(&self) -> PrivilegePolicy {
        PrivilegePolicy {
            sudoers_subject: "cynic".to_string(),
            helper_dir: self.helper_dir.clone(),
            config_dir: self.config_dir.clone(),
            sudoers_dir: self.sudoers_dir.clone(),
            denial_proof_required: true,
            dedicated_helpers: self.dedicated_helpers(),
        }
    }

    fn dedicated_helpers(&self) -> Vec<String> {
        match self.kind {
            HostFamilyKind::DebianSystemd => vec![
                "cynic-network-actuator".to_string(),
                "cynic-service-actuator".to_string(),
                "cynic-dns-actuator".to_string(),
                "cynic-host-exec".to_string(),
            ],
            HostFamilyKind::ProxmoxHypervisor => vec!["cynic-host-exec".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HostFamilyKind, HostFamilyProfile};
    use crate::runtime_layout::current_runtime_layout;

    #[test]
    fn debian_systemd_family_declares_expected_runtime_surfaces() {
        let family = HostFamilyProfile::debian_systemd();
        assert_eq!(family.kind, HostFamilyKind::DebianSystemd);
        assert_eq!(family.config_dir, current_runtime_layout().config_dir);
        assert_eq!(family.helper_dir, current_runtime_layout().helper_dir);
        assert!(
            family
                .bootstrap_packages
                .contains(&"wireguard-tools".to_string())
        );
        assert!(
            family
                .service_units
                .iter()
                .any(|unit| unit.unit_name == "systemd-resolved.service")
        );
    }

    #[test]
    fn proxmox_hypervisor_family_declares_hypervisor_surfaces() {
        let family = HostFamilyProfile::proxmox_hypervisor();
        assert_eq!(family.kind, HostFamilyKind::ProxmoxHypervisor);
        assert!(
            family
                .bootstrap_packages
                .contains(&"proxmox-ve".to_string())
        );
        assert!(
            family
                .service_units
                .iter()
                .any(|unit| unit.unit_name == "pveproxy.service")
        );
    }
}
