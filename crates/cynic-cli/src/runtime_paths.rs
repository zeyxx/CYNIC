use std::env;
use std::path::{Path, PathBuf};

use cynic_host::runtime_authority::{RuntimeProjectionClass, default_runtime_authority_paths};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeMode {
    ExplicitFlag,
    Environment,
    Installed,
    DevWorktree,
    UnresolvedFallback,
}

impl RuntimeMode {
    pub(crate) fn projection_class(self) -> RuntimeProjectionClass {
        match self {
            Self::Installed => RuntimeProjectionClass::RuntimeAuthority,
            Self::ExplicitFlag | Self::Environment => RuntimeProjectionClass::OperatorProjection,
            Self::DevWorktree | Self::UnresolvedFallback => {
                RuntimeProjectionClass::DevWorktreeFallback
            }
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitFlag => "explicit_flag",
            Self::Environment => "environment",
            Self::Installed => "installed",
            Self::DevWorktree => "dev_worktree",
            Self::UnresolvedFallback => "unresolved_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimePaths {
    pub(crate) registry_path: PathBuf,
    pub(crate) config_root: PathBuf,
    pub(crate) policy_root: PathBuf,
    pub(crate) libexec_root: PathBuf,
    pub(crate) sudoers_dir: PathBuf,
    pub(crate) state_root: PathBuf,
    pub(crate) log_root: PathBuf,
    pub(crate) provider_hosts_path: PathBuf,
    pub(crate) dns_hosts_path: PathBuf,
    pub(crate) dns_corefile_path: PathBuf,
    pub(crate) dns_service_path: PathBuf,
    pub(crate) resolved_dropin_path: PathBuf,
    pub(crate) host_exec_helper_path: PathBuf,
    pub(crate) host_exec_sudoers_path: PathBuf,
    pub(crate) network_actuator_helper_path: PathBuf,
    pub(crate) network_actuator_policy_path: PathBuf,
    pub(crate) network_actuator_sudoers_path: PathBuf,
    pub(crate) mode: RuntimeMode,
    pub(crate) projection_class: RuntimeProjectionClass,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeContext {
    pub(crate) explicit_registry: Option<PathBuf>,
    pub(crate) env_registry: Option<PathBuf>,
    pub(crate) current_dir: Option<PathBuf>,
    pub(crate) current_exe: Option<PathBuf>,
    pub(crate) installed_config_root: Option<PathBuf>,
    pub(crate) env_config_root: Option<PathBuf>,
    pub(crate) env_policy_root: Option<PathBuf>,
    pub(crate) env_libexec_root: Option<PathBuf>,
    pub(crate) env_sudoers_dir: Option<PathBuf>,
    pub(crate) env_state_root: Option<PathBuf>,
    pub(crate) env_log_root: Option<PathBuf>,
    pub(crate) env_provider_hosts_path: Option<PathBuf>,
    pub(crate) env_dns_hosts_path: Option<PathBuf>,
    pub(crate) env_dns_corefile_path: Option<PathBuf>,
    pub(crate) env_dns_service_path: Option<PathBuf>,
    pub(crate) env_resolved_dropin_path: Option<PathBuf>,
}

impl RuntimeContext {
    pub(crate) fn from_env(explicit_registry: Option<String>) -> Self {
        let authority = default_runtime_authority_paths();
        Self {
            explicit_registry: explicit_registry.map(PathBuf::from),
            env_registry: env::var("CYNIC_REGISTRY").ok().map(PathBuf::from),
            current_dir: env::current_dir().ok(),
            current_exe: env::current_exe().ok(),
            installed_config_root: Some(authority.config_root),
            env_config_root: env::var("CYNIC_CONFIG_ROOT").ok().map(PathBuf::from),
            env_policy_root: env::var("CYNIC_POLICY_ROOT").ok().map(PathBuf::from),
            env_libexec_root: env::var("CYNIC_LIBEXEC_ROOT").ok().map(PathBuf::from),
            env_sudoers_dir: env::var("CYNIC_SUDOERS_DIR").ok().map(PathBuf::from),
            env_state_root: env::var("CYNIC_STATE_ROOT").ok().map(PathBuf::from),
            env_log_root: env::var("CYNIC_LOG_ROOT").ok().map(PathBuf::from),
            env_provider_hosts_path: env::var("CYNIC_PROVIDER_HOSTS_PATH")
                .ok()
                .map(PathBuf::from),
            env_dns_hosts_path: env::var("CYNIC_DNS_HOSTS_PATH").ok().map(PathBuf::from),
            env_dns_corefile_path: env::var("CYNIC_DNS_COREFILE_PATH").ok().map(PathBuf::from),
            env_dns_service_path: env::var("CYNIC_DNS_SERVICE_PATH").ok().map(PathBuf::from),
            env_resolved_dropin_path: env::var("CYNIC_RESOLVED_DROPIN_PATH")
                .ok()
                .map(PathBuf::from),
        }
    }
}

pub(crate) fn resolve_runtime_paths(context: &RuntimeContext) -> RuntimePaths {
    if let Some(path) = context.explicit_registry.as_ref() {
        let config_root = effective_config_root(context, path);
        return RuntimePaths {
            registry_path: path.clone(),
            config_root: config_root.clone(),
            policy_root: effective_policy_root(context, &config_root),
            libexec_root: effective_libexec_root(context),
            sudoers_dir: effective_sudoers_dir(context),
            state_root: effective_state_root(context),
            log_root: effective_log_root(context),
            provider_hosts_path: effective_provider_hosts_path(context, &config_root),
            dns_hosts_path: effective_dns_hosts_path(context),
            dns_corefile_path: effective_dns_corefile_path(context),
            dns_service_path: effective_dns_service_path(context),
            resolved_dropin_path: effective_resolved_dropin_path(context),
            host_exec_helper_path: effective_host_exec_helper_path(context),
            host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
            network_actuator_helper_path: effective_network_actuator_helper_path(context),
            network_actuator_policy_path: effective_network_actuator_policy_path(
                context,
                &config_root,
            ),
            network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
            mode: RuntimeMode::ExplicitFlag,
            projection_class: RuntimeMode::ExplicitFlag.projection_class(),
        };
    }

    if let Some(path) = context.env_registry.as_ref()
        && path.exists()
    {
        let config_root = effective_config_root(context, path);
        return RuntimePaths {
            registry_path: path.clone(),
            config_root: config_root.clone(),
            policy_root: effective_policy_root(context, &config_root),
            libexec_root: effective_libexec_root(context),
            sudoers_dir: effective_sudoers_dir(context),
            state_root: effective_state_root(context),
            log_root: effective_log_root(context),
            provider_hosts_path: effective_provider_hosts_path(context, &config_root),
            dns_hosts_path: effective_dns_hosts_path(context),
            dns_corefile_path: effective_dns_corefile_path(context),
            dns_service_path: effective_dns_service_path(context),
            resolved_dropin_path: effective_resolved_dropin_path(context),
            host_exec_helper_path: effective_host_exec_helper_path(context),
            host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
            network_actuator_helper_path: effective_network_actuator_helper_path(context),
            network_actuator_policy_path: effective_network_actuator_policy_path(
                context,
                &config_root,
            ),
            network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
            mode: RuntimeMode::Environment,
            projection_class: RuntimeMode::Environment.projection_class(),
        };
    }

    if let Some(config_root) = effective_installed_config_root(context) {
        let installed_registry = config_root.join("registry.json");
        if installed_registry.exists() {
            return RuntimePaths {
                registry_path: installed_registry,
                config_root: config_root.clone(),
                policy_root: effective_policy_root(context, &config_root),
                libexec_root: effective_libexec_root(context),
                sudoers_dir: effective_sudoers_dir(context),
                state_root: effective_state_root(context),
                log_root: effective_log_root(context),
                provider_hosts_path: effective_provider_hosts_path(context, &config_root),
                dns_hosts_path: effective_dns_hosts_path(context),
                dns_corefile_path: effective_dns_corefile_path(context),
                dns_service_path: effective_dns_service_path(context),
                resolved_dropin_path: effective_resolved_dropin_path(context),
                host_exec_helper_path: effective_host_exec_helper_path(context),
                host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
                network_actuator_helper_path: effective_network_actuator_helper_path(context),
                network_actuator_policy_path: effective_network_actuator_policy_path(
                    context,
                    &config_root,
                ),
                network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
                mode: RuntimeMode::Installed,
                projection_class: RuntimeMode::Installed.projection_class(),
            };
        }
    }

    if let Some(path) = find_registry_from(context.current_dir.as_deref()) {
        let config_root = effective_config_root(context, &path);
        return RuntimePaths {
            config_root: config_root.clone(),
            policy_root: effective_policy_root(context, &config_root),
            registry_path: path,
            libexec_root: effective_libexec_root(context),
            sudoers_dir: effective_sudoers_dir(context),
            state_root: effective_state_root(context),
            log_root: effective_log_root(context),
            provider_hosts_path: effective_provider_hosts_path(context, &config_root),
            dns_hosts_path: effective_dns_hosts_path(context),
            dns_corefile_path: effective_dns_corefile_path(context),
            dns_service_path: effective_dns_service_path(context),
            resolved_dropin_path: effective_resolved_dropin_path(context),
            host_exec_helper_path: effective_host_exec_helper_path(context),
            host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
            network_actuator_helper_path: effective_network_actuator_helper_path(context),
            network_actuator_policy_path: effective_network_actuator_policy_path(
                context,
                &config_root,
            ),
            network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
            mode: RuntimeMode::DevWorktree,
            projection_class: RuntimeMode::DevWorktree.projection_class(),
        };
    }

    if let Some(path) = find_registry_from(context.current_exe.as_deref()) {
        let config_root = effective_config_root(context, &path);
        return RuntimePaths {
            config_root: config_root.clone(),
            policy_root: effective_policy_root(context, &config_root),
            registry_path: path,
            libexec_root: effective_libexec_root(context),
            sudoers_dir: effective_sudoers_dir(context),
            state_root: effective_state_root(context),
            log_root: effective_log_root(context),
            provider_hosts_path: effective_provider_hosts_path(context, &config_root),
            dns_hosts_path: effective_dns_hosts_path(context),
            dns_corefile_path: effective_dns_corefile_path(context),
            dns_service_path: effective_dns_service_path(context),
            resolved_dropin_path: effective_resolved_dropin_path(context),
            host_exec_helper_path: effective_host_exec_helper_path(context),
            host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
            network_actuator_helper_path: effective_network_actuator_helper_path(context),
            network_actuator_policy_path: effective_network_actuator_policy_path(
                context,
                &config_root,
            ),
            network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
            mode: RuntimeMode::DevWorktree,
            projection_class: RuntimeMode::DevWorktree.projection_class(),
        };
    }

    let config_root = context
        .env_config_root
        .clone()
        .or_else(|| context.installed_config_root.clone())
        .unwrap_or_else(|| PathBuf::from("/etc/cynic"));
    RuntimePaths {
        registry_path: config_root.join("registry.json"),
        config_root: config_root.clone(),
        policy_root: effective_policy_root(context, &config_root),
        libexec_root: effective_libexec_root(context),
        sudoers_dir: effective_sudoers_dir(context),
        state_root: effective_state_root(context),
        log_root: effective_log_root(context),
        provider_hosts_path: effective_provider_hosts_path(context, &config_root),
        dns_hosts_path: effective_dns_hosts_path(context),
        dns_corefile_path: effective_dns_corefile_path(context),
        dns_service_path: effective_dns_service_path(context),
        resolved_dropin_path: effective_resolved_dropin_path(context),
        host_exec_helper_path: effective_host_exec_helper_path(context),
        host_exec_sudoers_path: effective_host_exec_sudoers_path(context),
        network_actuator_helper_path: effective_network_actuator_helper_path(context),
        network_actuator_policy_path: effective_network_actuator_policy_path(context, &config_root),
        network_actuator_sudoers_path: effective_network_actuator_sudoers_path(context),
        mode: RuntimeMode::UnresolvedFallback,
        projection_class: RuntimeMode::UnresolvedFallback.projection_class(),
    }
}

fn effective_installed_config_root(context: &RuntimeContext) -> Option<PathBuf> {
    context
        .env_config_root
        .clone()
        .or_else(|| context.installed_config_root.clone())
}

fn effective_config_root(context: &RuntimeContext, path: &Path) -> PathBuf {
    context
        .env_config_root
        .clone()
        .unwrap_or_else(|| infer_config_root(path, context.installed_config_root.as_deref()))
}

fn effective_policy_root(context: &RuntimeContext, config_root: &Path) -> PathBuf {
    context
        .env_policy_root
        .clone()
        .unwrap_or_else(|| config_root.to_path_buf())
}

fn effective_libexec_root(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_libexec_root
        .clone()
        .unwrap_or(authority.helper_root)
}

fn effective_sudoers_dir(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_sudoers_dir
        .clone()
        .unwrap_or(authority.sudoers_dir)
}

fn effective_state_root(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_state_root
        .clone()
        .unwrap_or(authority.principal.state_root)
}

fn effective_log_root(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_log_root
        .clone()
        .unwrap_or(authority.principal.log_root)
}

fn effective_provider_hosts_path(context: &RuntimeContext, config_root: &Path) -> PathBuf {
    context
        .env_provider_hosts_path
        .clone()
        .unwrap_or_else(|| config_root.join("provider-fallback.hosts"))
}

fn effective_dns_hosts_path(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_dns_hosts_path
        .clone()
        .unwrap_or(authority.dns_hosts_path)
}

fn effective_dns_corefile_path(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_dns_corefile_path
        .clone()
        .unwrap_or(authority.dns_corefile_path)
}

fn effective_dns_service_path(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_dns_service_path
        .clone()
        .unwrap_or(authority.dns_service_path)
}

fn effective_resolved_dropin_path(context: &RuntimeContext) -> PathBuf {
    let authority = default_runtime_authority_paths();
    context
        .env_resolved_dropin_path
        .clone()
        .unwrap_or(authority.resolved_dropin_path)
}

fn effective_host_exec_helper_path(context: &RuntimeContext) -> PathBuf {
    effective_libexec_root(context).join("cynic-host-exec")
}

fn effective_host_exec_sudoers_path(context: &RuntimeContext) -> PathBuf {
    effective_sudoers_dir(context).join("cynic-host-exec")
}

fn effective_network_actuator_helper_path(context: &RuntimeContext) -> PathBuf {
    effective_libexec_root(context).join("cynic-network-actuator")
}

fn effective_network_actuator_policy_path(
    _context: &RuntimeContext,
    config_root: &Path,
) -> PathBuf {
    config_root.join("network-actuator.policy.json")
}

fn effective_network_actuator_sudoers_path(context: &RuntimeContext) -> PathBuf {
    effective_sudoers_dir(context).join("cynic-network-actuator")
}

fn infer_config_root(path: &Path, installed_config_root: Option<&Path>) -> PathBuf {
    if path.file_name().and_then(|value| value.to_str()) == Some("registry.json")
        && path.parent().is_some()
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("/etc/cynic"));
        if parent.ends_with("infra") {
            return installed_config_root
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("/etc/cynic"));
        }
        return parent.to_path_buf();
    }

    installed_config_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/etc/cynic"))
}

fn find_registry_from(origin: Option<&Path>) -> Option<PathBuf> {
    let path = origin?;
    let start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    for ancestor in start.ancestors() {
        let candidate = ancestor.join("infra/registry.json");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn prefers_installed_registry_without_repo() {
        let root = temp_fixture("installed");
        let etc = root.join("etc/cynic");
        fs::create_dir_all(&etc).expect("etc/cynic");
        fs::write(etc.join("registry.json"), "{}\n").expect("registry");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            installed_config_root: Some(etc.clone()),
            current_dir: Some(root.join("nowhere")),
            current_exe: Some(root.join("bin/cynic")),
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.mode, RuntimeMode::Installed);
        assert_eq!(
            resolved.projection_class,
            RuntimeProjectionClass::RuntimeAuthority
        );
        assert_eq!(resolved.registry_path, etc.join("registry.json"));
        assert_eq!(resolved.config_root, etc);
        assert_eq!(resolved.sudoers_dir, PathBuf::from("/etc/sudoers.d"));
        assert_eq!(
            resolved.provider_hosts_path,
            root.join("etc/cynic/provider-fallback.hosts")
        );
        assert_eq!(
            resolved.dns_hosts_path,
            PathBuf::from("/etc/coredns/synapse.hosts")
        );
    }

    #[test]
    fn falls_back_to_dev_worktree_when_repo_present() {
        let root = temp_fixture("dev");
        let repo = root.join("repo");
        fs::create_dir_all(repo.join("infra")).expect("infra");
        fs::write(repo.join("infra/registry.json"), "{}\n").expect("registry");
        let nested = repo.join("crates/cynic-cli");
        fs::create_dir_all(&nested).expect("nested");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            current_dir: Some(nested),
            installed_config_root: Some(root.join("etc/cynic")),
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.mode, RuntimeMode::DevWorktree);
        assert_eq!(
            resolved.projection_class,
            RuntimeProjectionClass::DevWorktreeFallback
        );
        assert_eq!(resolved.registry_path, repo.join("infra/registry.json"));
        assert_eq!(resolved.config_root, root.join("etc/cynic"));
        assert_eq!(resolved.policy_root, root.join("etc/cynic"));
        assert_eq!(
            resolved.network_actuator_policy_path,
            root.join("etc/cynic/network-actuator.policy.json")
        );
    }

    #[test]
    fn explicit_flag_wins_even_when_installed_exists() {
        let root = temp_fixture("explicit");
        let explicit = root.join("custom/registry.json");
        fs::create_dir_all(explicit.parent().expect("parent")).expect("custom");
        fs::write(&explicit, "{}\n").expect("explicit registry");
        let etc = root.join("etc/cynic");
        fs::create_dir_all(&etc).expect("etc");
        fs::write(etc.join("registry.json"), "{}\n").expect("installed registry");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            explicit_registry: Some(explicit.clone()),
            installed_config_root: Some(etc),
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.mode, RuntimeMode::ExplicitFlag);
        assert_eq!(
            resolved.projection_class,
            RuntimeProjectionClass::OperatorProjection
        );
        assert_eq!(resolved.registry_path, explicit);
        assert_eq!(resolved.libexec_root, PathBuf::from("/usr/local/libexec"));
        assert_eq!(
            resolved.host_exec_helper_path,
            PathBuf::from("/usr/local/libexec/cynic-host-exec")
        );
    }

    #[test]
    fn env_roots_override_installed_defaults() {
        let root = temp_fixture("env-roots");
        let config_root = root.join("config");
        let policy_root = root.join("policy");
        let libexec_root = root.join("libexec");
        let state_root = root.join("state");
        let log_root = root.join("log");
        let provider_hosts_path = root.join("custom/provider-fallback.hosts");
        let registry = root.join("custom/registry.json");
        fs::create_dir_all(registry.parent().expect("registry parent")).expect("custom");
        fs::write(&registry, "{}\n").expect("registry");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            explicit_registry: Some(registry),
            env_config_root: Some(config_root.clone()),
            env_policy_root: Some(policy_root.clone()),
            env_libexec_root: Some(libexec_root.clone()),
            env_state_root: Some(state_root.clone()),
            env_log_root: Some(log_root.clone()),
            env_provider_hosts_path: Some(provider_hosts_path.clone()),
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.config_root, config_root);
        assert_eq!(resolved.policy_root, policy_root);
        assert_eq!(resolved.libexec_root, libexec_root);
        assert_eq!(resolved.state_root, state_root);
        assert_eq!(resolved.log_root, log_root);
        assert_eq!(resolved.provider_hosts_path, provider_hosts_path);
        assert_eq!(resolved.sudoers_dir, PathBuf::from("/etc/sudoers.d"));
        assert_eq!(
            resolved.dns_service_path,
            PathBuf::from("/etc/systemd/system/coredns-synapse.service")
        );
    }

    #[test]
    fn env_dns_paths_override_defaults() {
        let root = temp_fixture("env-dns");
        let registry = root.join("custom/registry.json");
        fs::create_dir_all(registry.parent().expect("registry parent")).expect("custom");
        fs::write(&registry, "{}\n").expect("registry");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            explicit_registry: Some(registry),
            env_dns_hosts_path: Some(root.join("dns/hosts")),
            env_dns_corefile_path: Some(root.join("dns/Corefile")),
            env_dns_service_path: Some(root.join("systemd/coredns.service")),
            env_resolved_dropin_path: Some(root.join("resolved/cynic.conf")),
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.dns_hosts_path, root.join("dns/hosts"));
        assert_eq!(resolved.dns_corefile_path, root.join("dns/Corefile"));
        assert_eq!(
            resolved.dns_service_path,
            root.join("systemd/coredns.service")
        );
        assert_eq!(
            resolved.resolved_dropin_path,
            root.join("resolved/cynic.conf")
        );
        assert_eq!(
            resolved.network_actuator_sudoers_path,
            PathBuf::from("/etc/sudoers.d/cynic-network-actuator")
        );
    }

    #[test]
    fn unresolved_fallback_uses_installed_config_root_registry() {
        let root = temp_fixture("unresolved");
        let config_root = root.join("etc").join("cynic");
        fs::create_dir_all(&config_root).expect("config root");
        fs::create_dir_all(root.join("etc/cynic").join("state")).expect("state");

        let resolved = resolve_runtime_paths(&RuntimeContext {
            installed_config_root: Some(config_root.clone()),
            env_registry: None,
            current_dir: Some(root.join("nowhere")),
            current_exe: None,
            ..RuntimeContext::default()
        });

        assert_eq!(resolved.mode, RuntimeMode::UnresolvedFallback);
        assert_eq!(
            resolved.projection_class,
            RuntimeMode::UnresolvedFallback.projection_class()
        );
        assert_eq!(resolved.config_root, config_root);
        assert_eq!(resolved.registry_path, config_root.join("registry.json"));
    }

    fn temp_fixture(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = env::temp_dir().join(format!("cynic-cli-{name}-{unique}"));
        fs::create_dir_all(&path).expect("fixture dir");
        path
    }
}
