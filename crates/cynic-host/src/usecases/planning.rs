use crate::domain::shared::SurfaceStatus;
use crate::ports::HostContractPort;
use crate::registry::HostProfileResolution;

use super::command_context::HostCommandContext;
use super::reports::{
    HOST_ACTIVATION_PLAN_SCHEMA, HOST_APPLY_REPORT_SCHEMA, HOST_BUNDLE_SCHEMA,
    HostActivationPlanReport, HostApplyReport, HostBundleReport, HostObservationReports,
};
use super::status::{blocked_domains, next_actions, overall_status};

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
    build_host_activation_plan_with_context(
        port,
        observed,
        &HostCommandContext::from_current_runtime(),
    )
}

pub fn build_host_activation_plan_with_context(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    command_context: &HostCommandContext,
) -> HostActivationPlanReport {
    build_host_activation_plan_with_resolution_and_context(
        port,
        observed,
        observed.host_resolution.clone(),
        command_context,
    )
}

pub fn build_host_activation_plan_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    host_resolution: Option<HostProfileResolution>,
) -> HostActivationPlanReport {
    build_host_activation_plan_with_resolution_and_context(
        port,
        observed,
        host_resolution,
        &HostCommandContext::from_current_runtime(),
    )
}

pub fn build_host_activation_plan_with_resolution_and_context(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    host_resolution: Option<HostProfileResolution>,
    command_context: &HostCommandContext,
) -> HostActivationPlanReport {
    let blocked_domains = blocked_domains(observed);
    HostActivationPlanReport {
        schema: HOST_ACTIVATION_PLAN_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port
            .desired_principal()
            .activation_plan_with_context(&command_context.principal),
        privilege: port
            .desired_privilege()
            .activation_plan_with_context(&command_context.privilege),
        packages: port
            .desired_packages()
            .activation_plan_with_context(&command_context.packages),
        services: port
            .desired_services()
            .activation_plan_with_context(&command_context.services),
    }
}

pub fn build_host_apply_report(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
) -> HostApplyReport {
    build_host_apply_report_with_context(
        port,
        observed,
        dry_run,
        &HostCommandContext::from_current_runtime(),
    )
}

pub fn build_host_apply_report_with_context(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    command_context: &HostCommandContext,
) -> HostApplyReport {
    build_host_apply_report_with_resolution_and_context(
        port,
        observed,
        dry_run,
        observed.host_resolution.clone(),
        command_context,
    )
}

pub fn build_host_apply_report_with_resolution(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    host_resolution: Option<HostProfileResolution>,
) -> HostApplyReport {
    build_host_apply_report_with_resolution_and_context(
        port,
        observed,
        dry_run,
        host_resolution,
        &HostCommandContext::from_current_runtime(),
    )
}

pub fn build_host_apply_report_with_resolution_and_context(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    host_resolution: Option<HostProfileResolution>,
    command_context: &HostCommandContext,
) -> HostApplyReport {
    let blocked_domains = blocked_domains(observed);
    HostApplyReport {
        schema: HOST_APPLY_REPORT_SCHEMA,
        host_resolution,
        status: overall_status(observed),
        dry_run,
        next_actions: next_actions(&blocked_domains),
        blocked_domains,
        principal: port
            .desired_principal()
            .apply_report_with_context(dry_run, &command_context.principal),
        privilege: port
            .desired_privilege()
            .apply_report_with_context(dry_run, &command_context.privilege),
        packages: port
            .desired_packages()
            .apply_report_with_context(dry_run, &command_context.packages),
        services: port
            .desired_services()
            .apply_report_with_context(dry_run, &command_context.services),
    }
}
