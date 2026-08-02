use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProjectionClass {
    RuntimeAuthority,
    OperatorProjection,
    DevWorktreeFallback,
}

impl RuntimeProjectionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeAuthority => "runtime_authority",
            Self::OperatorProjection => "operator_projection",
            Self::DevWorktreeFallback => "dev_worktree_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrincipalLayout {
    pub runtime_user: String,
    pub runtime_group: String,
    pub home_dir: PathBuf,
    pub state_root: PathBuf,
    pub log_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAuthorityPaths {
    pub config_root: PathBuf,
    pub kernel_socket_path: PathBuf,
    pub agent_access_group: String,
    pub policy_root: PathBuf,
    pub helper_root: PathBuf,
    pub sudoers_dir: PathBuf,
    pub service_unit_dir: PathBuf,
    pub dns_hosts_path: PathBuf,
    pub dns_corefile_path: PathBuf,
    pub dns_service_path: PathBuf,
    pub resolved_dropin_path: PathBuf,
    pub host_exec_bundle_dir: PathBuf,
    pub network_actuator_bundle_dir: PathBuf,
    pub dns_bundle_dir: PathBuf,
    pub provider_fallback_bundle_dir: PathBuf,
    pub principal: RuntimePrincipalLayout,
}

pub fn default_runtime_principal_layout() -> RuntimePrincipalLayout {
    RuntimePrincipalLayout {
        runtime_user: "cynic".to_string(),
        runtime_group: "cynic".to_string(),
        // Runtime state is the canonical anchor. A Unix home may exist, but it
        // should not outrank the runtime-owned state root in authority.
        home_dir: PathBuf::from("/var/lib/cynic"),
        state_root: PathBuf::from("/var/lib/cynic"),
        log_root: PathBuf::from("/var/log/cynic"),
    }
}

pub fn default_runtime_authority_paths() -> RuntimeAuthorityPaths {
    let principal = default_runtime_principal_layout();
    let staging_root = principal.state_root.join("staging");
    RuntimeAuthorityPaths {
        config_root: PathBuf::from("/etc/cynic"),
        kernel_socket_path: PathBuf::from("/run/cynic/kernel.sock"),
        agent_access_group: "cynic-agent".to_string(),
        policy_root: PathBuf::from("/etc/cynic"),
        helper_root: PathBuf::from("/usr/local/libexec"),
        sudoers_dir: PathBuf::from("/etc/sudoers.d"),
        service_unit_dir: PathBuf::from("/etc/systemd/system"),
        dns_hosts_path: PathBuf::from("/etc/coredns/synapse.hosts"),
        dns_corefile_path: PathBuf::from("/etc/coredns/Corefile"),
        dns_service_path: PathBuf::from("/etc/systemd/system/coredns-synapse.service"),
        resolved_dropin_path: PathBuf::from("/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf"),
        // Install bundles are durable, inspectable activation artifacts. `/tmp`
        // is suitable only when an explicit lab layout injects it.
        host_exec_bundle_dir: staging_root.join("host-exec"),
        network_actuator_bundle_dir: staging_root.join("network-actuator"),
        dns_bundle_dir: staging_root.join("synapse-dns"),
        provider_fallback_bundle_dir: staging_root.join("provider-fallback-hosts"),
        principal,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeProjectionClass, default_runtime_authority_paths, default_runtime_principal_layout,
    };
    use std::path::PathBuf;

    #[test]
    fn default_principal_layout_is_state_anchored() {
        let principal = default_runtime_principal_layout();
        assert_eq!(principal.runtime_user, "cynic");
        assert_eq!(principal.home_dir, PathBuf::from("/var/lib/cynic"));
        assert_eq!(principal.state_root, PathBuf::from("/var/lib/cynic"));
    }

    #[test]
    fn default_authority_paths_are_root_and_runtime_split() {
        let authority = default_runtime_authority_paths();
        assert_eq!(authority.config_root, PathBuf::from("/etc/cynic"));
        assert_eq!(
            authority.kernel_socket_path,
            PathBuf::from("/run/cynic/kernel.sock")
        );
        assert_eq!(authority.agent_access_group, "cynic-agent");
        assert_eq!(authority.helper_root, PathBuf::from("/usr/local/libexec"));
        assert_eq!(
            authority.principal.log_root,
            PathBuf::from("/var/log/cynic")
        );
        assert_eq!(
            authority.host_exec_bundle_dir,
            PathBuf::from("/var/lib/cynic/staging/host-exec")
        );
        assert_eq!(
            authority.network_actuator_bundle_dir,
            PathBuf::from("/var/lib/cynic/staging/network-actuator")
        );
    }

    #[test]
    fn projection_class_strings_are_stable() {
        assert_eq!(
            RuntimeProjectionClass::RuntimeAuthority.as_str(),
            "runtime_authority"
        );
        assert_eq!(
            RuntimeProjectionClass::OperatorProjection.as_str(),
            "operator_projection"
        );
        assert_eq!(
            RuntimeProjectionClass::DevWorktreeFallback.as_str(),
            "dev_worktree_fallback"
        );
    }
}
