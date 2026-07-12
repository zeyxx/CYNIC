use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::runtime_context::{HostRuntimeContext, observe_current_host_runtime_context};

pub(crate) fn observe_network_host_runtime_context() -> HostRuntimeContext {
    observe_current_host_runtime_context()
}

pub(crate) fn blocked_domains(context: &HostRuntimeContext) -> Vec<&'static str> {
    let mut blocked = Vec::new();
    let reports = &context.host_substrate;
    if reports.principal.status != SurfaceStatus::Active {
        blocked.push("host_principal_substrate_blocked");
    }
    if reports.privilege.status != SurfaceStatus::Active {
        blocked.push("host_privilege_substrate_blocked");
    }
    if reports.packages.status != SurfaceStatus::Active {
        blocked.push("host_package_substrate_blocked");
    }
    if reports.services.status != SurfaceStatus::Active {
        blocked.push("host_service_substrate_blocked");
    }
    blocked
}

pub(crate) fn bootstrap_blocked(context: &HostRuntimeContext) -> bool {
    context.host_bootstrap.status != SurfaceStatus::Active
}
