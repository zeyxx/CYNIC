use crate::domain::shared::SubstrateDomainContract;
use crate::domain::shared::SurfaceStatus;
use crate::ports::{
    HostContractPort, HostPackageObservationPort, HostPrincipalObservationPort,
    HostPrivilegeObservationPort, HostServiceObservationPort,
};
use crate::registry::HostProfileResolution;

mod execution;
mod reports;
mod status;

pub use execution::execute_host_apply;
pub use reports::{
    ExecutedHostCommand, ExecutedHostCommandResult, HOST_ACTIVATION_PLAN_SCHEMA,
    HOST_APPLY_EXECUTION_SCHEMA, HOST_APPLY_REPORT_SCHEMA, HOST_BUNDLE_SCHEMA,
    HOST_OBSERVATION_SCHEMA, HostActivationPlanReport, HostApplyExecutionReport, HostApplyReport,
    HostBundleReport, HostObservationReports,
};
use status::{blocked_domain_statuses, blocked_domains, next_actions, overall_status};

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

pub fn build_host_bundle(port: &impl HostContractPort) -> HostBundleReport {
    build_host_bundle_with_resolution(port, None)
}

pub fn build_host_bundle_with_resolution(
    port: &impl HostContractPort,
    host_resolution: Option<HostProfileResolution>,
) -> HostBundleReport {
    HostBundleReport {
        schema: HOST_BUNDLE_SCHEMA,
        host_resolution,
        status: SurfaceStatus::Planned,
        blocked_domains: Vec::new(),
        principal: port.desired_principal().bundle(),
        privilege: port.desired_privilege().bundle(),
        packages: port.desired_packages().bundle(),
        services: port.desired_services().bundle(),
    }
}

pub fn build_host_activation_plan(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
) -> HostActivationPlanReport {
    build_host_activation_plan_with_resolution(port, observed, observed.host_resolution.clone())
}

pub fn build_host_activation_plan_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    host_resolution: Option<HostProfileResolution>,
) -> HostActivationPlanReport {
    let blocked_domains = blocked_domains(observed);
    HostActivationPlanReport {
        schema: HOST_ACTIVATION_PLAN_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port.desired_principal().activation_plan(),
        privilege: port.desired_privilege().activation_plan(),
        packages: port.desired_packages().activation_plan(),
        services: port.desired_services().activation_plan(),
    }
}

pub fn build_host_apply_report(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
) -> HostApplyReport {
    build_host_apply_report_with_resolution(
        port,
        observed,
        dry_run,
        observed.host_resolution.clone(),
    )
}

pub fn build_host_apply_report_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    host_resolution: Option<HostProfileResolution>,
) -> HostApplyReport {
    let blocked_domains = blocked_domains(observed);
    HostApplyReport {
        schema: HOST_APPLY_REPORT_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        dry_run,
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port.desired_principal().apply_report(dry_run),
        privilege: port.desired_privilege().apply_report(dry_run),
        packages: port.desired_packages().apply_report(dry_run),
        services: port.desired_services().apply_report(dry_run),
    }
}

#[cfg(test)]
mod tests;
