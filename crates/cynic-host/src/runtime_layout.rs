use std::env;
use std::fs;
use std::path::PathBuf;

use crate::process_observation::command_stdout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCapabilities {
    pub identity: IdentityCapability,
    pub name_service: NameServiceCapability,
    pub service_manager: ServiceManagerCapability,
    pub package_manager: PackageManagerCapability,
    pub route_tool: RouteToolCapability,
    pub wireguard_tool: WireguardToolCapability,
    pub network_manager: NetworkManagerCapability,
    pub dns_provider: DnsProviderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCapability {
    pub hostname_path: String,
    pub hostname_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameServiceCapability {
    pub resolver_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManagerCapability {
    pub kind: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManagerCapability {
    pub installer: String,
    pub query: String,
    pub rpm_query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteToolCapability {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireguardToolCapability {
    pub command: String,
    pub mesh_interface: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkManagerCapability {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsProviderCapability {
    pub provider: String,
    pub command: String,
    pub service_unit: String,
    pub resolved_unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRuntimeLayout {
    pub config_dir: String,
    pub helper_dir: String,
    pub sudoers_dir: String,
    pub service_unit_dir: String,
    pub runtime_home_dir: String,
    pub state_dir: String,
    pub log_dir: String,
    pub host_exec_bundle_dir: String,
    pub network_actuator_bundle_dir: String,
    pub dns_bundle_dir: String,
    pub provider_fallback_bundle_dir: String,
}

impl HostRuntimeLayout {
    pub fn from_env() -> Self {
        Self {
            config_dir: env_path("CYNIC_CONFIG_ROOT", "/etc/cynic"),
            helper_dir: env_path("CYNIC_LIBEXEC_ROOT", "/usr/local/libexec"),
            sudoers_dir: env_path("CYNIC_SUDOERS_DIR", "/etc/sudoers.d"),
            service_unit_dir: env_path("CYNIC_SERVICE_UNIT_DIR", "/etc/systemd/system"),
            runtime_home_dir: env_path("CYNIC_RUNTIME_HOME", "/home/cynic"),
            state_dir: env_path("CYNIC_STATE_ROOT", "/var/lib/cynic"),
            log_dir: env_path("CYNIC_LOG_ROOT", "/var/log/cynic"),
            host_exec_bundle_dir: env_path("CYNIC_HOST_EXEC_BUNDLE_DIR", "/tmp/cynic-host-exec"),
            network_actuator_bundle_dir: env_path(
                "CYNIC_NETWORK_ACTUATOR_BUNDLE_DIR",
                "/tmp/cynic-network-actuator",
            ),
            dns_bundle_dir: env_path("CYNIC_DNS_BUNDLE_DIR", "/tmp/cynic-synapse-dns"),
            provider_fallback_bundle_dir: env_path(
                "CYNIC_PROVIDER_FALLBACK_BUNDLE_DIR",
                "/tmp/cynic-provider-fallback-hosts",
            ),
        }
    }

    pub fn state_dirs(&self) -> Vec<String> {
        vec![self.state_dir.clone(), self.log_dir.clone()]
    }

    pub fn host_exec_helper_path(&self) -> String {
        PathBuf::from(&self.helper_dir)
            .join("cynic-host-exec")
            .display()
            .to_string()
    }

    pub fn host_exec_sudoers_path(&self) -> String {
        PathBuf::from(&self.sudoers_dir)
            .join("cynic-host-exec")
            .display()
            .to_string()
    }

    pub fn host_exec_policy_path(&self) -> String {
        PathBuf::from(&self.config_dir)
            .join("host-exec.policy.json")
            .display()
            .to_string()
    }

    pub fn network_actuator_helper_path(&self) -> String {
        PathBuf::from(&self.helper_dir)
            .join("cynic-network-actuator")
            .display()
            .to_string()
    }

    pub fn network_actuator_sudoers_path(&self) -> String {
        PathBuf::from(&self.sudoers_dir)
            .join("cynic-network-actuator")
            .display()
            .to_string()
    }

    pub fn network_actuator_policy_path(&self) -> String {
        PathBuf::from(&self.config_dir)
            .join("network-actuator.policy.json")
            .display()
            .to_string()
    }

    pub fn provider_fallback_hosts_path(&self) -> String {
        PathBuf::from(&self.config_dir)
            .join("provider-fallback.hosts")
            .display()
            .to_string()
    }

    pub fn dns_hosts_path(&self) -> String {
        "/etc/coredns/synapse.hosts".to_string()
    }

    pub fn dns_corefile_path(&self) -> String {
        "/etc/coredns/Corefile".to_string()
    }

    pub fn dns_service_path(&self) -> String {
        PathBuf::from(&self.service_unit_dir)
            .join("coredns-synapse.service")
            .display()
            .to_string()
    }

    pub fn dns_resolved_dropin_path(&self) -> String {
        "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf".to_string()
    }

    pub fn hostname_path(&self) -> String {
        env_path("CYNIC_HOSTNAME_PATH", "/etc/hostname")
    }

    pub fn mesh_interface(&self) -> String {
        env_path("CYNIC_MESH_INTERFACE", "wg0")
    }

    pub fn wg_quick_unit(&self) -> String {
        format!("wg-quick@{}.service", self.mesh_interface())
    }

    pub fn dns_service_unit(&self) -> String {
        "coredns-synapse.service".to_string()
    }

    pub fn resolved_service_unit(&self) -> String {
        "systemd-resolved.service".to_string()
    }

    pub fn capabilities(&self) -> HostCapabilities {
        HostCapabilities {
            identity: IdentityCapability {
                hostname_path: self.hostname_path(),
                hostname_command: self.hostname_bin(),
            },
            name_service: NameServiceCapability {
                resolver_command: self.getent_bin(),
            },
            service_manager: ServiceManagerCapability {
                kind: self.service_manager_kind(),
                command: self.service_manager_bin(),
            },
            package_manager: PackageManagerCapability {
                installer: self.package_install_bin(),
                query: self.package_query_bin(),
                rpm_query: self.rpm_bin(),
            },
            route_tool: RouteToolCapability {
                command: self.ip_bin(),
            },
            wireguard_tool: WireguardToolCapability {
                command: self.wg_bin(),
                mesh_interface: self.mesh_interface(),
            },
            network_manager: NetworkManagerCapability {
                command: self.nmcli_bin(),
            },
            dns_provider: DnsProviderCapability {
                provider: self.coredns_bin(),
                command: self.coredns_bin(),
                service_unit: self.dns_service_unit(),
                resolved_unit: self.resolved_service_unit(),
            },
        }
    }

    pub fn sudo_bin(&self) -> String {
        env_path("CYNIC_SUDO_BIN", "sudo")
    }

    pub fn systemctl_bin(&self) -> String {
        env_path("CYNIC_SYSTEMCTL_BIN", "systemctl")
    }

    pub fn service_manager_kind(&self) -> String {
        env_path("CYNIC_SERVICE_MANAGER_KIND", "systemd")
    }

    pub fn service_manager_bin(&self) -> String {
        match self.service_manager_kind().as_str() {
            "openrc" => self.rc_service_bin(),
            _ => self.systemctl_bin(),
        }
    }

    pub fn rc_service_bin(&self) -> String {
        env_path("CYNIC_RC_SERVICE_BIN", "rc-service")
    }

    pub fn rc_update_bin(&self) -> String {
        env_path("CYNIC_RC_UPDATE_BIN", "rc-update")
    }

    pub fn ip_bin(&self) -> String {
        env_path("CYNIC_IP_BIN", "ip")
    }

    pub fn wg_bin(&self) -> String {
        env_path("CYNIC_WG_BIN", "wg")
    }

    pub fn hostname_bin(&self) -> String {
        env_path("CYNIC_HOSTNAME_BIN", "hostname")
    }

    pub fn getent_bin(&self) -> String {
        env_path("CYNIC_GETENT_BIN", "getent")
    }

    pub fn package_query_bin(&self) -> String {
        env_path("CYNIC_PACKAGE_QUERY_BIN", "dpkg")
    }

    pub fn package_install_bin(&self) -> String {
        env_path("CYNIC_PACKAGE_INSTALL_BIN", "apt-get")
    }

    pub fn rpm_bin(&self) -> String {
        env_path("CYNIC_RPM_BIN", "rpm")
    }

    pub fn which_bin(&self) -> String {
        env_path("CYNIC_WHICH_BIN", "which")
    }

    pub fn visudo_bin(&self) -> String {
        env_path("CYNIC_VISUDO_BIN", "visudo")
    }

    pub fn nmcli_bin(&self) -> String {
        env_path("CYNIC_NMCLI_BIN", "nmcli")
    }

    pub fn coredns_bin(&self) -> String {
        env_path("CYNIC_COREDNS_BIN", "coredns")
    }

    pub fn headscale_bin(&self) -> String {
        env_path("CYNIC_HEADSCALE_BIN", "headscale")
    }

    pub fn lxc_exec_bridge_bin(&self) -> String {
        env_path("CYNIC_LXC_EXEC_BRIDGE_BIN", "cynic-lxc-exec-bridge")
    }

    pub fn nvidia_smi_bin(&self) -> String {
        env_path("CYNIC_NVIDIA_SMI_BIN", "nvidia-smi")
    }

    pub fn sudo_noninteractive_command(&self, argv: &[&str]) -> String {
        if argv.is_empty() {
            return format!("{} -n", self.sudo_bin());
        }
        format!("{} -n {}", self.sudo_bin(), argv.join(" "))
    }

    pub fn forbidden_raw_invocation_patterns(&self) -> Vec<String> {
        let caps = self.capabilities();
        vec![
            caps.route_tool.command,
            caps.wireguard_tool.command,
            "wg-quick".to_string(),
            self.nmcli_bin(),
            "/bin/sh".to_string(),
            "/usr/bin/bash".to_string(),
        ]
    }
}

pub fn current_runtime_layout() -> HostRuntimeLayout {
    HostRuntimeLayout::from_env()
}

pub fn observed_hostname() -> String {
    let identity = current_runtime_layout().capabilities().identity;

    fs::read_to_string(&identity.hostname_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("HOSTNAME")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .or_else(|| command_stdout(&identity.hostname_command, &[]))
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn observed_local_ips() -> Vec<String> {
    let hostname_command = current_runtime_layout()
        .capabilities()
        .identity
        .hostname_command;
    command_stdout(&hostname_command, &["-I"])
        .map(|stdout| stdout.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

pub fn observed_service_status(service: &str) -> String {
    let systemctl_bin = current_runtime_layout()
        .capabilities()
        .service_manager
        .command;
    command_stdout(&systemctl_bin, &["is-active", service]).unwrap_or_else(|| "unknown".to_string())
}

fn env_path(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::HostRuntimeLayout;

    #[test]
    fn derives_helper_and_sudoers_paths_from_layout_roots() {
        let layout = HostRuntimeLayout {
            config_dir: "/etc/cynic".to_string(),
            helper_dir: "/opt/cynic/libexec".to_string(),
            sudoers_dir: "/opt/cynic/sudoers.d".to_string(),
            service_unit_dir: "/etc/systemd/system".to_string(),
            runtime_home_dir: "/home/cynic".to_string(),
            state_dir: "/var/lib/cynic".to_string(),
            log_dir: "/var/log/cynic".to_string(),
            host_exec_bundle_dir: "/tmp/cynic-host-exec".to_string(),
            network_actuator_bundle_dir: "/tmp/cynic-network-actuator".to_string(),
            dns_bundle_dir: "/tmp/cynic-synapse-dns".to_string(),
            provider_fallback_bundle_dir: "/tmp/cynic-provider-fallback-hosts".to_string(),
        };

        assert_eq!(
            layout.host_exec_helper_path(),
            "/opt/cynic/libexec/cynic-host-exec"
        );
        assert_eq!(
            layout.network_actuator_sudoers_path(),
            "/opt/cynic/sudoers.d/cynic-network-actuator"
        );
    }

    #[test]
    fn capabilities_can_select_openrc_service_manager_from_env() {
        unsafe {
            std::env::set_var("CYNIC_SERVICE_MANAGER_KIND", "openrc");
            std::env::set_var("CYNIC_RC_SERVICE_BIN", "rc-service-test");
        }
        let capabilities = HostRuntimeLayout::from_env().capabilities();
        unsafe {
            std::env::remove_var("CYNIC_SERVICE_MANAGER_KIND");
            std::env::remove_var("CYNIC_RC_SERVICE_BIN");
        }

        assert_eq!(capabilities.service_manager.kind, "openrc");
        assert_eq!(capabilities.service_manager.command, "rc-service-test");
    }
}
