use crate::domain::shared::SubstrateDomainContract;
use crate::domain::shared::SurfaceStatus;
use crate::ports::{
    HostContractPort, HostPackageObservationPort, HostPrincipalObservationPort,
    HostPrivilegeObservationPort, HostServiceObservationPort,
};
use crate::registry::HostProfileResolution;

mod command_context;
mod execution;
mod planning;
mod reports;
mod status;

pub use command_context::HostCommandContext;
pub use execution::execute_host_apply;
pub use planning::{
    build_host_activation_plan, build_host_activation_plan_with_context,
    build_host_activation_plan_with_resolution,
    build_host_activation_plan_with_resolution_and_context, build_host_apply_report,
    build_host_apply_report_with_context, build_host_apply_report_with_resolution,
    build_host_apply_report_with_resolution_and_context, build_host_bundle,
    build_host_bundle_with_resolution,
};
pub use reports::{
    ExecutedHostCommand, ExecutedHostCommandResult, HOST_ACTIVATION_PLAN_SCHEMA,
    HOST_APPLY_EXECUTION_SCHEMA, HOST_APPLY_REPORT_SCHEMA, HOST_BUNDLE_SCHEMA,
    HOST_OBSERVATION_SCHEMA, HostActivationPlanReport, HostApplyExecutionReport, HostApplyReport,
    HostBundleReport, HostObservationReports,
};
use status::blocked_domain_statuses;

pub fn collect_contracts(port: &impl HostContractPort) -> Vec<SubstrateDomainContract> {
    port.declared_contracts()
}

pub fn observe_host(
    desired: &impl HostContractPort,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> HostObservationReports {
    observe_host_with_resolution(desired, observed, None)
}

pub fn observe_host_with_resolution(
    desired: &impl HostContractPort,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
    host_resolution: Option<HostProfileResolution>,
) -> HostObservationReports {
    let principal_policy = desired.desired_principal();
    let privilege_policy = desired.desired_privilege();
    let package_policy = desired.desired_packages();
    let service_policy = desired.desired_services();
    let principal = principal_policy.status(&observed.observe_principal(&principal_policy));
    let privilege = privilege_policy.status(&observed.observe_privilege(&privilege_policy));
    let packages = package_policy.status(&observed.observe_packages(&package_policy));
    let services = service_policy.status(&observed.observe_services(&service_policy));
    let blocked_domains = blocked_domain_statuses(
        &principal.status,
        &privilege.status,
        &packages.status,
        &services.status,
    );
    let status = if blocked_domains.is_empty() {
        SurfaceStatus::Active
    } else {
        SurfaceStatus::Blocked
    };

    HostObservationReports {
        schema: HOST_OBSERVATION_SCHEMA,
        host_resolution,
        status,
        blocked_domains,
        principal,
        privilege,
        packages,
        services,
    }
}

#[cfg(test)]
mod tests;
