use cynic_host::runtime_layout::current_runtime_layout;

use crate::network_actuator_contract::{executor_path, membrane_admission, membrane_path, policy_path};

#[derive(Debug, Clone)]
pub(crate) struct NetworkActuatorBundleContext {
    pub(crate) executor: String,
    pub(crate) policy_target: String,
    pub(crate) membrane_target: String,
    pub(crate) membrane_admission: String,
    pub(crate) config_dir: String,
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
            executor: executor_path(),
            policy_target: policy_path(),
            membrane_target: membrane_path(),
            membrane_admission: membrane_admission(),
            config_dir: runtime_layout.config_dir.clone(),
            sudo_bin: runtime_layout.sudo_bin(),
            route_bin: capabilities.route_tool.command.clone(),
            wireguard_bin: capabilities.wireguard_tool.command.clone(),
            denied_shell_bin: "/bin/sh".to_string(),
        }
    }
}
