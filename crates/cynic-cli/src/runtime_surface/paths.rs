use std::process::ExitCode;

use serde::Serialize;

use crate::runtime_paths::{RuntimeContext, RuntimeMode, resolve_runtime_paths};
use crate::{emit_json, runtime_paths};

#[derive(Serialize)]
struct RuntimePathsView {
    schema: &'static str,
    mode: &'static str,
    registry_path: String,
    config_root: String,
    policy_root: String,
    libexec_root: String,
    sudoers_dir: String,
    state_root: String,
    log_root: String,
    provider_hosts_path: String,
    dns_hosts_path: String,
    dns_corefile_path: String,
    dns_service_path: String,
    resolved_dropin_path: String,
    host_exec_helper_path: String,
    host_exec_sudoers_path: String,
    network_actuator_helper_path: String,
    network_actuator_policy_path: String,
    network_actuator_sudoers_path: String,
}

pub(crate) fn execute_paths(json: bool, registry: Option<String>) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let resolved = resolve_runtime_paths(&RuntimeContext::from_env(registry));
    emit_json(&runtime_paths_view(&resolved), false)
}

fn runtime_paths_view(paths: &runtime_paths::RuntimePaths) -> RuntimePathsView {
    RuntimePathsView {
        schema: "cynic.runtime_paths.v1",
        mode: match paths.mode {
            RuntimeMode::ExplicitFlag => "explicit_flag",
            RuntimeMode::Environment => "environment",
            RuntimeMode::Installed => "installed",
            RuntimeMode::ProjectRootEnv => "project_root_env",
            RuntimeMode::DevWorktree => "dev_worktree",
            RuntimeMode::UnresolvedFallback => "unresolved_fallback",
        },
        registry_path: paths.registry_path.to_string_lossy().to_string(),
        config_root: paths.config_root.to_string_lossy().to_string(),
        policy_root: paths.policy_root.to_string_lossy().to_string(),
        libexec_root: paths.libexec_root.to_string_lossy().to_string(),
        sudoers_dir: paths.sudoers_dir.to_string_lossy().to_string(),
        state_root: paths.state_root.to_string_lossy().to_string(),
        log_root: paths.log_root.to_string_lossy().to_string(),
        provider_hosts_path: paths.provider_hosts_path.to_string_lossy().to_string(),
        dns_hosts_path: paths.dns_hosts_path.to_string_lossy().to_string(),
        dns_corefile_path: paths.dns_corefile_path.to_string_lossy().to_string(),
        dns_service_path: paths.dns_service_path.to_string_lossy().to_string(),
        resolved_dropin_path: paths.resolved_dropin_path.to_string_lossy().to_string(),
        host_exec_helper_path: paths.host_exec_helper_path.to_string_lossy().to_string(),
        host_exec_sudoers_path: paths.host_exec_sudoers_path.to_string_lossy().to_string(),
        network_actuator_helper_path: paths
            .network_actuator_helper_path
            .to_string_lossy()
            .to_string(),
        network_actuator_policy_path: paths
            .network_actuator_policy_path
            .to_string_lossy()
            .to_string(),
        network_actuator_sudoers_path: paths
            .network_actuator_sudoers_path
            .to_string_lossy()
            .to_string(),
    }
}
