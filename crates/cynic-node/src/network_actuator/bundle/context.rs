use cynic_host::bounded_helper::BoundedHelperInstallRuntime;
use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator_contract::{helper_path, policy_path, sudoers_path, sudoers_rule};

#[derive(Debug, Clone)]
pub(crate) struct NetworkActuatorBundleContext {
    pub(crate) helper_target: String,
    pub(crate) policy_target: String,
    pub(crate) sudoers_target: String,
    pub(crate) sudoers_rule: String,
    pub(crate) helper_dir: String,
    pub(crate) config_dir: String,
    pub(crate) sudoers_dir: String,
    pub(crate) sudo_bin: String,
    pub(crate) route_bin: String,
    pub(crate) wireguard_bin: String,
    pub(crate) denied_shell_bin: String,
}

impl NetworkActuatorBundleContext {
    pub(crate) fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            helper_target: helper_path(),
            policy_target: policy_path(),
            sudoers_target: sudoers_path(),
            sudoers_rule: sudoers_rule(),
            helper_dir: runtime_layout.helper_dir.clone(),
            config_dir: runtime_layout.config_dir.clone(),
            sudoers_dir: runtime_layout.sudoers_dir.clone(),
            sudo_bin: runtime_layout.sudo_bin(),
            route_bin: capabilities.route_tool.command.clone(),
            wireguard_bin: capabilities.wireguard_tool.command.clone(),
            denied_shell_bin: "/bin/sh".to_string(),
        }
    }
}

pub(crate) fn helper_install_runtime(
    context: &NetworkActuatorBundleContext,
) -> BoundedHelperInstallRuntime {
    BoundedHelperInstallRuntime {
        helper_target: context.helper_target.clone(),
        sudoers_target: context.sudoers_target.clone(),
        helper_dir: context.helper_dir.clone(),
        sudoers_dir: context.sudoers_dir.clone(),
    }
}
