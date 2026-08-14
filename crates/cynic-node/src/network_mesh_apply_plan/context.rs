use cynic_host::runtime_layout::current_runtime_layout;

use super::command_plan::MeshApplyRuntimeContext;

#[derive(Debug, Clone)]
pub(super) struct NetworkMeshApplyPlanContext {
    pub(super) runtime: MeshApplyRuntimeContext,
}

impl NetworkMeshApplyPlanContext {
    pub(super) fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        let capabilities = runtime_layout.capabilities();
        Self {
            runtime: MeshApplyRuntimeContext {
                mesh_interface: capabilities.wireguard_tool.mesh_interface.clone(),
                ip_bin: capabilities.route_tool.command.clone(),
                wg_bin: capabilities.wireguard_tool.command.clone(),
                getent_bin: capabilities.name_service.resolver_command.clone(),
            },
        }
    }
}
