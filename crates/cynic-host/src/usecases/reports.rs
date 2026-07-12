use serde::Serialize;

use crate::command_surface::HostCommandSurface;
use crate::domain::shared::SurfaceStatus;
use crate::registry::HostProfileResolution;

pub const HOST_OBSERVATION_SCHEMA: &str = "cynic.host_substrate.observation.v1";
pub const HOST_BUNDLE_SCHEMA: &str = "cynic.host_substrate.bundle.v1";
pub const HOST_ACTIVATION_PLAN_SCHEMA: &str = "cynic.host_substrate.activation_plan.v1";
pub const HOST_APPLY_REPORT_SCHEMA: &str = "cynic.host_substrate.apply_report.v1";
pub const HOST_APPLY_EXECUTION_SCHEMA: &str = "cynic.host_substrate.apply_execution.v1";

#[derive(Debug, Serialize)]
pub struct HostObservationReports {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalStatusReport,
    pub privilege: crate::domain::privilege::PrivilegeStatusReport,
    pub packages: crate::domain::packages::PackageStatusReport,
    pub services: crate::domain::services::ServiceStatusReport,
}

#[derive(Debug, Serialize)]
pub struct HostBundleReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalBundleReport,
    pub privilege: crate::domain::privilege::PrivilegeBundleReport,
    pub packages: crate::domain::packages::PackageBundleReport,
    pub services: crate::domain::services::ServiceBundleReport,
}

#[derive(Debug, Serialize)]
pub struct HostActivationPlanReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalActivationPlan,
    pub privilege: crate::domain::privilege::PrivilegeActivationPlan,
    pub packages: crate::domain::packages::PackageActivationPlan,
    pub services: crate::domain::services::ServiceActivationPlan,
}

#[derive(Debug, Serialize)]
pub struct HostApplyReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub principal: crate::domain::principal::PrincipalApplyReport,
    pub privilege: crate::domain::privilege::PrivilegeApplyReport,
    pub packages: crate::domain::packages::PackageApplyReport,
    pub services: crate::domain::services::ServiceApplyReport,
}

#[derive(Debug, Serialize)]
pub struct HostApplyExecutionReport {
    pub schema: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub status: SurfaceStatus,
    pub dry_run: bool,
    pub blocked_domains: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
    pub commands: Vec<ExecutedHostCommand>,
}

#[derive(Debug, Serialize)]
pub struct ExecutedHostCommand {
    pub domain: &'static str,
    pub command: String,
    pub executed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ExecutedHostCommandResult>,
}

#[derive(Debug, Serialize)]
pub struct ExecutedHostCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub(crate) fn rendered_dry_run_command(
    domain: &'static str,
    command: &HostCommandSurface,
) -> ExecutedHostCommand {
    ExecutedHostCommand {
        domain,
        command: crate::command_surface::render_command(command),
        executed: false,
        result: None,
    }
}
